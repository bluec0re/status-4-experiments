use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::path::Path;

pub const MAP_SIZE: f32 = 340.0;
pub const HALF_MAP: f32 = MAP_SIZE * 0.5;
pub const GRID_RES: usize = 170;

/// Altitude threshold below which terrain is covered by water (lakes, rivers, basins)
pub const WATER_THRESHOLD: f32 = 7.6;

pub const HEIGHTMAP_FILE: &str = "assets/heightmap.png";
pub const TEX_WIDTH: u32 = 512;
pub const TEX_HEIGHT: u32 = 512;
pub const MIN_ALTITUDE: f32 = 1.0;
pub const MAX_ALTITUDE: f32 = 24.0;

#[derive(Resource, Clone)]
pub struct HeightmapData {
    pub width: u32,
    pub height: u32,
    pub map_size: f32,
    pub heights: Vec<f32>,
}

impl HeightmapData {
    /// Loads heightmap from assets/heightmap.png or generates a new 16-bit PNG if missing
    pub fn load_or_generate() -> Self {
        let path = Path::new(HEIGHTMAP_FILE);
        if path.exists() {
            if let Ok(img) = image::open(path) {
                let gray = img.to_luma16();
                let (w, h) = gray.dimensions();
                let mut heights = Vec::with_capacity((w * h) as usize);
                for y in 0..h {
                    for x in 0..w {
                        let val = gray.get_pixel(x, y).0[0] as f32 / 65535.0;
                        let height_m = MIN_ALTITUDE + val * (MAX_ALTITUDE - MIN_ALTITUDE);
                        heights.push(height_m);
                    }
                }
                return Self {
                    width: w,
                    height: h,
                    map_size: MAP_SIZE,
                    heights,
                };
            }
        }

        Self::generate_and_save(path)
    }

    /// Generates a smooth countryside heightmap suitable for towns & cities,
    /// guaranteed C^2 smooth across all 4 quadrants without coordinate boundary artifacts.
    pub fn generate_and_save(path: &Path) -> Self {
        let mut img_buf = image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::new(TEX_WIDTH, TEX_HEIGHT);
        let mut heights = Vec::with_capacity((TEX_WIDTH * TEX_HEIGHT) as usize);

        for y in 0..TEX_HEIGHT {
            let v = y as f32 / (TEX_HEIGHT - 1) as f32;
            let world_z = -HALF_MAP + v * MAP_SIZE;

            for x in 0..TEX_WIDTH {
                let u = x as f32 / (TEX_WIDTH - 1) as f32;
                let world_x = -HALF_MAP + u * MAP_SIZE;

                let h_val = generate_smooth_landscape_math(world_x, world_z);
                heights.push(h_val);

                let norm_val = ((h_val - MIN_ALTITUDE) / (MAX_ALTITUDE - MIN_ALTITUDE)).clamp(0.0, 1.0);
                let u16_val = (norm_val * 65535.0) as u16;
                img_buf.put_pixel(x, y, image::Luma([u16_val]));
            }
        }

        let _ = img_buf.save(path);

        Self {
            width: TEX_WIDTH,
            height: TEX_HEIGHT,
            map_size: MAP_SIZE,
            heights,
        }
    }

    /// Continuous bilinear sampling from the heightmap texture
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let u = ((x + HALF_MAP) / self.map_size).clamp(0.0, 1.0);
        let v = ((z + HALF_MAP) / self.map_size).clamp(0.0, 1.0);

        let fx = u * (self.width - 1) as f32;
        let fz = v * (self.height - 1) as f32;

        let x0 = fx.floor() as u32;
        let z0 = fz.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let z1 = (z0 + 1).min(self.height - 1);

        let tx = fx - x0 as f32;
        let tz = fz - z0 as f32;

        let h00 = self.heights[(z0 * self.width + x0) as usize];
        let h10 = self.heights[(z0 * self.width + x1) as usize];
        let h01 = self.heights[(z1 * self.width + x0) as usize];
        let h11 = self.heights[(z1 * self.width + x1) as usize];

        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;

        h0 * (1.0 - tz) + h1 * tz
    }

    /// Analytical central difference normal from heightmap texture
    pub fn sample_normal(&self, x: f32, z: f32) -> Vec3 {
        let eps = 0.75;
        let h_l = self.sample(x - eps, z);
        let h_r = self.sample(x + eps, z);
        let h_d = self.sample(x, z - eps);
        let h_u = self.sample(x, z + eps);

        Vec3::new(h_l - h_r, 2.0 * eps, h_d - h_u).normalize()
    }

    /// Raycast against the texture-backed terrain surface
    pub fn raycast(&self, origin: Vec3, dir: Vec3) -> Option<Vec3> {
        if dir.y >= 0.0 && origin.y > 60.0 {
            return None;
        }

        let t_start = 0.0f32;
        let t_end = 650.0f32;
        let step = 1.4f32;
        let mut t = t_start;
        let mut prev_t = t;
        let mut hit = false;

        while t < t_end {
            let p = origin + dir * t;
            if p.x.abs() <= HALF_MAP + 10.0 && p.z.abs() <= HALF_MAP + 10.0 {
                let h = self.sample(p.x, p.z);
                if p.y <= h {
                    hit = true;
                    break;
                }
            }
            prev_t = t;
            t += step;
        }

        if !hit {
            return None;
        }

        // Binary search refinement
        let mut lo = prev_t;
        let mut hi = t;
        for _ in 0..12 {
            let mid = (lo + hi) * 0.5;
            let p = origin + dir * mid;
            let h = self.sample(p.x, p.z);
            if p.y <= h {
                hi = mid;
            } else {
                lo = mid;
            }
        }

        let final_t = (lo + hi) * 0.5;
        Some(origin + dir * final_t)
    }
}

// -------------------------------------------------------------------------------------------------
// High quality, C^2 smooth procedural noise functions
// Integer hashing ensures flawless continuity across negative coordinates in all quadrants!
// -------------------------------------------------------------------------------------------------

#[inline]
fn hash2d(ix: i32, iy: i32) -> u32 {
    let mut n = (ix as u32).wrapping_mul(0x9E3779B1) ^ (iy as u32).wrapping_mul(0x85EBCA6B);
    n = (n ^ (n >> 15)).wrapping_mul(0xC2B2AE35);
    n = (n ^ (n >> 13)).wrapping_mul(0x27D4EB2F);
    n ^ (n >> 16)
}

#[inline]
fn grad2d(ix: i32, iy: i32) -> Vec2 {
    let h = hash2d(ix, iy);
    let angle = (h & 0xFFFF) as f32 * (std::f32::consts::TAU / 65536.0);
    Vec2::new(angle.cos(), angle.sin())
}

/// 2D Perlin gradient noise with quintic Hermite smoothing (C^2 continuous everywhere)
pub fn perlin_noise(p: Vec2) -> f32 {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;

    // Fractional offset strictly in [0.0, 1.0)
    let fx = p.x - p.x.floor();
    let fy = p.y - p.y.floor();

    // Quintic polynomial: 6t^5 - 15t^4 + 10t^3
    let ux = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let uy = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);

    let g00 = grad2d(ix, iy);
    let g10 = grad2d(ix + 1, iy);
    let g01 = grad2d(ix, iy + 1);
    let g11 = grad2d(ix + 1, iy + 1);

    let v00 = g00.x * fx + g00.y * fy;
    let v10 = g10.x * (fx - 1.0) + g10.y * fy;
    let v01 = g01.x * fx + g01.y * (fy - 1.0);
    let v11 = g11.x * (fx - 1.0) + g11.y * (fy - 1.0);

    let v0 = v00 * (1.0 - ux) + v10 * ux;
    let v1 = v01 * (1.0 - ux) + v11 * ux;

    v0 * (1.0 - uy) + v1 * uy
}

pub fn generate_smooth_landscape_math(x: f32, z: f32) -> f32 {
    let p = Vec2::new(x, z);

    // 1. Broad rolling regional plains across all quadrants (wavelength ~250m)
    let regional_1 = perlin_noise(p * 0.004) * 6.5;

    // 2. Secondary gentle undulating swell (wavelength ~120m)
    let regional_2 = perlin_noise(p * 0.0085 + Vec2::new(125.4, -84.3)) * 3.2;

    // 3. Smooth gentle upper town plateau on North-East quadrant
    let plateau_dist = (x * 0.6 + z * 0.8 - 40.0) * 0.015;
    let plateau_factor = ((plateau_dist + 1.0) * 0.5).clamp(0.0, 1.0);
    let plateau_ease = plateau_factor * plateau_factor * (3.0 - 2.0 * plateau_factor);
    let plateau_elevation = plateau_ease * 6.0;

    // 4. Smooth countryside meadows in South-West and peripheral quadrants
    let hills = perlin_noise(p * 0.012 + Vec2::new(-63.2, 94.7)) * 2.2;

    // 5. Winding river valley basin through the landscape
    let river_x = (z * 0.016).sin() * 40.0 - 15.0;
    let river_dist = (x - river_x).abs();
    let river_width = 40.0;
    let river_factor = (river_dist / river_width).clamp(0.0, 1.0);
    let river_valley = (1.0 - river_factor * river_factor * (3.0 - 2.0 * river_factor)) * 4.0;

    // 6. Subtle micro terrain variation (wavelength ~35m)
    let micro = perlin_noise(p * 0.028 + Vec2::new(43.8, 17.2)) * 0.35;

    let base_y = 10.0;
    let total = base_y + regional_1 + regional_2 + hills + plateau_elevation - river_valley + micro;
    total.clamp(MIN_ALTITUDE, MAX_ALTITUDE)
}

/// Builds the 3D terrain mesh by sampling the HeightmapData texture, with wet riverbed shading
pub fn build_terrain_mesh(heightmap: &HeightmapData) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((GRID_RES + 1) * (GRID_RES + 1));
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((GRID_RES + 1) * (GRID_RES + 1));
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((GRID_RES + 1) * (GRID_RES + 1));
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity((GRID_RES + 1) * (GRID_RES + 1));
    let mut indices: Vec<u32> = Vec::with_capacity(GRID_RES * GRID_RES * 6);

    let step = MAP_SIZE / (GRID_RES as f32);

    for j in 0..=GRID_RES {
        let z = -HALF_MAP + (j as f32) * step;
        let v = (j as f32) / (GRID_RES as f32);

        for i in 0..=GRID_RES {
            let x = -HALF_MAP + (i as f32) * step;
            let u = (i as f32) / (GRID_RES as f32);

            let y = heightmap.sample(x, z);
            let n = heightmap.sample_normal(x, z);

            positions.push([x, y, z]);
            normals.push([n.x, n.y, n.z]);
            uvs.push([u * 28.0, v * 28.0]);

            // Realistic countryside coloring for cities and towns:
            let field_variation = ((x * 0.015).sin() * (z * 0.015).cos()) * 0.5 + 0.5;

            let meadow_green = [0.28, 0.40, 0.20, 1.0];
            let farmland_golden = [0.35, 0.42, 0.23, 1.0];
            let plateau_grass = [0.32, 0.44, 0.25, 1.0];
            let riverbank_soil = [0.44, 0.40, 0.30, 1.0];
            let riverbed_gravel = [0.25, 0.28, 0.24, 1.0];

            let mut col = [
                meadow_green[0] * (1.0 - field_variation) + farmland_golden[0] * field_variation,
                meadow_green[1] * (1.0 - field_variation) + farmland_golden[1] * field_variation,
                meadow_green[2] * (1.0 - field_variation) + farmland_golden[2] * field_variation,
                1.0,
            ];

            // Upper plateau coloring
            if y > 14.0 {
                let p_fac = ((y - 14.0) / 8.0).clamp(0.0, 1.0);
                col[0] = col[0] * (1.0 - p_fac) + plateau_grass[0] * p_fac;
                col[1] = col[1] * (1.0 - p_fac) + plateau_grass[1] * p_fac;
                col[2] = col[2] * (1.0 - p_fac) + plateau_grass[2] * p_fac;
            }

            // Water threshold shoreline & submerged riverbed shading
            if y < WATER_THRESHOLD + 1.2 {
                if y <= WATER_THRESHOLD {
                    // Submerged underwater riverbed
                    let depth_fac = ((WATER_THRESHOLD - y) / 2.5).clamp(0.0, 1.0);
                    col[0] = riverbank_soil[0] * (1.0 - depth_fac) + riverbed_gravel[0] * depth_fac;
                    col[1] = riverbank_soil[1] * (1.0 - depth_fac) + riverbed_gravel[1] * depth_fac;
                    col[2] = riverbank_soil[2] * (1.0 - depth_fac) + riverbed_gravel[2] * depth_fac;
                } else {
                    // Wet riverbank shoreline transition
                    let shore_fac = 1.0 - ((y - WATER_THRESHOLD) / 1.2);
                    col[0] = col[0] * (1.0 - shore_fac) + riverbank_soil[0] * shore_fac;
                    col[1] = col[1] * (1.0 - shore_fac) + riverbank_soil[1] * shore_fac;
                    col[2] = col[2] * (1.0 - shore_fac) + riverbank_soil[2] * shore_fac;
                }
            }

            colors.push(col);
        }
    }

    for j in 0..GRID_RES {
        for i in 0..GRID_RES {
            let row1 = (j * (GRID_RES + 1) + i) as u32;
            let row2 = ((j + 1) * (GRID_RES + 1) + i) as u32;

            indices.push(row1);
            indices.push(row2);
            indices.push(row1 + 1);

            indices.push(row1 + 1);
            indices.push(row2);
            indices.push(row2 + 1);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Generates procedural realistic terrain normal map for micro surface detail
pub fn create_terrain_normal_texture(heightmap: &HeightmapData) -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let v = y as f32 / size as f32;
        let wz = -HALF_MAP + v * MAP_SIZE;

        for x in 0..size {
            let u = x as f32 / size as f32;
            let wx = -HALF_MAP + u * MAP_SIZE;

            let norm = heightmap.sample_normal(wx, wz);
            let r = ((norm.x * 0.5 + 0.5) * 255.0) as u8;
            let g = ((norm.y * 0.5 + 0.5) * 255.0) as u8;
            let b = ((norm.z * 0.5 + 0.5) * 255.0) as u8;

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255);
        }
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    )
}

/// Builds the water plane mesh at WATER_THRESHOLD
pub fn build_water_mesh() -> Mesh {
    let res = 64;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((res + 1) * (res + 1));
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((res + 1) * (res + 1));
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((res + 1) * (res + 1));
    let mut indices: Vec<u32> = Vec::with_capacity(res * res * 6);

    let step = MAP_SIZE / (res as f32);

    for j in 0..=res {
        let z = -HALF_MAP + (j as f32) * step;
        let v = (j as f32) / (res as f32);

        for i in 0..=res {
            let x = -HALF_MAP + (i as f32) * step;
            let u = (i as f32) / (res as f32);

            positions.push([x, WATER_THRESHOLD, z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([u * 32.0, v * 32.0]);
        }
    }

    for j in 0..res {
        for i in 0..res {
            let row1 = (j * (res + 1) + i) as u32;
            let row2 = ((j + 1) * (res + 1) + i) as u32;

            indices.push(row1);
            indices.push(row2);
            indices.push(row1 + 1);

            indices.push(row1 + 1);
            indices.push(row2);
            indices.push(row2 + 1);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Generates a normal map for realistic animated water ripples
pub fn create_water_normal_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let v = y as f32 / size as f32 * std::f32::consts::TAU * 4.0;
        for x in 0..size {
            let u = x as f32 / size as f32 * std::f32::consts::TAU * 4.0;

            let nx = (u * 2.0).cos() * 0.15 + (u * 5.0 + v * 3.0).sin() * 0.1;
            let nz = (v * 2.5).sin() * 0.15 + (v * 4.0 - u * 2.0).cos() * 0.1;
            let ny = (1.0 - (nx * nx + nz * nz).min(0.9)).sqrt();

            let norm = Vec3::new(nx, ny, nz).normalize();
            let r = ((norm.x * 0.5 + 0.5) * 255.0) as u8;
            let g = ((norm.y * 0.5 + 0.5) * 255.0) as u8;
            let b = ((norm.z * 0.5 + 0.5) * 255.0) as u8;

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255);
        }
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heightmap_texture_load_and_sample() {
        let hm = HeightmapData::load_or_generate();
        assert_eq!(hm.width, TEX_WIDTH);
        assert_eq!(hm.height, TEX_HEIGHT);
        let h = hm.sample(0.0, 0.0);
        assert!(h >= MIN_ALTITUDE && h <= MAX_ALTITUDE);

        let mut min_h = 999.0f32;
        let mut max_h = -999.0f32;
        let mut underwater_count = 0usize;
        for &val in &hm.heights {
            if val < min_h { min_h = val; }
            if val > max_h { max_h = val; }
            if val < WATER_THRESHOLD { underwater_count += 1; }
        }
        let total_count = hm.heights.len();
        let underwater_pct = underwater_count as f32 / total_count as f32 * 100.0;
        println!("Heightmap bounds: min = {:.2}m, max = {:.2}m", min_h, max_h);
        println!("Underwater coverage at threshold {:.2}m: {:.1}%", WATER_THRESHOLD, underwater_pct);
    }
}
