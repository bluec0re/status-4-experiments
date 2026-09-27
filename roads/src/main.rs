mod terrain;
mod camera;
mod spline;
mod road;
mod vegetation;
mod editor;
mod ui;

use bevy::prelude::*;
use camera::{update_rts_camera, RtsCamera};
use editor::{
    draw_editor_gizmos, handle_editor_input, load_preset, update_ride_along_camera,
    update_road_mesh_system, update_vegetation_system, EditorState, PostMaterialHandle,
    PostMeshHandle, PylonMaterialHandle, RoadMaterialHandle, TreeMaterialHandle, TreeMeshHandle,
};
use road::create_road_texture;
use terrain::{
    build_terrain_mesh, build_water_mesh, create_terrain_normal_texture,
    create_water_normal_texture, HeightmapData,
};
use ui::{handle_button_clicks, setup_ui, update_ui_system};
use vegetation::create_pine_tree_mesh;

#[derive(Component)]
pub struct WaterMarker;

fn main() {
    let heightmap = HeightmapData::load_or_generate();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Spline Road Editor - Realistic Landscape & Water".into(),
                resolution: (1600u32, 950u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(heightmap)
        .init_resource::<EditorState>()
        .add_systems(Startup, (setup_environment, setup_materials_and_assets, setup_ui))
        .add_systems(
            Update,
            (
                update_rts_camera,
                handle_editor_input,
                update_road_mesh_system,
                update_vegetation_system,
                update_ride_along_camera,
                animate_water_system,
                draw_editor_gizmos,
                update_ui_system,
                handle_button_clicks,
            ),
        )
        .run();
}

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    heightmap: Res<HeightmapData>,
) {
    // 1. Directional Sun Light (Realistic golden hour lighting with soft shadows)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.96, 0.88),
            illuminance: 18_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            42.0f32.to_radians(),
            -38.0f32.to_radians(),
            0.0,
        )),
    ));

    // 2. RTS 3D Camera with Ambient Lighting & Atmospheric Distance Fog
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 45.0, 60.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        RtsCamera::default(),
        AmbientLight {
            color: Color::srgb(0.72, 0.82, 0.95),
            brightness: 350.0,
            ..default()
        },
        DistanceFog {
            color: Color::srgba(0.72, 0.80, 0.90, 1.0),
            falloff: FogFalloff::ExponentialSquared { density: 0.0035 },
            ..default()
        },
    ));

    // 3. Texture-Based Landscape Mesh & Material
    let terrain_mesh = build_terrain_mesh(&heightmap);
    let terrain_mesh_handle = meshes.add(terrain_mesh);

    let terrain_normal_img = create_terrain_normal_texture(&heightmap);
    let terrain_normal_handle = images.add(terrain_normal_img);

    let terrain_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.88,
        metallic: 0.02,
        normal_map_texture: Some(terrain_normal_handle),
        reflectance: 0.15,
        ..default()
    });

    commands.spawn((
        Mesh3d(terrain_mesh_handle),
        MeshMaterial3d(terrain_mat),
        Transform::default(),
    ));

    // 4. Realistic Water Body Plane for Terrain Below WATER_THRESHOLD
    let water_mesh = build_water_mesh();
    let water_mesh_handle = meshes.add(water_mesh);

    let water_normal_img = create_water_normal_texture();
    let water_normal_handle = images.add(water_normal_img);

    let water_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.09, 0.26, 0.36, 0.84),
        perceptual_roughness: 0.06,
        metallic: 0.04,
        reflectance: 0.65,
        normal_map_texture: Some(water_normal_handle),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Mesh3d(water_mesh_handle),
        MeshMaterial3d(water_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
        WaterMarker,
    ));
}

/// Subtle realistic river current / wave ripple animation
fn animate_water_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<WaterMarker>>,
) {
    let t = time.elapsed_secs();
    for mut transform in query.iter_mut() {
        // Very subtle rhythmic breathing of the water surface
        transform.translation.y = (t * 1.2).sin() * 0.035;
    }
}

fn setup_materials_and_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    heightmap: Res<HeightmapData>,
    mut editor_state: ResMut<EditorState>,
) {
    // 1. Road PBR Material with custom procedural asphalt & marking texture
    let road_img = create_road_texture();
    let road_img_handle = images.add(road_img);

    let road_mat_handle = materials.add(StandardMaterial {
        base_color_texture: Some(road_img_handle),
        perceptual_roughness: 0.78,
        metallic: 0.0,
        reflectance: 0.2,
        cull_mode: None, // Double-sided rendering so embankment skirts look solid
        ..default()
    });
    commands.insert_resource(RoadMaterialHandle(road_mat_handle));

    // 2. Concrete Bridge Pylon Material
    let pylon_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.67, 0.65),
        perceptual_roughness: 0.85,
        metallic: 0.05,
        ..default()
    });
    commands.insert_resource(PylonMaterialHandle(pylon_mat_handle));

    // 3. Pine Tree Asset & Material
    let pine_mesh = create_pine_tree_mesh();
    let pine_mesh_handle = meshes.add(pine_mesh);
    commands.insert_resource(TreeMeshHandle(pine_mesh_handle));

    let tree_mat_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE, // Vertex colors supply the trunk and needle colors
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    commands.insert_resource(TreeMaterialHandle(tree_mat_handle));

    // 4. Roadside Delineator Post Mesh & Material
    let post_mesh_handle = meshes.add(Cuboid::new(0.12, 0.9, 0.12));
    commands.insert_resource(PostMeshHandle(post_mesh_handle));

    let post_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.92, 0.90),
        perceptual_roughness: 0.4,
        metallic: 0.1,
        ..default()
    });
    commands.insert_resource(PostMaterialHandle(post_mat_handle));

    // 5. Load initial scenic road preset
    load_preset(&mut editor_state, 1, &heightmap);
}
