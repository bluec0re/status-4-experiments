use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, update_rts_camera);
    }
}

/// RTS / Orbit inspection camera matching the responsive controls in status-4-roads and houses
#[derive(Component)]
#[require(Camera3d, Transform, Visibility)]
pub struct RtsCamera {
    // Current smoothed state
    pub target: Vec3,
    pub distance: f32,
    pub pitch: f32,
    pub yaw: f32,

    // Target desired state for smooth damping
    pub desired_target: Vec3,
    pub desired_distance: f32,
    pub desired_pitch: f32,
    pub desired_yaw: f32,

    // Navigation configuration
    pub pan_speed: f32,
    pub rotate_speed: f32,
    pub zoom_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,
}

impl Default for RtsCamera {
    fn default() -> Self {
        let initial_target = Vec3::new(0.0, 2.0, 0.0);
        let initial_dist = 52.0;
        let initial_pitch = 48.0f32.to_radians();
        let initial_yaw = -30.0f32.to_radians();

        Self {
            target: initial_target,
            distance: initial_dist,
            pitch: initial_pitch,
            yaw: initial_yaw,

            desired_target: initial_target,
            desired_distance: initial_dist,
            desired_pitch: initial_pitch,
            desired_yaw: initial_yaw,

            pan_speed: 45.0,
            rotate_speed: 0.004,
            zoom_speed: 0.15,
            min_distance: 6.0,
            max_distance: 220.0,
            min_pitch: 12.0f32.to_radians(),
            max_pitch: 85.0f32.to_radians(),
        }
    }
}

fn setup_camera(mut commands: Commands) {
    let cam = RtsCamera::default();
    let eye = cam.target
        + Vec3::new(
            cam.yaw.sin() * cam.pitch.cos() * cam.distance,
            cam.pitch.sin() * cam.distance,
            cam.yaw.cos() * cam.pitch.cos() * cam.distance,
        );

    commands.spawn((
        cam,
        Transform::from_translation(eye).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
    ));
}

fn update_rts_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<(&mut RtsCamera, &mut Transform)>,
) {
    let Ok((mut cam, mut transform)) = query.single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    // 1. Orbit Rotation (Right Mouse Drag or Q/E)
    if mouse_button.pressed(MouseButton::Right) {
        cam.desired_yaw -= mouse_motion.delta.x * cam.rotate_speed;
        cam.desired_pitch = (cam.desired_pitch + mouse_motion.delta.y * cam.rotate_speed)
            .clamp(cam.min_pitch, cam.max_pitch);
    }
    if keys.pressed(KeyCode::KeyQ) {
        cam.desired_yaw += 1.6 * dt;
    }
    if keys.pressed(KeyCode::KeyE) {
        cam.desired_yaw -= 1.6 * dt;
    }

    // 2. Zoom (Mouse scroll or +/- keys)
    if mouse_scroll.delta.y.abs() > 0.0 {
        let zoom_factor = 1.0 - mouse_scroll.delta.y * cam.zoom_speed;
        cam.desired_distance = (cam.desired_distance * zoom_factor)
            .clamp(cam.min_distance, cam.max_distance);
    }
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        cam.desired_distance = (cam.desired_distance * (1.0 - 1.5 * dt))
            .clamp(cam.min_distance, cam.max_distance);
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        cam.desired_distance = (cam.desired_distance * (1.0 + 1.5 * dt))
            .clamp(cam.min_distance, cam.max_distance);
    }

    // 3. Pan Movement (WASD / Arrows or Middle Mouse Drag)
    let forward_flat = Vec3::new(-cam.yaw.sin(), 0.0, -cam.yaw.cos()).normalize_or_zero();
    let right_flat = Vec3::new(cam.yaw.cos(), 0.0, -cam.yaw.sin()).normalize_or_zero();

    let mut move_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        move_dir += forward_flat;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        move_dir -= forward_flat;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        move_dir -= right_flat;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        move_dir += right_flat;
    }

    let speed_mult = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        2.5
    } else {
        1.0
    };

    if move_dir.length_squared() > 0.0 {
        let speed = cam.pan_speed * (cam.distance / 50.0).clamp(0.4, 3.0) * speed_mult;
        cam.desired_target += move_dir.normalize() * speed * dt;
    }

    // Middle mouse drag pan
    if mouse_button.pressed(MouseButton::Middle) {
        let pan_scale = cam.distance * 0.0018;
        let delta = mouse_motion.delta.clamp_length_max(80.0);
        cam.desired_target -= (right_flat * delta.x - forward_flat * delta.y) * pan_scale;
    }

    // Clamp camera target within map bounds
    const HALF_MAP: f32 = 80.0;
    cam.desired_target.x = cam.desired_target.x.clamp(-HALF_MAP, HALF_MAP);
    cam.desired_target.z = cam.desired_target.z.clamp(-HALF_MAP, HALF_MAP);
    cam.desired_target.y = cam.desired_target.y.clamp(0.0, 30.0);

    // Reset camera target focus with 'F' key
    if keys.just_pressed(KeyCode::KeyF) {
        cam.desired_target = Vec3::new(0.0, 2.0, 0.0);
        cam.desired_distance = 52.0;
        cam.desired_pitch = 48.0f32.to_radians();
        cam.desired_yaw = -30.0f32.to_radians();
    }

    // 4. Smooth Damping (Lerp)
    let lerp_factor = (15.0 * dt).min(1.0);
    cam.target = cam.target.lerp(cam.desired_target, lerp_factor);
    cam.distance = cam.distance + (cam.desired_distance - cam.distance) * lerp_factor;
    cam.pitch = cam.pitch + (cam.desired_pitch - cam.pitch) * lerp_factor;
    cam.yaw = cam.yaw + (cam.desired_yaw - cam.yaw) * lerp_factor;

    // 5. Update Transform
    let rot = Quat::from_euler(EulerRot::YXZ, cam.yaw, -cam.pitch, 0.0);
    let eye = cam.target + rot * Vec3::new(0.0, 0.0, cam.distance);
    transform.translation = eye;
    transform.look_at(cam.target, Vec3::Y);
}

/// Raycast from viewport mouse coordinate to arbitrary horizontal Y plane
pub fn raycast_ground_plane(
    camera: &Camera,
    cam_transform: &GlobalTransform,
    cursor_pos: Vec2,
    plane_y: f32,
) -> Option<Vec3> {
    let ray = camera.viewport_to_world(cam_transform, cursor_pos).ok()?;
    let dir = ray.direction;
    if dir.y.abs() < 1e-5 {
        return None;
    }
    let t = (plane_y - ray.origin.y) / dir.y;
    if t >= 0.0 {
        Some(ray.origin + *dir * t)
    } else {
        None
    }
}
