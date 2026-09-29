#![allow(dead_code)]

use bevy::prelude::*;

/// Classification of combustible and non-combustible materials
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MaterialType {
    /// Pine / fir trees, needles, dry forest canopy
    PineTree,
    /// Dry underbrush, tall grass, shrubbery, leaves
    DryVegetation,
    /// Structural wood framing, timber beams, wooden floorboards, furniture
    WoodTimber,
    /// Thatch roofing, canvas awnings, curtains, textiles
    ThatchFabric,
    /// Traditional plaster, gypsum, lime stucco exterior rendering
    PlasterStucco,
    /// Brick, stone, granite, concrete foundation (incombustible thermal mass)
    StoneBrick,
    /// Zinc, sheet metal, copper roofing (incombustible, thermal conductor)
    MetalZinc,
    /// Window pane glass (shatters under thermal stress, ventilating fire)
    GlassWindow,
    /// Soil, dirt, damp terrain, gravel, water-soaked ground
    GroundSoil,
}

impl MaterialType {
    /// Friendly display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::PineTree => "Pine Tree Canopy",
            Self::DryVegetation => "Dry Brush & Foliage",
            Self::WoodTimber => "Structural Timber",
            Self::ThatchFabric => "Thatch / Canvas Awnings",
            Self::PlasterStucco => "Plaster & Stucco",
            Self::StoneBrick => "Stone & Brick Masonry",
            Self::MetalZinc => "Sheet Zinc / Metal",
            Self::GlassWindow => "Glazed Window Pane",
            Self::GroundSoil => "Soil / Wet Terrain",
        }
    }

    /// Whether this material can catch fire and sustain combustion
    pub fn is_combustible(&self) -> bool {
        match self {
            Self::PineTree | Self::DryVegetation | Self::WoodTimber | Self::ThatchFabric => true,
            Self::GlassWindow => false, // Does not combust, but shatters under heat
            Self::PlasterStucco | Self::StoneBrick | Self::MetalZinc | Self::GroundSoil => false,
        }
    }

    /// Temperature in Celsius required for spontaneous ignition / flame catch
    pub fn ignition_temperature(&self) -> f32 {
        match self {
            Self::DryVegetation => 210.0, // Very low ignition threshold
            Self::ThatchFabric => 230.0,
            Self::PineTree => 250.0,      // Pine resin fuels ignition once dry
            Self::WoodTimber => 290.0,    // Standard timber pyrolysis ignition
            Self::GlassWindow => 360.0,   // Shatter temperature under thermal gradient
            Self::PlasterStucco => 800.0, // Plaster calcination / degradation
            Self::MetalZinc => 1200.0,
            Self::StoneBrick => 1500.0,
            Self::GroundSoil => 9999.0,
        }
    }

    /// Initial fuel mass capacity (arbitrary normalized thermal units)
    pub fn default_fuel_capacity(&self) -> f32 {
        match self {
            Self::DryVegetation => 35.0,  // Burns out very quickly
            Self::ThatchFabric => 45.0,
            Self::PineTree => 90.0,       // Sustained crown fire
            Self::WoodTimber => 130.0,    // Long, steady burn duration
            Self::GlassWindow => 10.0,    // Shatters quickly
            _ => 0.0,
        }
    }

    /// Rate of fuel consumption per second during blazing combustion
    pub fn burn_rate(&self) -> f32 {
        match self {
            Self::DryVegetation => 9.0, // Fast flash fire
            Self::ThatchFabric => 7.0,
            Self::PineTree => 5.5,
            Self::WoodTimber => 3.2,    // Steady deep burn
            Self::GlassWindow => 8.0,
            _ => 0.0,
        }
    }

    /// Rate of thermal energy transferred to surrounding air/elements per second (kW equivalent)
    pub fn heat_output(&self) -> f32 {
        match self {
            Self::PineTree => 220.0,      // Fierce crown heat
            Self::WoodTimber => 180.0,    // High radiant heat
            Self::DryVegetation => 140.0, // Intense but short-lived
            Self::ThatchFabric => 150.0,
            _ => 0.0,
        }
    }

    /// Thermal conductivity factor (how quickly heat passes through this material)
    pub fn thermal_conductivity(&self) -> f32 {
        match self {
            Self::MetalZinc => 0.85,     // Fast thermal conduction
            Self::StoneBrick => 0.40,    // High thermal mass, absorbs heat
            Self::WoodTimber => 0.28,    // Low conductor, insulates until surface burns
            Self::PlasterStucco => 0.18, // Fire-retardant barrier
            Self::GlassWindow => 0.35,
            Self::PineTree => 0.25,
            Self::DryVegetation => 0.30,
            Self::ThatchFabric => 0.20,
            Self::GroundSoil => 0.22,
        }
    }

    /// Propensity to launch airborne burning sparks and embers into the wind
    pub fn ember_emission_rate(&self) -> f32 {
        match self {
            Self::PineTree => 1.8,       // Conifer pine cones & needles create massive spot fires
            Self::DryVegetation => 1.2,
            Self::ThatchFabric => 1.4,
            Self::WoodTimber => 0.7,
            _ => 0.0,
        }
    }

    /// Smoke opacity / generation density
    pub fn smoke_production(&self) -> f32 {
        match self {
            Self::PineTree => 2.0,      // Thick black resinous smoke
            Self::WoodTimber => 1.4,
            Self::DryVegetation => 1.1, // White/grey brush smoke
            Self::ThatchFabric => 1.3,
            _ => 0.0,
        }
    }

    /// Standard unburnt color
    pub fn base_color(&self) -> Color {
        match self {
            Self::PineTree => Color::srgb(0.14, 0.28, 0.16),
            Self::DryVegetation => Color::srgb(0.48, 0.45, 0.24),
            Self::WoodTimber => Color::srgb(0.55, 0.38, 0.24),
            Self::ThatchFabric => Color::srgb(0.72, 0.62, 0.42),
            Self::PlasterStucco => Color::srgb(0.85, 0.82, 0.76),
            Self::StoneBrick => Color::srgb(0.52, 0.50, 0.48),
            Self::MetalZinc => Color::srgb(0.42, 0.46, 0.50),
            Self::GlassWindow => Color::srgba(0.35, 0.65, 0.85, 0.6),
            Self::GroundSoil => Color::srgb(0.24, 0.20, 0.15),
        }
    }

    /// Fully charred / burnt out ash color
    pub fn charred_color(&self) -> Color {
        match self {
            Self::GlassWindow => Color::srgba(0.1, 0.1, 0.1, 0.15),
            Self::StoneBrick | Self::PlasterStucco => Color::srgb(0.22, 0.20, 0.20),
            _ => Color::srgb(0.08, 0.07, 0.07), // Black charcoal
        }
    }
}
