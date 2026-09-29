mod camera;
mod environment;
mod fire_sim;
mod house_compat;
mod interaction;
mod material;
mod rendering;
mod scenarios;
mod ui;

use bevy::prelude::*;
use camera::CameraPlugin;
use environment::EnvironmentPlugin;
use fire_sim::FireSimPlugin;
use interaction::InteractionPlugin;
use rendering::FireRenderingPlugin;
use scenarios::ScenarioPlugin;
use ui::UiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Thermodynamic Fire Propagation & Suppression Simulation".into(),
                resolution: (1600u32, 950u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            CameraPlugin,
            EnvironmentPlugin,
            FireSimPlugin,
            FireRenderingPlugin,
            InteractionPlugin,
            ScenarioPlugin,
            UiPlugin,
        ))
        .run();
}
