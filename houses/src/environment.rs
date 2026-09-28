use bevy::prelude::*;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.06, 0.09, 0.14)))
            .add_systems(Startup, setup_environment);
    }
}

/// Marker component for the base lot plane
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct GroundLotMarker;

/// Marker component for the lot boundary visualization
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct LotBoundaryMarker;

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
) {
    ambient_light.color = Color::srgb(0.85, 0.90, 1.0);
    ambient_light.brightness = 400.0;
    // 1. Sun Directional Light with soft shadows
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.97, 0.92),
            illuminance: 16_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            45.0f32.to_radians(),
            -40.0f32.to_radians(),
            0.0,
        )),
    ));

    // 2. Secondary soft fill light from opposite angle
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.7, 0.8, 0.95),
            illuminance: 4_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            -135.0f32.to_radians(),
            -25.0f32.to_radians(),
            0.0,
        )),
    ));

    // 3. Ground Base Landscape Plane (60x60m subtle yard)
    let ground_mesh = meshes.add(Plane3d::default().mesh().size(60.0, 60.0));
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.16, 0.13), // deep muted lawn tone
        perceptual_roughness: 0.9,
        metallic: 0.05,
        ..default()
    });

    commands.spawn((
        GroundLotMarker,
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // 4. Default House Lot Boundary (e.g. 24x20m parcel)
    let lot_mesh = meshes.add(Plane3d::default().mesh().size(24.0, 20.0));
    let lot_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.22, 0.26), // building parcel stone/pavement pad
        perceptual_roughness: 0.75,
        metallic: 0.1,
        ..default()
    });

    commands.spawn((
        LotBoundaryMarker,
        Mesh3d(lot_mesh),
        MeshMaterial3d(lot_material),
        Transform::from_xyz(0.0, 0.02, 0.0), // slightly above ground to prevent z-fighting
    ));
}
