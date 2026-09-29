#![allow(dead_code)]

use bevy::prelude::*;
use crate::fire_sim::{EnvironmentConditions, SimulationStats, TimeOfDay};
use crate::interaction::{InteractionState, ToolMode};
use crate::scenarios::{ScenarioState, ScenarioType};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(Update, (handle_ui_buttons, update_ui_labels));
    }
}

/// Dynamic UI text roles for targeted updates
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiTextRole {
    StatsHeader,
    StatsBody,
    ToolStatus,
    WindStatus,
    TimeOfDayBtn,
    SpeedBtn,
    PressureBtn,
    HeatmapBtn,
    HoverInfo,
}

/// Interactive button actions
#[derive(Component, Clone, Debug)]
pub enum UiAction {
    SelectScenario(ScenarioType),
    SelectTool(ToolMode),
    ToggleWaterPressure,
    ToggleHeatmap,
    SetTimeOfDay(TimeOfDay),
    CycleSimSpeed,
    SetWindDirection(Vec2),
    AdjustWindSpeed(f32),
    ResetScenario,
}

pub fn setup_ui(mut commands: Commands) {
    // Root container spanning full viewport
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(14.0)),
            ..default()
        })
        .with_children(|root| {
            // ================================================================
            // TOP SECTION: Header bar with Title, Scenario Tabs, Day/Night, Speed
            // ================================================================
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            })
            .insert((
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.92)),
                BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.4)),
            ))
            .with_children(|top_bar| {
                // Title & Subtitle Badge
                top_bar
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        ..default()
                    })
                    .with_children(|col| {
                        col.spawn((
                            Text::new("STATUS 4: THERMODYNAMIC FIRE PROPAGATION"),
                            TextFont {
                                font_size: FontSize::Px(14.5),
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.65, 0.2)),
                        ));
                        col.spawn((
                            Text::new("Material Physics, Wind Convection, Radiative Spread & Suppression"),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.65, 0.75, 0.85)),
                        ));
                    });

                // Scenario Selector Tabs
                top_bar
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        for scenario in ScenarioType::ALL {
                            let label = match scenario {
                                ScenarioType::CityFire => "City Block",
                                ScenarioType::Wildfire => "Forest Wildfire",
                                ScenarioType::SuburbanResidence => "Suburban Yard",
                                ScenarioType::IndustrialDepot => "Timber Depot",
                            };
                            let bg = match scenario {
                                ScenarioType::CityFire => Color::srgb(0.22, 0.18, 0.12),
                                ScenarioType::Wildfire => Color::srgb(0.18, 0.24, 0.14),
                                ScenarioType::SuburbanResidence => Color::srgb(0.15, 0.22, 0.28),
                                ScenarioType::IndustrialDepot => Color::srgb(0.24, 0.16, 0.18),
                            };

                            spawn_btn(row, UiAction::SelectScenario(scenario), bg, |b| {
                                b.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: FontSize::Px(11.5),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.95, 0.95, 0.98)),
                                ));
                            });
                        }
                    });

                // Environment & Simulation Quick Controls
                top_bar
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        // Time of Day Toggle
                        spawn_btn(
                            row,
                            UiAction::SetTimeOfDay(TimeOfDay::Night),
                            Color::srgb(0.12, 0.15, 0.24),
                            |b| {
                                b.spawn((
                                    UiTextRole::TimeOfDayBtn,
                                    Text::new("Time: Night"),
                                    TextFont {
                                        font_size: FontSize::Px(11.5),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.9, 1.0)),
                                ));
                            },
                        );

                        // Sim Speed
                        spawn_btn(row, UiAction::CycleSimSpeed, Color::srgb(0.18, 0.22, 0.28), |b| {
                            b.spawn((
                                UiTextRole::SpeedBtn,
                                Text::new("Speed: 1x"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.9, 0.7)),
                            ));
                        });

                        // Reset Button
                        spawn_btn(row, UiAction::ResetScenario, Color::srgb(0.32, 0.15, 0.15), |b| {
                            b.spawn((
                                Text::new("Reset Scenario"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.7, 0.7)),
                            ));
                        });
                    });
            });

            // ================================================================
            // MIDDLE SECTION: Left Tool Palette & Right Wind Controller
            // ================================================================
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                width: Val::Percent(100.0),
                margin: UiRect::vertical(Val::Px(10.0)),
                ..default()
            })
            .with_children(|mid| {
                // Left Tool Palette
                mid.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.0)),
                        min_width: Val::Px(240.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.90)),
                    BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.4)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("INTERACTION TOOLS"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.95, 1.0)),
                    ));

                    // Water Hose button (Primary tool)
                    spawn_btn(
                        card,
                        UiAction::SelectTool(ToolMode::WaterHose),
                        Color::srgb(0.12, 0.28, 0.42),
                        |b| {
                            b.spawn((
                                Text::new("💧 Water Hose (Cool/Extinguish)"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.6, 0.9, 1.0)),
                            ));
                        },
                    );

                    // Hose Pressure Switch
                    spawn_btn(
                        card,
                        UiAction::ToggleWaterPressure,
                        Color::srgb(0.15, 0.22, 0.32),
                        |b| {
                            b.spawn((
                                UiTextRole::PressureBtn,
                                Text::new("  Water Pressure: Deluge (High)"),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.85, 0.95)),
                            ));
                        },
                    );

                    // Torch (Ignite)
                    spawn_btn(
                        card,
                        UiAction::SelectTool(ToolMode::FireStarter),
                        Color::srgb(0.38, 0.20, 0.12),
                        |b| {
                            b.spawn((
                                Text::new("🔥 Torch (Ignite Surface)"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.7, 0.4)),
                            ));
                        },
                    );

                    // Flashover Blast
                    spawn_btn(
                        card,
                        UiAction::SelectTool(ToolMode::Explosion),
                        Color::srgb(0.42, 0.14, 0.14),
                        |b| {
                            b.spawn((
                                Text::new("💥 Flashover Blast"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.5, 0.5)),
                            ));
                        },
                    );

                    // Firebreak
                    spawn_btn(
                        card,
                        UiAction::SelectTool(ToolMode::Firebreak),
                        Color::srgb(0.24, 0.22, 0.14),
                        |b| {
                            b.spawn((
                                Text::new("🚜 Bulldozer (Firebreak)"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.85, 0.5)),
                            ));
                        },
                    );

                    // Inspector
                    spawn_btn(
                        card,
                        UiAction::SelectTool(ToolMode::Inspect),
                        Color::srgb(0.18, 0.24, 0.28),
                        |b| {
                            b.spawn((
                                Text::new("🔍 Thermal Inspector"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.85, 0.95)),
                            ));
                        },
                    );

                    // Thermal Heatmap Mode Toggle
                    spawn_btn(
                        card,
                        UiAction::ToggleHeatmap,
                        Color::srgb(0.28, 0.14, 0.32),
                        |b| {
                            b.spawn((
                                UiTextRole::HeatmapBtn,
                                Text::new("📷 View: Photorealistic (PBR)"),
                                TextFont {
                                    font_size: FontSize::Px(11.5),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.7, 1.0)),
                            ));
                        },
                    );
                });

                // Right Wind Direction & Weather Controller
                mid.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.0)),
                        min_width: Val::Px(240.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.90)),
                    BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.4)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("WIND & CONVECTIVE CONE"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.4, 0.85, 1.0)),
                    ));

                    card.spawn((
                        UiTextRole::WindStatus,
                        Text::new("Wind: East (8.5 m/s)"),
                        TextFont {
                            font_size: FontSize::Px(11.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.85, 0.9, 0.95)),
                    ));

                    // Wind Compass 2x4 grid
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_wind_btn(row, "← W", Vec2::new(-1.0, 0.0));
                        spawn_wind_btn(row, "→ E", Vec2::new(1.0, 0.0));
                        spawn_wind_btn(row, "↑ N", Vec2::new(0.0, -1.0));
                        spawn_wind_btn(row, "↓ S", Vec2::new(0.0, 1.0));
                    });

                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_wind_btn(row, "↗ NE", Vec2::new(1.0, -1.0).normalize());
                        spawn_wind_btn(row, "↘ SE", Vec2::new(1.0, 1.0).normalize());
                        spawn_wind_btn(row, "↖ NW", Vec2::new(-1.0, -1.0).normalize());
                        spawn_wind_btn(row, "↙ SW", Vec2::new(-1.0, 1.0).normalize());
                    });

                    // Wind Speed presets
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_speed_preset(row, "Calm (2m/s)", 2.0);
                        spawn_speed_preset(row, "Breeze (8m/s)", 8.0);
                    });

                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_speed_preset(row, "Gale (16m/s)", 16.0);
                        spawn_speed_preset(row, "Storm (28m/s)", 28.0);
                    });
                });
            });

            // ================================================================
            // BOTTOM SECTION: Telemetry Dashboard & Controls Legend
            // ================================================================
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            })
            .insert((
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.94)),
                BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.4)),
            ))
            .with_children(|bot| {
                // Top telemetry row: Stats metrics
                bot.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        UiTextRole::StatsBody,
                        Text::new("Active Fires: 0 | Max Temp: 22°C | Water Applied: 0 L | Ash: 0"),
                        TextFont {
                            font_size: FontSize::Px(12.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.92, 0.95)),
                    ));

                    row.spawn((
                        UiTextRole::ToolStatus,
                        Text::new("Active Tool: Water Hose | L-Click to spray water"),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.4, 0.8, 1.0)),
                    ));
                });

                // Middle telemetry row: Hovered Element Thermal Readout
                bot.spawn((
                    UiTextRole::HoverInfo,
                    Text::new("Target: Hover over buildings, trees, or roofs to inspect thermal status"),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.78, 0.88)),
                ));

                // Bottom row: Navigation Controls Legend
                bot.spawn((
                    Text::new("Controls: [Left Click & Drag] Use Tool  |  [Right Click & Drag / Q,E] Orbit  |  [WASD / Middle Drag] Pan  |  [Mouse Scroll] Zoom  |  [F] Reset Camera"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.45, 0.55, 0.65)),
                ));
            });
        });
}

// ---------------------------------------------------------------------------
// BUTTON HELPERS
// ---------------------------------------------------------------------------

fn spawn_btn<F: FnOnce(&mut ChildSpawnerCommands)>(
    parent: &mut ChildSpawnerCommands,
    action: UiAction,
    bg_color: Color,
    spawn_content: F,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(Color::srgba(0.35, 0.45, 0.55, 0.4)),
        ))
        .with_children(spawn_content);
}

fn spawn_wind_btn(parent: &mut ChildSpawnerCommands, text: &str, dir: Vec2) {
    spawn_btn(
        parent,
        UiAction::SetWindDirection(dir),
        Color::srgb(0.14, 0.18, 0.24),
        |b| {
            b.spawn((
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.85, 0.95)),
            ));
        },
    );
}

fn spawn_speed_preset(parent: &mut ChildSpawnerCommands, text: &str, speed: f32) {
    spawn_btn(
        parent,
        UiAction::AdjustWindSpeed(speed),
        Color::srgb(0.15, 0.20, 0.26),
        |b| {
            b.spawn((
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(10.5),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.85, 0.95)),
            ));
        },
    );
}

// ---------------------------------------------------------------------------
// BUTTON EVENT DISPATCHER
// ---------------------------------------------------------------------------

pub fn handle_ui_buttons(
    interaction_query: Query<(&Interaction, &UiAction), (Changed<Interaction>, With<Button>)>,
    mut scenario_state: ResMut<ScenarioState>,
    mut interaction_state: ResMut<InteractionState>,
    mut env: ResMut<EnvironmentConditions>,
) {
    for (interaction, action) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match action {
                UiAction::SelectScenario(scen) => {
                    scenario_state.pending_switch = Some(*scen);
                }
                UiAction::SelectTool(tool) => {
                    interaction_state.current_tool = *tool;
                }
                UiAction::ToggleWaterPressure => {
                    if interaction_state.water_hose_pressure < 2.0 {
                        interaction_state.water_hose_pressure = 2.5; // High pressure deluge
                    } else {
                        interaction_state.water_hose_pressure = 1.0; // Standard stream
                    }
                }
                UiAction::ToggleHeatmap => {
                    env.heatmap_mode = !env.heatmap_mode;
                }
                UiAction::SetTimeOfDay(_) => {
                    env.time_of_day = env.time_of_day.next();
                }
                UiAction::CycleSimSpeed => {
                    if env.sim_paused {
                        env.sim_paused = false;
                        env.sim_speed = 1.0;
                    } else if (env.sim_speed - 1.0).abs() < 0.1 {
                        env.sim_speed = 2.0;
                    } else if (env.sim_speed - 2.0).abs() < 0.1 {
                        env.sim_speed = 5.0;
                    } else {
                        env.sim_paused = true;
                        env.sim_speed = 0.0;
                    }
                }
                UiAction::SetWindDirection(dir) => {
                    env.wind_direction = *dir;
                }
                UiAction::AdjustWindSpeed(speed) => {
                    env.wind_speed = *speed;
                }
                UiAction::ResetScenario => {
                    scenario_state.pending_switch = Some(scenario_state.current);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HUD LABELS UPDATER
// ---------------------------------------------------------------------------

pub fn update_ui_labels(
    env: Res<EnvironmentConditions>,
    stats: Res<SimulationStats>,
    interaction: Res<InteractionState>,
    mut query: Query<(&UiTextRole, &mut Text, &mut TextColor)>,
) {
    for (role, mut text, mut color) in query.iter_mut() {
        match role {
            UiTextRole::StatsBody => {
                text.0 = format!(
                    "Active Flames: {} | Max Temp: {:.1}°C | Water Deluge: {:.0} L | Burnt Out: {}",
                    stats.active_fires,
                    stats.max_temperature,
                    stats.water_used_liters,
                    stats.burnt_elements
                );
            }
            UiTextRole::ToolStatus => {
                text.0 = format!(
                    "Tool: {} | Press L-Click in 3D scene",
                    interaction.current_tool.name()
                );
                color.0 = match interaction.current_tool {
                    ToolMode::WaterHose => Color::srgb(0.4, 0.85, 1.0),
                    ToolMode::FireStarter => Color::srgb(1.0, 0.65, 0.2),
                    ToolMode::Explosion => Color::srgb(1.0, 0.45, 0.45),
                    ToolMode::Firebreak => Color::srgb(0.9, 0.85, 0.4),
                    ToolMode::Inspect => Color::srgb(0.6, 0.8, 1.0),
                };
            }
            UiTextRole::WindStatus => {
                let dir_name = if env.wind_direction.x > 0.4 && env.wind_direction.y.abs() < 0.4 {
                    "East (→)"
                } else if env.wind_direction.x < -0.4 && env.wind_direction.y.abs() < 0.4 {
                    "West (←)"
                } else if env.wind_direction.y < -0.4 && env.wind_direction.x.abs() < 0.4 {
                    "North (↑)"
                } else if env.wind_direction.y > 0.4 && env.wind_direction.x.abs() < 0.4 {
                    "South (↓)"
                } else if env.wind_direction.x > 0.0 && env.wind_direction.y < 0.0 {
                    "North-East (↗)"
                } else if env.wind_direction.x > 0.0 && env.wind_direction.y > 0.0 {
                    "South-East (↘)"
                } else if env.wind_direction.x < 0.0 && env.wind_direction.y < 0.0 {
                    "North-West (↖)"
                } else {
                    "South-West (↙)"
                };
                text.0 = format!("Wind: {} ({:.1} m/s)", dir_name, env.wind_speed);
            }
            UiTextRole::TimeOfDayBtn => {
                text.0 = format!("Time: {}", env.time_of_day.name());
                color.0 = match env.time_of_day {
                    TimeOfDay::Day => Color::srgb(1.0, 0.95, 0.7),
                    TimeOfDay::Sunset => Color::srgb(1.0, 0.6, 0.4),
                    TimeOfDay::Night => Color::srgb(0.7, 0.85, 1.0),
                };
            }
            UiTextRole::SpeedBtn => {
                if env.sim_paused {
                    text.0 = "Speed: PAUSED".to_string();
                    color.0 = Color::srgb(1.0, 0.5, 0.5);
                } else {
                    text.0 = format!("Speed: {:.0}x", env.sim_speed);
                    color.0 = Color::srgb(0.6, 0.95, 0.6);
                }
            }
            UiTextRole::PressureBtn => {
                if interaction.water_hose_pressure > 2.0 {
                    text.0 = "  Water: Deluge (High)".to_string();
                    color.0 = Color::srgb(0.4, 0.85, 1.0);
                } else {
                    text.0 = "  Water: Standard Stream".to_string();
                    color.0 = Color::srgb(0.8, 0.88, 0.95);
                }
            }
            UiTextRole::HeatmapBtn => {
                if env.heatmap_mode {
                    text.0 = "📷 View: Thermal FLIR (Heatmap)".to_string();
                    color.0 = Color::srgb(1.0, 0.4, 0.9);
                } else {
                    text.0 = "📷 View: Photorealistic (PBR)".to_string();
                    color.0 = Color::srgb(0.8, 0.9, 1.0);
                }
            }
            UiTextRole::HoverInfo => {
                if let Some(ref info) = stats.hovered_info {
                    text.0 = info.clone();
                    color.0 = Color::srgb(1.0, 0.85, 0.4);
                } else {
                    text.0 = "Target: Hover over building walls, roofs, trees, or brush to inspect thermal stats".to_string();
                    color.0 = Color::srgb(0.65, 0.72, 0.82);
                }
            }
            _ => {}
        }
    }
}
