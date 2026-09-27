use bevy::prelude::*;
use crate::editor::{EditorAction, EditorSet, EditorState, EditorTool};
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
            // TOP BAR: Title & Mode & Stats Panel
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

                // Top Right: Quick Action Buttons (Presets & Tools)
                top_row
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

                        // Preset 2: River Bridge
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
            });

            // BOTTOM BAR: Controls Cheat Sheet
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(1.0)),
                    max_width: Val::Px(640.0),
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
                    "• R / V: Raise / Lower Node Elevation (create viaducts/bridges with automatic pylons!)",
                    "• Delete / X: Remove Selected Node  |  [ / ]: Adjust Road Width",
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

/// Updates UI text with live road statistics and selected node details (runs when EditorState changes)
pub fn update_ui_system(
    state: Res<EditorState>,
    heightmap: Res<HeightmapData>,
    mut stats_query: Query<&mut Text, (With<StatsTextMarker>, Without<NodeInfoTextMarker>, Without<ToolBtnTextMarker>)>,
    mut node_info_query: Query<&mut Text, (With<NodeInfoTextMarker>, Without<StatsTextMarker>, Without<ToolBtnTextMarker>)>,
    mut tool_btn_text: Query<&mut Text, (With<ToolBtnTextMarker>, Without<StatsTextMarker>, Without<NodeInfoTextMarker>)>,
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
        "Tool: {} | Width: {:.1}m | Length: {:.1} m | Nodes: {} | Max Grade: {:.1}% ({})",
        tool_str,
        state.road_width,
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
        (&Interaction, &mut BackgroundColor, Option<&PresetBtnMarker>, Option<&ClearBtnMarker>, Option<&RideBtnMarker>, Option<&ToolBtnMarker>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg_color, preset, clear, ride, tool_btn) in interaction_query.iter_mut() {
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
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.22, 0.28, 0.38, 0.95));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9));
            }
        }
    }
}
