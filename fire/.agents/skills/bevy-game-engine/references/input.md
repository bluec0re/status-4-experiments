# Input Handling

Reference material for [`bevy-game-engine`](../SKILL.md). Load this when
reading player input — keyboard, mouse, or gamepad — or handling entity picking.

## Keyboard

```rust
use bevy::prelude::*;

fn player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    let mut direction = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

    for mut velocity in &mut query {
        velocity.0 = direction.normalize_or_zero() * 200.0;
    }
}
```

`ButtonInput<T>` distinguishes three states:
- `pressed(key)`: Returns `true` continuously while held down.
- `just_pressed(key)`: Returns `true` only on the initial down-transition frame.
- `just_released(key)`: Returns `true` only on the release-transition frame.

## Mouse & Cursor Position

```rust
use bevy::prelude::*;

fn mouse_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(position) = window.cursor_position() {
                println!("Clicked at logical cursor position: {:?}", position);
            }
        }
    }
}
```

Cursor position is queried from the `Window` component, not the input resource,
and is `None` when the cursor is outside the window.

## Pointer Events & Picking

Modern Bevy includes built-in picking observers for UI and 2D/3D entities. Attach observer callbacks directly to entities:

```rust
use bevy::prelude::*;

fn spawn_clickable_button(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Px(120.0),
            height: Val::Px(40.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
    ))
    .observe(|_click: On<Pointer<Click>>| {
        println!("Button clicked!");
    })
    .observe(|_over: On<Pointer<Over>>| {
        println!("Pointer hovered button!");
    });
}
```

Picking events include: `Pointer<Press>`, `Pointer<Release>`, `Pointer<Click>`, `Pointer<Over>`, `Pointer<Out>`, `Pointer<DragStart>`, `Pointer<Drag>`, `Pointer<DragEnd>`.

## Gamepad and Advanced Input

Gamepad buttons and axes follow the same `ButtonInput<GamepadButton>` and `Axis<GamepadAxis>` patterns.
For configurable action bindings, layered contexts, and multi-input abstractions, use `leafwing-input-manager`.
