#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crate::house::EuropeanStyle;

/// Procedural material generator and asset cache for European building textures
#[derive(Resource)]
pub struct BuildingMaterials {
    pub ground_wall_rusticated: Handle<StandardMaterial>,
    pub upper_wall_stucco: Handle<StandardMaterial>,
    pub upper_wall_brick: Handle<StandardMaterial>,
    pub roof_zinc: Handle<StandardMaterial>,
    pub roof_terracotta: Handle<StandardMaterial>,
    pub roof_slate: Handle<StandardMaterial>,
    pub floor_parquet: Handle<StandardMaterial>,
    pub floor_marble_tile: Handle<StandardMaterial>,
    pub interior_wall: Handle<StandardMaterial>,
    pub window_frame: Handle<StandardMaterial>,
    pub window_glass: Handle<StandardMaterial>,
    pub door_wood: Handle<StandardMaterial>,
    pub metal_trim: Handle<StandardMaterial>,
    pub road_asphalt: Handle<StandardMaterial>,
    pub sidewalk_stone: Handle<StandardMaterial>,
}

impl FromWorld for BuildingMaterials {
    fn from_world(world: &mut World) -> Self {
        let (rusticated_img, stucco_img, brick_img, zinc_img, terracotta_img, slate_img, parquet_img, marble_img) = {
            let mut images = world.resource_mut::<Assets<Image>>();
            (
                images.add(create_rusticated_stone_texture()),
                images.add(create_stucco_texture([235, 228, 215, 255])),
                images.add(create_brick_texture()),
                images.add(create_zinc_roof_texture()),
                images.add(create_terracotta_roof_texture()),
                images.add(create_slate_roof_texture()),
                images.add(create_parquet_floor_texture()),
                images.add(create_marble_checker_texture()),
            )
        };

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();

        let ground_wall_rusticated = materials.add(StandardMaterial {
            base_color_texture: Some(rusticated_img),
            perceptual_roughness: 0.85,
            metallic: 0.05,
            ..default()
        });

        let upper_wall_stucco = materials.add(StandardMaterial {
            base_color_texture: Some(stucco_img),
            perceptual_roughness: 0.75,
            metallic: 0.02,
            ..default()
        });

        let upper_wall_brick = materials.add(StandardMaterial {
            base_color_texture: Some(brick_img),
            perceptual_roughness: 0.9,
            metallic: 0.02,
            ..default()
        });

        let roof_zinc = materials.add(StandardMaterial {
            base_color_texture: Some(zinc_img),
            perceptual_roughness: 0.35,
            metallic: 0.75,
            ..default()
        });

        let roof_terracotta = materials.add(StandardMaterial {
            base_color_texture: Some(terracotta_img),
            perceptual_roughness: 0.85,
            metallic: 0.05,
            ..default()
        });

        let roof_slate = materials.add(StandardMaterial {
            base_color_texture: Some(slate_img),
            perceptual_roughness: 0.65,
            metallic: 0.15,
            ..default()
        });

        let floor_parquet = materials.add(StandardMaterial {
            base_color_texture: Some(parquet_img),
            perceptual_roughness: 0.45,
            metallic: 0.05,
            ..default()
        });

        let floor_marble_tile = materials.add(StandardMaterial {
            base_color_texture: Some(marble_img),
            perceptual_roughness: 0.25,
            metallic: 0.1,
            ..default()
        });

        let interior_wall = materials.add(StandardMaterial {
            base_color: Color::srgb(0.92, 0.91, 0.88),
            perceptual_roughness: 0.85,
            metallic: 0.0,
            ..default()
        });

        let window_frame = materials.add(StandardMaterial {
            base_color: Color::srgb(0.20, 0.22, 0.24),
            perceptual_roughness: 0.5,
            metallic: 0.1,
            ..default()
        });

        let window_glass = materials.add(StandardMaterial {
            base_color: Color::srgba(0.35, 0.55, 0.75, 0.65),
            perceptual_roughness: 0.1,
            metallic: 0.8,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        let door_wood = materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.16, 0.10),
            perceptual_roughness: 0.4,
            metallic: 0.1,
            ..default()
        });

        let metal_trim = materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.15, 0.18),
            perceptual_roughness: 0.35,
            metallic: 0.85,
            ..default()
        });

        let road_asphalt = materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.19, 0.21),
            perceptual_roughness: 0.9,
            metallic: 0.05,
            ..default()
        });

        let sidewalk_stone = materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.44, 0.42),
            perceptual_roughness: 0.8,
            metallic: 0.05,
            ..default()
        });

        Self {
            ground_wall_rusticated,
            upper_wall_stucco,
            upper_wall_brick,
            roof_zinc,
            roof_terracotta,
            roof_slate,
            floor_parquet,
            floor_marble_tile,
            interior_wall,
            window_frame,
            window_glass,
            door_wood,
            metal_trim,
            road_asphalt,
            sidewalk_stone,
        }
    }
}

impl BuildingMaterials {
    /// Select facade wall material based on style and story level
    pub fn wall_material_for_story(&self, style: EuropeanStyle, story: u32) -> Handle<StandardMaterial> {
        if story == 0 {
            // Ground floor European rusticated stone base
            self.ground_wall_rusticated.clone()
        } else {
            match style {
                EuropeanStyle::AmsterdamCanal => self.upper_wall_brick.clone(),
                _ => self.upper_wall_stucco.clone(),
            }
        }
    }

    /// Select roof material based on style
    pub fn roof_material_for_style(&self, style: EuropeanStyle) -> Handle<StandardMaterial> {
        match style {
            EuropeanStyle::Haussmannian => self.roof_zinc.clone(),
            EuropeanStyle::BerlinAltbau => self.roof_slate.clone(),
            EuropeanStyle::VienneseNeoclassical => self.roof_terracotta.clone(),
            EuropeanStyle::AmsterdamCanal => self.roof_terracotta.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// PROCEDURAL PIXEL TEXTURE SYNTHESIS (256x256 repeating maps)
// ---------------------------------------------------------------------------

fn make_rgba_image(width: u32, height: u32, data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Ashlar rusticated stone with deep horizontal grooves and subtle mortar joints
fn create_rusticated_stone_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        // Horizontal rustication grooves every 32 pixels
        let is_groove = (y % 32) < 4;
        for x in 0..size {
            let row = y / 32;
            let stagger = (row % 2) * 64;
            let is_vertical_joint = ((x + stagger) % 128) < 3;

            let noise = ((x * 17 + y * 31) % 19) as i32 - 9;
            if is_groove || (is_vertical_joint && (y % 32) >= 4) {
                // Dark shadowed mortar groove
                data.extend_from_slice(&[110, 105, 98, 255]);
            } else {
                // Warm Parisian limestone block
                let base = 215 + noise;
                let r = base.clamp(0, 255) as u8;
                let g = (base - 10).clamp(0, 255) as u8;
                let b = (base - 22).clamp(0, 255) as u8;
                data.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    make_rgba_image(size, size, data)
}

/// Smooth European plaster/stucco with subtle mineral grain
fn create_stucco_texture(tint: [u8; 4]) -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            let n = ((x * 37 + y * 73 + ((x ^ y) * 19)) % 15) as i32 - 7;
            let r = (tint[0] as i32 + n).clamp(0, 255) as u8;
            let g = (tint[1] as i32 + n).clamp(0, 255) as u8;
            let b = (tint[2] as i32 + n).clamp(0, 255) as u8;
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    make_rgba_image(size, size, data)
}

/// Flemish bond red brickwork
fn create_brick_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let row = y / 16;
        let is_mortar_h = (y % 16) < 3;
        let stagger = (row % 2) * 32;

        for x in 0..size {
            let is_mortar_v = ((x + stagger) % 64) < 3;
            if is_mortar_h || is_mortar_v {
                // Light grey mortar
                data.extend_from_slice(&[190, 185, 180, 255]);
            } else {
                // Varied terracotta/red Dutch brick tone
                let brick_idx = ((x + stagger) / 64) + row * 11;
                let variation = ((brick_idx * 43) % 25) as i32 - 12;
                let noise = ((x * 13 + y * 29) % 11) as i32 - 5;
                let r = (165 + variation + noise).clamp(0, 255) as u8;
                let g = (72 + (variation / 2) + noise).clamp(0, 255) as u8;
                let b = (52 + (variation / 3) + noise).clamp(0, 255) as u8;
                data.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    make_rgba_image(size, size, data)
}

/// Parisian standing-seam zinc roof sheet (blue-grey metallic with parallel seams)
fn create_zinc_roof_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            let is_seam = (x % 32) < 3;
            let grain = ((x * 19 + y * 23) % 9) as i32 - 4;
            if is_seam {
                // Raised standing seam shadow & highlight
                if (x % 32) == 0 {
                    data.extend_from_slice(&[180, 195, 210, 255]); // highlight
                } else {
                    data.extend_from_slice(&[75, 85, 95, 255]); // shadow
                }
            } else {
                let base = 125 + grain;
                let r = (base - 10).clamp(0, 255) as u8;
                let g = (base).clamp(0, 255) as u8;
                let b = (base + 15).clamp(0, 255) as u8; // soft blue zinc tint
                data.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    make_rgba_image(size, size, data)
}

/// European barrel clay terracotta tiles
fn create_terracotta_roof_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let row = y / 20;
        let is_lap = (y % 20) < 3;
        let stagger = (row % 2) * 16;

        for x in 0..size {
            let tile_x = (x + stagger) % 32;
            let curve = ((tile_x as f32 - 16.0) / 16.0).powi(2);
            let shade = (curve * 30.0) as i32;

            if is_lap {
                data.extend_from_slice(&[90, 40, 25, 255]); // overlap shadow
            } else {
                let r = (195 - shade).clamp(0, 255) as u8;
                let g = (85 - shade / 2).clamp(0, 255) as u8;
                let b = (45 - shade / 3).clamp(0, 255) as u8;
                data.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    make_rgba_image(size, size, data)
}

/// Dark grey/black slate roof shingles
fn create_slate_roof_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let row = y / 24;
        let is_course = (y % 24) < 3;
        let stagger = (row % 2) * 24;

        for x in 0..size {
            let is_joint = ((x + stagger) % 48) < 2;
            if is_course || is_joint {
                data.extend_from_slice(&[30, 32, 36, 255]); // joint shadow
            } else {
                let n = ((x * 23 + y * 41) % 17) as i32 - 8;
                let val = (65 + n).clamp(0, 255) as u8;
                data.extend_from_slice(&[val, val + 2, val + 6, 255]);
            }
        }
    }
    make_rgba_image(size, size, data)
}

/// Hardwood herringbone / chevron oak parquet flooring
fn create_parquet_floor_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let iy = y as i32;
        for x in 0..size {
            let ix = x as i32;
            // Herringbone pattern diagonal stripes
            let band = ((ix + iy) / 16).rem_euclid(2);
            let plank = if band == 0 { (ix - iy) / 48 } else { (iy - ix) / 48 };
            let wood_grain = ((ix * 7 + iy * 13).rem_euclid(11)) - 5;
            let plank_tint = ((plank * 37).rem_euclid(21)) - 10;

            let is_seam = ((ix + iy) % 16) == 0;
            if is_seam {
                data.extend_from_slice(&[70, 45, 25, 255]);
            } else {
                let r = (185 + plank_tint + wood_grain).clamp(0, 255) as u8;
                let g = (135 + plank_tint + wood_grain).clamp(0, 255) as u8;
                let b = (85 + plank_tint / 2 + wood_grain).clamp(0, 255) as u8;
                data.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    make_rgba_image(size, size, data)
}

/// Black and white marble checkerboard for lobbies & hallways
fn create_marble_checker_texture() -> Image {
    let size = 256;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let tile_y = y / 64;
        let is_grout_y = (y % 64) < 2;

        for x in 0..size {
            let tile_x = x / 64;
            let is_grout_x = (x % 64) < 2;

            if is_grout_x || is_grout_y {
                data.extend_from_slice(&[100, 100, 100, 255]);
            } else {
                let is_white = (tile_x + tile_y) % 2 == 0;
                let vein = ((x * 19 + y * 29) % 13) as i32;
                if is_white {
                    let v = (240 - vein * 2).clamp(0, 255) as u8;
                    data.extend_from_slice(&[v, v, v - 2, 255]);
                } else {
                    let v = (35 + vein * 2).clamp(0, 255) as u8;
                    data.extend_from_slice(&[v, v + 2, v + 4, 255]);
                }
            }
        }
    }
    make_rgba_image(size, size, data)
}
