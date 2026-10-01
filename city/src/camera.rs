use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;

pub struct CityCameraPlugin;

impl Plugin for CityCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_city_camera, handle_camera_shortcuts));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraViewPreset {
    Satellite,
    Overview,
    Isometric,
    Street,
}

#[derive(Component)]
#[require(Camera3d, Transform, Visibility)]
pub struct CityCamera {
    pub target: Vec3,
    pub distance: f32,
    pub pitch: f32,
    pub yaw: f32,

    pub desired_target: Vec3,
    pub desired_distance: f32,
    pub desired_pitch: f32,
    pub desired_yaw: f32,

    pub min_distance: f32,
    pub max_distance: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,

    pub is_cinematic: bool,
}

impl Default for CityCamera {
    fn default() -> Self {
        let initial_target = Vec3::ZERO;
        let initial_distance = 2400.0;
        let initial_pitch = 45.0f32.to_radians();
        let initial_yaw = -30.0f32.to_radians();

        Self {
            target: initial_target,
            distance: initial_distance,
            pitch: initial_pitch,
            yaw: initial_yaw,

            desired_target: initial_target,
            desired_distance: initial_distance,
            desired_pitch: initial_pitch,
            desired_yaw: initial_yaw,

            min_distance: 25.0,
            max_distance: 9000.0,
            min_pitch: 5.0f32.to_radians(),
            max_pitch: 88.0f32.to_radians(),

            is_cinematic: false,
        }
    }
}

impl CityCamera {
    pub fn apply_preset(&mut self, preset: CameraViewPreset) {
        match preset {
            CameraViewPreset::Satellite => {
                self.desired_distance = 5800.0;
                self.desired_pitch = 87.0f32.to_radians();
                self.desired_yaw = 0.0;
            }
            CameraViewPreset::Overview => {
                self.desired_distance = 2600.0;
                self.desired_pitch = 45.0f32.to_radians();
                self.desired_yaw = -35.0f32.to_radians();
            }
            CameraViewPreset::Isometric => {
                self.desired_distance = 1200.0;
                self.desired_pitch = 35.264f32.to_radians();
                self.desired_yaw = 45.0f32.to_radians();
            }
            CameraViewPreset::Street => {
                self.desired_distance = 90.0;
                self.desired_pitch = 14.0f32.to_radians();
                self.desired_yaw = -20.0f32.to_radians();
            }
        }
    }
}

fn update_city_camera(
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<(&mut CityCamera, &mut Transform)>,
) {
    let Ok((mut cam, mut transform)) = query.single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // 1. Zoom via mouse scroll wheel
    let scroll = mouse_scroll.delta.y;
    if scroll.abs() > 0.001 {
        let zoom_factor = 1.0 - scroll * 0.12;
        cam.desired_distance = (cam.desired_distance * zoom_factor).clamp(cam.min_distance, cam.max_distance);
    }

    // 2. Right-click drag: Orbit rotation (pitch & yaw)
    if mouse_buttons.pressed(MouseButton::Right) {
        let rot_speed = 0.004;
        cam.desired_yaw -= mouse_motion.delta.x * rot_speed;
        cam.desired_pitch += mouse_motion.delta.y * rot_speed;
        cam.desired_pitch = cam.desired_pitch.clamp(cam.min_pitch, cam.max_pitch);
    }

    // 3. Middle-click drag: Panning
    if mouse_buttons.pressed(MouseButton::Middle) {
        let forward = Vec3::new(-cam.yaw.sin(), 0.0, -cam.yaw.cos()).normalize_or_zero();
        let right = Vec3::new(cam.yaw.cos(), 0.0, -cam.yaw.sin()).normalize_or_zero();
        let drag_speed = cam.distance * 0.0015;
        let delta = -right * (mouse_motion.delta.x * drag_speed) + forward * (mouse_motion.delta.y * drag_speed);
        cam.desired_target += delta;
    }

    // 4. WASD keyboard navigation
    let mut move_dir = Vec3::ZERO;
    let forward = Vec3::new(-cam.yaw.sin(), 0.0, -cam.yaw.cos()).normalize_or_zero();
    let right = Vec3::new(cam.yaw.cos(), 0.0, -cam.yaw.sin()).normalize_or_zero();

    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        move_dir += forward;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        move_dir -= forward;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        move_dir += right;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        move_dir -= right;
    }

    if move_dir.length_squared() > 1e-4 {
        move_dir = move_dir.normalize();
        let pan_speed = (cam.distance * 0.45).clamp(40.0, 1600.0);
        let boost = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            2.5
        } else {
            1.0
        };
        cam.desired_target += move_dir * pan_speed * boost * dt;
    }

    // Restrict target within 10x10km area (±5000m)
    cam.desired_target.x = cam.desired_target.x.clamp(-5500.0, 5500.0);
    cam.desired_target.z = cam.desired_target.z.clamp(-5500.0, 5500.0);
    cam.desired_target.y = 0.0;

    // 5. Cinematic auto-orbit
    if cam.is_cinematic {
        cam.desired_yaw += 0.06 * dt;
    }

    // 6. Smooth exponential damping towards desired values
    let ease = (12.0 * dt).min(1.0);
    cam.distance += (cam.desired_distance - cam.distance) * ease;
    cam.pitch += (cam.desired_pitch - cam.pitch) * ease;
    cam.yaw += (cam.desired_yaw - cam.yaw) * ease;
    cam.target = cam.target.lerp(cam.desired_target, ease);

    // 7. Update camera transform
    let rot = Quat::from_rotation_y(cam.yaw) * Quat::from_rotation_x(-cam.pitch);
    let offset = rot * Vec3::new(0.0, 0.0, cam.distance);
    transform.translation = cam.target + offset;
    transform.look_at(cam.target, Vec3::Y);
}

fn handle_camera_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut CityCamera>,
) {
    let Ok(mut cam) = query.single_mut() else {
        return;
    };

    if keys.just_pressed(KeyCode::Digit1) {
        cam.apply_preset(CameraViewPreset::Satellite);
    } else if keys.just_pressed(KeyCode::Digit2) {
        cam.apply_preset(CameraViewPreset::Overview);
    } else if keys.just_pressed(KeyCode::Digit3) {
        cam.apply_preset(CameraViewPreset::Isometric);
    } else if keys.just_pressed(KeyCode::Digit4) {
        cam.apply_preset(CameraViewPreset::Street);
    }

    if keys.just_pressed(KeyCode::KeyR) {
        cam.desired_target = Vec3::ZERO;
        cam.apply_preset(CameraViewPreset::Overview);
    }

    if keys.just_pressed(KeyCode::Space) {
        cam.is_cinematic = !cam.is_cinematic;
    }
}
