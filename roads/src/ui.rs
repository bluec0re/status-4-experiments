use bevy::prelude::*;
use crate::editor::{EditorAction, EditorSet, EditorState, EditorTool, WidthMode};
use crate::terrain::{HeightmapData, WATER_THRESHOLD};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    handle_button_clicks.in_set(EditorSet::Input),
                    update_ui_system
                        .in_set(EditorSet::PostUpdate)
                        .run_if(resource_changed::<EditorState>),
                ),
            );
    }
}

#[derive(Component)]
pub struct StatsTextMarker;

#[derive(Component)]
pub struct NodeInfoTextMarker;

#[derive(Component)]
pub struct ToolBtnMarker;

#[derive(Component)]
pub struct ToolBtnTextMarker;

#[derive(Component)]
pub struct PresetBtnMarker(pub usize);

#[derive(Component)]
pub struct ClearBtnMarker;

#[derive(Component)]
pub struct RideBtnMarker;

#[derive(Component)]
pub struct LaneBtnMarker(pub usize);

#[derive(Component)]
pub struct WidthModeBtnMarker;

#[derive(Component)]
pub struct WidthModeBtnTextMarker;

#[derive(Component)]
pub struct WidthAdjustBtnMarker(pub f32);

pub fn setup_ui(mut commands: Commands) {
    // Root container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
        ))
        .with_children(|root| {
            // TOP BAR: Title & Mode & Stats Panel on Left, Action & Width Controls on Right
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexStart,
                    width: Val::Percent(100.0),
                    ..default()
                },
            ))
            .with_children(|top_row| {
                // Top Left: Title & Stats Card
                top_row
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(6.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.10, 0.14, 0.88)),
                        BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.4)),
                    ))
                    .with_children(|card| {
                        // Title
                        card.spawn((
                            Text::new("BEVY REALISTIC ROAD & TOWN EDITOR"),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.95, 0.98)),
                        ));

                        // Subtitle
                        card.spawn((
                            Text::new(format!(
                                "Landscape (assets/heightmap.png) | Water Threshold: {:.1}m",
                                WATER_THRESHOLD
                            )),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.60, 0.78, 0.90)),
                        ));

                        // Dynamic Stats Text
                        card.spawn((
                            Text::new("Length: 0.0 m | Nodes: 0 | Grade: 0.0%"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.85, 0.90, 0.70)),
                            StatsTextMarker,
                        ));

                        // Dynamic Node Details
                        card.spawn((
                            Text::new("No node selected"),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.70, 0.80, 0.90)),
                            NodeInfoTextMarker,
                        ));
                    });

                // Top Right: Control Panel (Row 1: Presets & Tools, Row 2: Road Width & Lanes)
                top_row
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            align_items: AlignItems::FlexEnd,
                            ..default()
                        },
                    ))
                    .with_children(|ctrl_col| {
                        // Row 1: Presets & General Tools
                        ctrl_col
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                            ))
                            .with_children(|btn_bar| {
                                // Tool Mode Toggle
                                btn_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.18, 0.24, 0.35, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.6, 0.8, 0.7)),
                                    ToolBtnMarker,
                                )).with_child((
                                    Text::new("Tool: Select (T)"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                    ToolBtnTextMarker,
                                ));

                                // Preset 1: Town Boulevard
                                btn_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    PresetBtnMarker(1),
                                )).with_child((
                                    Text::new("1: Town Blvd"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // Preset 2: River Bridge Expressway
                                btn_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    PresetBtnMarker(2),
                                )).with_child((
                                    Text::new("2: River Bridge"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // Preset 3: Ring Road
                                btn_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    PresetBtnMarker(3),
                                )).with_child((
                                    Text::new("3: Ring Road"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // Clear
                                btn_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    ClearBtnMarker,
                                )).with_child((
                                    Text::new("Clear (C)"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // Ride Along
                                btn_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    RideBtnMarker,
                                )).with_child((
                                    Text::new("Ride (F)"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));
                            });

                        // Row 2: Road Width & Lanes Toolbar
                        ctrl_col
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(6.0),
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.08, 0.10, 0.15, 0.88)),
                                BorderColor::all(Color::srgba(0.3, 0.4, 0.55, 0.5)),
                            ))
                            .with_children(|width_bar| {
                                width_bar.spawn((
                                    Text::new("Road Width:"),
                                    TextFont { font_size: FontSize::Px(11.5), ..default() },
                                    TextColor(Color::srgb(0.75, 0.85, 0.95)),
                                ));

                                // Mode toggle button (Lanes vs Seamless)
                                width_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.18, 0.24, 0.35, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.6, 0.8, 0.7)),
                                    WidthModeBtnMarker,
                                )).with_child((
                                    Text::new("Mode: Lanes (L)"),
                                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                    WidthModeBtnTextMarker,
                                ));

                                // 1 Lane button
                                width_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    LaneBtnMarker(1),
                                )).with_child((
                                    Text::new("1 Lane"),
                                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // 2 Lanes button
                                width_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    LaneBtnMarker(2),
                                )).with_child((
                                    Text::new("2 Lanes"),
                                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // 4 Lanes button
                                width_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    LaneBtnMarker(4),
                                )).with_child((
                                    Text::new("4 Lanes"),
                                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // -0.5m button
                                width_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    WidthAdjustBtnMarker(-0.5),
                                )).with_child((
                                    Text::new("- ["),
                                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));

                                // +0.5m button
                                width_bar.spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                    BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                    WidthAdjustBtnMarker(0.5),
                                )).with_child((
                                    Text::new("+ ]"),
                                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));
                            });
                    });
            });

            // BOTTOM BAR: Controls Cheat Sheet
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(1.0)),
                    max_width: Val::Px(680.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.84)),
                BorderColor::all(Color::srgba(0.25, 0.35, 0.45, 0.4)),
            ))
            .with_children(|help_box| {
                help_box.spawn((
                    Text::new("RTS CAMERA & ROAD EDITOR CONTROLS"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.8, 0.3)),
                ));

                let controls = [
                    "• WASD / Middle Mouse Drag: Pan Camera  |  Right Mouse Drag / Q, E: Orbit View",
                    "• Mouse Scroll: Zoom In / Out (Tracks landscape height smoothly)",
                    "• Left Click: Select Node / Add Node (T to toggle tool) | Drag: Move along ground",
                    "• R / V: Raise / Lower Node Elevation (Bridge Viaducts & Pylons)",
                    "• Delete / X: Remove Selected Node  |  [ / ]: Adjust Road Width (L: Toggle Lanes / Seamless)",
                    "• Road Width Buttons: 1 Lane (4m), 2 Lanes (8m), 4 Lanes (15m) or Seamless Continuous",
                    "• F: Ride-Along Cinematic Cam  |  1, 2, 3: Load Town & Country Presets",
                ];

                for line in controls {
                    help_box.spawn((
                        Text::new(line),
                        TextFont {
                            font_size: FontSize::Px(10.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.78, 0.82, 0.88)),
                    ));
                }
            });
        });
}

/// Updates UI text and button highlights with live road statistics and selected node details
pub fn update_ui_system(
    state: Res<EditorState>,
    heightmap: Res<HeightmapData>,
    mut stats_query: Query<&mut Text, (With<StatsTextMarker>, Without<NodeInfoTextMarker>, Without<ToolBtnTextMarker>, Without<WidthModeBtnTextMarker>)>,
    mut node_info_query: Query<&mut Text, (With<NodeInfoTextMarker>, Without<StatsTextMarker>, Without<ToolBtnTextMarker>, Without<WidthModeBtnTextMarker>)>,
    mut tool_btn_text: Query<&mut Text, (With<ToolBtnTextMarker>, Without<StatsTextMarker>, Without<NodeInfoTextMarker>, Without<WidthModeBtnTextMarker>)>,
    mut width_mode_btn_text: Query<&mut Text, (With<WidthModeBtnTextMarker>, Without<StatsTextMarker>, Without<NodeInfoTextMarker>, Without<ToolBtnTextMarker>)>,
    mut lane_btn_query: Query<(&LaneBtnMarker, &mut BackgroundColor, &mut BorderColor), Without<WidthModeBtnMarker>>,
    mut width_mode_btn_query: Query<(&mut BackgroundColor, &mut BorderColor), With<WidthModeBtnMarker>>,
) {
    let tool_str = match state.tool {
        EditorTool::SelectMove => "Select & Move [T: Add]",
        EditorTool::Add => "Add Waypoint [T: Select]",
    };

    let btn_tool_label = match state.tool {
        EditorTool::SelectMove => "Tool: Select (T)",
        EditorTool::Add => "Tool: Add (T)",
    };

    for mut text in tool_btn_text.iter_mut() {
        text.0 = btn_tool_label.to_string();
    }

    let mode_btn_label = match state.width_mode {
        WidthMode::Lanes => "Mode: Lanes (L)",
        WidthMode::Seamless => "Mode: Seamless (L)",
    };

    for mut text in width_mode_btn_text.iter_mut() {
        text.0 = mode_btn_label.to_string();
    }

    let mode_desc = match state.width_mode {
        WidthMode::Lanes => "Lanes",
        WidthMode::Seamless => "Seamless",
    };

    let effective_lanes = state.effective_lanes();
    let lane_label = if effective_lanes == 1 {
        "1 Lane".to_string()
    } else {
        format!("{} Lanes", effective_lanes)
    };

    // Update highlights on Lane buttons (1, 2, 4)
    for (lane_marker, mut bg_color, mut border_color) in lane_btn_query.iter_mut() {
        if lane_marker.0 == effective_lanes && state.width_mode == WidthMode::Lanes {
            *bg_color = BackgroundColor(Color::srgba(0.20, 0.38, 0.58, 0.95));
            *border_color = BorderColor::all(Color::srgba(0.40, 0.75, 1.0, 0.95));
        } else {
            *bg_color = BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9));
            *border_color = BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6));
        }
    }

    // Update highlight on Width Mode button
    for (mut bg_color, mut border_color) in width_mode_btn_query.iter_mut() {
        if state.width_mode == WidthMode::Seamless {
            *bg_color = BackgroundColor(Color::srgba(0.32, 0.22, 0.45, 0.95));
            *border_color = BorderColor::all(Color::srgba(0.75, 0.55, 0.95, 0.95));
        } else {
            *bg_color = BackgroundColor(Color::srgba(0.18, 0.24, 0.35, 0.9));
            *border_color = BorderColor::all(Color::srgba(0.4, 0.6, 0.8, 0.7));
        }
    }

    let grade_desc = if state.max_grade < 3.0 {
        "Level / Flat"
    } else if state.max_grade < 6.0 {
        "Gentle Slope"
    } else if state.max_grade < 10.0 {
        "Moderate Incline"
    } else {
        "Steep Hill"
    };

    let stats_text = format!(
        "Tool: {} | Width: {:.1}m ({}, {}) | Length: {:.1} m | Nodes: {} | Max Grade: {:.1}% ({})",
        tool_str,
        state.road_width,
        lane_label,
        mode_desc,
        state.total_length,
        state.waypoints.len(),
        state.max_grade,
        grade_desc
    );

    for mut text in stats_query.iter_mut() {
        text.0 = stats_text.clone();
    }

    let node_text = if let Some(sel) = state.selected_node {
        if sel < state.waypoints.len() {
            let p = state.waypoints[sel].pos;
            let ground_y = heightmap.sample(p.x, p.z);
            let elev = p.y - ground_y;
            let water_status = if ground_y < WATER_THRESHOLD {
                let clearance = p.y - WATER_THRESHOLD;
                format!(" [Bridge: +{:.1}m over Water]", clearance)
            } else {
                "".to_string()
            };

            format!(
                "Node #{}: X: {:.1}, Y: {:.1}, Z: {:.1} (Elev: +{:.1}m above ground{}) [R/V to adjust]",
                sel + 1,
                p.x,
                p.y,
                p.z,
                elev,
                water_status
            )
        } else {
            "No node selected (Click a waypoint or ground)".to_string()
        }
    } else {
        "No node selected (Click a waypoint or ground)".to_string()
    };

    for mut text in node_info_query.iter_mut() {
        text.0 = node_text.clone();
    }
}

/// Handles UI button interactions and emits decoupled EditorAction messages
pub fn handle_button_clicks(
    mut action_writer: MessageWriter<EditorAction>,
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&PresetBtnMarker>,
            Option<&ClearBtnMarker>,
            Option<&RideBtnMarker>,
            Option<&ToolBtnMarker>,
            Option<&LaneBtnMarker>,
            Option<&WidthModeBtnMarker>,
            Option<&WidthAdjustBtnMarker>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg_color, preset, clear, ride, tool_btn, lane_btn, mode_btn, adjust_btn) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgba(0.3, 0.4, 0.6, 0.95));
                if let Some(p) = preset {
                    action_writer.write(EditorAction::LoadPreset(p.0));
                } else if clear.is_some() {
                    action_writer.write(EditorAction::ClearWaypoints);
                } else if ride.is_some() {
                    action_writer.write(EditorAction::ToggleRideAlong);
                } else if tool_btn.is_some() {
                    action_writer.write(EditorAction::ToggleTool);
                } else if let Some(lane) = lane_btn {
                    action_writer.write(EditorAction::SetLanes(lane.0));
                } else if mode_btn.is_some() {
                    action_writer.write(EditorAction::ToggleWidthMode);
                } else if let Some(adj) = adjust_btn {
                    action_writer.write(EditorAction::AdjustRoadWidth(adj.0));
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.22, 0.28, 0.38, 0.95));
            }
            Interaction::None => {}
        }
    }
}
