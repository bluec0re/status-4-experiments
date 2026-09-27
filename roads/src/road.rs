use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crate::spline::SplineSample;
use crate::terrain::HeightmapData;

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct RoadMeshMarker;

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct RoadPylonMarker;

/// Builds the 3D road ribbon mesh along the spline samples,
/// conforming strictly to the terrain texture to eliminate ground clipping.
pub fn build_road_mesh(samples: &[SplineSample], heightmap: &HeightmapData) -> Mesh {
    if samples.len() < 2 {
        return Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    }

    // Cross-section has 9 points across:
    // 0: Left underground skirt anchor (buried -0.8m into terrain)
    // 1: Left surface embankment verge
    // 2: Left gravel shoulder
    // 3: Left asphalt edge
    // 4: Center crown
    // 5: Right asphalt edge
    // 6: Right gravel shoulder
    // 7: Right surface embankment verge
    // 8: Right underground skirt anchor (buried -0.8m into terrain)
    let num_slices = samples.len();
    let num_verts_per_slice = 9;
    let total_verts = num_slices * num_verts_per_slice;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_verts);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(total_verts);
    let mut indices: Vec<u32> = Vec::with_capacity((num_slices - 1) * 6 * (num_verts_per_slice - 1));

    let v_scale = 0.25; // Repeats texture every 4 meters

    for s in samples.iter() {
        let center = s.pos;
        let right = s.normal;
        let up = s.binormal;
        let half_w = s.width * 0.5;
        let shoulder_w = 1.0;
        let embankment_w = 2.8;

        let v = s.distance * v_scale;

        // Elevation difference relative to ground
        let center_ground_y = heightmap.sample(center.x, center.z);
        let is_bridge = (center.y - center_ground_y) > 2.8;

        // Slice profiles:
        // Position offsets: (lateral_offset, height_offset, u_coord, vertex_color)
        let pts: [(f32, f32, f32, [f32; 4]); 9] = if is_bridge {
            // Elevated viaduct bridge profile with safety parapets/railings
            [
                (-(half_w + 0.4), -1.2, 0.0,  [0.35, 0.35, 0.38, 1.0]), // Bridge bottom-left
                (-(half_w + 0.4), -0.6, 0.02, [0.45, 0.45, 0.48, 1.0]), // Bridge girder-left
                (-(half_w + 0.3),  0.4, 0.05, [0.72, 0.72, 0.76, 1.0]), // Railing top-left
                (-half_w,          0.0, 0.12, [0.35, 0.35, 0.38, 1.0]), // Deck road left
                ( 0.0,             0.04, 0.5, [0.35, 0.35, 0.38, 1.0]), // Crown center
                ( half_w,          0.0, 0.88, [0.35, 0.35, 0.38, 1.0]), // Deck road right
                ( half_w + 0.3,    0.4, 0.95, [0.72, 0.72, 0.76, 1.0]), // Railing top-right
                ( half_w + 0.4,   -0.6, 0.98, [0.45, 0.45, 0.48, 1.0]), // Bridge girder-right
                ( half_w + 0.4,   -1.2, 1.0,  [0.35, 0.35, 0.38, 1.0]), // Bridge bottom-right
            ]
        } else {
            // Embankment cross-section smoothly blended into hillside
            [
                (-(half_w + shoulder_w + embankment_w + 0.4), 0.0, 0.0,  [0.26, 0.32, 0.20, 1.0]), // Subterranean left
                (-(half_w + shoulder_w + embankment_w),       0.0, 0.02, [0.28, 0.36, 0.22, 1.0]), // Surface embankment left
                (-(half_w + shoulder_w),                     -0.05, 0.08, [0.42, 0.40, 0.36, 1.0]), // Shoulder left
                (-half_w,                                     0.0, 0.15, [0.32, 0.32, 0.34, 1.0]), // Road edge left
                ( 0.0,                                        0.05, 0.5,  [0.32, 0.32, 0.34, 1.0]), // Crown center
                ( half_w,                                     0.0, 0.85, [0.32, 0.32, 0.34, 1.0]), // Road edge right
                ( half_w + shoulder_w,                       -0.05, 0.92, [0.42, 0.40, 0.36, 1.0]), // Shoulder right
                ( half_w + shoulder_w + embankment_w,         0.0, 0.98, [0.28, 0.36, 0.22, 1.0]), // Surface embankment right
                ( half_w + shoulder_w + embankment_w + 0.4,   0.0, 1.0,  [0.26, 0.32, 0.20, 1.0]), // Subterranean right
            ]
        };

        for (idx, &(lat_off, h_off, u_coord, vertex_color)) in pts.iter().enumerate() {
            let mut pt = center + right * lat_off + up * h_off;

            if !is_bridge {
                let local_ground = heightmap.sample(pt.x, pt.z);
                match idx {
                    0 | 8 => {
                        // Subterranean anchors: bury deep into the earth to prevent gaps
                        pt.y = local_ground - 0.75;
                    }
                    1 | 7 => {
                        // Surface embankment edge: snap directly into ground
                        pt.y = local_ground - 0.03;
                    }
                    2 | 6 => {
                        // Gravel shoulder: sit slightly above ground
                        pt.y = pt.y.max(local_ground + 0.06);
                    }
                    3 | 5 => {
                        // Road edges: guaranteed clearance
                        pt.y = pt.y.max(local_ground + 0.12);
                    }
                    4 => {
                        // Center crown: maximum clearance
                        pt.y = pt.y.max(local_ground + 0.16);
                    }
                    _ => {}
                }
            }

            positions.push([pt.x, pt.y, pt.z]);
            normals.push([up.x, up.y, up.z]);
            uvs.push([u_coord, v]);
            colors.push(vertex_color);
        }
    }

    // Build triangle index grid
    for i in 0..(num_slices - 1) {
        let row_curr = (i * num_verts_per_slice) as u32;
        let row_next = ((i + 1) * num_verts_per_slice) as u32;

        for j in 0..(num_verts_per_slice - 1) {
            let j_u32 = j as u32;
            let c0 = row_curr + j_u32;
            let c1 = row_next + j_u32;
            let c2 = row_curr + j_u32 + 1;
            let c3 = row_next + j_u32 + 1;

            indices.push(c0);
            indices.push(c1);
            indices.push(c2);

            indices.push(c2);
            indices.push(c1);
            indices.push(c3);
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

/// Generates bridge pillar positions where road is elevated high above terrain
pub fn generate_bridge_pylons(samples: &[SplineSample], heightmap: &HeightmapData) -> Vec<(Vec3, f32)> {
    let mut pylons = Vec::new();
    let min_spacing = 18.0;
    let mut last_pylon_dist = -100.0;

    for s in samples.iter() {
        let ground_y = heightmap.sample(s.pos.x, s.pos.z);
        let height_above_ground = s.pos.y - ground_y;

        if height_above_ground > 3.0 && (s.distance - last_pylon_dist) >= min_spacing {
            let base_pos = Vec3::new(s.pos.x, ground_y + height_above_ground * 0.5, s.pos.z);
            pylons.push((base_pos, height_above_ground));
            last_pylon_dist = s.distance;
        }
    }

    pylons
}

/// Generates roadside delineators / reflectors along sharp curves
pub fn generate_road_posts(samples: &[SplineSample]) -> Vec<(Vec3, Quat)> {
    let mut posts = Vec::new();
    let step = 10.0;
    let mut next_dist = 5.0;

    for s in samples.iter() {
        if s.distance >= next_dist {
            let half_w = s.width * 0.5 + 0.5;
            let post_pos_left = s.pos - s.normal * half_w;
            let post_pos_right = s.pos + s.normal * half_w;

            let rot = Quat::from_rotation_arc(Vec3::Z, s.tangent);
            posts.push((post_pos_left, rot));
            posts.push((post_pos_right, rot));

            next_dist += step;
        }
    }

    posts
}

/// Generates realistic procedural PBR road asphalt texture with lane stripes
pub fn create_road_texture() -> Image {
    let width = 512;
    let height = 512;
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        let v = y as f32 / height as f32;
        // Seamless periodic repeat of dashes along V (exactly 4 complete cycles per texture height)
        let is_dash_white = (v * 4.0).fract() < 0.55;

        for x in 0..width {
            let u = x as f32 / width as f32;

            let noise_val = (((x * 97 + y * 131) ^ (x * 17)) & 0x1F) as f32 / 31.0;
            let mut r = 42.0 + noise_val * 14.0;
            let mut g = 42.0 + noise_val * 14.0;
            let mut b = 45.0 + noise_val * 14.0;

            // Left & Right solid edge lines
            if (u >= 0.15 && u <= 0.175) || (u >= 0.825 && u <= 0.85) {
                let edge_noise = (noise_val - 0.5) * 20.0;
                r = (220.0 + edge_noise).clamp(160.0, 245.0);
                g = (220.0 + edge_noise).clamp(160.0, 245.0);
                b = (215.0 + edge_noise).clamp(160.0, 240.0);
            }
            // Center dashed line
            else if u >= 0.485 && u <= 0.515 && is_dash_white {
                let line_noise = (noise_val - 0.5) * 15.0;
                r = (240.0 + line_noise).clamp(180.0, 255.0);
                g = (205.0 + line_noise).clamp(160.0, 230.0);
                b = (40.0 + line_noise).clamp(30.0, 80.0);
            }
            // Wheel tracks subtle darkening
            else if (u >= 0.25 && u <= 0.40) || (u >= 0.60 && u <= 0.75) {
                r *= 0.86;
                g *= 0.86;
                b *= 0.88;
            }
            // Shoulders gravel
            else if u < 0.15 || u > 0.85 {
                let gravel_noise = (((x * 67 + y * 43) * 73) & 0x3F) as f32 / 63.0;
                r = 85.0 + gravel_noise * 30.0;
                g = 80.0 + gravel_noise * 25.0;
                b = 70.0 + gravel_noise * 25.0;
            }

            data.push(r as u8);
            data.push(g as u8);
            data.push(b as u8);
            data.push(255);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    // CRITICAL: Set sampler addressing to Repeat so texture tiles seamlessly along the spline V axis!
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..default()
    });

    image
}
