use crate::camera::CityCamera;
use crate::osm::{OsmManager, OsmWaterway, RoadTier};
use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use status_4_roads::road::{build_road_mesh, create_road_texture, merge_road_meshes};
use status_4_roads::spline::{sample_spline, RoadWaypoint};
use status_4_roads::terrain::HeightmapData;
use std::collections::HashMap;
use std::time::Instant;

pub struct CityRoadsPlugin;

impl Plugin for CityRoadsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CityStats>()
            .add_systems(Startup, setup_city_environment)
            .add_systems(Update, rebuild_city_road_network_system);
    }
}

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct CityMeshMarker;

#[derive(Component)]
pub struct GroundMarker;

#[derive(Resource)]
pub struct GroundMapTexture {
    pub image_handle: Handle<Image>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct CityStats {
    pub total_roads: usize,
    pub total_waterways: usize,
    pub total_km: f32,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub generation_ms: u128,
}

#[derive(Resource)]
pub struct CityMaterials {
    pub motorway: Handle<StandardMaterial>,
    pub primary: Handle<StandardMaterial>,
    pub secondary: Handle<StandardMaterial>,
    pub residential: Handle<StandardMaterial>,
}

fn setup_city_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // 1. Spawning RTS/City Camera with Ambient Light
    commands.spawn((
        CityCamera::default(),
        AmbientLight {
            color: Color::srgb(0.72, 0.78, 0.88),
            brightness: 260.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2000.0, 2000.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 2. Sunlight / Directional Light with Shadows
    commands.spawn((
        DirectionalLight {
            illuminance: 36000.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.96, 0.90),
            ..default()
        },
        Transform::from_xyz(4000.0, 7000.0, 3000.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 4. Ground Plane (12km x 12km) at y = -0.15 so road deck crowns sit cleanly above
    let initial_map_img = generate_ground_map_image(&[], 1024, 1024);
    let ground_image_handle = images.add(initial_map_img);

    let ground_mesh = meshes.add(Plane3d::default().mesh().size(12000.0, 12000.0));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(ground_image_handle.clone()),
        perceptual_roughness: 0.96,
        metallic: 0.02,
        reflectance: 0.1,
        ..default()
    });
    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, -0.15, 0.0),
        GroundMarker,
    ));

    commands.insert_resource(GroundMapTexture {
        image_handle: ground_image_handle,
    });

    // 5. Tier-specific PBR road materials with procedural asphalt textures from roads
    let motorway_img = images.add(create_road_texture(15.0, 4));
    let primary_img = images.add(create_road_texture(11.0, 3));
    let secondary_img = images.add(create_road_texture(8.5, 2));
    let residential_img = images.add(create_road_texture(6.2, 2));

    let motorway_mat = materials.add(StandardMaterial {
        base_color_texture: Some(motorway_img),
        perceptual_roughness: 0.75,
        metallic: 0.0,
        reflectance: 0.2,
        cull_mode: None,
        ..default()
    });
    let primary_mat = materials.add(StandardMaterial {
        base_color_texture: Some(primary_img),
        perceptual_roughness: 0.78,
        metallic: 0.0,
        reflectance: 0.2,
        cull_mode: None,
        ..default()
    });
    let secondary_mat = materials.add(StandardMaterial {
        base_color_texture: Some(secondary_img),
        perceptual_roughness: 0.82,
        metallic: 0.0,
        reflectance: 0.18,
        cull_mode: None,
        ..default()
    });
    let residential_mat = materials.add(StandardMaterial {
        base_color_texture: Some(residential_img),
        perceptual_roughness: 0.85,
        metallic: 0.0,
        reflectance: 0.15,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(CityMaterials {
        motorway: motorway_mat,
        primary: primary_mat,
        secondary: secondary_mat,
        residential: residential_mat,
    });
}

fn rebuild_city_road_network_system(
    mut commands: Commands,
    mut manager: ResMut<OsmManager>,
    mut stats: ResMut<CityStats>,
    materials: Res<CityMaterials>,
    ground_map: Res<GroundMapTexture>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing_query: Query<Entity, With<CityMeshMarker>>,
) {
    if !manager.dirty {
        return;
    }
    manager.dirty = false;
    println!(
        "[rebuild_city_road_network_system] Dirty flag triggered! Rebuilding meshes for '{}' ({} roads, {} waterways)",
        manager.current_city_name,
        manager.roads.len(),
        manager.waterways.len()
    );

    // Update ground texture with waterway overlay
    if let Some(mut ground_img) = images.get_mut(&ground_map.image_handle) {
        *ground_img = generate_ground_map_image(&manager.waterways, 1024, 1024);
    }

    let start_instant = Instant::now();

    // Despawn previous road meshes
    for entity in &existing_query {
        commands.entity(entity).despawn();
    }

    if manager.roads.is_empty() {
        println!("[rebuild_city_road_network_system] Roads list is empty, cleared meshes.");
        *stats = CityStats::default();
        return;
    }

    // 10x10km flat heightmap (12000m map size)
    let heightmap = HeightmapData::flat(12000.0, 0.0);
    let sample_step = 10.0;

    // Group roads by tier for batch rendering
    let mut tier_roads: HashMap<RoadTier, Vec<&crate::osm::OsmRoad>> = HashMap::new();
    for road in &manager.roads {
        tier_roads.entry(road.tier).or_default().push(road);
    }

    let mut total_verts = 0;
    let mut total_indices = 0;
    let mut total_km = 0.0f32;

    for (tier, roads) in tier_roads {
        let mut tier_meshes = Vec::new();

        for road in roads {
            if road.points.len() < 2 {
                continue;
            }

            // Calculate length
            for i in 1..road.points.len() {
                total_km += (road.points[i] - road.points[i - 1]).length() / 1000.0;
            }

            // Convert to RoadWaypoints
            let waypoints: Vec<RoadWaypoint> = road
                .points
                .iter()
                .map(|p| RoadWaypoint::new(*p, road.width))
                .collect();

            let samples = sample_spline(&waypoints, sample_step, &heightmap);
            if samples.len() >= 2 {
                let road_mesh = build_road_mesh(&samples, &heightmap);
                tier_meshes.push(road_mesh);
            }
        }

        if tier_meshes.is_empty() {
            continue;
        }

        // Merge all road meshes in this tier into a single draw-call mesh
        let merged_mesh = merge_road_meshes(&tier_meshes);

        if let Some(pos) = merged_mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            total_verts += pos.len();
        }
        if let Some(indices) = merged_mesh.indices() {
            total_indices += indices.len();
        }

        let mat_handle = match tier {
            RoadTier::Motorway => materials.motorway.clone(),
            RoadTier::Primary => materials.primary.clone(),
            RoadTier::Secondary => materials.secondary.clone(),
            RoadTier::Residential => materials.residential.clone(),
        };

        commands.spawn((
            Mesh3d(meshes.add(merged_mesh)),
            MeshMaterial3d(mat_handle),
            CityMeshMarker,
        ));
    }

    let elapsed = start_instant.elapsed().as_millis();
    println!(
        "[rebuild_city_road_network_system] Mesh generation finished in {} ms ({} verts, {} tris, {:.1} km)",
        elapsed, total_verts, total_indices / 3, total_km
    );
    *stats = CityStats {
        total_roads: manager.roads.len(),
        total_waterways: manager.waterways.len(),
        total_km,
        total_vertices: total_verts,
        total_triangles: total_indices / 3,
        generation_ms: elapsed,
    };
}

const MAP_WIDTH_M: f32 = 12000.0;
const MAP_HALF_WIDTH_M: f32 = MAP_WIDTH_M / 2.0;

pub fn generate_ground_map_image(
    waterways: &[OsmWaterway],
    tex_w: u32,
    tex_h: u32,
) -> Image {
    let mut data = vec![0u8; (tex_w * tex_h * 4) as usize];

    // Background color: Dark slate (#171C24)
    let bg_color = [23, 28, 36, 255];
    let grid_color = [28, 34, 44, 255];

    // Fill background with subtle 1.5km grid lines
    let grid_step = (tex_w / 8).max(1);
    for y in 0..tex_h {
        let is_grid_y = y % grid_step == 0;
        for x in 0..tex_w {
            let idx = ((y * tex_w + x) * 4) as usize;
            let col = if is_grid_y || x % grid_step == 0 {
                grid_color
            } else {
                bg_color
            };
            data[idx..idx + 4].copy_from_slice(&col);
        }
    }

    // Waterway blue colors
    let water_core = [38, 128, 218, 255]; // Vibrant river blue
    let water_edge = [24, 82, 148, 255];  // Deeper shoreline blue

    for waterway in waterways {
        if waterway.points.len() < 2 {
            continue;
        }

        // Determine line stroke radius based on waterway width
        let radius = if waterway.width >= 45.0 {
            4i32 // Coastline / Bay / Ocean / Lake: ~9px wide
        } else if waterway.width >= 30.0 {
            3i32 // Major rivers: ~7px wide
        } else if waterway.width >= 15.0 {
            2i32 // Canals & medium streams: ~5px wide
        } else {
            1i32 // Minor streams & creeks: ~3px wide
        };

        for i in 1..waterway.points.len() {
            let p0 = waterway.points[i - 1];
            let p1 = waterway.points[i];

            let x0 = (p0.x + MAP_HALF_WIDTH_M) / MAP_WIDTH_M * (tex_w as f32);
            let y0 = (p0.z + MAP_HALF_WIDTH_M) / MAP_WIDTH_M * (tex_h as f32);
            let x1 = (p1.x + MAP_HALF_WIDTH_M) / MAP_WIDTH_M * (tex_w as f32);
            let y1 = (p1.z + MAP_HALF_WIDTH_M) / MAP_WIDTH_M * (tex_h as f32);

            rasterize_water_line(
                &mut data,
                tex_w,
                tex_h,
                x0,
                y0,
                x1,
                y1,
                radius,
                water_core,
                water_edge,
            );
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: tex_w,
            height: tex_h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::ClampToEdge,
        address_mode_v: bevy::image::ImageAddressMode::ClampToEdge,
        address_mode_w: bevy::image::ImageAddressMode::ClampToEdge,
        ..default()
    });

    image
}

fn rasterize_water_line(
    buffer: &mut [u8],
    w: u32,
    h: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    radius: i32,
    core_color: [u8; 4],
    edge_color: [u8; 4],
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = (dist * 1.5).ceil().max(1.0) as usize;
    let r_sq = radius * radius;

    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let cx = (x0 + t * dx).round() as i32;
        let cy = (y0 + t * dy).round() as i32;

        for ry in -radius..=radius {
            for rx in -radius..=radius {
                let dist_sq = rx * rx + ry * ry;
                if dist_sq <= r_sq {
                    let px = cx + rx;
                    let py = cy + ry;

                    if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                        let idx = ((py as usize * w as usize) + px as usize) * 4;
                        let col = if dist_sq == r_sq && radius > 1 {
                            edge_color
                        } else {
                            core_color
                        };
                        buffer[idx..idx + 4].copy_from_slice(&col);
                    }
                }
            }
        }
    }
}
