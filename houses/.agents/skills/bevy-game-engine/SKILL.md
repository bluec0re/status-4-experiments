---
created: 2025-12-16
modified: 2026-09-27
reviewed: 2026-09-27
name: bevy-game-engine
description: "Bevy game engine: ECS, rendering, input, and asset management. Use when building Bevy games, working with entities/components/systems, or mentioning Rust gamedev or 2D/3D games."
user-invocable: false
allowed-tools: Glob, Grep, Read, Bash(cargo *), Edit, Write, TodoWrite, WebFetch, WebSearch
---

# Bevy Game Engine

Expert knowledge for developing games with modern Bevy (0.19+), the data-driven game engine built in Rust with a focus on ergonomics, modularity, and performance.

## When to Use This Skill

| Use this skill when... | Use bevy-ecs-patterns instead when... |
|------------------------|---------------------------------------|
| Starting a new Bevy game project | Optimizing ECS query performance or archetype layout |
| Learning or applying basic ECS concepts | Implementing complex system scheduling or ordering |
| Handling input (keyboard, mouse, gamepad) | Using change detection (`Changed<T>`, `Added<T>`) |
| Managing game states and transitions | Working with `ParamSet` or parallel query iteration |
| Loading and managing assets | Designing entity relationship hierarchies (`ChildOf`, `Children`) |
| Setting up plugins and app structure | Debugging archetype fragmentation or storage strategies |
| Working with messages, events, and observers | Implementing batch spawn or deferred operations |

## Core Expertise

**Bevy Architecture**
- **Entity Component System (ECS)**: Data-oriented design with entities, components, and systems
- **Required Components**: Modern component composition via `#[require(Transform, Visibility)]`
- **Plugin System**: Modular game organization with reusable plugins
- **Schedules**: System ordering and execution timing (`Update`, `FixedUpdate`, `Startup`)
- **Resources**: Global singleton data accessible to systems (`Res<T>`, `ResMut<T>`)
- **Messages & Events**: Double-buffered `Message` passing across systems, and reactive `Event`/`EntityEvent` triggers with `Observer`
- **States**: Game state management, transitions, and scoped lifetimes (`DespawnOnExit`)

**Rendering**
- **2D Rendering**: `Sprite`, `Mesh2d`, `MeshMaterial2d`, text rendering, 2D cameras (`Camera2d`)
- **3D Rendering**: `Mesh3d`, `MeshMaterial3d`, PBR `StandardMaterial`, lighting (`DirectionalLight`, `PointLight`), shadows, 3D cameras (`Camera3d`)
- **UI**: Modern `bevy_ui` layout (`Node`, `ImageNode`, `Text`, `TextFont`, `TextColor`)
- **Shaders**: Custom WGSL shaders, materials, and render pipelines

## Reference Files

The ECS core, project setup, and the command set below are everything a first
pass needs. Follow one link when the task calls for it — nothing under
`references/` is loaded unless you open it.

| Path you are on | File | Carries |
|---|---|---|
| Reading player input | [`references/input.md`](references/input.md) | `ButtonInput` keyboard/mouse polling, pressed vs just-pressed, cursor position, pointer picking events, and gamepad pointers |
| Loading assets, or driving the state machine | [`references/assets-and-states.md`](references/assets-and-states.md) | `AssetServer` handles and `LoadState` gating, `States` enum, `OnEnter`/`OnExit`/`run_if(in_state)`, `DespawnOnExit` state-scoped cleanup, `NextState` timing |
| Decoupling systems with messages or observers | [`references/events.md`](references/events.md) | `#[derive(Message)]` with `MessageWriter`/`MessageReader` for buffered queues, and `#[derive(Event)]`/`#[derive(EntityEvent)]` with Observers (`On<E>`) |
| Laying out a growing game, or chasing frame time | [`references/project-architecture.md`](references/project-architecture.md) | Directory layout, plugin/marker-component organization, query-filter guidance, required components, and system sets |

## Key Capabilities

**ECS Fundamentals**
```rust
use bevy::prelude::*;

// Components are plain data structs.
// In Bevy 0.15+, use required components instead of bundles.
#[derive(Component)]
#[require(Transform, Visibility)]
struct Player;

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct Velocity(Vec2);

// Spawn entities with component tuples
fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        Health(100.0),
        Velocity(Vec2::ZERO),
        Sprite::default(),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

// Systems query for components
fn move_player(
    time: Res<Time>,
    mut query: Query<(&Velocity, &mut Transform), With<Player>>,
) {
    for (velocity, mut transform) in &mut query {
        transform.translation += velocity.0.extend(0.0) * time.delta_secs();
    }
}
```

**App Structure**
```rust
use bevy::prelude::*;

fn main() {
    App::new()
        // Default plugins (window, rendering, input, UI, etc.)
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins(GamePlugin)
        // Resources
        .insert_resource(GameSettings::default())
        // Startup systems (run once)
        .add_systems(Startup, setup)
        // Update systems (run every frame)
        .add_systems(Update, (
            player_movement,
            collision_detection,
            update_score,
        ))
        .run();
}

// Organize with plugins
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
           .add_systems(Update, player_input);
    }
}
```

## Essential Commands

```bash
# Create new Bevy project from the official template
cargo generate --git https://github.com/TheBevyFlock/bevy_new_2d --name my_game

# Or start from an empty crate
cargo new my_game
cd my_game
cargo add bevy

# Run with fast compile times (debug)
cargo run

# Run with optimizations
cargo run --release

# Enable dynamic linking for faster compiles (dev only)
cargo run --features bevy/dynamic_linking

# Common dev dependencies
cargo add bevy_egui           # Debug UI
cargo add bevy_rapier2d       # 2D physics
cargo add bevy_rapier3d       # 3D physics
cargo add bevy_asset_loader   # Asset loading helpers
cargo add leafwing-input-manager  # Advanced input
```

## Agentic Optimizations

| Context | Command |
|---------|---------|
| Quick compile check | `cargo check 2>&1 \| head -30` |
| Fast test run | `cargo test --lib -- --test-threads=1 -q` |
| Run with fast compiles (dev) | `cargo run --features bevy/dynamic_linking` |
| Run optimized build | `cargo run --release` |
| Check for common issues | `cargo clippy -- -W clippy::all 2>&1 \| head -50` |
| List plugins in project | `grep -rn "impl Plugin for" src/ --include="*.rs"` |
| List game states | `grep -rn "derive.*States" src/ --include="*.rs"` |
| Find message & event definitions | `grep -rn "derive.*\(Message\|Event\|EntityEvent\)" src/ --include="*.rs"` |
| List dependencies | `cargo metadata --format-version=1 \| jq -r '.packages[0].dependencies[].name'` |

For detailed ECS patterns, advanced queries, relationship hierarchies, and system scheduling, see the bevy-ecs-patterns skill.
