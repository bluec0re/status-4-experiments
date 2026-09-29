#![allow(dead_code)]

use bevy::prelude::*;
use crate::fire_sim::{Combustible, EnvironmentConditions, SimulationStats, fastrand_range};
use crate::house_compat::{BuildingElementKind, FireBuilding, HouseSpawner};
use crate::material::MaterialType;

pub struct ScenarioPlugin;

impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScenarioState {
            current: ScenarioType::CityFire,
            pending_switch: Some(ScenarioType::CityFire),
        })
        .add_systems(Update, handle_scenario_switch);
    }
}

/// Scenario choices available to the user
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScenarioType {
    #[default]
    CityFire,
    Wildfire,
    SuburbanResidence,
    IndustrialDepot,
}

impl ScenarioType {
    pub const ALL: [ScenarioType; 4] = [
        ScenarioType::CityFire,
        ScenarioType::Wildfire,
        ScenarioType::SuburbanResidence,
        ScenarioType::IndustrialDepot,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::CityFire => "Old Town City Block",
            Self::Wildfire => "Forest Wildfire & Canyon",
            Self::SuburbanResidence => "Suburban Home & Yard",
            Self::IndustrialDepot => "Timber & Fuel Depot",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::CityFire => "Multi-story European buildings, storefronts, and narrow alleys. Fire spreads through windows and leaps across roofs.",
            Self::Wildfire => "Dense pine forest and dry underbrush driven by gusty wind. Spotting embers ignite fires ahead of the main front.",
            Self::SuburbanResidence => "Standalone houses, wooden fences, and garden foliage showing structural defense and wildfire-urban interface.",
            Self::IndustrialDepot => "Stacked lumber timber piles, pallets, and metal storage sheds with high radiant heat loads.",
        }
    }
}

#[derive(Resource)]
pub struct ScenarioState {
    pub current: ScenarioType,
    pub pending_switch: Option<ScenarioType>,
}

/// Component tagging all entities belonging to the active scenario for clean teardown
#[derive(Component)]
pub struct ScenarioEntity;

fn handle_scenario_switch(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<ScenarioState>,
    mut env: ResMut<EnvironmentConditions>,
    mut stats: ResMut<SimulationStats>,
    existing_entities: Query<Entity, With<ScenarioEntity>>,
    building_roots: Query<Entity, With<FireBuilding>>,
) {
    let Some(next_scenario) = state.pending_switch.take() else {
        return;
    };

    state.current = next_scenario;

    // 1. Despawn existing scenario entities
    for ent in existing_entities.iter() {
        commands.entity(ent).despawn();
    }
    for ent in building_roots.iter() {
        commands.entity(ent).despawn();
    }

    // 2. Reset telemetry stats
    stats.active_fires = 0;
    stats.burning_elements = 0;
    stats.burnt_elements = 0;
    stats.water_used_liters = 0.0;
    stats.max_temperature = 22.0;

    // 3. Build selected scenario
    match next_scenario {
        ScenarioType::CityFire => {
            env.wind_direction = Vec2::new(1.0, 0.2).normalize();
            env.wind_speed = 8.5;
            spawn_city_scenario(&mut commands, &mut meshes, &mut materials);
        }
        ScenarioType::Wildfire => {
            env.wind_direction = Vec2::new(1.0, -0.1).normalize();
            env.wind_speed = 16.0; // High wildfire wind!
            spawn_wildfire_scenario(&mut commands, &mut meshes, &mut materials);
        }
        ScenarioType::SuburbanResidence => {
            env.wind_direction = Vec2::new(0.6, 0.8).normalize();
            env.wind_speed = 7.0;
            spawn_suburban_scenario(&mut commands, &mut meshes, &mut materials);
        }
        ScenarioType::IndustrialDepot => {
            env.wind_direction = Vec2::new(0.8, -0.6).normalize();
            env.wind_speed = 11.0;
            spawn_industrial_scenario(&mut commands, &mut meshes, &mut materials);
        }
    }
}

/// Scenario 1: European City Block
fn spawn_city_scenario(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Cobblestone Main Street & Sidewalks
    let street_mesh = meshes.add(Plane3d::default().mesh().size(120.0, 16.0));
    let street_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.20, 0.22),
        perceptual_roughness: 0.75,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(street_mesh),
        MeshMaterial3d(street_mat),
        Transform::from_xyz(0.0, 0.01, 0.0),
    ));

    // North side buildings (3 buildings along the avenue)
    // Building A: 4-Story Haussmannian
    let b1 = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(-18.0, 0.0, 16.0),
        14.0,
        12.0,
        4,
        3.4,
        MaterialType::PlasterStucco,
        MaterialType::MetalZinc,
        "Haussmannian Grand Block",
    );
    commands.entity(b1).insert(ScenarioEntity);

    // Building B: 3-Story Altbau (Bakery / Fire Source)
    let b2 = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(0.0, 0.0, 16.0),
        16.0,
        12.0,
        3,
        3.5,
        MaterialType::WoodTimber,
        MaterialType::ThatchFabric,
        "Altbau Bakery & Apartments",
    );
    commands.entity(b2).insert(ScenarioEntity);

    // Building C: 4-Story Neoclassical
    let b3 = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(19.0, 0.0, 16.0),
        15.0,
        12.0,
        4,
        3.4,
        MaterialType::PlasterStucco,
        MaterialType::WoodTimber,
        "Neoclassical Corner Block",
    );
    commands.entity(b3).insert(ScenarioEntity);

    // South side building across the 16m street
    let b4 = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(0.0, 0.0, -16.0),
        22.0,
        12.0,
        3,
        3.5,
        MaterialType::PlasterStucco,
        MaterialType::MetalZinc,
        "South Street Market Hall",
    );
    commands.entity(b4).insert(ScenarioEntity);

    // Street wooden stalls & awnings
    for x in [-8.0, 6.0, -20.0, 15.0] {
        let stall_mesh = meshes.add(Cuboid::new(2.5, 2.2, 2.0));
        let stall_mat = materials.add(StandardMaterial {
            base_color: MaterialType::WoodTimber.base_color(),
            perceptual_roughness: 0.8,
            ..default()
        });
        commands.spawn((
            ScenarioEntity,
            Mesh3d(stall_mesh),
            MeshMaterial3d(stall_mat),
            Transform::from_xyz(x, 1.1, 7.5),
            Combustible::new(
                MaterialType::WoodTimber,
                BuildingElementKind::Furniture,
                Vec3::new(2.5, 2.2, 2.0),
            ),
        ));

        // Canvas awning
        let awning_mesh = meshes.add(Cuboid::new(2.8, 0.1, 1.4));
        let awning_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.25, 0.22),
            perceptual_roughness: 0.7,
            ..default()
        });
        commands.spawn((
            ScenarioEntity,
            Mesh3d(awning_mesh),
            MeshMaterial3d(awning_mat),
            Transform::from_xyz(x, 2.3, 8.2),
            Combustible::new(
                MaterialType::ThatchFabric,
                BuildingElementKind::Furniture,
                Vec3::new(2.8, 0.2, 1.4),
            ),
        ));
    }

    // Street ornamental trees
    for x in [-14.0, -4.0, 10.0, 22.0] {
        spawn_street_tree(commands, meshes, materials, Vec3::new(x, 0.0, -7.5));
    }

    // Ignite initial fire at the center market stall and ground floor bakery
    let mut initial_fire = Combustible::new(
        MaterialType::WoodTimber,
        BuildingElementKind::Furniture,
        Vec3::new(3.0, 2.4, 2.5),
    );
    initial_fire.ignite();

    let init_mesh = meshes.add(Cuboid::new(3.0, 2.4, 2.5));
    let init_mat = materials.add(StandardMaterial {
        base_color: MaterialType::WoodTimber.base_color(),
        emissive: LinearRgba::from(Color::srgb(1.0, 0.4, 0.05)) * 2.0,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(init_mesh),
        MeshMaterial3d(init_mat),
        Transform::from_xyz(-1.0, 1.2, 7.5),
        initial_fire,
    ));
}

/// Scenario 2: Wildfire Forest & Ridge
fn spawn_wildfire_scenario(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Dense conifer forest spread across 100x80m
    for _i in 0..75 {
        let x = fastrand_range(-50.0, 50.0);
        let z = fastrand_range(-35.0, 35.0);
        let scale = fastrand_range(0.8, 1.4);
        spawn_pine_tree(commands, meshes, materials, Vec3::new(x, 0.0, z), scale);
    }

    // Dry underbrush and fallen timber logs
    for _ in 0..60 {
        let x = fastrand_range(-50.0, 50.0);
        let z = fastrand_range(-35.0, 35.0);
        spawn_dry_brush(commands, meshes, materials, Vec3::new(x, 0.0, z));
    }

    // Isolated wooden mountain cabin
    let cabin = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(20.0, 0.0, 5.0),
        9.0,
        7.0,
        2,
        3.0,
        MaterialType::WoodTimber,
        MaterialType::WoodTimber,
        "Forest Ranger Cabin",
    );
    commands.entity(cabin).insert(ScenarioEntity);

    // Initial wildfire front on the upwind (West) side
    for z in [-18.0, -9.0, 0.0, 9.0, 18.0] {
        let mut fire_brush = Combustible::new(
            MaterialType::DryVegetation,
            BuildingElementKind::VegetationBush,
            Vec3::new(3.5, 2.0, 3.5),
        );
        fire_brush.ignite();

        let brush_mesh = meshes.add(Sphere::new(1.8).mesh().ico(1).expect("sphere mesh"));
        let brush_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.45, 0.1),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.4, 0.05)) * 2.5,
            ..default()
        });
        commands.spawn((
            ScenarioEntity,
            Mesh3d(brush_mesh),
            MeshMaterial3d(brush_mat),
            Transform::from_xyz(-44.0, 1.2, z),
            fire_brush,
        ));
    }
}

/// Scenario 3: Suburban House & Courtyard
fn spawn_suburban_scenario(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // House 1 (Main Residence)
    let h1 = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(-14.0, 0.0, 0.0),
        12.0,
        10.0,
        2,
        3.2,
        MaterialType::WoodTimber,
        MaterialType::WoodTimber,
        "Suburban Residence",
    );
    commands.entity(h1).insert(ScenarioEntity);

    // House 2 (Neighbor)
    let h2 = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(16.0, 0.0, 0.0),
        11.0,
        9.0,
        2,
        3.2,
        MaterialType::PlasterStucco,
        MaterialType::MetalZinc,
        "Neighboring Cottage",
    );
    commands.entity(h2).insert(ScenarioEntity);

    // Wooden perimeter fence running between parcels
    for z in (-20..=20).step_by(3) {
        let fence_mesh = meshes.add(Cuboid::new(0.2, 1.6, 2.8));
        let fence_mat = materials.add(StandardMaterial {
            base_color: MaterialType::WoodTimber.base_color(),
            perceptual_roughness: 0.85,
            ..default()
        });
        commands.spawn((
            ScenarioEntity,
            Mesh3d(fence_mesh),
            MeshMaterial3d(fence_mat),
            Transform::from_xyz(1.0, 0.8, z as f32),
            Combustible::new(
                MaterialType::WoodTimber,
                BuildingElementKind::Furniture,
                Vec3::new(0.2, 1.6, 2.8),
            ),
        ));
    }

    // Detached garden timber shed
    let shed_mesh = meshes.add(Cuboid::new(4.0, 2.8, 3.5));
    let shed_mat = materials.add(StandardMaterial {
        base_color: MaterialType::WoodTimber.base_color(),
        perceptual_roughness: 0.8,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(shed_mesh),
        MeshMaterial3d(shed_mat),
        Transform::from_xyz(-18.0, 1.4, -16.0),
        Combustible::new(
            MaterialType::WoodTimber,
            BuildingElementKind::Furniture,
            Vec3::new(4.0, 2.8, 3.5),
        ),
    ));

    // Garden yard trees & dry flowerbeds
    for x in [-22.0, -8.0, 10.0, 24.0] {
        spawn_street_tree(commands, meshes, materials, Vec3::new(x, 0.0, 12.0));
        spawn_dry_brush(commands, meshes, materials, Vec3::new(x + 2.0, 0.0, 8.0));
    }

    // Fire starts at garden shed
    let mut shed_fire = Combustible::new(
        MaterialType::WoodTimber,
        BuildingElementKind::Furniture,
        Vec3::new(4.0, 2.8, 3.5),
    );
    shed_fire.ignite();

    let fire_mesh = meshes.add(Cuboid::new(3.0, 2.5, 3.0));
    let fire_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.4, 0.1),
        emissive: LinearRgba::from(Color::srgb(1.0, 0.4, 0.05)) * 2.5,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(fire_mesh),
        MeshMaterial3d(fire_mat),
        Transform::from_xyz(-18.0, 1.4, -16.0),
        shed_fire,
    ));
}

/// Scenario 4: Industrial Timber & Fuel Depot
fn spawn_industrial_scenario(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Metal warehouse shed
    let wh = HouseSpawner::spawn_european_building(
        commands,
        meshes,
        materials,
        Vec3::new(-16.0, 0.0, 0.0),
        16.0,
        14.0,
        1,
        5.5,
        MaterialType::MetalZinc,
        MaterialType::MetalZinc,
        "Corrugated Metal Warehouse",
    );
    commands.entity(wh).insert(ScenarioEntity);

    // Stacks of lumber timber planks
    for z in [-14.0, -6.0, 4.0, 12.0] {
        for x in [6.0, 16.0, 24.0] {
            let stack_mesh = meshes.add(Cuboid::new(5.0, 2.4, 2.4));
            let stack_mat = materials.add(StandardMaterial {
                base_color: MaterialType::WoodTimber.base_color(),
                perceptual_roughness: 0.8,
                ..default()
            });
            commands.spawn((
                ScenarioEntity,
                Mesh3d(stack_mesh),
                MeshMaterial3d(stack_mat),
                Transform::from_xyz(x, 1.2, z),
                Combustible::new(
                    MaterialType::WoodTimber,
                    BuildingElementKind::Furniture,
                    Vec3::new(5.0, 2.4, 2.4),
                ),
            ));
        }
    }

    // Ignite initial stack of timber
    let mut timber_fire = Combustible::new(
        MaterialType::WoodTimber,
        BuildingElementKind::Furniture,
        Vec3::new(5.0, 2.4, 2.4),
    );
    timber_fire.ignite();

    let init_mesh = meshes.add(Cuboid::new(5.0, 2.4, 2.4));
    let init_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.1),
        emissive: LinearRgba::from(Color::srgb(1.0, 0.4, 0.05)) * 2.5,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(init_mesh),
        MeshMaterial3d(init_mat),
        Transform::from_xyz(6.0, 1.2, -14.0),
        timber_fire,
    ));
}

/// Helper: Spawn Conifer Pine Tree
fn spawn_pine_tree(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    scale: f32,
) {
    let trunk_h = 4.0 * scale;
    let trunk_r = 0.35 * scale;
    let trunk_mesh = meshes.add(Cylinder::new(trunk_r, trunk_h));
    let trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.22, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Trunk
    commands.spawn((
        ScenarioEntity,
        Mesh3d(trunk_mesh),
        MeshMaterial3d(trunk_mat),
        Transform::from_translation(pos + Vec3::new(0.0, trunk_h * 0.5, 0.0)),
        Combustible::new(
            MaterialType::WoodTimber,
            BuildingElementKind::VegetationTree,
            Vec3::new(trunk_r * 2.0, trunk_h, trunk_r * 2.0),
        ),
    ));

    // Foliage Needle Cone
    let cone_h = 7.0 * scale;
    let cone_r = 2.4 * scale;
    let cone_mesh = meshes.add(Cone::new(cone_r, cone_h));
    let cone_mat = materials.add(StandardMaterial {
        base_color: MaterialType::PineTree.base_color(),
        perceptual_roughness: 0.85,
        ..default()
    });

    commands.spawn((
        ScenarioEntity,
        Mesh3d(cone_mesh),
        MeshMaterial3d(cone_mat),
        Transform::from_translation(pos + Vec3::new(0.0, trunk_h + cone_h * 0.5 - 0.5, 0.0)),
        Combustible::new(
            MaterialType::PineTree,
            BuildingElementKind::VegetationTree,
            Vec3::new(cone_r * 2.0, cone_h, cone_r * 2.0),
        ),
    ));
}

/// Helper: Spawn Street Broadleaf Tree
fn spawn_street_tree(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
) {
    let trunk_mesh = meshes.add(Cylinder::new(0.3, 3.5));
    let trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.28, 0.18),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(trunk_mesh),
        MeshMaterial3d(trunk_mat),
        Transform::from_translation(pos + Vec3::new(0.0, 1.75, 0.0)),
        Combustible::new(
            MaterialType::WoodTimber,
            BuildingElementKind::VegetationTree,
            Vec3::new(0.6, 3.5, 0.6),
        ),
    ));

    let crown_mesh = meshes.add(Sphere::new(2.2).mesh().ico(1).expect("sphere mesh"));
    let crown_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.42, 0.18),
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(crown_mesh),
        MeshMaterial3d(crown_mat),
        Transform::from_translation(pos + Vec3::new(0.0, 4.5, 0.0)),
        Combustible::new(
            MaterialType::DryVegetation,
            BuildingElementKind::VegetationTree,
            Vec3::new(4.4, 4.4, 4.4),
        ),
    ));
}

/// Helper: Spawn Dry Brush / Shrub
fn spawn_dry_brush(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
) {
    let brush_mesh = meshes.add(Sphere::new(1.2).mesh().ico(1).expect("sphere mesh"));
    let brush_mat = materials.add(StandardMaterial {
        base_color: MaterialType::DryVegetation.base_color(),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        ScenarioEntity,
        Mesh3d(brush_mesh),
        MeshMaterial3d(brush_mat),
        Transform::from_translation(pos + Vec3::new(0.0, 0.6, 0.0))
            .with_scale(Vec3::new(1.4, 0.7, 1.4)),
        Combustible::new(
            MaterialType::DryVegetation,
            BuildingElementKind::VegetationBush,
            Vec3::new(2.8, 1.0, 2.8),
        ),
    ));
}
