#![allow(dead_code)]

use bevy::prelude::*;
use crate::fire_sim::{Combustible, EmberContainer, EnvironmentConditions, fastrand_chance, fastrand_range};

pub struct FireRenderingPlugin;

impl Plugin for FireRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FireVisualState>()
            .init_resource::<ParticleAssets>()
            .add_systems(
                Update,
                (
                    manage_dynamic_fire_lights,
                    spawn_flame_and_smoke_particles,
                    update_flame_and_smoke_particles,
                    render_airborne_embers,
                    update_water_and_steam_particles,
                ),
            );
    }
}

/// Pre-allocated shared meshes and materials for particle types to enable GPU instancing/batching
#[derive(Resource)]
pub struct ParticleAssets {
    pub flame_mesh: Handle<Mesh>,
    pub flame_core_mat: Handle<StandardMaterial>,
    pub flame_tongue_mat: Handle<StandardMaterial>,
    pub smoke_mat: Handle<StandardMaterial>,
    pub ember_mesh: Handle<Mesh>,
    pub ember_mat: Handle<StandardMaterial>,
    pub water_mesh: Handle<Mesh>,
    pub water_mat: Handle<StandardMaterial>,
    pub steam_mesh: Handle<Mesh>,
    pub steam_mat: Handle<StandardMaterial>,
}

impl FromWorld for ParticleAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let flame_mesh = meshes.add(Sphere::new(0.5).mesh().ico(1).expect("sphere mesh"));
        let ember_mesh = meshes.add(Sphere::new(0.12).mesh().ico(0).expect("sphere mesh"));
        let water_mesh = meshes.add(Sphere::new(0.22).mesh().ico(0).expect("sphere mesh"));
        let steam_mesh = meshes.add(Sphere::new(0.4).mesh().ico(0).expect("sphere mesh"));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let flame_core_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.92, 0.55),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.85, 0.4)) * 3.5,
            unlit: true,
            ..default()
        });
        let flame_tongue_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.38, 0.05),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.3, 0.02)) * 2.8,
            unlit: true,
            ..default()
        });
        let smoke_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.12, 0.12, 0.12, 0.75),
            perceptual_roughness: 0.95,
            ..default()
        });
        let ember_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.75, 0.15),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.65, 0.1)) * 4.0,
            unlit: true,
            ..default()
        });
        let water_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.35, 0.75, 1.0, 0.7),
            perceptual_roughness: 0.1,
            unlit: true,
            ..default()
        });
        let steam_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.9, 0.95, 1.0, 0.5),
            perceptual_roughness: 0.9,
            unlit: true,
            ..default()
        });

        Self {
            flame_mesh,
            flame_core_mat,
            flame_tongue_mat,
            smoke_mat,
            ember_mesh,
            ember_mat,
            water_mesh,
            water_mat,
            steam_mesh,
            steam_mat,
        }
    }
}

/// Global tracking for active flame particle meshes and dynamic lights
#[derive(Resource, Default)]
pub struct FireVisualState {
    pub flame_timer: f32,
    pub active_light_entities: Vec<Entity>,
}

/// Dynamic flickering point light attached to blazing fire zones
#[derive(Component)]
pub struct FirePointLight {
    pub base_intensity: f32,
    pub flicker_seed: f32,
    pub target_pos: Vec3,
}

/// Flame tongue or smoke particle component
#[derive(Component)]
pub struct FireParticle {
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub particle_type: ParticleType,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParticleType {
    FlameCore,
    FlameTongue,
    SmokePlume,
    WaterDroplet,
    SteamVapor,
    EmberSpark,
}

/// Visual particle mesh marker
#[derive(Component)]
pub struct EmberSparkMesh;

/// 1. Manage Dynamic Point Lights for Fire Night Casting
/// Allocates flickering warm lights to clusters of active fires, respecting the fire_shadows setting
fn manage_dynamic_fire_lights(
    mut commands: Commands,
    time: Res<Time>,
    env: Res<EnvironmentConditions>,
    mut visual_state: ResMut<FireVisualState>,
    query_comb: Query<(&GlobalTransform, &Combustible)>,
    mut light_query: Query<(Entity, &mut Transform, &mut PointLight, &mut FirePointLight)>,
) {
    let t = time.elapsed_secs();

    // Collect hot fire cluster centers
    let mut fire_clusters: Vec<(Vec3, f32)> = Vec::new();
    for (gt, comb) in query_comb.iter() {
        if comb.is_burning && comb.flame_intensity > 0.15 {
            let pos = gt.translation() + Vec3::new(0.0, comb.bounding_size.y * 0.4, 0.0);
            let mut merged = false;
            for cluster in fire_clusters.iter_mut() {
                if cluster.0.distance(pos) < 6.0 {
                    cluster.0 = cluster.0.lerp(pos, 0.4);
                    cluster.1 += comb.flame_intensity;
                    merged = true;
                    break;
                }
            }
            if !merged && fire_clusters.len() < 10 {
                fire_clusters.push((pos, comb.flame_intensity));
            }
        }
    }

    // Existing active lights: update transforms, intensities, and shadow maps toggle
    let mut existing_lights: Vec<(Entity, Vec3)> = Vec::new();
    for (ent, mut trans, mut light, mut fire_light) in light_query.iter_mut() {
        if light.shadow_maps_enabled != env.fire_shadows {
            light.shadow_maps_enabled = env.fire_shadows;
        }

        // Find nearest fire cluster
        if let Some(closest_idx) = fire_clusters
            .iter()
            .position(|c| c.0.distance(trans.translation) < 14.0)
        {
            let (target_pos, intensity) = fire_clusters.remove(closest_idx);
            fire_light.target_pos = target_pos;

            // Organic flickering equation combining two sine frequencies
            let flicker = ((t * 14.0 + fire_light.flicker_seed).sin() * 0.22
                + (t * 27.0 + fire_light.flicker_seed * 2.1).sin() * 0.14)
                * intensity.min(2.5);

            let night_boost = if env.time_of_day == crate::fire_sim::TimeOfDay::Night {
                1.35
            } else {
                0.9
            };

            let base_lux = (12_000.0 * intensity * night_boost).clamp(3_000.0, 35_000.0);
            light.intensity = (base_lux * (1.0 + flicker)).max(100.0);
            light.range = 24.0 + intensity * 6.0;

            // Slight position tremor mimicking leaping flames
            let jitter = Vec3::new(
                (t * 7.0 + fire_light.flicker_seed).sin() * 0.25,
                (t * 11.0).sin() * 0.35,
                (t * 8.0 + fire_light.flicker_seed).cos() * 0.25,
            );
            trans.translation = trans.translation.lerp(target_pos + jitter, 0.2);

            existing_lights.push((ent, trans.translation));
        } else {
            // No fire nearby, despawn excess light
            commands.entity(ent).despawn();
        }
    }

    // Spawn new lights for remaining unassigned fire clusters
    for (pos, intensity) in fire_clusters {
        let flicker_seed = fastrand_range(0.0, 100.0);
        let night_boost = if env.time_of_day == crate::fire_sim::TimeOfDay::Night {
            1.35
        } else {
            0.9
        };
        let lux = (14_000.0 * intensity * night_boost).clamp(4_000.0, 35_000.0);

        let new_light = commands
            .spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.52, 0.14), // warm fiery glow
                    intensity: lux,
                    range: 26.0,
                    shadow_maps_enabled: env.fire_shadows,
                    ..default()
                },
                Transform::from_translation(pos + Vec3::new(0.0, 1.2, 0.0)),
                FirePointLight {
                    base_intensity: lux,
                    flicker_seed,
                    target_pos: pos,
                },
            ))
            .id();
        visual_state.active_light_entities.push(new_light);
    }
}

/// 2. Spawn Billboard / Volumetric Flame & Smoke Particles from Active Fires
/// Uses pre-allocated shared materials and meshes to enable single-draw-call GPU instancing
fn spawn_flame_and_smoke_particles(
    mut commands: Commands,
    time: Res<Time>,
    env: Res<EnvironmentConditions>,
    mut visual_state: ResMut<FireVisualState>,
    particle_assets: Res<ParticleAssets>,
    particle_query: Query<&FireParticle>,
    query: Query<(&GlobalTransform, &Combustible)>,
) {
    if env.sim_paused || env.sim_speed <= 0.0 {
        return;
    }

    let dt = (time.delta_secs() * env.sim_speed).min(0.1);
    visual_state.flame_timer += dt;

    if visual_state.flame_timer < 0.045 {
        return; // Throttle particle spawn rate for 60fps smoothness
    }
    visual_state.flame_timer = 0.0;

    // Hard particle cap to maintain smooth 60fps even with hundreds of burning objects
    const MAX_PARTICLES: usize = 1200;
    let particle_count = particle_query.iter().count();
    if particle_count >= MAX_PARTICLES {
        return;
    }

    let wind_vec3 = Vec3::new(
        env.wind_direction.x * env.wind_speed,
        0.0,
        env.wind_direction.y * env.wind_speed,
    );

    // Adaptive spawn chance based on count of active fires
    let burning_count = query.iter().filter(|(_, c)| c.is_burning && c.flame_intensity >= 0.08).count();
    let spawn_prob = if burning_count > 30 {
        (30.0 / (burning_count as f32)).clamp(0.12, 1.0)
    } else {
        1.0
    };

    for (gt, comb) in query.iter() {
        if !comb.is_burning || comb.flame_intensity < 0.08 {
            continue;
        }

        if spawn_prob < 1.0 && !fastrand_chance(spawn_prob) {
            continue;
        }

        let origin = gt.translation();
        let half_size = comb.bounding_size * 0.4;
        let random_offset = Vec3::new(
            fastrand_range(-half_size.x, half_size.x),
            fastrand_range(0.0, half_size.y * 0.5),
            fastrand_range(-half_size.z, half_size.z),
        );
        let spawn_pos = origin + random_offset;

        // A. Blazing Flame Core (White-Yellow hot) - instanced
        commands.spawn((
            Mesh3d(particle_assets.flame_mesh.clone()),
            MeshMaterial3d(particle_assets.flame_core_mat.clone()),
            Transform::from_translation(spawn_pos).with_scale(Vec3::splat(0.6 * comb.flame_intensity)),
            FireParticle {
                velocity: Vec3::new(0.0, fastrand_range(2.8, 5.0), 0.0) + wind_vec3 * 0.15,
                life: 0.0,
                max_life: fastrand_range(0.35, 0.75),
                start_scale: 0.55 * comb.flame_intensity,
                end_scale: 0.2,
                particle_type: ParticleType::FlameCore,
            },
        ));

        // B. Leaping Fiery Orange Flame Tongue - instanced
        commands.spawn((
            Mesh3d(particle_assets.flame_mesh.clone()),
            MeshMaterial3d(particle_assets.flame_tongue_mat.clone()),
            Transform::from_translation(spawn_pos + Vec3::new(0.0, 0.4, 0.0))
                .with_scale(Vec3::splat(0.8 * comb.flame_intensity)),
            FireParticle {
                velocity: Vec3::new(0.0, fastrand_range(3.5, 6.5), 0.0) + wind_vec3 * 0.3,
                life: 0.0,
                max_life: fastrand_range(0.5, 1.1),
                start_scale: 0.8 * comb.flame_intensity,
                end_scale: 0.25,
                particle_type: ParticleType::FlameTongue,
            },
        ));

        // C. Rising Dark Smoke Plume - instanced
        if fastrand_chance(0.4) {
            commands.spawn((
                Mesh3d(particle_assets.flame_mesh.clone()),
                MeshMaterial3d(particle_assets.smoke_mat.clone()),
                Transform::from_translation(spawn_pos + Vec3::new(0.0, 1.2, 0.0))
                    .with_scale(Vec3::splat(0.6)),
                FireParticle {
                    velocity: Vec3::new(
                        fastrand_range(-0.5, 0.5),
                        fastrand_range(3.0, 5.5),
                        fastrand_range(-0.5, 0.5),
                    ) + wind_vec3 * 0.55,
                    life: 0.0,
                    max_life: fastrand_range(1.5, 3.2),
                    start_scale: 0.7,
                    end_scale: fastrand_range(2.2, 3.8),
                    particle_type: ParticleType::SmokePlume,
                },
            ));
        }
    }
}

/// 3. Update Particle Positions, Wind Drag, Lifespans, and Scaling
fn update_flame_and_smoke_particles(
    mut commands: Commands,
    time: Res<Time>,
    env: Res<EnvironmentConditions>,
    mut query: Query<(Entity, &mut Transform, &mut FireParticle)>,
) {
    let dt = time.delta_secs();
    let wind_force = Vec3::new(
        env.wind_direction.x * env.wind_speed,
        0.0,
        env.wind_direction.y * env.wind_speed,
    );

    for (ent, mut trans, mut p) in query.iter_mut() {
        p.life += dt;
        if p.life >= p.max_life {
            commands.entity(ent).despawn();
            continue;
        }

        let progress = p.life / p.max_life;

        // Wind drag pushes particles along the wind vector
        p.velocity += wind_force * (0.8 * dt);
        trans.translation += p.velocity * dt;

        // Dynamic scaling
        match p.particle_type {
            ParticleType::FlameCore | ParticleType::FlameTongue => {
                // Shrinks as flame burns out
                let scale = p.start_scale.lerp(p.end_scale, progress);
                trans.scale = Vec3::splat(scale.max(0.05));
            }
            ParticleType::SmokePlume => {
                // Expands and diffuses as it climbs
                let scale = p.start_scale.lerp(p.end_scale, progress);
                trans.scale = Vec3::splat(scale);
            }
            _ => {}
        }
    }
}

/// 4. Render Airborne Embers & Sparks Carried by Wind
/// Reuses existing spark entities in place without despawning and recreating every frame
fn render_airborne_embers(
    mut commands: Commands,
    embers: Res<EmberContainer>,
    particle_assets: Res<ParticleAssets>,
    mut existing_sparks: Query<(Entity, &mut Transform), With<EmberSparkMesh>>,
) {
    let mut spark_iter = existing_sparks.iter_mut();

    for ember in &embers.embers {
        if let Some((_ent, mut trans)) = spark_iter.next() {
            trans.translation = ember.pos;
        } else {
            commands.spawn((
                EmberSparkMesh,
                Mesh3d(particle_assets.ember_mesh.clone()),
                MeshMaterial3d(particle_assets.ember_mat.clone()),
                Transform::from_translation(ember.pos),
            ));
        }
    }

    // Despawn excess spark entities if ember count decreased
    for (ent, _trans) in spark_iter {
        commands.entity(ent).despawn();
    }
}

/// 5. Update Water Stream & Steam Condensation Particles
fn update_water_and_steam_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut FireParticle)>,
) {
    let dt = time.delta_secs();

    for (ent, mut trans, mut p) in query.iter_mut() {
        if p.particle_type != ParticleType::WaterDroplet && p.particle_type != ParticleType::SteamVapor {
            continue;
        }

        p.life += dt;
        if p.life >= p.max_life {
            commands.entity(ent).despawn();
            continue;
        }

        let progress = p.life / p.max_life;

        if p.particle_type == ParticleType::WaterDroplet {
            // High speed stream with downward gravity arc
            p.velocity += Vec3::new(0.0, -14.0, 0.0) * dt;
            trans.translation += p.velocity * dt;
        } else if p.particle_type == ParticleType::SteamVapor {
            // Billowing steam rising and diffusing
            p.velocity += Vec3::new(0.0, 3.2, 0.0) * dt;
            trans.translation += p.velocity * dt;
            let scale = p.start_scale.lerp(p.end_scale, progress);
            trans.scale = Vec3::splat(scale);
        }
    }
}
