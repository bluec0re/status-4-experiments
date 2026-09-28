# Project Architecture

Reference material for [`bevy-game-engine`](../SKILL.md). Load this when laying
out a growing game — where files go, how to group systems, and what to reach for
when the frame budget slips.

## Project Structure

```
my_game/
├── Cargo.toml
├── assets/
│   ├── sprites/
│   ├── fonts/
│   ├── sounds/
│   └── shaders/
└── src/
    ├── main.rs
    ├── lib.rs           # Optional library crate
    ├── plugins/
    │   ├── mod.rs
    │   ├── player.rs
    │   ├── enemy.rs
    │   └── ui.rs
    ├── components/
    │   └── mod.rs
    ├── resources/
    │   └── mod.rs
    ├── systems/
    │   └── mod.rs
    └── messages/        # Buffered messages or observer events
        └── mod.rs
```

`assets/` is resolved relative to the working directory at run time, not
compiled in — ship it alongside the binary.

## Performance

- Use `Query` filters (`With<T>`, `Without<T>`) to narrow iteration.
- Avoid `Query::iter()` when you need specific entities (use `get()` / `get_mut()`).
- Use `Changed<T>` and `Added<T>` filters for reactive systems.
- Use `par_iter()` / `par_iter_mut()` for parallel component processing across chunks.
- Profile with `bevy_diagnostic` and Tracy.

## Code Organization

- Group related components, systems, and messages into modular plugins.
- Use marker components for entity classification (`Player`, `Enemy`, `Obstacle`).
- Use **Required Components** (`#[require(...)]`) to define dependency invariants automatically.
- Keep systems focused and single-purpose.
- Prefer messages/observers over direct cross-system coupling.

## Modern Component Composition & System Sets

```rust
use bevy::prelude::*;

// Marker components
#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Health(f32);

// In Bevy 0.15+, Required Components replace Bundles:
// Spawning Enemy will automatically spawn Transform and Visibility if omitted!
#[derive(Component)]
#[require(Transform, Visibility)]
struct Character;

// Spawning with tuple compositions
fn spawn_enemy(mut commands: Commands) {
    commands.spawn((
        Enemy,
        Character,
        Health(100.0),
        Sprite::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

// System sets for ordering
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameSet {
    Input,
    Movement,
    Collision,
    Render,
}

fn configure_sets(app: &mut App) {
    app.configure_sets(Update, (
        GameSet::Input,
        GameSet::Movement.after(GameSet::Input),
        GameSet::Collision.after(GameSet::Movement),
        GameSet::Render.after(GameSet::Collision),
    ));
}
```

For archetype layout, parallel query iteration, change-detection mechanics, and
system-set scheduling in depth, use the `bevy-ecs-patterns` skill.
