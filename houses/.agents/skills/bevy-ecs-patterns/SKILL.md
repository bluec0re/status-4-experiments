---
created: 2025-12-16
modified: 2026-09-27
reviewed: 2026-09-27
name: bevy-ecs-patterns
description: "Advanced Bevy ECS: complex queries, system scheduling, change detection, and performance tuning. Use when optimizing Bevy architecture or implementing complex game systems."
user-invocable: false
allowed-tools: Glob, Grep, Read, Bash(cargo *), Edit, Write, TodoWrite, WebFetch, WebSearch
---

# Bevy ECS Patterns

Advanced patterns and techniques for Bevy's Entity Component System (0.19+), focusing on performance, maintainability, and complex game architectures.

## When to Use This Skill

| Use this skill when... | Use bevy-game-engine instead when... |
|------------------------|--------------------------------------|
| Optimizing ECS query performance | Setting up a new Bevy project from scratch |
| Implementing entity relationships or hierarchies (`ChildOf`, `Children`) | Learning basic ECS concepts (components, systems, resources) |
| Designing system scheduling and ordering | Handling basic input, assets, or simple game states |
| Using change detection (`Changed<T>`, `Added<T>`, `Ref<T>`) | Writing basic game logic or app structure |
| Working with `ParamSet`, parallel iteration, or batch operations | Configuring rendering, UI, or audio |
| Debugging archetype fragmentation or storage strategies | Adding plugins or common Bevy dependencies |

## Core Expertise

**Advanced Queries**
- Complex query filters and combinations (`With<T>`, `Without<T>`, `Or<(...)>`)
- Query state and caching
- Entity relationships (`ChildOf`, `Children`)
- Parallel iteration strategies (`par_iter`, `par_iter_mut`)

**System Scheduling**
- System ordering and dependencies (`.after()`, `.before()`, `.chain()`)
- Run conditions and gating (`run_if(...)`)
- System sets and explicit scheduling (`configure_sets`)
- Fixed timestep systems (`FixedUpdate`, `Time<Fixed>`)

**Change Detection**
- `Changed<T>` and `Added<T>` filters
- `Ref<T>` for fine-grained change tracking
- Efficient reactive systems and observer triggers

## Advanced Query Patterns

**Query Filters**
```rust
use bevy::prelude::*;

// Multiple filters
fn targeting_system(
    query: Query<
        (&Transform, &mut Target),
        (With<Enemy>, Without<Dead>, Changed<Health>),
    >,
) {
    for (transform, mut target) in &query {
        // Only enemies that are alive and recently damaged
    }
}

// Optional components
fn flexible_system(
    query: Query<(&Transform, Option<&Velocity>, Option<&Acceleration>)>,
) {
    for (transform, velocity, acceleration) in &query {
        if let Some(vel) = velocity {
            // Has velocity
        }
    }
}

// Or filters
fn pickup_system(
    query: Query<Entity, Or<(With<HealthPickup>, With<AmmoPickup>)>>,
) {
    for entity in &query {
        // Process any pickup type
    }
}
```

**Multiple Queries with Conflicts (`ParamSet`)**
```rust
use bevy::prelude::*;

// Use ParamSet when queries have conflicting component access
fn combat_system(
    mut param_set: ParamSet<(
        Query<&mut Health, With<Player>>,
        Query<&mut Health, With<Enemy>>,
    )>,
) {
    // Access one query at a time
    for mut health in param_set.p0().iter_mut() {
        health.0 += 1.0; // Heal player
    }

    for mut health in param_set.p1().iter_mut() {
        health.0 -= 10.0; // Damage enemies
    }
}
```

**Entity Relationships & Hierarchy (Bevy 0.16+)**

In modern Bevy, parent-child relationships use `ChildOf` on the child entity and `Children` on the parent:

```rust
use bevy::prelude::*;

#[derive(Component)]
struct Inventory;

#[derive(Component)]
struct InventorySlot(usize);

// Spawning hierarchies
fn spawn_inventory(mut commands: Commands) {
    let parent = commands.spawn((
        Inventory,
        Transform::default(),
        Visibility::default(),
    )).id();

    // Spawning children directly with ChildOf
    for i in 0..10 {
        commands.spawn((
            InventorySlot(i),
            ChildOf(parent),
            Transform::default(),
        ));
    }

    // Alternatively, spawn via with_children / with_child
    commands.spawn((Inventory, Transform::default()))
        .with_children(|p| {
            for i in 0..10 {
                p.spawn(InventorySlot(i));
            }
        });
}

// Querying parent from child
fn child_system(
    children: Query<(&InventorySlot, &ChildOf)>,
    parents: Query<&Inventory>,
) {
    for (slot, child_of) in &children {
        let parent_entity = child_of.parent();
        if let Ok(inventory) = parents.get(parent_entity) {
            // Access parent inventory
        }
    }
}

// Querying children from parent
fn parent_system(
    parents: Query<&Children, With<Inventory>>,
    slots: Query<&InventorySlot>,
) {
    for children in &parents {
        for &child in children.iter() {
            if let Ok(slot) = slots.get(child) {
                // Access child slot
            }
        }
    }
}
```

## Agentic Optimizations

| Context | Command |
|---------|---------|
| Quick compile check | `cargo check 2>&1 \| head -30` |
| Fast test run | `cargo test --lib -- --test-threads=1 -q` |
| Check ECS usage errors | `cargo clippy -- -W clippy::all 2>&1 \| head -50` |
| List components in project | `grep -rn "derive.*Component" src/ --include="*.rs"` |
| List systems in project | `grep -rn "fn.*Query<" src/ --include="*.rs"` |
| Find system ordering | `grep -rn "\.add_systems\|\.chain()\|\.after(\|\.before(" src/ --include="*.rs"` |
| Check for ParamSet usage | `grep -rn "ParamSet" src/ --include="*.rs"` |

For system scheduling, change detection, performance patterns, custom entity commands, and design best practices, see [REFERENCE.md](REFERENCE.md).
