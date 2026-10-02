use crate::camera::{CameraViewPreset, CityCamera};
use crate::city_roads::CityStats;
use crate::osm::{FetchStatus, OsmManager};
use crate::presets::CITY_PRESETS;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

pub struct CityUiPlugin;

impl Plugin for CityUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, setup_city_ui)
            .add_systems(
                Update,
                (
                    handle_city_ui_buttons,
                    update_city_ui_labels,
                    update_fps_counter,
                ),
            );
    }
}

#[derive(Component, Clone, Debug)]
pub enum CityUiAction {
    SelectPreset(usize),
    FetchOsm,
    LoadOffline,
    AdjustLat(f64),
    AdjustLon(f64),
    SetCameraPreset(CameraViewPreset),
    ToggleCinematic,
}

#[derive(Component)]
pub struct CityCoordsTextMarker;

#[derive(Component)]
pub struct CityStatusTextMarker;

#[derive(Component)]
pub struct CityStatsTextMarker;

#[derive(Component)]
pub struct CityFpsTextMarker;

#[derive(Component)]
pub struct CityCinematicBtnMarker;

fn setup_city_ui(mut commands: Commands) {
    // Root full-viewport container
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
            // ----------------------------------------------------------------
            // TOP SECTION: Header & City Location Selector
            // ----------------------------------------------------------------
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            })
            .insert((
                BackgroundColor(Color::srgba(0.06, 0.09, 0.14, 0.92)),
                BorderColor::all(Color::srgba(0.22, 0.74, 0.97, 0.35)),
            ))
            .with_children(|top_bar| {
                // Title and Subtitle
                top_bar
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|col| {
                        col.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("STATUS 4 // CITY EXPERIMENT"),
                                TextFont {
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.98, 1.0)),
                            ));

                            // Badge
                            row.spawn(Node {
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            })
                            .insert(BackgroundColor(Color::srgba(0.22, 0.74, 0.97, 0.2)))
                            .with_children(|badge| {
                                badge.spawn((
                                    Text::new("10x10 km OSM Splines"),
                                    TextFont {
                                        font_size: FontSize::Px(10.5),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.22, 0.74, 0.97)),
                                ));
                            });
                        });

                        col.spawn((
                            Text::new("Fetches OpenStreetMap bounding box coordinates & renders 3D spline road ribbons"),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.60, 0.68, 0.78)),
                        ));
                    });

                // Top Right: FPS Counter & Presets Row
                top_bar
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new("60 FPS"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.35, 0.85, 0.55)),
                            CityFpsTextMarker,
                        ));

                        // Preset buttons
                        for (idx, preset) in CITY_PRESETS.iter().enumerate() {
                            row.spawn((
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.12, 0.16, 0.24, 0.8)),
                                BorderColor::all(Color::srgba(0.3, 0.4, 0.55, 0.3)),
                                CityUiAction::SelectPreset(idx),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(preset.name),
                                    TextFont {
                                        font_size: FontSize::Px(11.5),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.90, 0.96)),
                                ));
                            });
                        }
                    });
            });

            // ----------------------------------------------------------------
            // MIDDLE ROW: Left Floating Control Card & Stats
            // ----------------------------------------------------------------
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|mid_row| {
                // Left Panel: Location & Fetch Controls
                mid_row
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(360.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        row_gap: Val::Px(12.0),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        ..default()
                    })
                    .insert((
                        BackgroundColor(Color::srgba(0.07, 0.10, 0.16, 0.94)),
                        BorderColor::all(Color::srgba(0.25, 0.35, 0.5, 0.4)),
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            Text::new("COORDINATES & 10x10km BBOX"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.22, 0.74, 0.97)),
                        ));

                        // Dynamic Coordinate Readout
                        panel.spawn((
                            Text::new("Center: Lat: 52.5163, Lon: 13.3777\nBBox: S 52.4714, W 13.3041 | N 52.5612, E 13.4513"),
                            TextFont {
                                font_size: FontSize::Px(11.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.88, 0.92, 0.98)),
                            CityCoordsTextMarker,
                        ));

                        // Coordinate adjustment buttons
                        panel
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(6.0),
                                ..default()
                            })
                            .with_children(|adj_row| {
                                spawn_mini_btn(adj_row, "Lat -0.05", CityUiAction::AdjustLat(-0.05));
                                spawn_mini_btn(adj_row, "Lat +0.05", CityUiAction::AdjustLat(0.05));
                                spawn_mini_btn(adj_row, "Lon -0.05", CityUiAction::AdjustLon(-0.05));
                                spawn_mini_btn(adj_row, "Lon +0.05", CityUiAction::AdjustLon(0.05));
                            });

                        // Action Buttons: Fetch OSM & Load Offline
                        panel
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(8.0),
                                margin: UiRect::top(Val::Px(4.0)),
                                ..default()
                            })
                            .with_children(|btn_row| {
                                // Primary Fetch Button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            flex_grow: 1.0,
                                            padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.0)),
                                            border_radius: BorderRadius::all(Val::Px(8.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.09, 0.45, 0.65, 0.95)),
                                        BorderColor::all(Color::srgb(0.22, 0.74, 0.97)),
                                        CityUiAction::FetchOsm,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new("[+] Fetch OSM (Overpass)"),
                                            TextFont {
                                                font_size: FontSize::Px(12.0),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(1.0, 1.0, 1.0)),
                                        ));
                                    });

                                // Offline Preset Button
                                btn_row
                                    .spawn((
                                        Button,
                                        Node {
                                            padding: UiRect::axes(Val::Px(10.0), Val::Px(10.0)),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.0)),
                                            border_radius: BorderRadius::all(Val::Px(8.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.18, 0.22, 0.32, 0.9)),
                                        BorderColor::all(Color::srgba(0.4, 0.5, 0.65, 0.4)),
                                        CityUiAction::LoadOffline,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new("Offline Model"),
                                            TextFont {
                                                font_size: FontSize::Px(11.5),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.85, 0.88, 0.94)),
                                        ));
                                    });
                            });

                        // Status Banner
                        panel
                            .spawn(Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            })
                            .insert(BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.8)))
                            .with_children(|status_box| {
                                status_box.spawn((
                                    Text::new("Status: Ready (Bundled network loaded)"),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.35, 0.85, 0.55)),
                                    CityStatusTextMarker,
                                ));
                            });

                        // Statistics Section
                        panel.spawn((
                            Text::new("ROAD NETWORK STATS"),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.85, 0.70, 0.30)),
                        ));

                        panel.spawn((
                            Text::new("Road Segments: 0\nTotal Network: 0.0 km\nVertices: 0 | Triangles: 0\nSpline Gen Time: 0 ms"),
                            TextFont {
                                font_size: FontSize::Px(11.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.88, 0.95)),
                            CityStatsTextMarker,
                        ));
                    });

                // Right Panel: Camera View Controls
                mid_row
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(180.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    })
                    .insert((
                        BackgroundColor(Color::srgba(0.07, 0.10, 0.16, 0.92)),
                        BorderColor::all(Color::srgba(0.25, 0.35, 0.5, 0.4)),
                    ))
                    .with_children(|cam_panel| {
                        cam_panel.spawn((
                            Text::new("CAMERA PRESETS"),
                            TextFont {
                                font_size: FontSize::Px(11.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.22, 0.74, 0.97)),
                        ));

                        spawn_cam_btn(cam_panel, "1. Satellite (Top)", CityUiAction::SetCameraPreset(CameraViewPreset::Satellite));
                        spawn_cam_btn(cam_panel, "2. Overview (45 deg)", CityUiAction::SetCameraPreset(CameraViewPreset::Overview));
                        spawn_cam_btn(cam_panel, "3. Isometric (35 deg)", CityUiAction::SetCameraPreset(CameraViewPreset::Isometric));
                        spawn_cam_btn(cam_panel, "4. Street Level", CityUiAction::SetCameraPreset(CameraViewPreset::Street));

                        cam_panel
                            .spawn((
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(6.0)),
                                    justify_content: JustifyContent::Center,
                                    margin: UiRect::top(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.15, 0.20, 0.30, 0.85)),
                                BorderColor::all(Color::srgba(0.35, 0.45, 0.6, 0.4)),
                                CityUiAction::ToggleCinematic,
                                CityCinematicBtnMarker,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new("Cinematic Orbit: OFF"),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.90, 0.96)),
                                ));
                            });
                    });
            });

            // ----------------------------------------------------------------
            // BOTTOM BAR: Controls Cheat-Sheet & OSM Attribution
            // ----------------------------------------------------------------
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                row_gap: Val::Px(4.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            })
            .insert((
                BackgroundColor(Color::srgba(0.06, 0.08, 0.13, 0.9)),
                BorderColor::all(Color::srgba(0.25, 0.32, 0.45, 0.3)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Text::new("WASD / Arrows: Pan City | Mouse Wheel: Zoom | Right-Drag: Orbit | Middle-Drag: Pan | 1-4: Camera | Space: Orbit | R: Reset"),
                    TextFont {
                        font_size: FontSize::Px(11.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.65, 0.72, 0.82)),
                ));

                bar.spawn((
                    Text::new("Data Source: OpenStreetMap (© OpenStreetMap contributors)"),
                    TextFont {
                        font_size: FontSize::Px(10.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.50, 0.58, 0.70)),
                ));
            });
        });
}

fn spawn_mini_btn(parent: &mut ChildSpawnerCommands, text: &str, action: CityUiAction) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.14, 0.18, 0.26, 0.8)),
            BorderColor::all(Color::srgba(0.3, 0.4, 0.55, 0.3)),
            action,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(10.5),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.88, 0.94)),
            ));
        });
}

fn spawn_cam_btn(parent: &mut ChildSpawnerCommands, text: &str, action: CityUiAction) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.13, 0.17, 0.25, 0.85)),
            BorderColor::all(Color::srgba(0.3, 0.4, 0.55, 0.3)),
            action,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.88, 0.94)),
            ));
        });
}

fn handle_city_ui_buttons(
    mut interaction_query: Query<
        (&Interaction, &CityUiAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut manager: ResMut<OsmManager>,
    mut cam_query: Query<&mut CityCamera>,
) {
    for (interaction, action, mut bg_color) in &mut interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            CityUiAction::SelectPreset(idx) => {
                if let Some(preset) = CITY_PRESETS.get(*idx) {
                    println!("[UI] Clicked Preset button [{}]: '{}' (lat: {:.4}, lon: {:.4})", idx, preset.name, preset.lat, preset.lon);
                    manager.load_offline_preset(preset.lat, preset.lon, preset.name);
                    *bg_color = BackgroundColor(Color::srgba(0.22, 0.74, 0.97, 0.9));
                }
            }
            CityUiAction::FetchOsm => {
                let lat = manager.current_bbox.center_lat;
                let lon = manager.current_bbox.center_lon;
                let name = manager.current_city_name.clone();
                println!("[UI] Clicked Fetch OSM button for '{}' (lat: {:.4}, lon: {:.4})", name, lat, lon);
                manager.trigger_fetch(lat, lon, &name);
                *bg_color = BackgroundColor(Color::srgba(0.22, 0.74, 0.97, 0.9));
            }
            CityUiAction::LoadOffline => {
                let lat = manager.current_bbox.center_lat;
                let lon = manager.current_bbox.center_lon;
                let name = manager.current_city_name.clone();
                println!("[UI] Clicked Load Offline button for '{}' (lat: {:.4}, lon: {:.4})", name, lat, lon);
                manager.load_offline_preset(lat, lon, &name);
            }
            CityUiAction::AdjustLat(delta) => {
                let new_lat = (manager.current_bbox.center_lat + delta).clamp(-85.0, 85.0);
                let lon = manager.current_bbox.center_lon;
                let name = manager.current_city_name.clone();
                manager.current_bbox = crate::osm::OsmBBox::from_center(new_lat, lon, 10.0);
                manager.load_offline_preset(new_lat, lon, &name);
            }
            CityUiAction::AdjustLon(delta) => {
                let lat = manager.current_bbox.center_lat;
                let mut new_lon = manager.current_bbox.center_lon + delta;
                if new_lon > 180.0 {
                    new_lon -= 360.0;
                } else if new_lon < -180.0 {
                    new_lon += 360.0;
                }
                let name = manager.current_city_name.clone();
                manager.current_bbox = crate::osm::OsmBBox::from_center(lat, new_lon, 10.0);
                manager.load_offline_preset(lat, new_lon, &name);
            }
            CityUiAction::SetCameraPreset(preset) => {
                if let Ok(mut cam) = cam_query.single_mut() {
                    cam.apply_preset(*preset);
                }
            }
            CityUiAction::ToggleCinematic => {
                if let Ok(mut cam) = cam_query.single_mut() {
                    cam.is_cinematic = !cam.is_cinematic;
                }
            }
        }
    }
}

fn update_city_ui_labels(
    manager: Res<OsmManager>,
    stats: Res<CityStats>,
    cam_query: Query<&CityCamera>,
    mut coords_query: Query<&mut Text, (With<CityCoordsTextMarker>, Without<CityStatusTextMarker>, Without<CityStatsTextMarker>)>,
    mut status_query: Query<(&mut Text, &mut TextColor), (With<CityStatusTextMarker>, Without<CityCoordsTextMarker>, Without<CityStatsTextMarker>)>,
    mut stats_query: Query<&mut Text, (With<CityStatsTextMarker>, Without<CityCoordsTextMarker>, Without<CityStatusTextMarker>)>,
    mut cinematic_btn_query: Query<&mut Text, (With<CityCinematicBtnMarker>, Without<CityCoordsTextMarker>, Without<CityStatusTextMarker>, Without<CityStatsTextMarker>)>,
) {
    // 1. Update Coordinates
    let bbox = &manager.current_bbox;
    let coords_str = format!(
        "Center: Lat: {:.4}, Lon: {:.4} ({})\nBBox: S {:.4}, W {:.4} | N {:.4}, E {:.4}",
        bbox.center_lat, bbox.center_lon, manager.current_city_name,
        bbox.south, bbox.west, bbox.north, bbox.east
    );
    for mut text in &mut coords_query {
        text.0 = coords_str.clone();
    }

    // 2. Update Status
    for (mut text, mut color) in &mut status_query {
        match &manager.status {
            FetchStatus::Idle => {
                text.0 = "Status: Ready".to_string();
                *color = TextColor(Color::srgb(0.35, 0.85, 0.55));
            }
            FetchStatus::Loading(msg) => {
                text.0 = format!("Status: {}", msg);
                *color = TextColor(Color::srgb(0.22, 0.74, 0.97));
            }
            FetchStatus::Success { roads_count, total_km, city_name } => {
                text.0 = format!("Status: Loaded {} ({} ways, {:.1} km)", city_name, roads_count, total_km);
                *color = TextColor(Color::srgb(0.35, 0.85, 0.55));
            }
            FetchStatus::Error(err) => {
                text.0 = format!("Status: Error: {}", err);
                *color = TextColor(Color::srgb(0.95, 0.35, 0.35));
            }
        }
    }

    // 3. Update Stats
    let stats_str = format!(
        "Road Segments: {} | Waterways: {}\nTotal Network: {:.1} km\nVertices: {} | Triangles: {}\nSpline Gen Time: {} ms",
        stats.total_roads,
        stats.total_waterways,
        stats.total_km,
        stats.total_vertices,
        stats.total_triangles,
        stats.generation_ms
    );
    for mut text in &mut stats_query {
        text.0 = stats_str.clone();
    }

    // 4. Update Cinematic Button
    if let Ok(cam) = cam_query.single() {
        for mut text in &mut cinematic_btn_query {
            text.0 = if cam.is_cinematic {
                "Cinematic Orbit: ON".to_string()
            } else {
                "Cinematic Orbit: OFF".to_string()
            };
        }
    }
}

fn update_fps_counter(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<CityFpsTextMarker>>,
) {
    let Some(fps_diag) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) else {
        return;
    };
    let Some(fps) = fps_diag.smoothed() else {
        return;
    };

    for mut text in &mut query {
        text.0 = format!("{:.0} FPS", fps);
    }
}
