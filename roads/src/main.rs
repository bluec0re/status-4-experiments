mod camera;
mod editor;
mod road;
mod spline;
mod terrain;
mod ui;
mod vegetation;

use bevy::prelude::*;
use camera::CameraPlugin;
use editor::RoadEditorPlugin;
use terrain::TerrainPlugin;
use ui::UiPlugin;
use vegetation::VegetationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Spline Road Editor - Realistic Landscape & Dynamic Trees".into(),
                resolution: (1600u32, 950u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            CameraPlugin,
            TerrainPlugin,
            RoadEditorPlugin,
            VegetationPlugin,
            UiPlugin,
        ))
        .run();
}
