use crate::editor::{EditorAction, EditorSet, EditorState, EditorTool, WidthMode};
use crate::road::detect_junctions;
use crate::terrain::{HeightmapData, WATER_THRESHOLD};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    handle_button_clicks.in_set(EditorSet::Input),
                    update_ui_system
                        .in_set(EditorSet::PostUpdate)
                        .run_if(resource_changed::<EditorState>),
                    update_wireframe_ui_text,
                    update_fps_text,
                ),
            );
    }
}

#[derive(Component)]
pub struct FpsTextMarker;

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

#[derive(Component)]
pub struct NewStreetBtnMarker;

#[derive(Component)]
pub struct JoinBtnMarker;

#[derive(Component)]
pub struct CycleStreetBtnMarker;

#[derive(Component)]
pub struct JunctionStyleBtnMarker;

#[derive(Component)]
pub struct JunctionStyleBtnTextMarker;

#[derive(Component)]
pub struct WireframeBtnMarker;

#[derive(Component)]
pub struct WireframeBtnTextMarker;

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
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.65, 0.75)),
                        ));

                        // Dynamic FPS Line
                        card.spawn((
                            Text::new("FPS: -- (0.0 ms)"),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.45, 0.90, 0.55)),
                            FpsTextMarker,
                        ));

                        // Dynamic Stats Line
                        card.spawn((
                            Text::new("Length: 0.0 m | Nodes: 0 | Grade: 0.0% | Trees Cleared: 0/0"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.85, 0.90, 0.70)),
                            StatsTextMarker,
                        ));

                        // Selected Node Details
                        card.spawn((
                            Text::new("No node selected (Click a waypoint or ground)"),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.65, 0.85, 0.95)),
                            NodeInfoTextMarker,
                        ));
                    });

                // Top Right: Controls & Presets Panel
                top_row
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(12.0)),
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.0)),
                            align_items: AlignItems::FlexEnd,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.10, 0.14, 0.88)),
                        BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.4)),
                    ))
                    .with_children(|actions_panel| {
                        // Row 1: Tool & View Actions
                        actions_panel
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                            ))
                            .with_children(|btn_row| {
                                // Tool toggle button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.18, 0.25, 0.35, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.6, 0.8, 0.6)),
                                        ToolBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Tool: Select (T)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                            ToolBtnTextMarker,
                                        ));
                                    });

                                // New Street button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.2, 0.35, 0.3, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.8, 0.6, 0.6)),
                                        NewStreetBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("+ New St (N)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });

                                // Join Streets button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.35, 0.28, 0.15, 0.9)),
                                        BorderColor::all(Color::srgba(0.8, 0.7, 0.3, 0.6)),
                                        JoinBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Join (J)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });

                                // Cycle Street button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.22, 0.22, 0.32, 0.9)),
                                        BorderColor::all(Color::srgba(0.5, 0.5, 0.7, 0.6)),
                                        CycleStreetBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("St > (Tab)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });

                                // Junction Style button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.28, 0.22, 0.12, 0.9)),
                                        BorderColor::all(Color::srgba(0.8, 0.6, 0.2, 0.6)),
                                        JunctionStyleBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Junc: Box (K)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                            JunctionStyleBtnTextMarker,
                                        ));
                                    });

                                // Ride-along camera button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.2, 0.35, 0.25, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.8, 0.5, 0.6)),
                                        RideBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Ride (F)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });

                                 // Clear Road button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.35, 0.15, 0.15, 0.9)),
                                        BorderColor::all(Color::srgba(0.8, 0.3, 0.3, 0.6)),
                                        ClearBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Clear (C)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });

                                // Wireframe toggle button (Native only)
                                #[cfg(not(target_arch = "wasm32"))]
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.18, 0.22, 0.28, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.6, 0.8, 0.6)),
                                        WireframeBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Wireframe (Z)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.8, 0.9, 1.0)),
                                            WireframeBtnTextMarker,
                                        ));
                                    });
                            });

                        // Row 2: Road Presets
                        actions_panel
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(6.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                            ))
                            .with_children(|p_row| {
                                p_row.spawn((
                                    Text::new("Presets:"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.75, 0.8)),
                                ));

                                let presets = [
                                    (1, "1: Town Blvd"),
                                    (2, "2: River Bridge"),
                                    (3, "3: Ring Road"),
                                    (4, "4: Junctions"),
                                ];

                                for (idx, label) in presets {
                                    p_row
                                        .spawn((
                                            Button,
                                            Node {
                                                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                                border: UiRect::all(Val::Px(1.0)),
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgba(0.15, 0.22, 0.32, 0.9)),
                                            BorderColor::all(Color::srgba(0.3, 0.5, 0.7, 0.5)),
                                            PresetBtnMarker(idx),
                                        ))
                                        .with_children(|b| {
                                            b.spawn((
                                                Text::new(label),
                                                TextFont {
                                                    font_size: FontSize::Px(11.5),
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                }
                            });

                        // Row 3: Road Width & Multi-Lane Configuration
                        actions_panel
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(6.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                            ))
                            .with_children(|w_row| {
                                w_row.spawn((
                                    Text::new("Width:"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.75, 0.8)),
                                ));

                                // Mode Toggle (Lanes vs Seamless)
                                w_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.18, 0.24, 0.35, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.6, 0.8, 0.7)),
                                        WidthModeBtnMarker,
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Mode: Lanes (L)"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.9, 0.9, 1.0)),
                                            WidthModeBtnTextMarker,
                                        ));
                                    });

                                // Quick Lanes buttons
                                let lanes_btns = [
                                    (1, "1 Lane"),
                                    (2, "2 Lanes"),
                                    (4, "4 Lanes"),
                                ];

                                for (lanes, label) in lanes_btns {
                                    w_row
                                        .spawn((
                                            Button,
                                            Node {
                                                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                                border: UiRect::all(Val::Px(1.0)),
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgba(0.15, 0.18, 0.25, 0.9)),
                                            BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.6)),
                                            LaneBtnMarker(lanes),
                                        ))
                                        .with_children(|b| {
                                            b.spawn((
                                                Text::new(label),
                                                TextFont {
                                                    font_size: FontSize::Px(11.5),
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                }

                                // Narrower [-]
                                w_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.6)),
                                        WidthAdjustBtnMarker(-0.5),
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("[-]"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });

                                // Wider [+]
                                w_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.6)),
                                        WidthAdjustBtnMarker(0.5),
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("[+]"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });
                            });
                    });
            });

            // BOTTOM BAR: Controls Reference Cheat Sheet
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.90)),
                BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.4)),
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
                    #[cfg(not(target_arch = "wasm32"))]
                    "• Z: Toggle Wireframe Overlay  |  WASD / Middle Mouse Drag: Pan Camera  |  Right Mouse Drag / Q, E: Orbit View",
                    #[cfg(target_arch = "wasm32")]
                    "• WASD / Middle Mouse Drag: Pan Camera  |  Right Mouse Drag / Q, E: Orbit View",
                    "• Mouse Scroll: Zoom In / Out (Tracks landscape height smoothly)",
                    "• Left Click: Select Node / Add Node (T to toggle tool) | Drag: Move along ground",
                    "• N: New Street  |  Click existing node in Add mode to branch / join into a Junction",
                    "• Drag node onto another: Magnetic Snap & Join  |  J: Join to nearest street | Tab: Cycle Street",
                    "• K: Cycle Junction Style (Box Junction, Turning Circle, Zebra Crossings) | P: Toggle Elevation (Warped / Planar)",
                    "• Dynamic Trees: Trees organically part and move out of the way as roads are created or dragged",
                    "• R / V: Raise / Lower Node Elevation (Bridge Viaducts & Pylons)",
                    "• Delete / X: Remove Selected Node  |  [ / ]: Adjust Road Width (L: Toggle Lanes / Seamless)",
                    "• Road Width Buttons: 1 Lane (4m), 2 Lanes (8m), 4 Lanes (15m) or Seamless Continuous",
                    "• F: Ride-Along Cinematic Cam  |  1, 2, 3: Country Presets  |  4: Town Junctions & Crossroads",
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
    mut stats_query: Query<
        &mut Text,
        (
            With<StatsTextMarker>,
            Without<NodeInfoTextMarker>,
            Without<ToolBtnTextMarker>,
            Without<WidthModeBtnTextMarker>,
            Without<JunctionStyleBtnTextMarker>,
        ),
    >,
    mut node_info_query: Query<
        &mut Text,
        (
            With<NodeInfoTextMarker>,
            Without<StatsTextMarker>,
            Without<ToolBtnTextMarker>,
            Without<WidthModeBtnTextMarker>,
            Without<JunctionStyleBtnTextMarker>,
        ),
    >,
    mut tool_btn_text: Query<
        &mut Text,
        (
            With<ToolBtnTextMarker>,
            Without<StatsTextMarker>,
            Without<NodeInfoTextMarker>,
            Without<WidthModeBtnTextMarker>,
            Without<JunctionStyleBtnTextMarker>,
        ),
    >,
    mut width_mode_btn_text: Query<
        &mut Text,
        (
            With<WidthModeBtnTextMarker>,
            Without<StatsTextMarker>,
            Without<NodeInfoTextMarker>,
            Without<ToolBtnTextMarker>,
            Without<JunctionStyleBtnTextMarker>,
        ),
    >,
    mut junc_style_btn_text: Query<
        &mut Text,
        (
            With<JunctionStyleBtnTextMarker>,
            Without<StatsTextMarker>,
            Without<NodeInfoTextMarker>,
            Without<ToolBtnTextMarker>,
            Without<WidthModeBtnTextMarker>,
        ),
    >,
    mut lane_btn_query: Query<
        (&LaneBtnMarker, &mut BackgroundColor, &mut BorderColor),
        Without<WidthModeBtnMarker>,
    >,
    mut width_mode_btn_query: Query<
        (&mut BackgroundColor, &mut BorderColor),
        With<WidthModeBtnMarker>,
    >,
) {
    let tool_str = match state.tool {
        EditorTool::SelectMove => "Select & Move [T: Add/Join]",
        EditorTool::Add => "Add & Join [T: Select/Move]",
    };

    let btn_tool_label = match state.tool {
        EditorTool::SelectMove => "Tool: Select (T)",
        EditorTool::Add => "Tool: Add/Join (T)",
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

    for mut text in junc_style_btn_text.iter_mut() {
        text.0 = format!("Junc: {} / {} (K/P)", state.junction_style.display_name(), state.junction_elevation_mode.display_name());
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
        "Tool: {} | St: {}/{} | Junc: {} | Width: {:.1}m ({}, {}) | Length: {:.1}m | Nodes: {} | Grade: {:.1}% ({}) | Trees Cleared: {}/{}",
        tool_str,
        state.active_street_idx + 1,
        state.streets.len(),
        state.junctions_count,
        state.road_width,
        lane_label,
        mode_desc,
        state.total_length,
        state.waypoints.len(),
        state.max_grade,
        grade_desc,
        state.displaced_trees_count,
        state.total_trees_count,
    );

    for mut text in stats_query.iter_mut() {
        text.0 = stats_text.clone();
    }

    let node_text = if let Some(sel) = state.selected_node
        && sel < state.waypoints.len()
    {
        let p = state.waypoints[sel].pos;
        let ground_y = heightmap.sample(p.x, p.z);
        let elev = p.y - ground_y;
        let water_status = if ground_y < WATER_THRESHOLD {
            let clearance = p.y - WATER_THRESHOLD;
            format!(" [Bridge: +{:.1}m over Water]", clearance)
        } else {
            "".to_string()
        };

        let junctions = detect_junctions(&state.waypoints, &state.streets);
        if let Some(j) = junctions.iter().find(|j| j.node_idx == sel) {
            format!(
                "★ Junction Node #{}: {} connecting arms | X: {:.1}, Y: {:.1}, Z: {:.1} (Elev: +{:.1}m{}) [R/V: Elev, N: New St, J: Join]",
                sel + 1,
                j.connected_arms.len(),
                p.x,
                p.y,
                p.z,
                elev,
                water_status
            )
        } else {
            let street_info = state
                .streets
                .iter()
                .find(|s| s.node_indices.contains(&sel))
                .map(|s| {
                    let pos = s.node_indices.iter().position(|&idx| idx == sel).unwrap_or(0);
                    format!("Street '{}' Node #{}/{}", s.name, pos + 1, s.node_indices.len())
                })
                .unwrap_or_else(|| format!("Node #{}", sel + 1));

            format!(
                "{}: X: {:.1}, Y: {:.1}, Z: {:.1} (Elev: +{:.1}m{}) [R/V: Elev, Drag: Move/Snap]",
                street_info,
                p.x,
                p.y,
                p.z,
                elev,
                water_status
            )
        }
    } else {
        "No node selected (Click node or ground to build, N for New Street)".to_string()
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
            Option<&NewStreetBtnMarker>,
            Option<&JoinBtnMarker>,
            Option<&CycleStreetBtnMarker>,
            Option<&JunctionStyleBtnMarker>,
            Option<&WireframeBtnMarker>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut wireframe_config: Option<ResMut<WireframeConfig>>,
) {
    for (
        interaction,
        mut bg_color,
        preset,
        clear,
        ride,
        tool_btn,
        lane_btn,
        mode_btn,
        adjust_btn,
        new_street_btn,
        join_btn,
        cycle_btn,
        junc_style_btn,
        wireframe_btn,
    ) in interaction_query.iter_mut()
    {
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
                } else if new_street_btn.is_some() {
                    action_writer.write(EditorAction::NewStreet);
                } else if join_btn.is_some() {
                    action_writer.write(EditorAction::JoinStreets);
                } else if cycle_btn.is_some() {
                    action_writer.write(EditorAction::CycleActiveStreet);
                } else if junc_style_btn.is_some() {
                    action_writer.write(EditorAction::CycleJunctionStyle);
                } else if wireframe_btn.is_some() {
                    if let Some(ref mut config) = wireframe_config {
                        config.global = !config.global;
                    }
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgba(0.25, 0.35, 0.48, 0.95));
            }
            Interaction::None => {
                // Handled in update_ui_system based on state
            }
        }
    }
}

pub fn update_wireframe_ui_text(
    wireframe_config: Option<Res<WireframeConfig>>,
    mut query: Query<&mut Text, With<WireframeBtnTextMarker>>,
) {
    let is_on = wireframe_config.as_ref().map_or(false, |c| c.global);
    for mut text in query.iter_mut() {
        let label = if is_on {
            "Wireframe: ON (Z)"
        } else {
            "Wireframe: OFF (Z)"
        };
        if text.0 != label {
            text.0 = label.to_string();
        }
    }
}

pub fn update_fps_text(
    diagnostics: Res<DiagnosticsStore>,
    mut fps_query: Query<&mut Text, With<FpsTextMarker>>,
) {
    if let Some(fps_diag) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps) = fps_diag.smoothed().or_else(|| fps_diag.average()).or_else(|| fps_diag.value()) {
            let frame_time_ms = if fps > 0.0 { 1000.0 / fps } else { 0.0 };
            for mut text in &mut fps_query {
                text.0 = format!("FPS: {:>3.0} ({:.1} ms)", fps, frame_time_ms);
            }
        }
    }
}
