mod camera;
mod catalog;
mod environment;
mod floorplan;
mod generator;
mod house;
mod polygon;
mod rts;
mod textures;
mod ui;

use bevy::prelude::*;
use camera::CameraPlugin;
use environment::EnvironmentPlugin;
use generator::HouseGeneratorPlugin;
use rts::RtsPlugin;
use ui::UiPlugin;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "European City Apartment Generator & RTS Navigation".into(),
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
        CameraPlugin,
        EnvironmentPlugin,
        HouseGeneratorPlugin,
        RtsPlugin,
        UiPlugin,
    ))
    .run();
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
