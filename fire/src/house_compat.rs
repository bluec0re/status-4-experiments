#![allow(dead_code)]

use bevy::prelude::*;
use crate::fire_sim::Combustible;
use crate::material::MaterialType;

/// Architectural element classification matching the procedural building system
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum BuildingElementKind {
    ExteriorWall,
    InteriorWall,
    FloorSlab,
    RoofTruss,
    RoofTile,
    Door,
    Window,
    Furniture,
    VegetationTree,
    VegetationBush,
    TerrainTile,
}

impl BuildingElementKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ExteriorWall => "Exterior Wall",
            Self::InteriorWall => "Interior Partition",
            Self::FloorSlab => "Wooden Floor Slab",
            Self::RoofTruss => "Roof Timber Truss",
            Self::RoofTile => "Roof Weather Tile",
            Self::Door => "Timber Doorway",
            Self::Window => "Glass Window Pane",
            Self::Furniture => "Combustible Furniture",
            Self::VegetationTree => "Conifer Forest Tree",
            Self::VegetationBush => "Dry Shrubbery",
            Self::TerrainTile => "Ground Parcel",
        }
    }
}

/// Root marker component for a simulated building
#[derive(Component, Debug, Clone)]
pub struct FireBuilding {
    pub name: String,
    pub stories: u32,
    pub floor_area_sqm: f32,
}

/// Helper struct for spawning modular house structures
pub struct HouseSpawner;

impl HouseSpawner {
    /// Spawns a multi-story European city building composed of combustible elements
    pub fn spawn_european_building(
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        origin: Vec3,
        width: f32,
        depth: f32,
        stories: u32,
        story_height: f32,
        wall_material: MaterialType,
        roof_material: MaterialType,
        name: &str,
    ) -> Entity {
        let building_root = commands
            .spawn((
                FireBuilding {
                    name: name.to_string(),
                    stories,
                    floor_area_sqm: width * depth * (stories as f32),
                },
                Transform::from_translation(origin),
                Visibility::default(),
            ))
            .id();

        let half_w = width * 0.5;
        let half_d = depth * 0.5;
        let wall_thick = 0.4;

        // 1. Stories: Floors, Exterior Walls, Windows, Interior partitions
        for s in 0..stories {
            let floor_y = s as f32 * story_height;
            let center_y = floor_y + story_height * 0.5;

            // Floor slab (WoodTimber floorboards)
            let slab_mesh = meshes.add(Cuboid::new(width, 0.25, depth));
            let slab_mat = materials.add(StandardMaterial {
                base_color: MaterialType::WoodTimber.base_color(),
                perceptual_roughness: 0.8,
                ..default()
            });
            let slab_ent = commands
                .spawn((
                    Mesh3d(slab_mesh),
                    MeshMaterial3d(slab_mat),
                    Transform::from_xyz(0.0, floor_y + 0.125, 0.0),
                    Combustible::new(
                        MaterialType::WoodTimber,
                        BuildingElementKind::FloorSlab,
                        Vec3::new(width, 0.25, depth),
                    ),
                ))
                .id();
            commands.entity(building_root).add_child(slab_ent);

            // Exterior Front & Back Walls with Windows
            let num_bays = (width / 3.5).round().max(2.0) as usize;
            let bay_w = width / num_bays as f32;

            for b in 0..num_bays {
                let bay_x = -half_w + (b as f32 + 0.5) * bay_w;

                // Front Wall Bay (North)
                spawn_wall_bay(
                    commands,
                    meshes,
                    materials,
                    building_root,
                    Vec3::new(bay_x, center_y, half_d - wall_thick * 0.5),
                    bay_w,
                    story_height,
                    wall_thick,
                    wall_material,
                    s == 0 && b == num_bays / 2, // Front door at ground center
                    BuildingElementKind::ExteriorWall,
                );

                // Back Wall Bay (South)
                spawn_wall_bay(
                    commands,
                    meshes,
                    materials,
                    building_root,
                    Vec3::new(bay_x, center_y, -half_d + wall_thick * 0.5),
                    bay_w,
                    story_height,
                    wall_thick,
                    wall_material,
                    false,
                    BuildingElementKind::ExteriorWall,
                );
            }

            // Left & Right Gable/Flank Walls
            let flank_mesh = meshes.add(Cuboid::new(wall_thick, story_height, depth - 0.2));
            let flank_mat = materials.add(StandardMaterial {
                base_color: wall_material.base_color(),
                perceptual_roughness: 0.85,
                ..default()
            });

            // West wall
            let west_ent = commands
                .spawn((
                    Mesh3d(flank_mesh.clone()),
                    MeshMaterial3d(flank_mat.clone()),
                    Transform::from_xyz(-half_w + wall_thick * 0.5, center_y, 0.0),
                    Combustible::new(
                        wall_material,
                        BuildingElementKind::ExteriorWall,
                        Vec3::new(wall_thick, story_height, depth),
                    ),
                ))
                .id();
            commands.entity(building_root).add_child(west_ent);

            // East wall
            let east_ent = commands
                .spawn((
                    Mesh3d(flank_mesh),
                    MeshMaterial3d(flank_mat),
                    Transform::from_xyz(half_w - wall_thick * 0.5, center_y, 0.0),
                    Combustible::new(
                        wall_material,
                        BuildingElementKind::ExteriorWall,
                        Vec3::new(wall_thick, story_height, depth),
                    ),
                ))
                .id();
            commands.entity(building_root).add_child(east_ent);

            // Interior partition wall dividing rooms
            let part_mesh = meshes.add(Cuboid::new(width * 0.6, story_height * 0.95, 0.15));
            let part_mat = materials.add(StandardMaterial {
                base_color: MaterialType::PlasterStucco.base_color(),
                perceptual_roughness: 0.9,
                ..default()
            });
            let part_ent = commands
                .spawn((
                    Mesh3d(part_mesh),
                    MeshMaterial3d(part_mat),
                    Transform::from_xyz(0.0, center_y, 0.0),
                    Combustible::new(
                        MaterialType::PlasterStucco,
                        BuildingElementKind::InteriorWall,
                        Vec3::new(width * 0.6, story_height, 0.15),
                    ),
                ))
                .id();
            commands.entity(building_root).add_child(part_ent);

            // Combustible furniture / interior wooden fittings
            let furn_mesh = meshes.add(Cuboid::new(1.8, 0.8, 1.0));
            let furn_mat = materials.add(StandardMaterial {
                base_color: MaterialType::WoodTimber.base_color(),
                perceptual_roughness: 0.6,
                ..default()
            });
            let furn_ent = commands
                .spawn((
                    Mesh3d(furn_mesh),
                    MeshMaterial3d(furn_mat),
                    Transform::from_xyz(-width * 0.25, floor_y + 0.4 + 0.25, 0.5),
                    Combustible::new(
                        MaterialType::WoodTimber,
                        BuildingElementKind::Furniture,
                        Vec3::new(1.8, 0.8, 1.0),
                    ),
                ))
                .id();
            commands.entity(building_root).add_child(furn_ent);
        }

        // 2. Roof System (Wooden Trusses + Pitched Roof Tiles + Gable Walls)
        let roof_base_y = stories as f32 * story_height;
        let roof_height = (depth * 0.3).clamp(2.4, 4.0);
        let pitch_angle = (roof_height / half_d).atan();
        let slope_length = (half_d * half_d + roof_height * roof_height).sqrt() + 0.4;

        // Timber roof trusses (combustible framing)
        let truss_mesh = meshes.add(Cuboid::new(width * 0.92, 0.3, depth * 0.92));
        let truss_mat = materials.add(StandardMaterial {
            base_color: MaterialType::WoodTimber.base_color(),
            perceptual_roughness: 0.85,
            ..default()
        });
        let truss_ent = commands
            .spawn((
                Mesh3d(truss_mesh),
                MeshMaterial3d(truss_mat),
                Transform::from_xyz(0.0, roof_base_y + 0.15, 0.0),
                Combustible::new(
                    MaterialType::WoodTimber,
                    BuildingElementKind::RoofTruss,
                    Vec3::new(width, 0.3, depth),
                ),
            ))
            .id();
        commands.entity(building_root).add_child(truss_ent);

        // Sloped roof halves
        let slope_mesh = meshes.add(Cuboid::new(width + 0.6, 0.22, slope_length));
        let roof_mat = materials.add(StandardMaterial {
            base_color: roof_material.base_color(),
            perceptual_roughness: 0.6,
            ..default()
        });

        // South slope (slopes upward towards ridge at Z = 0)
        let south_slope_ent = commands
            .spawn((
                Mesh3d(slope_mesh.clone()),
                MeshMaterial3d(roof_mat.clone()),
                Transform::from_xyz(0.0, roof_base_y + roof_height * 0.5, -half_d * 0.5)
                    .with_rotation(Quat::from_rotation_x(-pitch_angle)),
                Combustible::new(
                    roof_material,
                    BuildingElementKind::RoofTile,
                    Vec3::new(width, 0.3, slope_length),
                ),
            ))
            .id();
        commands.entity(building_root).add_child(south_slope_ent);

        // North slope (slopes upward towards ridge at Z = 0)
        let north_slope_ent = commands
            .spawn((
                Mesh3d(slope_mesh),
                MeshMaterial3d(roof_mat),
                Transform::from_xyz(0.0, roof_base_y + roof_height * 0.5, half_d * 0.5)
                    .with_rotation(Quat::from_rotation_x(pitch_angle)),
                Combustible::new(
                    roof_material,
                    BuildingElementKind::RoofTile,
                    Vec3::new(width, 0.3, slope_length),
                ),
            ))
            .id();
        commands.entity(building_root).add_child(north_slope_ent);

        // Gable end walls (West & East facades filling the triangular pediment)
        let gable_mesh = meshes.add(Cuboid::new(wall_thick, roof_height * 0.85, depth * 0.55));
        let gable_mat = materials.add(StandardMaterial {
            base_color: wall_material.base_color(),
            perceptual_roughness: 0.85,
            ..default()
        });

        let west_gable = commands
            .spawn((
                Mesh3d(gable_mesh.clone()),
                MeshMaterial3d(gable_mat.clone()),
                Transform::from_xyz(-half_w + wall_thick * 0.5, roof_base_y + roof_height * 0.42, 0.0),
                Combustible::new(
                    wall_material,
                    BuildingElementKind::ExteriorWall,
                    Vec3::new(wall_thick, roof_height, depth * 0.6),
                ),
            ))
            .id();
        commands.entity(building_root).add_child(west_gable);

        let east_gable = commands
            .spawn((
                Mesh3d(gable_mesh),
                MeshMaterial3d(gable_mat),
                Transform::from_xyz(half_w - wall_thick * 0.5, roof_base_y + roof_height * 0.42, 0.0),
                Combustible::new(
                    wall_material,
                    BuildingElementKind::ExteriorWall,
                    Vec3::new(wall_thick, roof_height, depth * 0.6),
                ),
            ))
            .id();
        commands.entity(building_root).add_child(east_gable);

        building_root
    }
}

/// Helper to construct a wall bay with pier and window or door
fn spawn_wall_bay(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    parent: Entity,
    center: Vec3,
    bay_w: f32,
    height: f32,
    thick: f32,
    wall_mat: MaterialType,
    is_door: bool,
    kind: BuildingElementKind,
) {
    if is_door {
        // Door opening
        let door_mesh = meshes.add(Cuboid::new(1.4, 2.4, thick * 0.9));
        let door_mat = materials.add(StandardMaterial {
            base_color: MaterialType::WoodTimber.base_color(),
            perceptual_roughness: 0.7,
            ..default()
        });
        let door_ent = commands
            .spawn((
                Mesh3d(door_mesh),
                MeshMaterial3d(door_mat),
                Transform::from_translation(center - Vec3::new(0.0, (height - 2.4) * 0.5, 0.0)),
                Combustible::new(
                    MaterialType::WoodTimber,
                    BuildingElementKind::Door,
                    Vec3::new(1.4, 2.4, thick),
                ),
            ))
            .id();
        commands.entity(parent).add_child(door_ent);

        // Surrounding wall lintel
        let lintel_h = height - 2.4;
        if lintel_h > 0.2 {
            let lintel_mesh = meshes.add(Cuboid::new(bay_w, lintel_h, thick));
            let wall_pbr = materials.add(StandardMaterial {
                base_color: wall_mat.base_color(),
                perceptual_roughness: 0.85,
                ..default()
            });
            let lintel_ent = commands
                .spawn((
                    Mesh3d(lintel_mesh),
                    MeshMaterial3d(wall_pbr),
                    Transform::from_translation(center + Vec3::new(0.0, height * 0.5 - lintel_h * 0.5, 0.0)),
                    Combustible::new(wall_mat, kind, Vec3::new(bay_w, lintel_h, thick)),
                ))
                .id();
            commands.entity(parent).add_child(lintel_ent);
        }
    } else {
        // Window Pier: Wall with central window aperture
        let win_w = 1.3;
        let win_h = 1.7;

        // Window pane
        let win_mesh = meshes.add(Cuboid::new(win_w, win_h, 0.08));
        let win_mat = materials.add(StandardMaterial {
            base_color: MaterialType::GlassWindow.base_color(),
            perceptual_roughness: 0.1,
            metallic: 0.1,
            ..default()
        });
        let win_ent = commands
            .spawn((
                Mesh3d(win_mesh),
                MeshMaterial3d(win_mat),
                Transform::from_translation(center),
                Combustible::new(
                    MaterialType::GlassWindow,
                    BuildingElementKind::Window,
                    Vec3::new(win_w, win_h, 0.1),
                ),
            ))
            .id();
        commands.entity(parent).add_child(win_ent);

        // Surrounding wall sections (Spandrel below, Lintel above, Side piers)
        let wall_pbr = materials.add(StandardMaterial {
            base_color: wall_mat.base_color(),
            perceptual_roughness: 0.85,
            ..default()
        });

        // Below window
        let spandrel_h = (height - win_h) * 0.5;
        let spandrel_mesh = meshes.add(Cuboid::new(bay_w, spandrel_h, thick));
        let spandrel_ent = commands
            .spawn((
                Mesh3d(spandrel_mesh),
                MeshMaterial3d(wall_pbr.clone()),
                Transform::from_translation(center - Vec3::new(0.0, (height - spandrel_h) * 0.5, 0.0)),
                Combustible::new(wall_mat, kind, Vec3::new(bay_w, spandrel_h, thick)),
            ))
            .id();
        commands.entity(parent).add_child(spandrel_ent);

        // Above window
        let lintel_mesh = meshes.add(Cuboid::new(bay_w, spandrel_h, thick));
        let lintel_ent = commands
            .spawn((
                Mesh3d(lintel_mesh),
                MeshMaterial3d(wall_pbr),
                Transform::from_translation(center + Vec3::new(0.0, (height - spandrel_h) * 0.5, 0.0)),
                Combustible::new(wall_mat, kind, Vec3::new(bay_w, spandrel_h, thick)),
            ))
            .id();
        commands.entity(parent).add_child(lintel_ent);
    }
}
