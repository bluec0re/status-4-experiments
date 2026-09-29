#![allow(dead_code)]

use bevy::prelude::*;
use crate::house_compat::BuildingElementKind;
use crate::material::MaterialType;

pub struct FireSimPlugin;

impl Plugin for FireSimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentConditions>()
            .init_resource::<SimulationStats>()
            .init_resource::<EmberContainer>()
            .add_systems(
                Update,
                (
                    simulate_heat_transfer,
                    update_combustion_and_fuel,
                    simulate_airborne_embers,
                    update_element_visual_states,
                    update_simulation_statistics,
                )
                    .chain(),
            );
    }
}

/// Time of day lighting preset
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TimeOfDay {
    Day,
    Sunset,
    #[default]
    Night,
}

impl TimeOfDay {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Day => "Bright Day",
            Self::Sunset => "Sunset Twilight",
            Self::Night => "Deep Night",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Day => Self::Sunset,
            Self::Sunset => Self::Night,
            Self::Night => Self::Day,
        }
    }
}

/// Global environment parameters influencing fire behavior
#[derive(Resource)]
pub struct EnvironmentConditions {
    pub ambient_temp: f32,
    pub wind_direction: Vec2, // 2D vector in XZ plane (normalized)
    pub wind_speed: f32,      // m/s (0.0 to 30.0)
    pub sim_speed: f32,       // 0.0=paused, 1.0, 2.0, 5.0
    pub sim_paused: bool,
    pub time_of_day: TimeOfDay,
    pub heatmap_mode: bool,   // Thermal FLIR camera view
}

impl Default for EnvironmentConditions {
    fn default() -> Self {
        Self {
            ambient_temp: 22.0,
            wind_direction: Vec2::new(1.0, 0.25).normalize(), // Breeze blowing East-North-East
            wind_speed: 10.0,
            sim_speed: 1.0,
            sim_paused: false,
            time_of_day: TimeOfDay::Night, // Night by default to wow user with fire light casting!
            heatmap_mode: false,
        }
    }
}

/// Global simulation telemetry statistics
#[derive(Resource, Default)]
pub struct SimulationStats {
    pub active_fires: usize,
    pub burning_elements: usize,
    pub burnt_elements: usize,
    pub extinguished_count: usize,
    pub max_temperature: f32,
    pub water_used_liters: f32,
    pub hovered_info: Option<String>,
}

/// Component attached to any combustible or heat-reactive 3D entity
#[derive(Component, Debug, Clone)]
pub struct Combustible {
    pub material: MaterialType,
    pub element_kind: BuildingElementKind,
    pub bounding_size: Vec3,

    pub temperature: f32,       // in Celsius
    pub fuel: f32,              // current fuel capacity
    pub max_fuel: f32,
    pub moisture: f32,          // 0.0 (bone dry) to 1.0 (water-soaked)
    pub is_burning: bool,
    pub flame_intensity: f32,   // 0.0 to 1.0
    pub burnout: bool,          // reduced to ash/charcoal
    pub shattered: bool,        // glass windows shatter under thermal stress
    pub original_base_color: Color,
    pub was_burning_before: bool,
}

impl Combustible {
    pub fn new(
        material: MaterialType,
        element_kind: BuildingElementKind,
        bounding_size: Vec3,
    ) -> Self {
        let max_fuel = material.default_fuel_capacity();
        let base_col = material.base_color();
        Self {
            material,
            element_kind,
            bounding_size,
            temperature: 22.0,
            fuel: max_fuel,
            max_fuel,
            moisture: if material == MaterialType::DryVegetation {
                0.04
            } else if material == MaterialType::PineTree {
                0.12
            } else {
                0.08
            },
            is_burning: false,
            flame_intensity: 0.0,
            burnout: false,
            shattered: false,
            original_base_color: base_col,
            was_burning_before: false,
        }
    }

    /// Helper to ignite immediately (e.g. for scenario setup or firestarter tool)
    pub fn ignite(&mut self) {
        if self.material.is_combustible() && self.fuel > 0.0 && !self.burnout {
            self.temperature = self.temperature.max(self.material.ignition_temperature() + 150.0);
            self.moisture = 0.0;
            self.is_burning = true;
            self.flame_intensity = 0.8;
        }
    }

    /// Apply water spray to element (cooling + dampening)
    pub fn apply_water(&mut self, water_liters: f32) {
        // Latent cooling: high specific heat of water drops temperature rapidly
        let temp_drop = water_liters * 180.0;
        self.temperature = (self.temperature - temp_drop).max(18.0);
        self.moisture = (self.moisture + water_liters * 0.45).clamp(0.0, 1.0);

        // Extinguish flames if temperature drops below boiling/ignition threshold
        if self.temperature < 140.0 || self.moisture > 0.4 {
            self.is_burning = false;
            self.flame_intensity = 0.0;
        }
    }
}

/// Airborne burning ember particle
#[derive(Clone, Copy, Debug)]
pub struct AirborneEmber {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub temperature: f32,
}

#[derive(Resource, Default)]
pub struct EmberContainer {
    pub embers: Vec<AirborneEmber>,
}

/// Step 1: Heat Transfer Simulation
/// Computes conduction, convection carried by wind, and radiant thermal flux
fn simulate_heat_transfer(
    time: Res<Time>,
    env: Res<EnvironmentConditions>,
    mut query: Query<(&GlobalTransform, &mut Combustible)>,
) {
    if env.sim_paused || env.sim_speed <= 0.0 {
        return;
    }

    let dt = (time.delta_secs() * env.sim_speed).min(0.1);

    // Snapshot burning sources
    let mut sources: Vec<(Vec3, f32, f32, MaterialType)> = Vec::new();
    for (gt, comb) in query.iter() {
        if comb.is_burning && comb.flame_intensity > 0.05 {
            sources.push((
                gt.translation(),
                comb.temperature,
                comb.flame_intensity,
                comb.material,
            ));
        }
    }

    // Apply heat from sources to all elements
    for (gt, mut comb) in query.iter_mut() {
        let pos = gt.translation();

        // 1. Natural cooling towards ambient temperature
        let ambient_diff = comb.temperature - env.ambient_temp;
        let cool_rate = if comb.is_burning { 0.05 } else { 0.35 };
        comb.temperature -= ambient_diff * cool_rate * dt;

        // 2. Heat absorption from all active fire sources
        let mut thermal_intake = 0.0f32;

        for &(src_pos, src_temp, src_flame, src_mat) in &sources {
            let delta = pos - src_pos;
            let dist_sq = delta.length_squared();
            let dist = dist_sq.sqrt();

            if dist < 0.2 {
                continue; // Self or overlapping
            }

            // Max effective interaction distance
            let max_reach = 18.0 + env.wind_speed * 0.4;
            if dist > max_reach {
                continue;
            }

            let delta_dir = delta / dist;
            let delta_xz = Vec2::new(delta_dir.x, delta_dir.z);

            // A. Convection: hot air rises (updraft) AND is pushed downwind
            let updraft = (delta_dir.y).max(0.0) * 1.8;
            let downwind_dot = delta_xz.dot(env.wind_direction.normalize_or_zero());
            let wind_convection = (downwind_dot.max(0.0) * (env.wind_speed * 0.28 + 0.15))
                / (1.0 + dist * 0.25);

            let convection = updraft + wind_convection;

            // B. Thermal Radiation: Stefan-Boltzmann inspired flux falling off with distance
            let radiation = 1.0 / (dist_sq * 0.4 + 1.0);

            // Combined heat transfer coefficient
            let heat_potency = src_mat.heat_output() * src_flame;
            let transfer_amount = heat_potency * (radiation * 1.2 + convection * 2.2);

            let temp_gradient = (src_temp - comb.temperature).max(0.0);
            thermal_intake += (transfer_amount * (temp_gradient / 600.0)).min(350.0);
        }

        // Apply thermal conductivity and insulation
        let conductivity = comb.material.thermal_conductivity();
        let net_heat_gain = thermal_intake * conductivity * dt;

        // Latent heat: if element has moisture, heat boils water instead of raising temp above 100°C
        if comb.moisture > 0.01 && comb.temperature > 80.0 {
            let boil_off = (net_heat_gain * 0.0018).min(comb.moisture);
            comb.moisture -= boil_off;
            // Clamped heating while wet
            comb.temperature += (net_heat_gain * 0.15).min(20.0);
            comb.temperature = comb.temperature.min(105.0);
        } else {
            comb.temperature += net_heat_gain;
        }

        // Window shattering behavior under high thermal stress
        if comb.material == MaterialType::GlassWindow
            && !comb.shattered
            && comb.temperature >= comb.material.ignition_temperature()
        {
            comb.shattered = true;
            comb.temperature += 40.0; // Flash of hot draft entering
        }
    }
}

/// Step 2: Combustion & Fuel Consumption
fn update_combustion_and_fuel(
    time: Res<Time>,
    env: Res<EnvironmentConditions>,
    mut query: Query<(&GlobalTransform, &mut Combustible)>,
    mut embers: ResMut<EmberContainer>,
) {
    if env.sim_paused || env.sim_speed <= 0.0 {
        return;
    }

    let dt = (time.delta_secs() * env.sim_speed).min(0.1);

    for (gt, mut comb) in query.iter_mut() {
        if comb.burnout {
            comb.is_burning = false;
            comb.flame_intensity = 0.0;
            continue;
        }

        // Check for spontaneous ignition
        let ign_temp = comb.material.ignition_temperature();
        if !comb.is_burning
            && comb.material.is_combustible()
            && comb.fuel > 0.0
            && comb.moisture < 0.15
            && comb.temperature >= ign_temp
        {
            comb.is_burning = true;
            comb.flame_intensity = 0.3;
        }

        // Process active combustion
        if comb.is_burning {
            // Internal combustion heat keeps temperature high (650°C to 1050°C)
            let target_burn_temp = 850.0 + comb.material.heat_output() * 0.8;
            comb.temperature = comb.temperature.lerp(target_burn_temp, (3.5 * dt).min(1.0));

            // Ramp up flame intensity
            comb.flame_intensity = (comb.flame_intensity + 1.2 * dt).min(1.0);

            // Fuel burnoff
            let fuel_consumed = comb.material.burn_rate() * comb.flame_intensity * dt;
            comb.fuel -= fuel_consumed;

            // Airborne ember generation
            let ember_rate = comb.material.ember_emission_rate() * comb.flame_intensity;
            if ember_rate > 0.0 && fastrand_chance(ember_rate * dt * 0.6) {
                let pos = gt.translation() + Vec3::new(0.0, comb.bounding_size.y * 0.5, 0.0);
                let wind_3d = Vec3::new(
                    env.wind_direction.x * env.wind_speed,
                    4.5 + fastrand_range(0.0, 3.0), // Thermal plume updraft
                    env.wind_direction.y * env.wind_speed,
                );
                let jitter = Vec3::new(
                    fastrand_range(-1.5, 1.5),
                    fastrand_range(-0.5, 1.0),
                    fastrand_range(-1.5, 1.5),
                );

                embers.embers.push(AirborneEmber {
                    pos,
                    vel: wind_3d * 0.65 + jitter,
                    life: 0.0,
                    max_life: fastrand_range(3.0, 6.5),
                    temperature: 600.0,
                });
            }

            // Burnout check
            if comb.fuel <= 0.0 {
                comb.fuel = 0.0;
                comb.is_burning = false;
                comb.flame_intensity = 0.0;
                comb.burnout = true;
            }
        }
    }
}

/// Step 3: Airborne Embers Flight & Downwind Spot Ignition
fn simulate_airborne_embers(
    time: Res<Time>,
    env: Res<EnvironmentConditions>,
    mut embers: ResMut<EmberContainer>,
    mut query: Query<(&GlobalTransform, &mut Combustible)>,
) {
    if env.sim_paused || env.sim_speed <= 0.0 {
        return;
    }

    let dt = (time.delta_secs() * env.sim_speed).min(0.1);
    let gravity = Vec3::new(0.0, -1.8, 0.0);

    embers.embers.retain_mut(|ember| {
        ember.life += dt;
        if ember.life >= ember.max_life {
            return false;
        }

        // Apply wind influence and drag
        ember.vel += gravity * dt;
        ember.pos += ember.vel * dt;

        // Check if ember hit the ground or an element
        if ember.pos.y <= 0.1 {
            // Hit ground, extinguish
            return false;
        }

        // Spot ignition check against combustible elements
        for (gt, mut comb) in query.iter_mut() {
            if comb.burnout || comb.is_burning || !comb.material.is_combustible() {
                continue;
            }

            let dist = ember.pos.distance(gt.translation());
            let threshold = comb.bounding_size.length() * 0.6 + 0.5;

            if dist < threshold {
                // Ember touches element: injects intense spark heat!
                comb.temperature += 65.0;
                if comb.temperature >= comb.material.ignition_temperature() && comb.moisture < 0.12 {
                    comb.ignite();
                }
                return false; // Ember consumed
            }
        }

        true
    });
}

/// Step 4: Visual Material States (Charring, Thermal Heatmap, Window Shattering)
fn update_element_visual_states(
    env: Res<EnvironmentConditions>,
    mut query: Query<(&Combustible, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (comb, mat_handle) in query.iter_mut() {
        let Some(mut mat) = materials.get_mut(&mat_handle.0) else {
            continue;
        };

        if env.heatmap_mode {
            // Thermal FLIR infrared color scale (Ambient=Blue, Warm=Green/Yellow, Blazing=White/Magenta)
            mat.base_color = temperature_to_flir_color(comb.temperature);
            mat.emissive = LinearRgba::from(temperature_to_flir_color(comb.temperature)) * 0.4;
        } else {
            // Photorealistic visual representation
            if comb.burnout {
                // Charred black ash
                mat.base_color = comb.material.charred_color();
                mat.emissive = LinearRgba::BLACK;
            } else if comb.shattered && comb.material == MaterialType::GlassWindow {
                // Broken jagged pane
                mat.base_color = Color::srgba(0.12, 0.12, 0.12, 0.15);
                mat.emissive = LinearRgba::BLACK;
            } else if comb.is_burning {
                // Blazing fire: incandescent glowing ember cracks
                let burn_fraction = 1.0 - (comb.fuel / comb.max_fuel.max(1.0));
                let charred = comb.material.charred_color();
                let base = comb.original_base_color;
                let blended = base.mix(&charred, burn_fraction.clamp(0.0, 1.0));
                mat.base_color = blended;

                // Glowing thermal emission
                let heat_glow = Color::srgb(1.0, 0.35, 0.05);
                mat.emissive = LinearRgba::from(heat_glow) * (comb.flame_intensity * 1.5);
            } else {
                // Unburnt or hot smoldering
                let heat_ratio = ((comb.temperature - 100.0) / 400.0).clamp(0.0, 1.0);
                if heat_ratio > 0.05 {
                    // Pre-charring / scorching brown
                    let scorched = Color::srgb(0.25, 0.18, 0.12);
                    let blended = comb.original_base_color.mix(&scorched, heat_ratio * 0.85);
                    mat.base_color = blended;
                    mat.emissive = LinearRgba::from(Color::srgb(1.0, 0.2, 0.02)) * (heat_ratio * 0.3);
                } else if comb.moisture > 0.3 {
                    // Dampened dark surface
                    let wet_tint = comb.original_base_color.mix(&Color::srgb(0.08, 0.10, 0.12), 0.35);
                    mat.base_color = wet_tint;
                    mat.emissive = LinearRgba::BLACK;
                } else {
                    mat.base_color = comb.original_base_color;
                    mat.emissive = LinearRgba::BLACK;
                }
            }
        }
    }
}

/// Convert temperature in Celsius to FLIR thermal false-color representation
fn temperature_to_flir_color(temp: f32) -> Color {
    if temp < 50.0 {
        // Deep blue to cyan
        let t = (temp - 20.0).max(0.0) / 30.0;
        Color::srgb(0.05, 0.1 + t * 0.4, 0.4 + t * 0.6)
    } else if temp < 180.0 {
        // Cyan to green
        let t = (temp - 50.0) / 130.0;
        Color::srgb(0.1 + t * 0.2, 0.5 + t * 0.5, 0.8 - t * 0.7)
    } else if temp < 400.0 {
        // Green to bright yellow
        let t = (temp - 180.0) / 220.0;
        Color::srgb(0.3 + t * 0.7, 1.0, 0.1)
    } else if temp < 750.0 {
        // Yellow to blazing orange-red
        let t = (temp - 400.0) / 350.0;
        Color::srgb(1.0, 1.0 - t * 0.7, 0.1)
    } else {
        // White-hot magenta / incandescent
        let t = ((temp - 750.0) / 300.0).clamp(0.0, 1.0);
        Color::srgb(1.0, 0.3 + t * 0.7, 0.3 + t * 0.7)
    }
}

/// Step 5: Telemetry stats aggregation
fn update_simulation_statistics(
    mut stats: ResMut<SimulationStats>,
    query: Query<&Combustible>,
) {
    let mut active = 0;
    let mut burning = 0;
    let mut burnt = 0;
    let mut max_t = 22.0f32;

    for comb in query.iter() {
        if comb.is_burning {
            active += 1;
            burning += 1;
        }
        if comb.burnout {
            burnt += 1;
        }
        if comb.temperature > max_t {
            max_t = comb.temperature;
        }
    }

    stats.active_fires = active;
    stats.burning_elements = burning;
    stats.burnt_elements = burnt;
    stats.max_temperature = max_t;
}

// Pseudo-random helpers without external dependencies
static mut RNG_STATE: u64 = 0x853c49e6748fea9b;

fn fastrand_next() -> u64 {
    unsafe {
        let mut x = RNG_STATE;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        RNG_STATE = x;
        x
    }
}

pub fn fastrand_range(min: f32, max: f32) -> f32 {
    let raw = (fastrand_next() & 0xFFFF_FFFF) as f32 / 4294967295.0;
    min + raw * (max - min)
}

pub fn fastrand_chance(prob: f32) -> bool {
    let raw = (fastrand_next() & 0xFFFF_FFFF) as f32 / 4294967295.0;
    raw < prob
}
