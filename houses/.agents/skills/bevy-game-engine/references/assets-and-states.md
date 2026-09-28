# Assets and Game States

Reference material for [`bevy-game-engine`](../SKILL.md). Load this when loading
assets, or when driving the game's state machine — the two pair up, because the
canonical loading screen is a state that exits once its handles resolve.

## Asset loading

```rust
use bevy::prelude::*;

#[derive(Resource)]
struct GameAssets {
    player_sprite: Handle<Image>,
    font: Handle<Font>,
    sound: Handle<AudioSource>,
}

fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(GameAssets {
        player_sprite: asset_server.load("sprites/player.png"),
        font: asset_server.load("fonts/game.ttf"),
        sound: asset_server.load("sounds/jump.ogg"),
    });
}

// Check if assets are loaded
fn check_assets_loaded(
    asset_server: Res<AssetServer>,
    assets: Res<GameAssets>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    use bevy::asset::LoadState;

    if asset_server.get_load_state(&assets.player_sprite) == Some(LoadState::Loaded) {
        next_state.set(GameState::Playing);
    }
}
```

`asset_server.load()` returns immediately with a `Handle<T>`; the load is
asynchronous. Hold the handles in a `Resource` so they are not dropped (a
dropped handle can unload the asset), and gate the transition out of the loading
state on `get_load_state` or `is_loaded_with_dependencies`.

For declarative loading states and collection derives, add `bevy_asset_loader`.

## Game states

```rust
use bevy::prelude::*;

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    Menu,
    Playing,
    Paused,
    GameOver,
}

fn setup_states(app: &mut App) {
    app.init_state::<GameState>()
       .add_systems(OnEnter(GameState::Menu), setup_menu)
       .add_systems(Update, menu_input.run_if(in_state(GameState::Menu)))
       .add_systems(Update, game_logic.run_if(in_state(GameState::Playing)));
}

// Spawn entities tagged with DespawnOnExit for automatic state cleanup
fn setup_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(GameState::Menu),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    )).with_child((
        Text::new("Main Menu"),
        TextFont::from_font_size(32.0),
        TextColor(Color::WHITE),
    ));
}

fn pause_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            _ => {}
        }
    }
}
```

### Scheduling Hooks & State-Scoped Lifetimes

| Hook / Component | Runs / Behavior |
|---|---|
| `OnEnter(S)` | Once, when state transitions to `S` — initialize systems and spawn screens |
| `OnExit(S)` | Once, when state leaves `S` — teardown systems |
| `.run_if(in_state(S))` | Every frame while state is `S` — runs systems conditionally |
| `DespawnOnExit(S)` | Component: automatically despawns the entity when state leaves `S` |
| `DespawnOnEnter(S)` | Component: automatically despawns the entity when state enters `S` |

`NextState` is applied at the state-transition point in the schedule, not
immediately, so a system that sets it still finishes the current frame.
