#![allow(dead_code)]

use bevy::prelude::*;
use crate::house::{Building, BuildingOccupant, FloorLevelMarker, Room};

pub struct RtsPlugin;

impl Plugin for RtsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RtsSelectionState>()
            .add_systems(Startup, setup_initial_rts_units)
            .add_systems(
                Update,
                (
                    handle_rts_selection,
                    rts_unit_movement,
                    update_unit_building_occupancy,
                    update_rts_selection_visuals,
                ),
            );
    }
}

/// Global selection state for player-controlled RTS units
#[derive(Resource, Default)]
pub struct RtsSelectionState {
    pub selected_unit: Option<Entity>,
}

/// Player-controlled RTS unit component
#[derive(Component, Debug)]
#[require(Transform, Visibility, BuildingOccupant)]
pub struct RtsUnit {
    pub unit_id: u32,
    pub name: String,
    pub color: Color,
    pub speed: f32,
    pub waypoints: Vec<Vec3>,
    pub current_target: Option<Vec3>,
}

/// Visual marker for unit selection ring indicator
#[derive(Component)]
pub struct SelectionRingMarker;

fn setup_initial_rts_units(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_mesh = meshes.add(Capsule3d::new(0.35, 1.1).mesh().build());
    let ring_mesh = meshes.add(Annulus::new(0.45, 0.60).mesh().build());

    let unit_configs = [
        (1, "Scout Hans", Color::srgb(0.2, 0.6, 1.0), Vec3::new(-5.0, 0.9, -15.0)),
        (2, "Commander Claire", Color::srgb(0.9, 0.3, 0.2), Vec3::new(0.0, 0.9, -16.0)),
        (3, "Engineer Leo", Color::srgb(0.2, 0.8, 0.4), Vec3::new(5.0, 0.9, -15.0)),
    ];

    for (id, name, color, start_pos) in unit_configs {
        let body_mat = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.5,
            metallic: 0.1,
            ..default()
        });

        let ring_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.9, 0.1, 0.8),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        let unit_entity = commands
            .spawn((
                RtsUnit {
                    unit_id: id,
                    name: name.into(),
                    color,
                    speed: 5.5,
                    waypoints: Vec::new(),
                    current_target: None,
                },
                Transform::from_translation(start_pos),
                Mesh3d(body_mesh.clone()),
                MeshMaterial3d(body_mat),
            ))
            .with_children(|b| {
                // Selection ring on ground beneath unit feet
                b.spawn((
                    SelectionRingMarker,
                    Mesh3d(ring_mesh.clone()),
                    MeshMaterial3d(ring_mat),
                    Transform::from_xyz(0.0, -0.88, 0.0)
                        * Transform::from_rotation(Quat::from_rotation_x(-core::f32::consts::FRAC_PI_2)),
                    Visibility::Hidden,
                ));
            })
            .id();

        // Default select unit 1
        if id == 1 {
            commands.insert_resource(RtsSelectionState {
                selected_unit: Some(unit_entity),
            });
        }
    }
}

/// Handle hotkeys (1, 2, 3) to select units
pub fn handle_rts_selection(
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<RtsSelectionState>,
    units: Query<(Entity, &RtsUnit)>,
) {
    let target_id = if keys.just_pressed(KeyCode::Digit1) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(3)
    } else {
        None
    };

    if let Some(id) = target_id {
        for (ent, unit) in &units {
            if unit.unit_id == id {
                selection.selected_unit = Some(ent);
                break;
            }
        }
    }
}

/// Move units towards their active waypoints
pub fn rts_unit_movement(
    time: Res<Time>,
    mut units: Query<(&mut Transform, &mut RtsUnit)>,
) {
    let dt = time.delta_secs();

    for (mut transform, mut unit) in &mut units {
        if unit.current_target.is_none() && !unit.waypoints.is_empty() {
            unit.current_target = Some(unit.waypoints.remove(0));
        }

        if let Some(target) = unit.current_target {
            let to_target = target - transform.translation;
            let dist = to_target.length();

            if dist < 0.25 {
                // Arrived at current waypoint, advance to next
                if !unit.waypoints.is_empty() {
                    unit.current_target = Some(unit.waypoints.remove(0));
                } else {
                    unit.current_target = None;
                }
            } else {
                let dir = to_target.normalize();
                transform.translation += dir * unit.speed * dt;

                // Rotate unit facing movement direction (in horizontal plane)
                let horizontal_dir = Vec2::new(dir.x, dir.z).normalize_or_zero();
                if horizontal_dir.length_squared() > 0.01 {
                    let angle = horizontal_dir.x.atan2(horizontal_dir.y);
                    transform.rotation = Quat::from_rotation_y(angle);
                }
            }
        }
    }
}

/// Detect which building, floor, and room the unit is currently inside
pub fn update_unit_building_occupancy(
    buildings: Query<(Entity, &Transform), With<Building>>,
    rooms: Query<(Entity, &Room, &FloorLevelMarker)>,
    mut units: Query<(&Transform, &mut BuildingOccupant), With<RtsUnit>>,
) {
    for (unit_tf, mut occupant) in &mut units {
        let pos = unit_tf.translation;
        let mut inside_any_room = false;

        for (room_ent, room, floor_marker) in &rooms {
            let min = room.bounds_min;
            let max = room.bounds_max;
            let margin = 0.5;

            let in_x = pos.x >= min.x - margin && pos.x <= max.x + margin;
            let in_z = pos.z >= min.y - margin && pos.z <= max.y + margin;
            let floor_y = floor_marker.0 as f32 * 3.2;
            let in_y = (pos.y - floor_y).abs() < 2.5;

            if in_x && in_z && in_y {
                occupant.current_floor = Some(floor_marker.0);
                occupant.current_room = Some(room_ent);
                inside_any_room = true;
                break;
            }
        }

        if !inside_any_room {
            // Check if outside or on street
            occupant.current_floor = None;
            occupant.current_room = None;
        }

        if occupant.current_building.is_none() {
            if let Some((b_ent, _)) = buildings.iter().next() {
                occupant.current_building = Some(b_ent);
            }
        }
    }
}

/// Update visual highlight on selected unit
pub fn update_rts_selection_visuals(
    selection: Res<RtsSelectionState>,
    units: Query<(Entity, &Children), With<RtsUnit>>,
    mut rings: Query<&mut Visibility, With<SelectionRingMarker>>,
) {
    for (unit_ent, children) in &units {
        let is_selected = selection.selected_unit == Some(unit_ent);
        for &child in children {
            if let Ok(mut vis) = rings.get_mut(child) {
                *vis = if is_selected {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RTS NAVIGATION PATH PLANNING HELPERS
// ---------------------------------------------------------------------------

/// Generate a multi-stage waypoint path for a unit to enter a building and navigate to a target room
pub fn plan_path_to_room(
    _unit_pos: Vec3,
    target_floor: u32,
    target_room_center: Vec3,
    exterior_entrance: Vec3,
    lobby_pos: Vec3,
    staircases: &[(u32, Vec3, Vec3)], // (from_floor, bottom_pos, top_pos)
) -> Vec<Vec3> {
    let mut waypoints = Vec::new();

    // 1. Walk from current unit position to exterior entrance door
    waypoints.push(exterior_entrance);

    // 2. Step through doorway into ground floor lobby
    waypoints.push(lobby_pos);

    // 3. If target is on upper floor, navigate stairs
    if target_floor > 0 {
        for floor in 0..target_floor {
            // Find staircase connecting floor -> floor + 1
            for &(from_f, bottom, top) in staircases {
                if from_f == floor {
                    waypoints.push(bottom);
                    waypoints.push(top);
                    break;
                }
            }
        }
    }

    // 4. Walk to final room destination on target floor
    waypoints.push(target_room_center + Vec3::new(0.0, 0.9, 0.0));
    waypoints
}

/// Generate a path to exit the building out onto the street
pub fn plan_path_to_street(
    _unit_pos: Vec3,
    current_floor: u32,
    exterior_entrance: Vec3,
    lobby_pos: Vec3,
    staircases: &[(u32, Vec3, Vec3)],
) -> Vec<Vec3> {
    let mut waypoints = Vec::new();

    // Climb down stairs if above ground
    if current_floor > 0 {
        for floor in (0..current_floor).rev() {
            for &(from_f, bottom, top) in staircases {
                if from_f == floor {
                    waypoints.push(top);
                    waypoints.push(bottom);
                    break;
                }
            }
        }
    }

    // Pass through lobby
    waypoints.push(lobby_pos);
    // Exit through entrance door
    waypoints.push(exterior_entrance);
    // Walk out to street
    waypoints.push(exterior_entrance + Vec3::new(0.0, 0.0, -4.0));
    waypoints
}
