#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use status_4_roads::camera::CameraPlugin;
use status_4_roads::editor::RoadEditorPlugin;
use status_4_roads::terrain::TerrainPlugin;
use status_4_roads::ui::UiPlugin;
use status_4_roads::vegetation::VegetationPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy Spline Road Editor - Realistic Landscape & Dynamic Trees".into(),
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
        TerrainPlugin,
        RoadEditorPlugin,
        VegetationPlugin,
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
