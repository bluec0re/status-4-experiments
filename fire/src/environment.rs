#![allow(dead_code)]

use bevy::prelude::*;
use crate::fire_sim::{EnvironmentConditions, TimeOfDay};

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.07)))
            .add_systems(Startup, setup_environment)
            .add_systems(Update, (update_lighting_for_time_of_day, update_wind_visualizer));
    }
}

/// Marker for the primary sun/moon directional light
#[derive(Component)]
pub struct SunLightMarker;

/// Marker for the ambient fill light
#[derive(Component)]
pub struct FillLightMarker;

/// Marker for the ground terrain mesh
#[derive(Component)]
pub struct GroundTerrainMarker;

/// Marker for 3D wind indicator arrow
#[derive(Component)]
pub struct WindVisualizerMarker;

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
) {
    ambient_light.color = Color::srgb(0.15, 0.18, 0.28);
    ambient_light.brightness = 60.0;

    // 1. Primary Directional Light (Sun / Moon)
    commands.spawn((
        SunLightMarker,
        DirectionalLight {
            color: Color::srgb(0.75, 0.85, 1.0),
            illuminance: 1_200.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            40.0f32.to_radians(),
            -45.0f32.to_radians(),
            0.0,
        )),
    ));

    // 2. Base Ground Terrain Plane (160x160m)
    let ground_mesh = meshes.add(Plane3d::default().mesh().size(180.0, 180.0));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.14, 0.17, 0.15),
        perceptual_roughness: 0.9,
        metallic: 0.05,
        ..default()
    });

    commands.spawn((
        GroundTerrainMarker,
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));

    // 3. 3D Floating Wind Indicator Arrow at scene corner
    let arrow_mesh = meshes.add(Cylinder::new(0.15, 4.0));
    let arrow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.8, 1.0),
        emissive: LinearRgba::from(Color::srgb(0.1, 0.7, 1.0)) * 1.5,
        unlit: true,
        ..default()
    });

    commands.spawn((
        WindVisualizerMarker,
        Mesh3d(arrow_mesh),
        MeshMaterial3d(arrow_mat),
        Transform::from_xyz(-35.0, 12.0, -35.0)
            .with_rotation(Quat::from_rotation_z(90.0f32.to_radians())),
    ));
}

/// Dynamically adjust ambient, directional sun, and sky clear color when time of day changes
fn update_lighting_for_time_of_day(
    env: Res<EnvironmentConditions>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    mut sun_query: Query<&mut DirectionalLight, With<SunLightMarker>>,
) {
    if !env.is_changed() {
        return;
    }

    let Ok(mut sun) = sun_query.single_mut() else {
        return;
    };

    match env.time_of_day {
        TimeOfDay::Day => {
            clear_color.0 = Color::srgb(0.15, 0.28, 0.45);
            ambient_light.color = Color::srgb(0.85, 0.90, 1.0);
            ambient_light.brightness = 450.0;

            sun.color = Color::srgb(1.0, 0.98, 0.94);
            sun.illuminance = 18_000.0;
        }
        TimeOfDay::Sunset => {
            clear_color.0 = Color::srgb(0.18, 0.10, 0.15);
            ambient_light.color = Color::srgb(0.85, 0.55, 0.40);
            ambient_light.brightness = 220.0;

            sun.color = Color::srgb(1.0, 0.60, 0.35);
            sun.illuminance = 7_500.0;
        }
        TimeOfDay::Night => {
            clear_color.0 = Color::srgb(0.015, 0.02, 0.04);
            ambient_light.color = Color::srgb(0.12, 0.16, 0.28);
            ambient_light.brightness = 50.0;

            sun.color = Color::srgb(0.65, 0.78, 1.0); // Soft moonlit blue
            sun.illuminance = 700.0;
        }
    }
}

/// Rotate and scale the 3D wind indicator according to active wind conditions
fn update_wind_visualizer(
    env: Res<EnvironmentConditions>,
    mut query: Query<&mut Transform, With<WindVisualizerMarker>>,
) {
    let Ok(mut trans) = query.single_mut() else {
        return;
    };

    let angle = (-env.wind_direction.x).atan2(env.wind_direction.y);
    trans.rotation = Quat::from_rotation_y(angle) * Quat::from_rotation_x(90.0f32.to_radians());
    let length_scale = (env.wind_speed / 8.0).clamp(0.6, 2.5);
    trans.scale = Vec3::new(1.0, length_scale, 1.0);
}
