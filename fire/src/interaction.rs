use bevy::prelude::*;
use crate::camera::{raycast_ground_plane, RtsCamera};
use crate::fire_sim::{Combustible, EmberContainer, SimulationStats, fastrand_chance, fastrand_range};
use crate::house_compat::BuildingElementKind;
use crate::rendering::{FireParticle, ParticleType};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InteractionState>()
            .add_systems(Update, handle_mouse_tools);
    }
}

/// Active user interaction tool
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToolMode {
    #[default]
    WaterHose,
    FireStarter,
    Explosion,
    Firebreak,
    Inspect,
}

impl ToolMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::WaterHose => "Water Hose (Cool / Extinguish)",
            Self::FireStarter => "Torch (Ignite Fire)",
            Self::Explosion => "Flashover Blast",
            Self::Firebreak => "Bulldozer (Firebreak)",
            Self::Inspect => "Thermal Inspector",
        }
    }
}

#[derive(Resource)]
pub struct InteractionState {
    pub current_tool: ToolMode,
    pub water_hose_pressure: f32, // 1.0 = Normal, 2.5 = High-pressure Deluge
    pub hose_timer: f32,
    pub hovered_pos: Option<Vec3>,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            current_tool: ToolMode::WaterHose,
            water_hose_pressure: 1.5,
            hose_timer: 0.0,
            hovered_pos: None,
        }
    }
}

fn handle_mouse_tools(
    mut commands: Commands,
    time: Res<Time>,
    windows: Query<&Window>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut interaction: ResMut<InteractionState>,
    mut stats: ResMut<SimulationStats>,
    mut embers: ResMut<EmberContainer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cam_query: Query<(&Camera, &GlobalTransform, &RtsCamera)>,
    mut comb_query: Query<(Entity, &GlobalTransform, &mut Combustible)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        interaction.hovered_pos = None;
        return;
    };

    let Ok((camera, cam_gt, _rts_cam)) = cam_query.single() else {
        return;
    };

    // Raycast against the ground plane Y = 0.0
    let ground_hit = raycast_ground_plane(camera, cam_gt, cursor_pos, 0.0);
    interaction.hovered_pos = ground_hit;

    // Hover inspection detection
    let mut closest_hover = None;
    let mut min_dist = f32::MAX;

    if let Some(hit_pt) = ground_hit {
        for (_ent, gt, comb) in comb_query.iter() {
            let dist = gt.translation().distance(hit_pt);
            let radius = comb.bounding_size.length() * 0.5 + 1.2;
            if dist < radius && dist < min_dist {
                min_dist = dist;
                closest_hover = Some(comb.clone());
            }
        }
    }

    if let Some(comb) = closest_hover {
        stats.hovered_info = Some(format!(
            "Target: {} | Temp: {:.1} deg C | Moisture: {:.0}% | Fuel: {:.0}/{:.0} | State: {}",
            comb.element_kind.name(),
            comb.temperature,
            comb.moisture * 100.0,
            comb.fuel,
            comb.max_fuel,
            if comb.burnout {
                "Burnt Out (Ash)"
            } else if comb.is_burning {
                "BLAZING"
            } else if comb.temperature > comb.material.ignition_temperature() * 0.7 {
                "Smoldering / Hot"
            } else {
                "Cool / Intact"
            }
        ));
    } else {
        stats.hovered_info = None;
    }

    let Some(target_pos) = ground_hit else {
        return;
    };

    let dt = time.delta_secs();
    interaction.hose_timer += dt;

    // Left Mouse Click or Drag Tool Execution
    if mouse_button.pressed(MouseButton::Left) {
        match interaction.current_tool {
            ToolMode::WaterHose => {
                let pressure = interaction.water_hose_pressure;
                let water_rate = 14.0 * pressure * dt;
                stats.water_used_liters += water_rate * 8.0;

                // Visual: Water Stream from camera towards target
                if interaction.hose_timer > 0.035 {
                    interaction.hose_timer = 0.0;
                    let cam_pos = cam_gt.translation();
                    let stream_dir = (target_pos - cam_pos).normalize_or_zero();
                    let spawn_pos = cam_pos + stream_dir * 3.5;

                    let water_mesh = meshes.add(Sphere::new(0.22).mesh().ico(0).expect("sphere mesh"));
                    let water_mat = materials.add(StandardMaterial {
                        base_color: Color::srgba(0.35, 0.75, 1.0, 0.7),
                        perceptual_roughness: 0.1,
                        unlit: true,
                        ..default()
                    });

                    // Water stream droplet
                    commands.spawn((
                        Mesh3d(water_mesh.clone()),
                        MeshMaterial3d(water_mat),
                        Transform::from_translation(spawn_pos),
                        FireParticle {
                            velocity: stream_dir * 55.0 + Vec3::new(fastrand_range(-1.0, 1.0), 3.0, fastrand_range(-1.0, 1.0)),
                            life: 0.0,
                            max_life: 0.85,
                            start_scale: 0.25 * pressure,
                            end_scale: 0.5 * pressure,
                            particle_type: ParticleType::WaterDroplet,
                        },
                    ));
                }

                // Affect combustible elements in splash zone
                let splash_radius = 4.2 * pressure.sqrt();
                for (_ent, gt, mut comb) in comb_query.iter_mut() {
                    let dist = gt.translation().distance(target_pos);
                    if dist <= splash_radius {
                        let falloff = 1.0 - (dist / splash_radius);
                        let water_portion = water_rate * falloff;
                        let old_temp = comb.temperature;

                        comb.apply_water(water_portion);

                        // If hot surface was hit, emit billowing white steam vapor!
                        if old_temp > 95.0 && fastrand_chance(0.35) {
                            let steam_mesh = meshes.add(Sphere::new(0.4).mesh().ico(0).expect("sphere mesh"));
                            let steam_mat = materials.add(StandardMaterial {
                                base_color: Color::srgba(0.9, 0.95, 1.0, 0.5),
                                perceptual_roughness: 0.9,
                                unlit: true,
                                ..default()
                            });
                            commands.spawn((
                                Mesh3d(steam_mesh),
                                MeshMaterial3d(steam_mat),
                                Transform::from_translation(gt.translation() + Vec3::new(0.0, 0.5, 0.0)),
                                FireParticle {
                                    velocity: Vec3::new(
                                        fastrand_range(-0.5, 0.5),
                                        fastrand_range(2.5, 4.5),
                                        fastrand_range(-0.5, 0.5),
                                    ),
                                    life: 0.0,
                                    max_life: 1.4,
                                    start_scale: 0.4,
                                    end_scale: 1.8,
                                    particle_type: ParticleType::SteamVapor,
                                },
                            ));
                        }
                    }
                }
            }

            ToolMode::FireStarter => {
                let ignite_radius = 3.2;
                for (_ent, gt, mut comb) in comb_query.iter_mut() {
                    if gt.translation().distance(target_pos) <= ignite_radius {
                        comb.ignite();
                    }
                }
            }

            ToolMode::Explosion => {
                if mouse_button.just_pressed(MouseButton::Left) {
                    let blast_radius = 9.0;
                    for (_ent, gt, mut comb) in comb_query.iter_mut() {
                        let dist = gt.translation().distance(target_pos);
                        if dist <= blast_radius {
                            let intensity = 1.0 - (dist / blast_radius);
                            comb.temperature += 650.0 * intensity;
                            comb.moisture = 0.0;
                            if comb.material.is_combustible() {
                                comb.ignite();
                            }
                        }
                    }

                    // Launch flying blast embers
                    for _ in 0..24 {
                        let vel = Vec3::new(
                            fastrand_range(-14.0, 14.0),
                            fastrand_range(8.0, 18.0),
                            fastrand_range(-14.0, 14.0),
                        );
                        embers.embers.push(crate::fire_sim::AirborneEmber {
                            pos: target_pos + Vec3::new(0.0, 1.0, 0.0),
                            vel,
                            life: 0.0,
                            max_life: fastrand_range(2.5, 5.0),
                            temperature: 800.0,
                        });
                    }
                }
            }

            ToolMode::Firebreak => {
                // Remove / clear vegetation or structures to halt fire front
                let break_radius = 4.5;
                for (ent, gt, comb) in comb_query.iter_mut() {
                    if gt.translation().distance(target_pos) <= break_radius {
                        if comb.element_kind == BuildingElementKind::VegetationBush
                            || comb.element_kind == BuildingElementKind::VegetationTree
                            || comb.element_kind == BuildingElementKind::Furniture
                        {
                            commands.entity(ent).despawn();
                        }
                    }
                }
            }

            ToolMode::Inspect => {
                // Handled in hover display
            }
        }
    }
}
