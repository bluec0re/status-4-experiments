use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::camera::RtsCamera;
use crate::road::{
    build_road_mesh, generate_bridge_pylons, generate_road_posts, RoadMeshMarker, RoadPylonMarker,
};
use crate::spline::{sample_spline, RoadWaypoint, SplineSample};
use crate::terrain::HeightmapData;
use crate::vegetation::{compute_tree_positions, VegetationMarker};

#[derive(Component)]
pub struct RoadPostMarker;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorTool {
    Add,
    SelectMove,
}

#[derive(Resource)]
pub struct EditorState {
    pub waypoints: Vec<RoadWaypoint>,
    pub selected_node: Option<usize>,
    pub hovered_node: Option<usize>,
    pub is_dragging: bool,
    pub tool: EditorTool,

    pub road_width: f32,
    pub height_offset: f32,
    pub dirty: bool,

    // Statistics
    pub total_length: f32,
    pub max_grade: f32,
    pub samples: Vec<SplineSample>,

    // Ride-along mode
    pub ride_along: bool,
    pub ride_dist: f32,
    pub ride_speed: f32,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            selected_node: None,
            hovered_node: None,
            is_dragging: false,
            tool: EditorTool::SelectMove,

            road_width: 8.0,
            height_offset: 0.18,
            dirty: true,

            total_length: 0.0,
            max_grade: 0.0,
            samples: Vec::new(),

            ride_along: false,
            ride_dist: 0.0,
            ride_speed: 18.0,
        }
    }
}

pub fn load_preset(state: &mut EditorState, preset_index: usize, heightmap: &HeightmapData) {
    state.waypoints.clear();
    state.selected_node = None;
    state.hovered_node = None;
    state.is_dragging = false;

    match preset_index {
        1 => {
            // Main Town Boulevard
            let pts = [
                Vec3::new(-110.0, 0.0, -60.0),
                Vec3::new(-60.0,  0.0, -30.0),
                Vec3::new(-10.0,  0.0, -10.0),
                Vec3::new( 35.0,  0.0,  15.0),
                Vec3::new( 75.0,  0.0,  50.0),
                Vec3::new( 115.0, 0.0,  85.0),
            ];
            for p in pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state.waypoints.push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), state.road_width));
            }
        }
        2 => {
            // River Crossing Expressway & Elevated Bridge
            let pts = [
                Vec3::new(-100.0, 0.0,  45.0),
                Vec3::new(-50.0,  0.0,  20.0),
                Vec3::new(  5.0,  0.0,   0.0),
                Vec3::new( 60.0,  0.0, -20.0),
                Vec3::new(110.0,  0.0, -45.0),
            ];
            for (i, p) in pts.iter().enumerate() {
                let mut y = heightmap.sample(p.x, p.z) + state.height_offset;
                if i == 2 {
                    y = y.max(13.8);
                } else if i == 1 || i == 3 {
                    y = y.max(12.0);
                }
                state.waypoints.push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), state.road_width + 1.2));
            }
        }
        3 => {
            // Suburban Ring Road & Plateau Climb
            let pts = [
                Vec3::new(-80.0, 0.0,  70.0),
                Vec3::new(-30.0, 0.0,  85.0),
                Vec3::new( 25.0, 0.0,  65.0),
                Vec3::new( 55.0, 0.0,  20.0),
                Vec3::new( 30.0, 0.0, -35.0),
                Vec3::new(-20.0, 0.0, -65.0),
                Vec3::new(-75.0, 0.0, -50.0),
            ];
            for p in pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state.waypoints.push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), state.road_width));
            }
        }
        _ => {}
    }

    state.dirty = true;
}

/// Ray vs Sphere collision test
fn ray_sphere_intersect(ray_origin: Vec3, ray_dir: Vec3, sphere_center: Vec3, radius: f32) -> Option<f32> {
    let m = ray_origin - sphere_center;
    let b = m.dot(ray_dir);
    let c = m.dot(m) - radius * radius;

    if c > 0.0 && b > 0.0 {
        return None;
    }

    let discr = b * b - c;
    if discr < 0.0 {
        return None;
    }

    let t = -b - discr.sqrt();
    if t > 0.0 {
        Some(t)
    } else {
        Some(-b + discr.sqrt())
    }
}

/// Handles editor inputs: selecting, dragging, adding, deleting, and tweaking waypoints
pub fn handle_editor_input(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    heightmap: Res<HeightmapData>,
    mut state: ResMut<EditorState>,
) {
    let (camera, cam_gt) = *camera_query;
    let cursor_pos = window.cursor_position();

    // Hotkeys for Presets
    if keys.just_pressed(KeyCode::Digit1) {
        load_preset(&mut state, 1, &heightmap);
    } else if keys.just_pressed(KeyCode::Digit2) {
        load_preset(&mut state, 2, &heightmap);
    } else if keys.just_pressed(KeyCode::Digit3) {
        load_preset(&mut state, 3, &heightmap);
    } else if keys.just_pressed(KeyCode::KeyC) {
        state.waypoints.clear();
        state.selected_node = None;
        state.hovered_node = None;
        state.dirty = true;
    }

    // Toggle Ride-along camera
    if keys.just_pressed(KeyCode::KeyF) {
        if !state.waypoints.is_empty() {
            state.ride_along = !state.ride_along;
            if state.ride_along {
                state.ride_dist = 0.0;
            }
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.ride_along = false;
    }

    // Road width adjustment
    if keys.just_pressed(KeyCode::BracketRight) {
        state.road_width = (state.road_width + 0.8).min(16.0);
        let w = state.road_width;
        for wp in state.waypoints.iter_mut() {
            wp.width = w;
        }
        state.dirty = true;
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        state.road_width = (state.road_width - 0.8).max(3.5);
        let w = state.road_width;
        for wp in state.waypoints.iter_mut() {
            wp.width = w;
        }
        state.dirty = true;
    }

    // Tool switching
    if keys.just_pressed(KeyCode::KeyT) {
        state.tool = match state.tool {
            EditorTool::SelectMove => EditorTool::Add,
            EditorTool::Add => EditorTool::SelectMove,
        };
    }

    // Elevation adjustment for selected node (R = up, V = down)
    if let Some(sel) = state.selected_node {
        if sel < state.waypoints.len() {
            if keys.pressed(KeyCode::KeyR) {
                state.waypoints[sel].pos.y += 0.15;
                state.dirty = true;
            }
            if keys.pressed(KeyCode::KeyV) {
                let ground_y = heightmap.sample(state.waypoints[sel].pos.x, state.waypoints[sel].pos.z);
                state.waypoints[sel].pos.y = (state.waypoints[sel].pos.y - 0.15).max(ground_y + 0.08);
                state.dirty = true;
            }
        }
    }

    // Delete selected node
    if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::KeyX) {
        if let Some(sel) = state.selected_node {
            if sel < state.waypoints.len() {
                state.waypoints.remove(sel);
                state.selected_node = None;
                state.hovered_node = None;
                state.is_dragging = false;
                state.dirty = true;
            }
        }
    }

    let Some(c_pos) = cursor_pos else { return; };
    let Ok(ray) = camera.viewport_to_world(cam_gt, c_pos) else { return; };

    // Find hovered waypoint sphere
    let mut closest_wp = None;
    let mut closest_dist = f32::MAX;
    let node_radius = 2.2f32;

    for (i, wp) in state.waypoints.iter().enumerate() {
        if let Some(t) = ray_sphere_intersect(ray.origin, ray.direction.into(), wp.pos, node_radius) {
            if t < closest_dist {
                closest_dist = t;
                closest_wp = Some(i);
            }
        }
    }

    state.hovered_node = closest_wp;

    // Mouse Press / Drag / Release logic
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(hovered) = state.hovered_node {
            state.selected_node = Some(hovered);
            state.is_dragging = true;
        } else {
            // Clicked on empty terrain
            if let Some(terrain_pt) = heightmap.raycast(ray.origin, ray.direction.into()) {
                if state.tool == EditorTool::Add {
                    // Append new waypoint
                    let pt = terrain_pt + Vec3::Y * state.height_offset;
                    let width = state.road_width;
                    state.waypoints.push(RoadWaypoint::new(pt, width));
                    let new_idx = state.waypoints.len() - 1;
                    state.selected_node = Some(new_idx);
                    state.is_dragging = true;
                    state.dirty = true;
                } else {
                    // Deselect
                    state.selected_node = None;
                }
            }
        }
    }

    if mouse_button.just_released(MouseButton::Left) {
        state.is_dragging = false;
    }

    // Dragging selected waypoint
    if state.is_dragging && mouse_button.pressed(MouseButton::Left) {
        if let Some(sel) = state.selected_node {
            if sel < state.waypoints.len() {
                if let Some(terrain_pt) = heightmap.raycast(ray.origin, ray.direction.into()) {
                    let keep_elevation = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
                    let y = if keep_elevation {
                        state.waypoints[sel].pos.y
                    } else {
                        heightmap.sample(terrain_pt.x, terrain_pt.z) + state.height_offset
                    };

                    state.waypoints[sel].pos = Vec3::new(terrain_pt.x, y, terrain_pt.z);
                    state.dirty = true;
                }
            }
        }
    }
}

/// Updates spline samples and rebuilds road mesh & bridge pylons whenever dirty
pub fn update_road_mesh_system(
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    mut meshes: ResMut<Assets<Mesh>>,
    road_material: Res<RoadMaterialHandle>,
    pylon_material: Res<PylonMaterialHandle>,
    post_mesh: Res<PostMeshHandle>,
    post_material: Res<PostMaterialHandle>,
    heightmap: Res<HeightmapData>,
    road_query: Query<Entity, With<RoadMeshMarker>>,
    pylon_query: Query<Entity, With<RoadPylonMarker>>,
    post_query: Query<Entity, With<RoadPostMarker>>,
) {
    if !state.dirty {
        return;
    }
    state.dirty = false;

    // 1. Recompute spline samples with terrain clearance
    let samples = sample_spline(&state.waypoints, 0.8, &heightmap);
    state.total_length = samples.last().map(|s| s.distance).unwrap_or(0.0);

    let mut max_gr = 0.0f32;
    for s in samples.iter() {
        if s.grade.abs() > max_gr {
            max_gr = s.grade.abs();
        }
    }
    state.max_grade = max_gr;
    state.samples = samples.clone();

    // 2. Clear old road mesh, pylons, and roadside posts
    for ent in road_query.iter() {
        commands.entity(ent).despawn();
    }
    for ent in pylon_query.iter() {
        commands.entity(ent).despawn();
    }
    for ent in post_query.iter() {
        commands.entity(ent).despawn();
    }

    if samples.len() < 2 {
        return;
    }

    // 3. Spawn new road mesh conformed to heightmap texture
    let road_mesh = build_road_mesh(&samples, &heightmap);
    let mesh_handle = meshes.add(road_mesh);

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(road_material.0.clone()),
        RoadMeshMarker,
    ));

    // 4. Spawn bridge pylons
    let pylons = generate_bridge_pylons(&samples, &heightmap);
    let cylinder_proto = meshes.add(Cylinder::new(1.1, 1.0));

    for (pos, height) in pylons {
        commands.spawn((
            Mesh3d(cylinder_proto.clone()),
            MeshMaterial3d(pylon_material.0.clone()),
            Transform::from_translation(pos).with_scale(Vec3::new(1.0, height, 1.0)),
            RoadPylonMarker,
        ));
    }

    // 5. Spawn roadside delineator posts
    let posts = generate_road_posts(&samples);
    for (pos, rot) in posts {
        commands.spawn((
            Mesh3d(post_mesh.0.clone()),
            MeshMaterial3d(post_material.0.clone()),
            Transform::from_translation(pos + Vec3::Y * 0.45).with_rotation(rot),
            RoadPostMarker,
        ));
    }
}

/// Updates tree clearing around road when road is modified
pub fn update_vegetation_system(
    mut commands: Commands,
    state: Res<EditorState>,
    tree_mesh_handle: Res<TreeMeshHandle>,
    tree_mat_handle: Res<TreeMaterialHandle>,
    heightmap: Res<HeightmapData>,
    trees_query: Query<Entity, With<VegetationMarker>>,
) {
    if !state.is_changed() && !state.dirty {
        return;
    }

    if state.is_added() || state.dirty {
        for ent in trees_query.iter() {
            commands.entity(ent).despawn();
        }

        let tree_positions = compute_tree_positions(&state.samples, &heightmap);
        for (pos, scale) in tree_positions {
            commands.spawn((
                Mesh3d(tree_mesh_handle.0.clone()),
                MeshMaterial3d(tree_mat_handle.0.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
                VegetationMarker,
            ));
        }
    }
}

/// Handles the Ride-Along camera mode (F key)
pub fn update_ride_along_camera(
    time: Res<Time>,
    mut state: ResMut<EditorState>,
    mut camera_query: Query<&mut Transform, With<RtsCamera>>,
) {
    if !state.ride_along || state.samples.is_empty() {
        return;
    }

    let Ok(mut cam_transform) = camera_query.single_mut() else { return; };
    let dt = time.delta_secs();

    state.ride_dist += state.ride_speed * dt;
    if state.ride_dist > state.total_length {
        state.ride_dist = 0.0;
    }

    let target_d = state.ride_dist;
    let mut curr_sample = &state.samples[0];
    for s in state.samples.iter() {
        if s.distance <= target_d {
            curr_sample = s;
        } else {
            break;
        }
    }

    let car_pos = curr_sample.pos + curr_sample.binormal * 0.7;
    let cam_pos = car_pos - curr_sample.tangent * 7.5 + curr_sample.binormal * 3.5;
    let look_at = car_pos + curr_sample.tangent * 15.0;

    cam_transform.translation = cam_pos;
    cam_transform.look_at(look_at, curr_sample.binormal);
}

/// Visual gizmos for waypoints, tangents, selection rings, and road center-line
pub fn draw_editor_gizmos(
    state: Res<EditorState>,
    heightmap: Res<HeightmapData>,
    mut gizmos: Gizmos,
) {
    if state.ride_along {
        return;
    }

    for (i, wp) in state.waypoints.iter().enumerate() {
        let is_selected = state.selected_node == Some(i);
        let is_hovered = state.hovered_node == Some(i);

        let (col, radius) = if is_selected {
            (Color::srgb(1.0, 0.25, 0.1), 1.6)
        } else if is_hovered {
            (Color::srgb(1.0, 0.85, 0.2), 1.4)
        } else {
            (Color::srgb(0.2, 0.85, 1.0), 1.1)
        };

        gizmos.sphere(Isometry3d::from_translation(wp.pos), radius, col);

        let ground_y = heightmap.sample(wp.pos.x, wp.pos.z);
        let ground_pt = Vec3::new(wp.pos.x, ground_y, wp.pos.z);
        gizmos.line(wp.pos, ground_pt, Color::srgba(col.to_srgba().red, col.to_srgba().green, col.to_srgba().blue, 0.6));

        gizmos.circle(Isometry3d::new(ground_pt + Vec3::Y * 0.05, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), radius * 1.5, col);
    }
}

#[derive(Resource)]
pub struct RoadMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub struct PylonMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub struct TreeMeshHandle(pub Handle<Mesh>);

#[derive(Resource)]
pub struct TreeMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub struct PostMeshHandle(pub Handle<Mesh>);

#[derive(Resource)]
pub struct PostMaterialHandle(pub Handle<StandardMaterial>);
