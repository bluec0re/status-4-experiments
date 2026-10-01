#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use status_4_city::camera::CityCameraPlugin;
use status_4_city::city_roads::CityRoadsPlugin;
use status_4_city::osm::{OsmManager, OsmPlugin};
use status_4_city::presets;
use status_4_city::ui::CityUiPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy 10x10km OSM City Road Network - Spline Technology".into(),
            resolution: (1600u32, 950u32).into(),
            ..default()
        }),
        ..default()
    }));

    #[cfg(not(target_arch = "wasm32"))]
    {
        app.add_plugins(WireframePlugin::default());
        app.add_systems(Update, toggle_wireframe);
    }

    app.add_plugins((
        CityCameraPlugin,
        OsmPlugin,
        CityRoadsPlugin,
        CityUiPlugin,
    ));

    app.add_systems(Startup, parse_cli_args_startup);

    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut config: ResMut<WireframeConfig>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyZ) {
        config.global = !config.global;
    }
}

fn parse_cli_args_startup(mut manager: ResMut<OsmManager>) {
    let args: Vec<String> = std::env::args().collect();
    let mut custom_lat = None;
    let mut custom_lon = None;
    let mut custom_city = None;

    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--lat" || args[i] == "-lat") && i + 1 < args.len() {
            custom_lat = args[i + 1].parse::<f64>().ok();
            i += 2;
        } else if (args[i] == "--lon" || args[i] == "-lon") && i + 1 < args.len() {
            custom_lon = args[i + 1].parse::<f64>().ok();
            i += 2;
        } else if (args[i] == "--coords" || args[i] == "-c") && i + 1 < args.len() {
            let parts: Vec<&str> = args[i + 1].split(',').collect();
            if parts.len() == 2
                && let Ok(lat) = parts[0].trim().parse::<f64>()
                && let Ok(lon) = parts[1].trim().parse::<f64>()
            {
                custom_lat = Some(lat);
                custom_lon = Some(lon);
            }
            i += 2;
        } else if (args[i] == "--city" || args[i] == "-city") && i + 1 < args.len() {
            custom_city = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }

    if let Some(lat) = custom_lat
        && let Some(lon) = custom_lon
    {
        let city_name = custom_city.unwrap_or_else(|| format!("{:.4}, {:.4}", lat, lon));
        manager.trigger_fetch(lat, lon, &city_name);
    } else if let Some(city_name) = custom_city
        && let Some(preset) = presets::CITY_PRESETS.iter().find(|p| p.name.eq_ignore_ascii_case(&city_name))
    {
        manager.load_offline_preset(preset.lat, preset.lon, preset.name);
    }
}
