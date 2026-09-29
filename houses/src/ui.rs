#![allow(dead_code)]

use bevy::prelude::*;
use bevy::pbr::wireframe::WireframeConfig;
use crate::generator::{
    GeneratorSettings, GeneratorStats, HouseAction, HouseSet, UnitOrder,
};
use crate::house::BuildingOccupant;
use crate::rts::{RtsSelectionState, RtsUnit};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    handle_ui_buttons.in_set(HouseSet::Input),
                    update_ui_labels.in_set(HouseSet::PostUpdate),
                ),
            );
    }
}

/// Unified role tag for dynamic UI text labels
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiTextRole {
    StatsInfo,
    PresetBtn,
    StyleBtn,
    StoriesBtn,
    InteriorBtn,
    CutawayBtn,
    UnitStatus,
    WireframeBtn,
}

// Button Action Markers
#[derive(Component)]
pub enum UiAction {
    Randomize,
    CyclePreset,
    CycleStyle,
    StoriesAdd,
    StoriesSub,
    ToggleInterior,
    CycleCutaway,
    SelectUnit(u32),
    OrderUnit(UnitOrder),
    ToggleWireframe,
}

pub fn setup_ui(mut commands: Commands) {
    // Root container (column layout spanning full window)
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        })
        .with_children(|root| {
            // TOP SECTION: Left Stats HUD + Right Generator Controls Panel
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|top_row| {
                // Top Left Card: Architectural & Simulation Stats
                top_row
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(16.0)),
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.0)),
                            min_width: Val::Px(380.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.92)),
                        BorderColor::all(Color::srgba(0.25, 0.35, 0.45, 0.5)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new("EUROPEAN CITY BLOCK GENERATOR"),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.95, 0.98)),
                        ));

                        card.spawn((
                            Text::new("Procedural Road-Enclosed Parcels & RTS Interior Navigation"),
                            TextFont {
                                font_size: FontSize::Px(11.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.70, 0.85)),
                        ));

                        // Divider line
                        card.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(1.0),
                                margin: UiRect::vertical(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.3, 0.4, 0.5, 0.3)),
                        ));

                        // Dynamic stats text
                        card.spawn((
                            UiTextRole::StatsInfo,
                            Text::new("Initializing simulation..."),
                            TextFont {
                                font_size: FontSize::Px(12.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.85, 0.88, 0.92)),
                        ));
                    });

                // Top Right Card: Building Controls
                top_row
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(16.0)),
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(1.0)),
                            min_width: Val::Px(360.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.92)),
                        BorderColor::all(Color::srgba(0.25, 0.35, 0.45, 0.5)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new("PARCEL & BUILDING CONTROLS"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.95, 0.98)),
                        ));

                        // Preset Switcher Button
                        spawn_button(
                            card,
                            UiAction::CyclePreset,
                            Color::srgb(0.16, 0.22, 0.30),
                            |btn| {
                                btn.spawn((
                                    UiTextRole::PresetBtn,
                                    Text::new("Parcel: Haussmann Block"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            },
                        );

                        // Style Switcher Button
                        spawn_button(
                            card,
                            UiAction::CycleStyle,
                            Color::srgb(0.16, 0.22, 0.30),
                            |btn| {
                                btn.spawn((
                                    UiTextRole::StyleBtn,
                                    Text::new("Style: Haussmannian"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            },
                        );

                        // Stories (+ / -) Row
                        card.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            width: Val::Percent(100.0),
                            ..default()
                        })
                        .with_children(|row| {
                            spawn_button_flex(row, UiAction::StoriesSub, Color::srgb(0.2, 0.15, 0.18), 1.0, |b| {
                                b.spawn((
                                    Text::new("[-] Floor"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::WHITE),
                                ));
                            });

                            row.spawn((
                                Node {
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    padding: UiRect::horizontal(Val::Px(12.0)),
                                    ..default()
                                },
                            ))
                            .with_children(|cell| {
                                cell.spawn((
                                    UiTextRole::StoriesBtn,
                                    Text::new("4 Stories"),
                                    TextFont { font_size: FontSize::Px(12.5), ..default() },
                                    TextColor(Color::srgb(0.9, 0.9, 0.95)),
                                ));
                            });

                            spawn_button_flex(row, UiAction::StoriesAdd, Color::srgb(0.15, 0.22, 0.18), 1.0, |b| {
                                b.spawn((
                                    Text::new("[+] Floor"),
                                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        });

                        // Interior Generation Toggle
                        spawn_button(
                            card,
                            UiAction::ToggleInterior,
                            Color::srgb(0.14, 0.26, 0.22),
                            |btn| {
                                btn.spawn((
                                    UiTextRole::InteriorBtn,
                                    Text::new("Internal Floors: Enabled"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            },
                        );

                        // Cutaway Selector Button
                        spawn_button(
                            card,
                            UiAction::CycleCutaway,
                            Color::srgb(0.28, 0.22, 0.12),
                            |btn| {
                                btn.spawn((
                                    UiTextRole::CutawayBtn,
                                    Text::new("Cutaway: Full Exterior"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            },
                        );

                        // Wireframe Toggle Button
                        spawn_button(
                            card,
                            UiAction::ToggleWireframe,
                            Color::srgb(0.18, 0.22, 0.28),
                            |btn| {
                                btn.spawn((
                                    UiTextRole::WireframeBtn,
                                    Text::new("[Z] Wireframe: OFF"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.8, 0.9, 1.0)),
                                ));
                            },
                        );

                        // Randomize Seed Button
                        spawn_button(
                            card,
                            UiAction::Randomize,
                            Color::srgb(0.24, 0.18, 0.32),
                            |btn| {
                                btn.spawn((
                                    Text::new("[R] Randomize Seed"),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            },
                        );
                    });
            });

            // BOTTOM BAR: RTS Squad Unit Commander Panel
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(1.0)),
                    width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.92)),
                BorderColor::all(Color::srgba(0.25, 0.35, 0.45, 0.5)),
            ))
            .with_children(|bottom| {
                bottom.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new("RTS SQUAD TACTICAL COMMANDER (Click or Press 1-3 to Select Unit)"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.95, 0.98)),
                    ));

                    header.spawn((
                        UiTextRole::UnitStatus,
                        Text::new("Active Unit: Hans (Outside Street)"),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.3, 0.8, 1.0)),
                    ));
                });

                // Unit selection & action buttons row
                bottom.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|btn_row| {
                    // Unit 1
                    spawn_button_flex(btn_row, UiAction::SelectUnit(1), Color::srgb(0.15, 0.25, 0.40), 1.0, |b| {
                        b.spawn((
                            Text::new("[1] Scout Hans"),
                            TextFont { font_size: FontSize::Px(11.5), ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    // Unit 2
                    spawn_button_flex(btn_row, UiAction::SelectUnit(2), Color::srgb(0.38, 0.18, 0.16), 1.0, |b| {
                        b.spawn((
                            Text::new("[2] Cmdr Claire"),
                            TextFont { font_size: FontSize::Px(11.5), ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    // Unit 3
                    spawn_button_flex(btn_row, UiAction::SelectUnit(3), Color::srgb(0.14, 0.32, 0.20), 1.0, |b| {
                        b.spawn((
                            Text::new("[3] Eng Leo"),
                            TextFont { font_size: FontSize::Px(11.5), ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    // Order: Enter Lobby
                    spawn_button_flex(btn_row, UiAction::OrderUnit(UnitOrder::EnterLobby), Color::srgb(0.20, 0.26, 0.32), 1.2, |b| {
                        b.spawn((
                            Text::new("-> Enter Lobby"),
                            TextFont { font_size: FontSize::Px(11.5), ..default() },
                            TextColor(Color::srgb(0.9, 0.9, 0.5)),
                        ));
                    });

                    // Order: Go to Floor 1
                    spawn_button_flex(btn_row, UiAction::OrderUnit(UnitOrder::GoToFloor(1)), Color::srgb(0.20, 0.26, 0.32), 1.2, |b| {
                        b.spawn((
                            Text::new("-> 1st Floor Apt"),
                            TextFont { font_size: FontSize::Px(11.5), ..default() },
                            TextColor(Color::srgb(0.5, 0.9, 0.6)),
                        ));
                    });

                    // Order: Go to Floor 2
                    spawn_button_flex(btn_row, UiAction::OrderUnit(UnitOrder::GoToFloor(2)), Color::srgb(0.20, 0.26, 0.32), 1.2, |b| {
                        b.spawn((
                            Text::new("-> 2nd Floor Apt"),
                            TextFont { font_size: FontSize::Px(11.5), ..default() },
                            TextColor(Color::srgb(0.5, 0.9, 0.6)),
                        ));
                    });

                    // Order: Exit to Street
                    spawn_button_flex(btn_row, UiAction::OrderUnit(UnitOrder::ExitToStreet), Color::srgb(0.28, 0.20, 0.22), 1.2, |b| {
                        b.spawn((
                            Text::new("<- Exit to Street"),
                            TextFont { font_size: FontSize::Px(11.5), ..default() },
                            TextColor(Color::srgb(1.0, 0.6, 0.6)),
                        ));
                    });
                });
            });
        });
}

// ---------------------------------------------------------------------------
// BUTTON HELPERS
// ---------------------------------------------------------------------------

fn spawn_button<F: FnOnce(&mut ChildSpawnerCommands)>(
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
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
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

fn spawn_button_flex<F: FnOnce(&mut ChildSpawnerCommands)>(
    parent: &mut ChildSpawnerCommands,
    action: UiAction,
    bg_color: Color,
    flex_grow: f32,
    spawn_content: F,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                flex_grow,
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(Color::srgba(0.35, 0.45, 0.55, 0.4)),
        ))
        .with_children(spawn_content);
}

// ---------------------------------------------------------------------------
// BUTTON CLICK EVENT DISPATCHER
// ---------------------------------------------------------------------------

pub fn handle_ui_buttons(
    interaction_query: Query<
        (&Interaction, &UiAction, &BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut action_writer: MessageWriter<HouseAction>,
    mut selection: ResMut<RtsSelectionState>,
    units: Query<(Entity, &RtsUnit)>,
    mut wireframe_config: Option<ResMut<WireframeConfig>>,
) {
    for (interaction, action, _bg) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match action {
                UiAction::Randomize => { action_writer.write(HouseAction::GenerateRandom); }
                UiAction::CyclePreset => { action_writer.write(HouseAction::CyclePreset); }
                UiAction::CycleStyle => { action_writer.write(HouseAction::CycleStyle); }
                UiAction::StoriesAdd => { action_writer.write(HouseAction::AdjustStories(1)); }
                UiAction::StoriesSub => { action_writer.write(HouseAction::AdjustStories(-1)); }
                UiAction::ToggleInterior => { action_writer.write(HouseAction::ToggleInterior); }
                UiAction::CycleCutaway => { action_writer.write(HouseAction::CycleCutaway); }
                UiAction::SelectUnit(id) => {
                    for (ent, unit) in &units {
                        if unit.unit_id == *id {
                            selection.selected_unit = Some(ent);
                            break;
                        }
                    }
                }
                UiAction::OrderUnit(order) => {
                    action_writer.write(HouseAction::DispatchUnitOrder(*order));
                }
                UiAction::ToggleWireframe => {
                    if let Some(ref mut config) = wireframe_config {
                        config.global = !config.global;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HUD LABEL UPDATER SYSTEM (SINGLE QUERY - NO B0001 CONFLICTS)
// ---------------------------------------------------------------------------

pub fn update_ui_labels(
    settings: Res<GeneratorSettings>,
    stats: Res<GeneratorStats>,
    selection: Res<RtsSelectionState>,
    units: Query<(Entity, &RtsUnit, &BuildingOccupant)>,
    wireframe_config: Option<Res<WireframeConfig>>,
    mut text_query: Query<(&mut Text, &UiTextRole)>,
) {
    let mut unit_status = "No unit selected".to_string();
    if let Some(sel_ent) = selection.selected_unit {
        for (ent, unit, occupant) in &units {
            if ent == sel_ent {
                if let Some(floor) = occupant.current_floor {
                    unit_status = format!("{}: Inside Building (Floor {})", unit.name, floor);
                } else {
                    unit_status = format!("{}: Outside on Street", unit.name);
                }
                break;
            }
        }
    }

    for (mut text, role) in &mut text_query {
        match role {
            UiTextRole::StatsInfo => {
                *text = Text::new(format!(
                    "Seed: {}\nParcel Area: {:.0} m^2 (Perimeter: {} edges)\nTotal Floor Area: {:.0} m^2\nBuilding Height: {:.1} m\nInternal Rooms: {}\nWindows: {}  |  Doorways: {}",
                    stats.seed,
                    stats.total_living_area_sqm / stats.stories.max(1) as f32,
                    settings.preset.build_polygon(settings.polygon_scale).edge_count(),
                    stats.total_living_area_sqm,
                    stats.building_height,
                    stats.total_rooms,
                    stats.windows_count,
                    stats.doorways_count,
                ));
            }
            UiTextRole::PresetBtn => {
                *text = Text::new(format!("Parcel: {}", settings.preset.name()));
            }
            UiTextRole::StyleBtn => {
                *text = Text::new(format!("Style: {}", settings.style.name()));
            }
            UiTextRole::StoriesBtn => {
                *text = Text::new(format!("{} Stories", settings.stories));
            }
            UiTextRole::InteriorBtn => {
                *text = Text::new(if settings.has_interior {
                    "Internal Floors: Enabled"
                } else {
                    "Internal Floors: Disabled"
                });
            }
            UiTextRole::CutawayBtn => {
                *text = Text::new(format!("Cutaway: {}", settings.cutaway.name()));
            }
            UiTextRole::UnitStatus => {
                *text = Text::new(unit_status.clone());
            }
            UiTextRole::WireframeBtn => {
                let is_on = wireframe_config.as_ref().map_or(false, |c| c.global);
                *text = Text::new(if is_on {
                    "[Z] Wireframe: ON"
                } else {
                    "[Z] Wireframe: OFF"
                });
            }
        }
    }
}
