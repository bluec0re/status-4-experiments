# Messages and Observers

Reference material for [`bevy-game-engine`](../SKILL.md). In modern Bevy (0.17+),
communication is cleanly divided into two patterns:
1. **Buffered Messages (`Message`)**: Queued message streams processed across systems and frames using `MessageWriter` and `MessageReader`.
2. **Reactive Observers (`Event` / `EntityEvent`)**: Push-based immediate triggers and lifecycle hooks observed via `On<E>`.

---

## 1. Buffered Messages (`Message`)

Use `Message` when decoupling systems that produce streams of data read by other systems across frames.

```rust
use bevy::prelude::*;

#[derive(Message)]
struct CollisionMessage {
    entity_a: Entity,
    entity_b: Entity,
}

#[derive(Message)]
struct ScoreMessage(u32);

fn detect_collisions(
    mut collision_messages: MessageWriter<CollisionMessage>,
    query: Query<(Entity, &Transform, &Collider)>,
) {
    for [(entity_a, transform_a, _), (entity_b, transform_b, _)] in query.iter_combinations() {
        if colliding(transform_a, transform_b) {
            collision_messages.write(CollisionMessage { entity_a, entity_b });
        }
    }
}

fn handle_collisions(
    mut collision_messages: MessageReader<CollisionMessage>,
    mut score_messages: MessageWriter<ScoreMessage>,
) {
    for msg in collision_messages.read() {
        score_messages.write(ScoreMessage(10));
    }
}

fn app_setup(app: &mut App) {
    app.add_message::<CollisionMessage>()
       .add_message::<ScoreMessage>();
}
```

### Characteristics
- Register with `app.add_message::<M>()`.
- Messages are double-buffered and retained for two frames.
- A reader scheduled before its writer reads messages one frame late. Use system ordering (`.after(...)`) when immediate same-frame handling is required.

---

## 2. Reactive Observers (`Event` & `EntityEvent`)

Use observers when you want immediate, reactive event propagation without polling queues every frame.

### Global Events
Global events do not target a specific entity. Derive `Event` and trigger via `commands.trigger(...)`.

```rust
use bevy::prelude::*;

#[derive(Event)]
struct GameOver;

fn check_game_over(
    mut commands: Commands,
    query: Query<&Health, With<Player>>,
) {
    if let Ok(health) = query.single() {
        if health.0 <= 0.0 {
            commands.trigger(GameOver);
        }
    }
}

fn on_game_over(_event: On<GameOver>) {
    println!("Game Over triggered!");
}

fn app_setup(app: &mut App) {
    app.add_observer(on_game_over);
}
```

### Entity-Targeted Events (`EntityEvent`)
Derive `EntityEvent` when an event targets a specific entity.

```rust
use bevy::prelude::*;

#[derive(EntityEvent)]
struct TakeDamage {
    entity: Entity,
    amount: f32,
}

// Triggering the entity event
fn attack_system(mut commands: Commands, target: Entity) {
    commands.trigger(TakeDamage {
        entity: target,
        amount: 25.0,
    });
}

// Observing the event globally or per entity
fn on_take_damage(
    event: On<TakeDamage>,
    mut query: Query<&mut Health>,
) {
    if let Ok(mut health) = query.get_mut(event.entity) {
        health.0 -= event.amount;
        println!("Entity {:?} took damage, health remaining: {}", event.entity, health.0);
    }
}

// Entity-scoped observer attached at spawn
fn spawn_boss(mut commands: Commands) {
    commands.spawn(Boss)
        .observe(|damage: On<TakeDamage>| {
            println!("Boss took {} damage!", damage.amount);
        });
}
```

### Component Lifecycle Observers
Bevy provides built-in lifecycle triggers: `Add`, `Insert`, `Replace`, `Remove`, `Despawn`.

```rust
fn on_player_added(add: On<Add, Player>) {
    println!("Player entity {:?} spawned and initialized!", add.entity);
}

fn app_setup(app: &mut App) {
    app.add_observer(on_player_added);
}
```
