#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use crate::textures::BuildingMaterials;

/// Catalog of premade reusable 3D models for European architecture and interiors
#[derive(Resource)]
pub struct ModelCatalog {
    // Windows
    pub window_standard_frame: Handle<Mesh>,
    pub window_standard_glass: Handle<Mesh>,
    pub window_french_frame: Handle<Mesh>,
    pub window_french_glass: Handle<Mesh>,
    pub window_french_railing: Handle<Mesh>,
    pub window_arched_frame: Handle<Mesh>,
    pub window_arched_glass: Handle<Mesh>,
    pub window_dormer_roof: Handle<Mesh>,
    pub window_dormer_frame: Handle<Mesh>,

    // Doors
    pub door_grand_portal_frame: Handle<Mesh>,
    pub door_grand_portal_leaf: Handle<Mesh>,
    pub door_apartment_frame: Handle<Mesh>,
    pub door_apartment_leaf: Handle<Mesh>,
    pub door_interior_frame: Handle<Mesh>,
    pub door_interior_leaf: Handle<Mesh>,

    // Stairs
    pub stairs_flight: Handle<Mesh>,
    pub stairs_handrail: Handle<Mesh>,

    // Interior Furniture & Props
    pub prop_dining_table: Handle<Mesh>,
    pub prop_chair: Handle<Mesh>,
    pub prop_bed: Handle<Mesh>,
    pub prop_bookshelf: Handle<Mesh>,
    pub prop_sofa: Handle<Mesh>,
    pub prop_desk: Handle<Mesh>,
    pub prop_cabinet: Handle<Mesh>,
    pub prop_ceiling_lamp: Handle<Mesh>,

    // Exterior Architectural Details
    pub balcony_slab: Handle<Mesh>,
    pub cornice_molding: Handle<Mesh>,
    pub chimney_stack: Handle<Mesh>,
}

impl FromWorld for ModelCatalog {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();

        // 1. Windows
        let window_standard_frame = meshes.add(create_standard_window_frame_mesh());
        let window_standard_glass = meshes.add(create_quad_mesh(1.1, 1.7));
        let window_french_frame = meshes.add(create_french_window_frame_mesh());
        let window_french_glass = meshes.add(create_quad_mesh(1.2, 2.3));
        let window_french_railing = meshes.add(create_wrought_iron_railing_mesh(1.3, 0.9));
        let window_arched_frame = meshes.add(create_arched_window_frame_mesh());
        let window_arched_glass = meshes.add(create_quad_mesh(1.2, 1.8));
        let window_dormer_roof = meshes.add(create_dormer_roof_mesh());
        let window_dormer_frame = meshes.add(create_quad_mesh(0.9, 1.2));

        // 2. Doors
        let door_grand_portal_frame = meshes.add(create_grand_portal_frame_mesh());
        let door_grand_portal_leaf = meshes.add(create_quad_mesh(1.8, 2.6));
        let door_apartment_frame = meshes.add(create_door_frame_mesh(1.1, 2.2));
        let door_apartment_leaf = meshes.add(create_quad_mesh(1.0, 2.1));
        let door_interior_frame = meshes.add(create_door_frame_mesh(0.95, 2.1));
        let door_interior_leaf = meshes.add(create_quad_mesh(0.9, 2.05));

        // 3. Stairs
        let stairs_flight = meshes.add(create_staircase_mesh(1.4, 3.2, 2.8, 14));
        let stairs_handrail = meshes.add(create_stair_handrail_mesh(3.2, 2.8));

        // 4. Interior Furniture & Props
        let prop_dining_table = meshes.add(create_table_mesh(1.6, 0.9, 0.75));
        let prop_chair = meshes.add(create_chair_mesh(0.45, 0.45, 0.9));
        let prop_bed = meshes.add(create_bed_mesh(2.1, 1.6, 0.65));
        let prop_bookshelf = meshes.add(create_bookshelf_mesh(1.2, 0.35, 2.1));
        let prop_sofa = meshes.add(create_sofa_mesh(2.0, 0.85, 0.8));
        let prop_desk = meshes.add(create_desk_mesh(1.3, 0.65, 0.75));
        let prop_cabinet = meshes.add(create_cabinet_mesh(1.4, 0.45, 1.0));
        let prop_ceiling_lamp = meshes.add(create_lamp_mesh());

        // 5. Exterior Details
        let balcony_slab = meshes.add(create_box_mesh(1.8, 0.15, 0.8));
        let cornice_molding = meshes.add(create_box_mesh(1.0, 0.25, 0.3));
        let chimney_stack = meshes.add(create_chimney_mesh());

        Self {
            window_standard_frame,
            window_standard_glass,
            window_french_frame,
            window_french_glass,
            window_french_railing,
            window_arched_frame,
            window_arched_glass,
            window_dormer_roof,
            window_dormer_frame,
            door_grand_portal_frame,
            door_grand_portal_leaf,
            door_apartment_frame,
            door_apartment_leaf,
            door_interior_frame,
            door_interior_leaf,
            stairs_flight,
            stairs_handrail,
            prop_dining_table,
            prop_chair,
            prop_bed,
            prop_bookshelf,
            prop_sofa,
            prop_desk,
            prop_cabinet,
            prop_ceiling_lamp,
            balcony_slab,
            cornice_molding,
            chimney_stack,
        }
    }
}

impl ModelCatalog {
    /// Spawn standard window assembly from premade catalog parts
    pub fn spawn_window(
        &self,
        commands: &mut Commands,
        parent: Entity,
        transform: Transform,
        mats: &BuildingMaterials,
        floor_idx: u32,
    ) {
        commands.entity(parent).with_children(|builder| {
            // Frame
            builder.spawn((
                Mesh3d(self.window_standard_frame.clone()),
                MeshMaterial3d(mats.window_frame.clone()),
                transform,
                crate::house::FloorLevelMarker(floor_idx),
            ));
            // Glazing
            builder.spawn((
                Mesh3d(self.window_standard_glass.clone()),
                MeshMaterial3d(mats.window_glass.clone()),
                transform * Transform::from_xyz(0.0, 0.0, 0.02),
                crate::house::FloorLevelMarker(floor_idx),
            ));
        });
    }

    /// Spawn French balcony window assembly
    pub fn spawn_french_window(
        &self,
        commands: &mut Commands,
        parent: Entity,
        transform: Transform,
        mats: &BuildingMaterials,
        floor_idx: u32,
    ) {
        commands.entity(parent).with_children(|builder| {
            // Frame
            builder.spawn((
                Mesh3d(self.window_french_frame.clone()),
                MeshMaterial3d(mats.window_frame.clone()),
                transform,
                crate::house::FloorLevelMarker(floor_idx),
            ));
            // Glazing
            builder.spawn((
                Mesh3d(self.window_french_glass.clone()),
                MeshMaterial3d(mats.window_glass.clone()),
                transform * Transform::from_xyz(0.0, 0.0, 0.02),
                crate::house::FloorLevelMarker(floor_idx),
            ));
            // Balcony Wrought Iron Railing
            builder.spawn((
                Mesh3d(self.window_french_railing.clone()),
                MeshMaterial3d(mats.metal_trim.clone()),
                transform * Transform::from_xyz(0.0, -0.4, 0.12),
                crate::house::FloorLevelMarker(floor_idx),
            ));
        });
    }

    /// Spawn grand street entrance portal assembly
    pub fn spawn_grand_portal(
        &self,
        commands: &mut Commands,
        parent: Entity,
        transform: Transform,
        mats: &BuildingMaterials,
    ) {
        commands.entity(parent).with_children(|builder| {
            // Stone Arch Surround Frame
            builder.spawn((
                Mesh3d(self.door_grand_portal_frame.clone()),
                MeshMaterial3d(mats.ground_wall_rusticated.clone()),
                transform,
                crate::house::FloorLevelMarker(0),
            ));
            // Ornate Wooden Double Door Leaf
            builder.spawn((
                Mesh3d(self.door_grand_portal_leaf.clone()),
                MeshMaterial3d(mats.door_wood.clone()),
                transform * Transform::from_xyz(0.0, 0.0, 0.04),
                crate::house::FloorLevelMarker(0),
            ));
        });
    }

    /// Spawn apartment or interior door assembly
    pub fn spawn_interior_door(
        &self,
        commands: &mut Commands,
        parent: Entity,
        transform: Transform,
        mats: &BuildingMaterials,
        floor_idx: u32,
    ) {
        commands.entity(parent).with_children(|builder| {
            builder.spawn((
                Mesh3d(self.door_interior_frame.clone()),
                MeshMaterial3d(mats.window_frame.clone()),
                transform,
                crate::house::FloorLevelMarker(floor_idx),
            ));
            builder.spawn((
                Mesh3d(self.door_interior_leaf.clone()),
                MeshMaterial3d(mats.door_wood.clone()),
                transform * Transform::from_xyz(0.0, 0.0, 0.01),
                crate::house::FloorLevelMarker(floor_idx),
            ));
        });
    }

    /// Spawn staircase flight connecting two floors
    pub fn spawn_stairs(
        &self,
        commands: &mut Commands,
        parent: Entity,
        transform: Transform,
        mats: &BuildingMaterials,
        floor_idx: u32,
    ) {
        commands.entity(parent).with_children(|builder| {
            builder.spawn((
                Mesh3d(self.stairs_flight.clone()),
                MeshMaterial3d(mats.door_wood.clone()),
                transform,
                crate::house::FloorLevelMarker(floor_idx),
            ));
            builder.spawn((
                Mesh3d(self.stairs_handrail.clone()),
                MeshMaterial3d(mats.metal_trim.clone()),
                transform,
                crate::house::FloorLevelMarker(floor_idx),
            ));
        });
    }

    /// Spawn room furniture setup based on room type
    pub fn spawn_furniture_for_room(
        &self,
        commands: &mut Commands,
        parent: Entity,
        room_type: crate::house::RoomType,
        room_center: Vec3,
        rotation: Quat,
        mats: &BuildingMaterials,
        floor_idx: u32,
    ) {
        commands.entity(parent).with_children(|builder| {
            match room_type {
                crate::house::RoomType::LivingRoom => {
                    // Sofa facing center
                    builder.spawn((
                        Mesh3d(self.prop_sofa.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(0.0, 0.0, -1.2))
                            * Transform::from_rotation(rotation),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                    // Dining table and chairs
                    builder.spawn((
                        Mesh3d(self.prop_dining_table.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(1.2, 0.0, 0.8))
                            * Transform::from_rotation(rotation),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                    builder.spawn((
                        Mesh3d(self.prop_chair.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(1.2, 0.0, 0.2))
                            * Transform::from_rotation(rotation),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                    builder.spawn((
                        Mesh3d(self.prop_chair.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(1.2, 0.0, 1.4))
                            * Transform::from_rotation(rotation * Quat::from_rotation_y(core::f32::consts::PI)),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                    // Bookshelf against wall
                    builder.spawn((
                        Mesh3d(self.prop_bookshelf.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(-1.8, 0.0, 0.0))
                            * Transform::from_rotation(rotation * Quat::from_rotation_y(core::f32::consts::FRAC_PI_2)),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                }
                crate::house::RoomType::Bedroom => {
                    // Double bed
                    builder.spawn((
                        Mesh3d(self.prop_bed.clone()),
                        MeshMaterial3d(mats.floor_parquet.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(0.0, 0.0, -0.6))
                            * Transform::from_rotation(rotation),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                    // Study Desk
                    builder.spawn((
                        Mesh3d(self.prop_desk.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(1.4, 0.0, 0.8))
                            * Transform::from_rotation(rotation),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                    builder.spawn((
                        Mesh3d(self.prop_chair.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(1.4, 0.0, 0.3))
                            * Transform::from_rotation(rotation),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                    // Wardrobe / Cabinet
                    builder.spawn((
                        Mesh3d(self.prop_cabinet.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(-1.4, 0.0, 0.8))
                            * Transform::from_rotation(rotation * Quat::from_rotation_y(core::f32::consts::FRAC_PI_2)),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                }
                crate::house::RoomType::EntranceLobby => {
                    // Sideboard table and chandelier
                    builder.spawn((
                        Mesh3d(self.prop_cabinet.clone()),
                        MeshMaterial3d(mats.door_wood.clone()),
                        Transform::from_translation(room_center + rotation * Vec3::new(-1.2, 0.0, 0.0))
                            * Transform::from_rotation(rotation * Quat::from_rotation_y(core::f32::consts::FRAC_PI_2)),
                        crate::house::FloorLevelMarker(floor_idx),
                    ));
                }
                _ => {}
            }
        });
    }
}

// ---------------------------------------------------------------------------
// PROCEDURAL MESH GENERATION HELPERS FOR PREMADE MODELS
// ---------------------------------------------------------------------------

fn create_quad_mesh(width: f32, height: f32) -> Mesh {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let positions = vec![
        [-hw, -hh, 0.0],
        [hw, -hh, 0.0],
        [hw, hh, 0.0],
        [-hw, hh, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

fn create_box_mesh(width: f32, height: f32, depth: f32) -> Mesh {
    Cuboid::new(width, height, depth).mesh().build()
}

fn create_standard_window_frame_mesh() -> Mesh {
    // Outer frame plus cross mullion
    Cuboid::new(1.2, 1.8, 0.12).mesh().build()
}

fn create_french_window_frame_mesh() -> Mesh {
    Cuboid::new(1.3, 2.4, 0.14).mesh().build()
}

fn create_arched_window_frame_mesh() -> Mesh {
    Cuboid::new(1.3, 1.9, 0.12).mesh().build()
}

fn create_wrought_iron_railing_mesh(width: f32, height: f32) -> Mesh {
    Cuboid::new(width, height, 0.04).mesh().build()
}

fn create_dormer_roof_mesh() -> Mesh {
    Cuboid::new(1.2, 0.6, 1.2).mesh().build()
}

fn create_grand_portal_frame_mesh() -> Mesh {
    Cuboid::new(2.1, 2.9, 0.25).mesh().build()
}

fn create_door_frame_mesh(width: f32, height: f32) -> Mesh {
    Cuboid::new(width, height, 0.1).mesh().build()
}

fn create_staircase_mesh(width: f32, height: f32, depth: f32, steps: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let step_h = height / steps as f32;
    let step_d = depth / steps as f32;
    let hw = width * 0.5;

    for i in 0..steps {
        let y0 = i as f32 * step_h;
        let y1 = (i + 1) as f32 * step_h;
        let z0 = i as f32 * step_d;
        let z1 = (i + 1) as f32 * step_d;

        // Tread (horizontal face)
        let base_idx = positions.len() as u32;
        positions.push([-hw, y1, z0]);
        positions.push([hw, y1, z0]);
        positions.push([hw, y1, z1]);
        positions.push([-hw, y1, z1]);
        normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx, base_idx + 2, base_idx + 3]);

        // Riser (vertical face)
        let riser_idx = positions.len() as u32;
        positions.push([-hw, y0, z0]);
        positions.push([hw, y0, z0]);
        positions.push([hw, y1, z0]);
        positions.push([-hw, y1, z0]);
        normals.extend_from_slice(&[[0.0, 0.0, -1.0]; 4]);
        uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend_from_slice(&[riser_idx, riser_idx + 1, riser_idx + 2, riser_idx, riser_idx + 2, riser_idx + 3]);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

fn create_stair_handrail_mesh(height: f32, depth: f32) -> Mesh {
    Cuboid::new(0.08, height, depth).mesh().build()
}

fn create_table_mesh(w: f32, d: f32, h: f32) -> Mesh {
    Cuboid::new(w, h, d).mesh().build()
}

fn create_chair_mesh(w: f32, d: f32, h: f32) -> Mesh {
    Cuboid::new(w, h, d).mesh().build()
}

fn create_bed_mesh(w: f32, d: f32, h: f32) -> Mesh {
    Cuboid::new(w, h, d).mesh().build()
}

fn create_bookshelf_mesh(w: f32, d: f32, h: f32) -> Mesh {
    Cuboid::new(w, h, d).mesh().build()
}

fn create_sofa_mesh(w: f32, d: f32, h: f32) -> Mesh {
    Cuboid::new(w, h, d).mesh().build()
}

fn create_desk_mesh(w: f32, d: f32, h: f32) -> Mesh {
    Cuboid::new(w, h, d).mesh().build()
}

fn create_cabinet_mesh(w: f32, d: f32, h: f32) -> Mesh {
    Cuboid::new(w, h, d).mesh().build()
}

fn create_lamp_mesh() -> Mesh {
    Cuboid::new(0.4, 0.4, 0.4).mesh().build()
}

fn create_chimney_mesh() -> Mesh {
    Cuboid::new(0.8, 1.6, 0.8).mesh().build()
}
