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

use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "European City Apartment Generator & RTS Navigation".into(),
                resolution: (1600u32, 950u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WireframePlugin::default())
        .add_plugins((
            CameraPlugin,
            EnvironmentPlugin,
            HouseGeneratorPlugin,
            RtsPlugin,
            UiPlugin,
        ))
        .add_systems(Update, toggle_wireframe)
        .run();
}

fn toggle_wireframe(
    mut config: ResMut<WireframeConfig>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyZ) {
        config.global = !config.global;
    }
}
