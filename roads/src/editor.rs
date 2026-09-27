use crate::camera::RtsCamera;
use crate::road::{
    build_junction_mesh, build_road_mesh, create_junction_texture, create_road_texture,
    detect_junctions, generate_bridge_pylons, generate_road_posts_filtered,
    split_street_samples, JunctionElevationMode, JunctionStyle, RoadMeshMarker, RoadPylonMarker, Street,
};
use crate::spline::{sample_spline, RoadWaypoint, SplineSample};
use crate::terrain::HeightmapData;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Message type decoupling UI button clicks and hotkeys from editor state mutations
#[derive(Message, Clone, Debug)]
pub enum EditorAction {
    LoadPreset(usize),
    ClearWaypoints,
    ToggleRideAlong,
    ToggleTool,
    AdjustRoadWidth(f32),
    SetLanes(usize),
    ToggleWidthMode,
    DeleteSelectedNode,
    AdjustNodeElevation(f32),
    NewStreet,
    JoinStreets,
    CycleActiveStreet,
    CycleJunctionStyle,
    ToggleJunctionElevationMode,
}

/// System sets establishing strict ordering across input, action processing, mesh rebuilding, and post-update
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditorSet {
    Input,
    ApplyActions,
    MeshRebuild,
    PostUpdate,
}

pub struct RoadEditorPlugin;

impl Plugin for RoadEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EditorAction>()
            .init_resource::<EditorState>()
            .add_systems(Startup, setup_materials_and_assets)
            .configure_sets(
                Update,
                (
                    EditorSet::Input,
                    EditorSet::ApplyActions.after(EditorSet::Input),
                    EditorSet::MeshRebuild.after(EditorSet::ApplyActions),
                    EditorSet::PostUpdate.after(EditorSet::MeshRebuild),
                ),
            )
            .add_systems(
                Update,
                (
                    handle_editor_input.in_set(EditorSet::Input),
                    apply_editor_actions.in_set(EditorSet::ApplyActions),
                    (update_road_mesh_system, update_ride_along_camera)
                        .in_set(EditorSet::MeshRebuild),
                    draw_editor_gizmos.in_set(EditorSet::PostUpdate),
                ),
            );
    }
}

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct RoadPostMarker;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorTool {
    SelectMove,
    Add,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WidthMode {
    Lanes,
    Seamless,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaneSetting {
    Auto,
    Fixed(usize),
}

#[derive(Resource)]
pub struct EditorState {
    pub waypoints: Vec<RoadWaypoint>,
    pub streets: Vec<Street>,
    pub active_street_idx: usize,

    pub selected_node: Option<usize>,
    pub hovered_node: Option<usize>,
    pub snap_target_node: Option<usize>,
    pub is_dragging: bool,
    pub tool: EditorTool,

    pub road_width: f32,
    pub width_mode: WidthMode,
    pub lane_setting: LaneSetting,
    pub height_offset: f32,
    pub dirty: bool,

    // Statistics
    pub total_length: f32,
    pub max_grade: f32,
    pub samples: Vec<SplineSample>,
    pub junctions_count: usize,
    pub junction_style: JunctionStyle,
    pub junction_elevation_mode: JunctionElevationMode,

    // Dynamic Tree Avoidance Tracking
    pub road_version: usize,
    pub displaced_trees_count: usize,
    pub total_trees_count: usize,

    // Ride-along mode
    pub ride_along: bool,
    pub ride_dist: f32,
    pub ride_speed: f32,
}

impl EditorState {
    pub fn effective_lanes(&self) -> usize {
        match self.lane_setting {
            LaneSetting::Fixed(l) => l,
            LaneSetting::Auto => {
                if self.road_width < 5.8 {
                    1
                } else if self.road_width < 11.2 {
                    2
                } else {
                    4
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn active_street(&self) -> Option<&Street> {
        self.streets.get(self.active_street_idx)
    }

    #[allow(dead_code)]
    pub fn active_street_mut(&mut self) -> Option<&mut Street> {
        self.streets.get_mut(self.active_street_idx)
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            streets: vec![Street::new(1, "Main Street", 8.0, 2)],
            active_street_idx: 0,
            selected_node: None,
            hovered_node: None,
            snap_target_node: None,
            is_dragging: false,
            tool: EditorTool::SelectMove,

            road_width: 8.0,
            width_mode: WidthMode::Lanes,
            lane_setting: LaneSetting::Fixed(2),
            height_offset: 0.18,
            dirty: true,

            total_length: 0.0,
            max_grade: 0.0,
            samples: Vec::new(),
            junctions_count: 0,
            junction_style: JunctionStyle::default(),
            junction_elevation_mode: JunctionElevationMode::default(),

            road_version: 0,
            displaced_trees_count: 0,
            total_trees_count: 0,

            ride_along: false,
            ride_dist: 0.0,
            ride_speed: 18.0,
        }
    }
}

pub fn load_preset(state: &mut EditorState, preset_index: usize, heightmap: &HeightmapData) {
    state.waypoints.clear();
    state.streets.clear();
    state.active_street_idx = 0;
    state.selected_node = None;
    state.hovered_node = None;
    state.snap_target_node = None;
    state.is_dragging = false;

    match preset_index {
        1 => {
            // Main Town Boulevard - 2 Lanes (8.0m)
            state.road_width = 8.0;
            state.lane_setting = LaneSetting::Fixed(2);
            let pts = [
                Vec3::new(-110.0, 0.0, -60.0),
                Vec3::new(-60.0, 0.0, -30.0),
                Vec3::new(-10.0, 0.0, -10.0),
                Vec3::new(35.0, 0.0, 15.0),
                Vec3::new(75.0, 0.0, 50.0),
                Vec3::new(115.0, 0.0, 85.0),
            ];
            for p in pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state
                    .waypoints
                    .push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), state.road_width));
            }
            state.streets.push(Street {
                id: 1,
                name: "Main Boulevard".to_string(),
                node_indices: (0..pts.len()).collect(),
                road_width: 8.0,
                lanes: 2,
            });
        }
        2 => {
            // River Crossing Expressway & Elevated Bridge - 4 Lanes (15.0m)
            state.road_width = 15.0;
            state.lane_setting = LaneSetting::Fixed(4);
            let pts = [
                Vec3::new(-100.0, 0.0, 45.0),
                Vec3::new(-50.0, 0.0, 20.0),
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(60.0, 0.0, -20.0),
                Vec3::new(110.0, 0.0, -45.0),
            ];
            for (i, p) in pts.iter().enumerate() {
                let mut y = heightmap.sample(p.x, p.z) + state.height_offset;
                if i == 2 {
                    y = y.max(13.8);
                } else if i == 1 || i == 3 {
                    y = y.max(12.0);
                }
                state
                    .waypoints
                    .push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), state.road_width));
            }
            state.streets.push(Street {
                id: 1,
                name: "River Expressway".to_string(),
                node_indices: (0..pts.len()).collect(),
                road_width: 15.0,
                lanes: 4,
            });
        }
        3 => {
            // Suburban Ring Road & Plateau Climb - 2 Lanes (8.0m)
            state.road_width = 8.0;
            state.lane_setting = LaneSetting::Fixed(2);
            let pts = [
                Vec3::new(-80.0, 0.0, 70.0),
                Vec3::new(-30.0, 0.0, 85.0),
                Vec3::new(25.0, 0.0, 65.0),
                Vec3::new(55.0, 0.0, 20.0),
                Vec3::new(30.0, 0.0, -35.0),
                Vec3::new(-20.0, 0.0, -65.0),
                Vec3::new(-75.0, 0.0, -50.0),
            ];
            for p in pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state
                    .waypoints
                    .push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), state.road_width));
            }
            state.streets.push(Street {
                id: 1,
                name: "Suburban Loop".to_string(),
                node_indices: (0..pts.len()).collect(),
                road_width: 8.0,
                lanes: 2,
            });
        }
        4 => {
            // Town Center & Junctions (3 Joined Streets: 4-Way Crossroads & 3-Way T-Junction)
            state.road_width = 8.0;
            state.lane_setting = LaneSetting::Fixed(2);

            // Street 1 (Main Avenue, 15m, 4 lanes): East-West corridor
            let main_pts = [
                Vec3::new(-105.0, 0.0, -10.0), // 0
                Vec3::new(-45.0, 0.0, -10.0),  // 1 (T-Junction with Street 3)
                Vec3::new(10.0, 0.0, -10.0),   // 2 (4-way Crossroads with Street 2)
                Vec3::new(65.0, 0.0, -10.0),   // 3
                Vec3::new(115.0, 0.0, -10.0),  // 4
            ];
            for p in main_pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state
                    .waypoints
                    .push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), 15.0));
            }

            // Street 2 (North-South Cross Street, 8m, 2 lanes):
            // Intersects Main Avenue at node 2
            let north_pts = [
                Vec3::new(10.0, 0.0, -85.0),  // 5
                Vec3::new(10.0, 0.0, -45.0),  // 6
            ];
            for p in north_pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state
                    .waypoints
                    .push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), 8.0));
            }
            let south_pts = [
                Vec3::new(10.0, 0.0, 30.0),   // 7
                Vec3::new(10.0, 0.0, 80.0),   // 8
            ];
            for p in south_pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state
                    .waypoints
                    .push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), 8.0));
            }

            // Street 3 (Hillside Spur / T-junction, 8m, 2 lanes):
            // Branches from node 1 northward
            let spur_pts = [
                Vec3::new(-45.0, 0.0, 35.0),  // 9
                Vec3::new(-65.0, 0.0, 75.0),  // 10
            ];
            for p in spur_pts {
                let y = heightmap.sample(p.x, p.z) + state.height_offset;
                state
                    .waypoints
                    .push(RoadWaypoint::new(Vec3::new(p.x, y, p.z), 8.0));
            }

            // Setup streets and connections:
            state.streets.push(Street {
                id: 1,
                name: "Main Avenue".to_string(),
                node_indices: vec![0, 1, 2, 3, 4],
                road_width: 15.0,
                lanes: 4,
            });

            // Street 2 connects through node 2 (Crossroads!):
            state.streets.push(Street {
                id: 2,
                name: "Cross Street".to_string(),
                node_indices: vec![5, 6, 2, 7, 8],
                road_width: 8.0,
                lanes: 2,
            });

            // Street 3 branches from node 1 (T-Junction!):
            state.streets.push(Street {
                id: 3,
                name: "Hillside Spur".to_string(),
                node_indices: vec![1, 9, 10],
                road_width: 8.0,
                lanes: 2,
            });

            state.active_street_idx = 0;
        }
        _ => {}
    }

    state.dirty = true;
}

/// Ray vs Sphere collision test
fn ray_sphere_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    sphere_center: Vec3,
    radius: f32,
) -> Option<f32> {
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

/// Handles editor inputs: emits EditorAction messages and tracks mouse raycast dragging
pub fn handle_editor_input(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    heightmap: Res<HeightmapData>,
    mut action_writer: MessageWriter<EditorAction>,
    mut state: ResMut<EditorState>,
) {
    let (camera, cam_gt) = *camera_query;
    let cursor_pos = window.cursor_position();

    // Hotkeys dispatched as decoupled messages
    if keys.just_pressed(KeyCode::Digit1) {
        action_writer.write(EditorAction::LoadPreset(1));
    } else if keys.just_pressed(KeyCode::Digit2) {
        action_writer.write(EditorAction::LoadPreset(2));
    } else if keys.just_pressed(KeyCode::Digit3) {
        action_writer.write(EditorAction::LoadPreset(3));
    } else if keys.just_pressed(KeyCode::Digit4) {
        action_writer.write(EditorAction::LoadPreset(4));
    } else if keys.just_pressed(KeyCode::KeyC) {
        action_writer.write(EditorAction::ClearWaypoints);
    } else if keys.just_pressed(KeyCode::KeyN) {
        action_writer.write(EditorAction::NewStreet);
    } else if keys.just_pressed(KeyCode::KeyJ) {
        action_writer.write(EditorAction::JoinStreets);
    } else if keys.just_pressed(KeyCode::Tab) {
        action_writer.write(EditorAction::CycleActiveStreet);
    } else if keys.just_pressed(KeyCode::KeyK) {
        action_writer.write(EditorAction::CycleJunctionStyle);
    } else if keys.just_pressed(KeyCode::KeyP) {
        action_writer.write(EditorAction::ToggleJunctionElevationMode);
    }

    // Toggle Ride-along camera
    if keys.just_pressed(KeyCode::KeyF) {
        action_writer.write(EditorAction::ToggleRideAlong);
    }
    if keys.just_pressed(KeyCode::Escape) && state.ride_along {
        action_writer.write(EditorAction::ToggleRideAlong);
    }

    // Road width adjustment: [ decreases, ] increases
    if keys.just_pressed(KeyCode::BracketRight) {
        action_writer.write(EditorAction::AdjustRoadWidth(0.5));
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        action_writer.write(EditorAction::AdjustRoadWidth(-0.5));
    }

    // Toggle Width Mode: Lanes vs Seamless (L key)
    if keys.just_pressed(KeyCode::KeyL) {
        action_writer.write(EditorAction::ToggleWidthMode);
    }

    // Tool switching
    if keys.just_pressed(KeyCode::KeyT) {
        action_writer.write(EditorAction::ToggleTool);
    }

    // Elevation adjustment for selected node (R = up, V = down)
    if keys.pressed(KeyCode::KeyR) {
        action_writer.write(EditorAction::AdjustNodeElevation(0.15));
    }
    if keys.pressed(KeyCode::KeyV) {
        action_writer.write(EditorAction::AdjustNodeElevation(-0.15));
    }

    // Delete selected node
    if keys.just_pressed(KeyCode::Delete)
        || keys.just_pressed(KeyCode::Backspace)
        || keys.just_pressed(KeyCode::KeyX)
    {
        action_writer.write(EditorAction::DeleteSelectedNode);
    }

    let Some(c_pos) = cursor_pos else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(cam_gt, c_pos) else {
        return;
    };

    // Find hovered waypoint sphere
    let mut closest_wp = None;
    let mut closest_dist = f32::MAX;
    let node_radius = 2.4f32;

    for (i, wp) in state.waypoints.iter().enumerate() {
        if let Some(t) = ray_sphere_intersect(ray.origin, ray.direction.into(), wp.pos, node_radius)
            && t < closest_dist
        {
            closest_dist = t;
            closest_wp = Some(i);
        }
    }

    state.hovered_node = closest_wp;

    // Mouse Press / Drag / Release logic
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(hovered) = state.hovered_node {
            if state.tool == EditorTool::Add {
                // JOIN OR BRANCH!
                if state.streets.is_empty() {
                    let rw = state.road_width;
                    let el = state.effective_lanes();
                    state.streets.push(Street::new(1, "Main Street", rw, el));
                    state.active_street_idx = 0;
                }

                let active_idx = state.active_street_idx;
                if state.streets[active_idx].node_indices.is_empty() {
                    // Branch from this existing node into a new street
                    state.streets[active_idx].node_indices.push(hovered);
                    state.selected_node = Some(hovered);
                    state.dirty = true;
                } else if state.streets[active_idx].node_indices.last() != Some(&hovered) {
                    // Join active street to this existing node -> creates a junction!
                    state.streets[active_idx].node_indices.push(hovered);
                    state.selected_node = Some(hovered);
                    state.dirty = true;
                }
            } else {
                state.selected_node = Some(hovered);
                state.is_dragging = true;
                // Switch active street to whichever street contains this node
                let found_street = state.streets.iter().enumerate().find_map(|(s_idx, s)| {
                    if s.node_indices.contains(&hovered) {
                        Some((s_idx, s.road_width, s.lanes))
                    } else {
                        None
                    }
                });
                if let Some((s_idx, rw, lanes)) = found_street {
                    state.active_street_idx = s_idx;
                    state.road_width = rw;
                    state.lane_setting = LaneSetting::Fixed(lanes);
                }
            }
        } else if let Some(terrain_pt) = heightmap.raycast(ray.origin, ray.direction.into()) {
            // Clicked on empty terrain
            if state.tool == EditorTool::Add {
                let pt = terrain_pt + Vec3::Y * state.height_offset;
                let width = state.road_width;
                state.waypoints.push(RoadWaypoint::new(pt, width));
                let new_idx = state.waypoints.len() - 1;

                if state.streets.is_empty() {
                    let rw = state.road_width;
                    let el = state.effective_lanes();
                    state.streets.push(Street::new(1, "Main Street", rw, el));
                    state.active_street_idx = 0;
                }
                let active_idx = state.active_street_idx;
                state.streets[active_idx].node_indices.push(new_idx);
                state.selected_node = Some(new_idx);
                state.is_dragging = true;
                state.dirty = true;
            } else {
                state.selected_node = None;
            }
        }
    }

    // Dragging selected waypoint
    if state.is_dragging
        && mouse_button.pressed(MouseButton::Left)
        && let Some(sel) = state.selected_node
        && sel < state.waypoints.len()
        && let Some(terrain_pt) = heightmap.raycast(ray.origin, ray.direction.into())
    {
        let keep_elevation = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let y = if keep_elevation {
            state.waypoints[sel].pos.y
        } else {
            heightmap.sample(terrain_pt.x, terrain_pt.z) + state.height_offset
        };

        state.waypoints[sel].pos = Vec3::new(terrain_pt.x, y, terrain_pt.z);
        state.dirty = true;

        // Check magnetic snap to other nearby waypoints (for join-on-drop)
        let mut snap_cand = None;
        let snap_threshold = 3.6f32;
        for (i, wp) in state.waypoints.iter().enumerate() {
            if i != sel && wp.pos.distance(state.waypoints[sel].pos) < snap_threshold {
                snap_cand = Some(i);
                break;
            }
        }
        state.snap_target_node = snap_cand;
    }

    if mouse_button.just_released(MouseButton::Left) {
        state.is_dragging = false;

        // If dropped near another node with snap target, merge & join into a junction!
        if let Some(sel) = state.selected_node
            && let Some(target) = state.snap_target_node
            && sel != target
            && sel < state.waypoints.len()
            && target < state.waypoints.len()
        {
            // Replace references to sel with target across all streets
            for s in state.streets.iter_mut() {
                for idx in s.node_indices.iter_mut() {
                    if *idx == sel {
                        *idx = target;
                    }
                }
                s.node_indices.dedup();
            }

            // Remove sel from state.waypoints
            state.waypoints.remove(sel);
            for s in state.streets.iter_mut() {
                for idx in s.node_indices.iter_mut() {
                    if *idx > sel {
                        *idx -= 1;
                    }
                }
            }
            let new_sel = if target > sel { target - 1 } else { target };
            state.selected_node = Some(new_sel);
            state.snap_target_node = None;
            state.dirty = true;
        } else {
            state.snap_target_node = None;
        }
    }
}

/// Applies decoupled EditorAction messages to EditorState
pub fn apply_editor_actions(
    mut actions: MessageReader<EditorAction>,
    mut state: ResMut<EditorState>,
    heightmap: Res<HeightmapData>,
) {
    for action in actions.read() {
        match action {
            EditorAction::LoadPreset(idx) => {
                load_preset(&mut state, *idx, &heightmap);
            }
            EditorAction::ClearWaypoints => {
                state.waypoints.clear();
                state.streets.clear();
                let rw = state.road_width;
                let el = state.effective_lanes();
                state.streets.push(Street::new(1, "Main Street", rw, el));
                state.active_street_idx = 0;
                state.selected_node = None;
                state.hovered_node = None;
                state.snap_target_node = None;
                state.is_dragging = false;
                state.dirty = true;
            }
            EditorAction::NewStreet => {
                let next_id = state.streets.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                let rw = state.road_width;
                let el = state.effective_lanes();
                let new_street = Street {
                    id: next_id,
                    name: format!("Street {}", next_id),
                    node_indices: Vec::new(),
                    road_width: rw,
                    lanes: el,
                };
                state.streets.push(new_street);
                state.active_street_idx = state.streets.len() - 1;
                state.tool = EditorTool::Add;
                state.dirty = true;
            }
            EditorAction::CycleActiveStreet => {
                if !state.streets.is_empty() {
                    state.active_street_idx = (state.active_street_idx + 1) % state.streets.len();
                    state.road_width = state.streets[state.active_street_idx].road_width;
                    state.lane_setting = LaneSetting::Fixed(state.streets[state.active_street_idx].lanes);
                    if let Some(&first_node) = state.streets[state.active_street_idx].node_indices.first() {
                        state.selected_node = Some(first_node);
                    }
                    state.dirty = true;
                }
            }
            EditorAction::JoinStreets => {
                if let Some(sel) = state.selected_node
                    && sel < state.waypoints.len()
                {
                    let sel_pos = state.waypoints[sel].pos;
                    let mut best_target = None;
                    let mut best_dist = 25.0f32;

                    for (i, wp) in state.waypoints.iter().enumerate() {
                        if i == sel {
                            continue;
                        }
                        let d = sel_pos.distance(wp.pos);
                        if d < best_dist {
                            best_dist = d;
                            best_target = Some(i);
                        }
                    }

                    if let Some(target) = best_target
                        && state.active_street_idx < state.streets.len()
                        && !state.streets[state.active_street_idx].node_indices.contains(&target)
                    {
                        let active_idx = state.active_street_idx;
                        state.streets[active_idx].node_indices.push(target);
                        state.dirty = true;
                    }
                }
            }
            EditorAction::CycleJunctionStyle => {
                state.junction_style = state.junction_style.next();
                state.dirty = true;
            }
            EditorAction::ToggleJunctionElevationMode => {
                state.junction_elevation_mode = state.junction_elevation_mode.toggle();
                state.dirty = true;
            }
            EditorAction::ToggleRideAlong => {
                if !state.waypoints.is_empty() {
                    state.ride_along = !state.ride_along;
                    if state.ride_along {
                        state.ride_dist = 0.0;
                    }
                }
            }
            EditorAction::ToggleTool => {
                state.tool = match state.tool {
                    EditorTool::SelectMove => EditorTool::Add,
                    EditorTool::Add => EditorTool::SelectMove,
                };
            }
            EditorAction::SetLanes(lanes) => {
                let target_width = match lanes {
                    1 => 4.0,
                    2 => 8.0,
                    4 => 15.0,
                    _ => 8.0,
                };
                state.road_width = target_width;
                state.lane_setting = LaneSetting::Fixed(*lanes);
                let active_idx = state.active_street_idx;
                if active_idx < state.streets.len() {
                    state.streets[active_idx].road_width = target_width;
                    state.streets[active_idx].lanes = *lanes;
                    let node_indices = state.streets[active_idx].node_indices.clone();
                    for idx in node_indices {
                        if idx < state.waypoints.len() {
                            state.waypoints[idx].width = target_width;
                        }
                    }
                }
                state.dirty = true;
            }
            EditorAction::ToggleWidthMode => {
                state.width_mode = match state.width_mode {
                    WidthMode::Lanes => WidthMode::Seamless,
                    WidthMode::Seamless => WidthMode::Lanes,
                };
                if state.width_mode == WidthMode::Seamless {
                    state.lane_setting = LaneSetting::Auto;
                } else {
                    let l = state.effective_lanes();
                    state.lane_setting = LaneSetting::Fixed(l);
                    state.road_width = match l {
                        1 => 4.0,
                        2 => 8.0,
                        4 => 15.0,
                        _ => 8.0,
                    };
                    let w = state.road_width;
                    let active_idx = state.active_street_idx;
                    if active_idx < state.streets.len() {
                        state.streets[active_idx].road_width = w;
                        state.streets[active_idx].lanes = l;
                        let node_indices = state.streets[active_idx].node_indices.clone();
                        for idx in node_indices {
                            if idx < state.waypoints.len() {
                                state.waypoints[idx].width = w;
                            }
                        }
                    }
                }
                state.dirty = true;
            }
            EditorAction::AdjustRoadWidth(delta) => {
                match state.width_mode {
                    WidthMode::Lanes => {
                        let current_lanes = state.effective_lanes();
                        let new_lanes = if *delta > 0.0 {
                            match current_lanes {
                                1 => 2,
                                2 => 4,
                                _ => 4,
                            }
                        } else {
                            match current_lanes {
                                4 => 2,
                                2 => 1,
                                _ => 1,
                            }
                        };
                        let target_width = match new_lanes {
                            1 => 4.0,
                            2 => 8.0,
                            4 => 15.0,
                            _ => 8.0,
                        };
                        state.road_width = target_width;
                        state.lane_setting = LaneSetting::Fixed(new_lanes);
                    }
                    WidthMode::Seamless => {
                        state.road_width = (state.road_width + delta).clamp(3.5, 18.0);
                        state.lane_setting = LaneSetting::Auto;
                    }
                }
                let w = state.road_width;
                let l = state.effective_lanes();
                let active_idx = state.active_street_idx;
                if active_idx < state.streets.len() {
                    state.streets[active_idx].road_width = w;
                    state.streets[active_idx].lanes = l;
                    let node_indices = state.streets[active_idx].node_indices.clone();
                    for idx in node_indices {
                        if idx < state.waypoints.len() {
                            state.waypoints[idx].width = w;
                        }
                    }
                }
                state.dirty = true;
            }
            EditorAction::DeleteSelectedNode => {
                if let Some(sel) = state.selected_node
                    && sel < state.waypoints.len()
                {
                    state.waypoints.remove(sel);
                    for s in state.streets.iter_mut() {
                        s.node_indices.retain(|&idx| idx != sel);
                        for idx in s.node_indices.iter_mut() {
                            if *idx > sel {
                                *idx -= 1;
                            }
                        }
                    }
                    state.streets.retain(|s| !s.node_indices.is_empty());
                    if state.streets.is_empty() {
                        let rw = state.road_width;
                        let el = state.effective_lanes();
                        state.streets.push(Street::new(1, "Main Street", rw, el));
                    }
                    state.active_street_idx = state.active_street_idx.min(state.streets.len() - 1);
                    state.selected_node = None;
                    state.hovered_node = None;
                    state.snap_target_node = None;
                    state.is_dragging = false;
                    state.dirty = true;
                }
            }
            EditorAction::AdjustNodeElevation(delta) => {
                if let Some(sel) = state.selected_node
                    && sel < state.waypoints.len()
                {
                    let ground_y =
                        heightmap.sample(state.waypoints[sel].pos.x, state.waypoints[sel].pos.z);
                    state.waypoints[sel].pos.y =
                        (state.waypoints[sel].pos.y + delta).max(ground_y + 0.08);
                    state.dirty = true;
                }
            }
        }
    }
}

/// Updates spline samples, road texture, road mesh, bridge pylons & roadside posts whenever dirty
pub fn update_road_mesh_system(
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    road_material: Res<RoadMaterialHandle>,
    road_texture_handle: Res<RoadTextureImageHandle>,
    junction_material: Res<JunctionMaterialHandle>,
    junction_texture_handle: Res<JunctionTextureImageHandle>,
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

    // 0. Update road and junction textures in-place to adjust to road width, lanes, and junction style
    if let Some(mut img) = images.get_mut(&road_texture_handle.0) {
        *img = create_road_texture(state.road_width, state.effective_lanes());
    }
    if let Some(mut img) = images.get_mut(&junction_texture_handle.0) {
        *img = create_junction_texture(state.junction_style);
    }

    // 1. Clear old road mesh, pylons, and roadside posts
    for ent in road_query.iter() {
        commands.entity(ent).despawn();
    }
    for ent in pylon_query.iter() {
        commands.entity(ent).despawn();
    }
    for ent in post_query.iter() {
        commands.entity(ent).despawn();
    }

    // 2. Detect junctions across all streets and waypoints
    let mut junctions = detect_junctions(&state.waypoints, &state.streets);
    state.junctions_count = junctions.len();

    let mut all_samples: Vec<SplineSample> = Vec::new();
    let mut total_len = 0.0f32;
    let mut max_gr = 0.0f32;

    let cylinder_proto = meshes.add(Cylinder::new(1.1, 1.0));
    let pylon_mat = pylon_material.0.clone();
    let p_mesh = post_mesh.0.clone();
    let p_mat = post_material.0.clone();

    // 3. Build road ribbon meshes and posts for each street
    for (street_idx, street) in state.streets.iter().enumerate() {
        if street.node_indices.len() < 2 {
            continue;
        }

        let street_wps: Vec<RoadWaypoint> = street
            .node_indices
            .iter()
            .filter_map(|&idx| state.waypoints.get(idx).copied())
            .collect();

        if street_wps.len() < 2 {
            continue;
        }

        let samples = sample_spline(&street_wps, 0.8, &heightmap);
        if samples.len() < 2 {
            continue;
        }

        total_len += samples.last().map(|s| s.distance).unwrap_or(0.0);
        for s in samples.iter() {
            if s.grade.abs() > max_gr {
                max_gr = s.grade.abs();
            }
        }

        // Split road spline into segments outside junctions and attach boundary mouth samples to junctions
        let segments = split_street_samples(street_idx, &samples, &mut junctions, state.junction_elevation_mode);
        for segment in segments {
            let road_mesh = build_road_mesh(&segment, &heightmap);
            let mesh_handle = meshes.add(road_mesh);

            commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(road_material.0.clone()),
                RoadMeshMarker,
            ));
        }

        // Batch spawn bridge pylons (scaled to street width)
        let pylons = generate_bridge_pylons(&samples, &heightmap);
        let pylon_radius = (street.road_width * 0.10).clamp(0.9, 2.2);

        let cyl_clone = cylinder_proto.clone();
        let p_mat_clone = pylon_mat.clone();
        commands.spawn_batch(pylons.into_iter().map(move |(pos, height)| {
            (
                Mesh3d(cyl_clone.clone()),
                MeshMaterial3d(p_mat_clone.clone()),
                Transform::from_translation(pos).with_scale(Vec3::new(
                    pylon_radius,
                    height,
                    pylon_radius,
                )),
                RoadPylonMarker,
            )
        }));

        // Batch spawn roadside posts, filtering out posts inside junctions
        let posts = generate_road_posts_filtered(&samples, &junctions);
        let post_mesh_clone = p_mesh.clone();
        let post_mat_clone = p_mat.clone();

        commands.spawn_batch(posts.into_iter().map(move |(pos, rot)| {
            (
                Mesh3d(post_mesh_clone.clone()),
                MeshMaterial3d(post_mat_clone.clone()),
                Transform::from_translation(pos + Vec3::Y * 0.45).with_rotation(rot),
                RoadPostMarker,
            )
        }));

        all_samples.extend(samples);
    }

    // 4. Build junction intersection meshes with dedicated junction material
    for junction in &junctions {
        if let Some(j_mesh) = build_junction_mesh(junction, &state.waypoints, &heightmap, state.junction_style, state.junction_elevation_mode) {
            let j_handle = meshes.add(j_mesh);
            commands.spawn((
                Mesh3d(j_handle),
                MeshMaterial3d(junction_material.0.clone()),
                RoadMeshMarker,
            ));
        }
    }

    state.total_length = total_len;
    state.max_grade = max_gr;
    state.samples = all_samples;
    state.road_version = state.road_version.wrapping_add(1);
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

    let Ok(mut cam_transform) = camera_query.single_mut() else {
        return;
    };
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

    let junctions = detect_junctions(&state.waypoints, &state.streets);
    let junction_node_indices: Vec<usize> = junctions.iter().map(|j| j.node_idx).collect();

    // 1. Draw street connection lines between waypoints
    for (s_idx, street) in state.streets.iter().enumerate() {
        let is_active = s_idx == state.active_street_idx;
        let line_col = if is_active {
            Color::srgba(0.2, 0.9, 1.0, 0.7)
        } else {
            Color::srgba(0.5, 0.6, 0.8, 0.4)
        };

        for window in street.node_indices.windows(2) {
            if let (Some(a), Some(b)) = (state.waypoints.get(window[0]), state.waypoints.get(window[1])) {
                gizmos.line(a.pos + Vec3::Y * 0.1, b.pos + Vec3::Y * 0.1, line_col);
            }
        }
    }

    // 2. Draw junction boundary indicators and spokes
    for junction in &junctions {
        let ground_y = heightmap.sample(junction.pos.x, junction.pos.z);
        let center = Vec3::new(junction.pos.x, junction.pos.y.max(ground_y + 0.1), junction.pos.z);

        // Golden amber junction circle
        gizmos.circle(
            Isometry3d::new(
                center + Vec3::Y * 0.06,
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            junction.radius,
            Color::srgb(1.0, 0.75, 0.15),
        );

        // Spokes pointing toward each connecting road arm
        for arm in &junction.connected_arms {
            let arm_dir = Vec3::new(arm.dir.x, 0.0, arm.dir.z).normalize_or_zero();
            let spoke_end = center + arm_dir * junction.radius;
            gizmos.line(center + Vec3::Y * 0.06, spoke_end + Vec3::Y * 0.06, Color::srgba(1.0, 0.8, 0.2, 0.7));
        }
    }

    // 3. Draw magnetic snap indicator if dragging near a snap target
    if let Some(sel) = state.selected_node
        && let Some(target) = state.snap_target_node
        && let (Some(sel_wp), Some(target_wp)) = (state.waypoints.get(sel), state.waypoints.get(target))
    {
        // Magnetic line connecting dragged node to snap target
        gizmos.line(
            sel_wp.pos,
            target_wp.pos,
            Color::srgb(0.2, 1.0, 0.4),
        );
        // Snap target pulsing ring
        gizmos.circle(
            Isometry3d::new(
                target_wp.pos + Vec3::Y * 0.1,
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            3.6,
            Color::srgb(0.2, 1.0, 0.4),
        );
    }

    // 4. Draw waypoint spheres and ground drop lines
    for (i, wp) in state.waypoints.iter().enumerate() {
        let is_selected = state.selected_node == Some(i);
        let is_hovered = state.hovered_node == Some(i);
        let is_junction = junction_node_indices.contains(&i);

        let (col, radius) = if is_selected {
            (Color::srgb(1.0, 0.25, 0.1), 1.6)
        } else if is_hovered {
            (Color::srgb(1.0, 0.85, 0.2), 1.4)
        } else if is_junction {
            (Color::srgb(1.0, 0.70, 0.15), 1.3)
        } else {
            (Color::srgb(0.2, 0.85, 1.0), 1.1)
        };

        gizmos.sphere(Isometry3d::from_translation(wp.pos), radius, col);

        let ground_y = heightmap.sample(wp.pos.x, wp.pos.z);
        let ground_pt = Vec3::new(wp.pos.x, ground_y, wp.pos.z);
        gizmos.line(
            wp.pos,
            ground_pt,
            Color::srgba(
                col.to_srgba().red,
                col.to_srgba().green,
                col.to_srgba().blue,
                0.6,
            ),
        );

        gizmos.circle(
            Isometry3d::new(
                ground_pt + Vec3::Y * 0.05,
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            radius * 1.5,
            col,
        );
    }
}

pub fn setup_materials_and_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    heightmap: Res<HeightmapData>,
    mut editor_state: ResMut<EditorState>,
) {
    // 1. Road PBR Material with procedural texture adapted to initial road width and lanes
    let initial_width = editor_state.road_width;
    let initial_lanes = editor_state.effective_lanes();
    let road_img = create_road_texture(initial_width, initial_lanes);
    let road_img_handle = images.add(road_img);
    commands.insert_resource(RoadTextureImageHandle(road_img_handle.clone()));

    let road_mat_handle = materials.add(StandardMaterial {
        base_color_texture: Some(road_img_handle),
        perceptual_roughness: 0.78,
        metallic: 0.0,
        reflectance: 0.2,
        cull_mode: None, // Double-sided rendering so embankment skirts look solid
        ..default()
    });
    commands.insert_resource(RoadMaterialHandle(road_mat_handle));

    // 1b. Dedicated Junction PBR Material with procedural intersection markings and turning scrub
    let initial_junc_style = editor_state.junction_style;
    let junc_img = create_junction_texture(initial_junc_style);
    let junc_img_handle = images.add(junc_img);
    commands.insert_resource(JunctionTextureImageHandle(junc_img_handle.clone()));

    let junc_mat_handle = materials.add(StandardMaterial {
        base_color_texture: Some(junc_img_handle),
        perceptual_roughness: 0.72,
        metallic: 0.0,
        reflectance: 0.22,
        cull_mode: None, // Double-sided rendering so embankment skirts look solid
        ..default()
    });
    commands.insert_resource(JunctionMaterialHandle(junc_mat_handle));

    // 2. Concrete Bridge Pylon Material
    let pylon_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.67, 0.65),
        perceptual_roughness: 0.85,
        metallic: 0.05,
        ..default()
    });
    commands.insert_resource(PylonMaterialHandle(pylon_mat_handle));

    // 3. Roadside Delineator Post Mesh & Material
    let post_mesh_handle = meshes.add(Cuboid::new(0.12, 0.9, 0.12));
    commands.insert_resource(PostMeshHandle(post_mesh_handle));

    let post_mat_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.92, 0.90),
        perceptual_roughness: 0.4,
        metallic: 0.1,
        ..default()
    });
    commands.insert_resource(PostMaterialHandle(post_mat_handle));

    // 4. Load initial scenic road preset
    load_preset(&mut editor_state, 1, &heightmap);
}

#[derive(Resource, Clone)]
pub struct RoadTextureImageHandle(pub Handle<Image>);

#[derive(Resource)]
pub struct RoadMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource, Clone)]
pub struct JunctionTextureImageHandle(pub Handle<Image>);

#[derive(Resource)]
pub struct JunctionMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub struct PylonMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Resource)]
pub struct PostMeshHandle(pub Handle<Mesh>);

#[derive(Resource)]
pub struct PostMaterialHandle(pub Handle<StandardMaterial>);
