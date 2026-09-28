#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use crate::catalog::ModelCatalog;
use crate::house::{Doorway, FloorLevelMarker, Room, RoomType, Staircase};
use crate::polygon::BuildingPolygon;
use crate::textures::BuildingMaterials;

/// Layout and navigation data produced during internal floor generation
pub struct GeneratedFloorPlan {
    pub room_entities: Vec<Entity>,
    pub doorway_entities: Vec<Entity>,
    pub staircase_entities: Vec<Entity>,
    pub street_entrance_pos: Vec3,
    pub lobby_pos: Vec3,
    pub total_rooms: usize,
}

/// Dynamic internal floor plan synthesizer
pub fn generate_internal_floors(
    commands: &mut Commands,
    building_entity: Entity,
    polygon: &BuildingPolygon,
    seed: u64,
    stories: u32,
    ground_story_h: f32,
    upper_story_h: f32,
    meshes: &mut Assets<Mesh>,
    materials: &BuildingMaterials,
    catalog: &ModelCatalog,
) -> GeneratedFloorPlan {
    let mut room_entities = Vec::new();
    let mut doorway_entities = Vec::new();
    let mut staircase_entities = Vec::new();

    let centroid = polygon.centroid();
    // Use front edge (edge 0) for street entrance
    let (p1, p2) = polygon.edge(0);
    let front_center = (p1 + p2) * 0.5;
    let front_normal = polygon.edge_outward_normal(0);
    let front_dir = Vec3::new(front_normal.x, 0.0, front_normal.y);

    let street_entrance_pos = Vec3::new(front_center.x, 0.0, front_center.y) + front_dir * 0.5;
    let lobby_pos = Vec3::new(
        front_center.x - front_normal.x * 2.5,
        0.0,
        front_center.y - front_normal.y * 2.5,
    );

    // Position stairwell slightly recessed towards centroid
    let stairwell_pos_2d = centroid + (centroid - front_center).normalize_or_zero() * 1.5;

    let mut current_y = 0.0;
    let mut room_counter = 0u32;

    for story in 0..stories {
        let story_h = if story == 0 { ground_story_h } else { upper_story_h };

        // 1. Spawn Floor Slab Mesh
        let slab_mesh = create_polygon_slab_mesh(polygon, 0.2);
        let slab_mat = if story == 0 {
            materials.floor_marble_tile.clone()
        } else {
            materials.floor_parquet.clone()
        };

        commands.entity(building_entity).with_children(|b| {
            b.spawn((
                Mesh3d(meshes.add(slab_mesh)),
                MeshMaterial3d(slab_mat),
                Transform::from_xyz(0.0, current_y, 0.0),
                FloorLevelMarker(story),
            ));
        });

        // 2. Spawn Stairwell Core & Staircase (connects up to next floor if not top)
        let stair_bottom = Vec3::new(stairwell_pos_2d.x, current_y, stairwell_pos_2d.y);
        let stair_top = Vec3::new(stairwell_pos_2d.x, current_y + story_h, stairwell_pos_2d.y + 2.8);

        let stair_room_ent = commands
            .spawn((
                Room {
                    room_id: room_counter,
                    floor_index: story,
                    room_type: RoomType::Stairwell,
                    center: stair_bottom + Vec3::new(0.0, 0.0, 1.4),
                    bounds_min: stairwell_pos_2d - Vec2::splat(2.0),
                    bounds_max: stairwell_pos_2d + Vec2::splat(2.0),
                },
                Transform::from_translation(stair_bottom),
                FloorLevelMarker(story),
                ChildOf(building_entity),
            ))
            .id();
        room_counter += 1;
        room_entities.push(stair_room_ent);

        if story + 1 < stories {
            // Spawn real 3D staircase from catalog connecting this floor to the next
            catalog.spawn_stairs(
                commands,
                building_entity,
                Transform::from_translation(stair_bottom),
                materials,
                story,
            );

            let staircase_ent = commands
                .spawn((
                    Staircase {
                        from_floor: story,
                        to_floor: story + 1,
                        bottom_pos: stair_bottom,
                        top_pos: stair_top,
                    },
                    Transform::from_translation(stair_bottom),
                    FloorLevelMarker(story),
                    ChildOf(building_entity),
                ))
                .id();
            staircase_entities.push(staircase_ent);
        }

        // 3. Generate Rooms & Partitions for this floor
        if story == 0 {
            // Ground Floor: Entrance Lobby + Corridor + 2 Rooms
            let lobby_ent = spawn_ground_lobby(
                commands,
                building_entity,
                lobby_pos,
                room_counter,
                catalog,
                materials,
            );
            room_counter += 1;
            room_entities.push(lobby_ent);

            // Doorway from street to lobby
            let exterior_door = commands
                .spawn((
                    Doorway {
                        from_room: None,
                        to_room: Some(lobby_ent),
                        world_pos: street_entrance_pos,
                        is_exterior: true,
                    },
                    Transform::from_translation(street_entrance_pos),
                    FloorLevelMarker(0),
                    ChildOf(building_entity),
                ))
                .id();
            doorway_entities.push(exterior_door);

            // Doorway from lobby to stairwell
            let lobby_stair_door = commands
                .spawn((
                    Doorway {
                        from_room: Some(lobby_ent),
                        to_room: Some(stair_room_ent),
                        world_pos: (lobby_pos + stair_bottom) * 0.5,
                        is_exterior: false,
                    },
                    Transform::from_translation((lobby_pos + stair_bottom) * 0.5),
                    FloorLevelMarker(0),
                    ChildOf(building_entity),
                ))
                .id();
            doorway_entities.push(lobby_stair_door);

            // Ground floor living / office suites
            spawn_side_rooms(
                commands,
                building_entity,
                polygon,
                current_y,
                story,
                &mut room_counter,
                &mut room_entities,
                &mut doorway_entities,
                stair_room_ent,
                catalog,
                materials,
                seed,
            );
        } else {
            // Upper Floor: Central Corridor + Apartments
            spawn_upper_apartments(
                commands,
                building_entity,
                polygon,
                current_y,
                story,
                &mut room_counter,
                &mut room_entities,
                &mut doorway_entities,
                stair_room_ent,
                stairwell_pos_2d,
                catalog,
                materials,
                seed,
            );
        }

        current_y += story_h;
    }

    GeneratedFloorPlan {
        total_rooms: room_entities.len(),
        room_entities,
        doorway_entities,
        staircase_entities,
        street_entrance_pos,
        lobby_pos,
    }
}

// ---------------------------------------------------------------------------
// ROOM & APARTMENT SPAWNING HELPERS
// ---------------------------------------------------------------------------

fn spawn_ground_lobby(
    commands: &mut Commands,
    building: Entity,
    lobby_pos: Vec3,
    room_id: u32,
    catalog: &ModelCatalog,
    mats: &BuildingMaterials,
) -> Entity {
    let lobby_ent = commands
        .spawn((
            Room {
                room_id,
                floor_index: 0,
                room_type: RoomType::EntranceLobby,
                center: lobby_pos,
                bounds_min: Vec2::new(lobby_pos.x - 2.5, lobby_pos.z - 2.5),
                bounds_max: Vec2::new(lobby_pos.x + 2.5, lobby_pos.z + 2.5),
            },
            Transform::from_translation(lobby_pos),
            FloorLevelMarker(0),
            ChildOf(building),
        ))
        .id();

    catalog.spawn_furniture_for_room(
        commands,
        building,
        RoomType::EntranceLobby,
        lobby_pos,
        Quat::IDENTITY,
        mats,
        0,
    );

    lobby_ent
}

fn spawn_side_rooms(
    commands: &mut Commands,
    building: Entity,
    polygon: &BuildingPolygon,
    current_y: f32,
    floor_idx: u32,
    room_counter: &mut u32,
    room_entities: &mut Vec<Entity>,
    doorway_entities: &mut Vec<Entity>,
    hallway_ent: Entity,
    catalog: &ModelCatalog,
    mats: &BuildingMaterials,
    _seed: u64,
) {
    let centroid = polygon.centroid();
    // Spawn left and right ground suites
    let offsets = [Vec2::new(-6.0, 0.0), Vec2::new(6.0, 0.0)];

    for (i, offset) in offsets.iter().enumerate() {
        let center_2d = centroid + *offset;
        let center_3d = Vec3::new(center_2d.x, current_y, center_2d.y);

        let room_type = if i == 0 { RoomType::LivingRoom } else { RoomType::Bedroom };
        let r_id = *room_counter;
        *room_counter += 1;

        let room_ent = commands
            .spawn((
                Room {
                    room_id: r_id,
                    floor_index: floor_idx,
                    room_type,
                    center: center_3d,
                    bounds_min: center_2d - Vec2::splat(3.0),
                    bounds_max: center_2d + Vec2::splat(3.0),
                },
                Transform::from_translation(center_3d),
                FloorLevelMarker(floor_idx),
                ChildOf(building),
            ))
            .id();
        room_entities.push(room_ent);

        // Furnish room with catalog models
        catalog.spawn_furniture_for_room(
            commands,
            building,
            room_type,
            center_3d,
            Quat::IDENTITY,
            mats,
            floor_idx,
        );

        // Doorway connecting to hallway
        let door_pos = (center_3d + Vec3::new(centroid.x, current_y, centroid.y)) * 0.5;
        let doorway = commands
            .spawn((
                Doorway {
                    from_room: Some(hallway_ent),
                    to_room: Some(room_ent),
                    world_pos: door_pos,
                    is_exterior: false,
                },
                Transform::from_translation(door_pos),
                FloorLevelMarker(floor_idx),
                ChildOf(building),
            ))
            .id();
        doorway_entities.push(doorway);

        // Catalog interior door model
        catalog.spawn_interior_door(
            commands,
            building,
            Transform::from_translation(door_pos),
            mats,
            floor_idx,
        );
    }
}

fn spawn_upper_apartments(
    commands: &mut Commands,
    building: Entity,
    polygon: &BuildingPolygon,
    current_y: f32,
    floor_idx: u32,
    room_counter: &mut u32,
    room_entities: &mut Vec<Entity>,
    doorway_entities: &mut Vec<Entity>,
    stair_ent: Entity,
    stair_pos: Vec2,
    catalog: &ModelCatalog,
    mats: &BuildingMaterials,
    _seed: u64,
) {
    let centroid = polygon.centroid();
    // 3 apartments on upper floors: Front-Left, Front-Right, Rear
    let room_defs = [
        (Vec2::new(-6.0, -4.0), RoomType::LivingRoom),
        (Vec2::new(6.0, -4.0), RoomType::LivingRoom),
        (Vec2::new(-6.0, 4.0), RoomType::Bedroom),
        (Vec2::new(6.0, 4.0), RoomType::Bedroom),
    ];

    for (offset, r_type) in room_defs {
        let center_2d = centroid + offset;
        let center_3d = Vec3::new(center_2d.x, current_y, center_2d.y);

        let r_id = *room_counter;
        *room_counter += 1;

        let room_ent = commands
            .spawn((
                Room {
                    room_id: r_id,
                    floor_index: floor_idx,
                    room_type: r_type,
                    center: center_3d,
                    bounds_min: center_2d - Vec2::splat(2.5),
                    bounds_max: center_2d + Vec2::splat(2.5),
                },
                Transform::from_translation(center_3d),
                FloorLevelMarker(floor_idx),
                ChildOf(building),
            ))
            .id();
        room_entities.push(room_ent);

        // Furniture
        catalog.spawn_furniture_for_room(
            commands,
            building,
            r_type,
            center_3d,
            Quat::IDENTITY,
            mats,
            floor_idx,
        );

        // Doorway
        let door_pos = (center_3d + Vec3::new(stair_pos.x, current_y, stair_pos.y)) * 0.5;
        let doorway = commands
            .spawn((
                Doorway {
                    from_room: Some(stair_ent),
                    to_room: Some(room_ent),
                    world_pos: door_pos,
                    is_exterior: false,
                },
                Transform::from_translation(door_pos),
                FloorLevelMarker(floor_idx),
                ChildOf(building),
            ))
            .id();
        doorway_entities.push(doorway);

        catalog.spawn_interior_door(
            commands,
            building,
            Transform::from_translation(door_pos),
            mats,
            floor_idx,
        );
    }
}

// ---------------------------------------------------------------------------
// 3D FLOOR SLAB MESH GENERATOR
// ---------------------------------------------------------------------------

fn create_polygon_slab_mesh(polygon: &BuildingPolygon, thickness: f32) -> Mesh {
    let tri_indices = polygon.triangulate();
    let n = polygon.vertices.len();

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Top face
    for &v in &polygon.vertices {
        positions.push([v.x, 0.0, v.y]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([v.x * 0.25, v.y * 0.25]);
    }
    for tri in &tri_indices {
        indices.push(tri[0]);
        indices.push(tri[1]);
        indices.push(tri[2]);
    }

    // Bottom face
    let bottom_offset = positions.len() as u32;
    for &v in &polygon.vertices {
        positions.push([v.x, -thickness, v.y]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([v.x * 0.25, v.y * 0.25]);
    }
    for tri in &tri_indices {
        // Reversed winding for bottom
        indices.push(bottom_offset + tri[0]);
        indices.push(bottom_offset + tri[2]);
        indices.push(bottom_offset + tri[1]);
    }

    // Side rim faces
    for i in 0..n {
        let next = (i + 1) % n;
        let base = positions.len() as u32;

        let p1 = polygon.vertices[i];
        let p2 = polygon.vertices[next];
        let normal = polygon.edge_outward_normal(i);

        positions.push([p1.x, 0.0, p1.y]);
        positions.push([p2.x, 0.0, p2.y]);
        positions.push([p2.x, -thickness, p2.y]);
        positions.push([p1.x, -thickness, p1.y]);

        for _ in 0..4 {
            normals.push([normal.x, 0.0, normal.y]);
            uvs.push([0.0, 0.0]);
        }

        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
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
