#![allow(dead_code)]

use bevy::prelude::*;
use crate::polygon::BuildingPolygon;

/// Architectural design style for European city apartment blocks
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum EuropeanStyle {
    #[default]
    Haussmannian,         // Parisian limestone, grand rusticated base, mansard zinc roof, ornate balconies
    BerlinAltbau,         // Classic Berlin Gründerzeit / Altbau, pastel stucco, steep pitched slate roof
    VienneseNeoclassical, // Imperial yellow / warm ochre, symmetrical pilasters, terracotta roof
    AmsterdamCanal,       // Dutch canal block, brick facades, stepped gables, large sash windows
}

impl EuropeanStyle {
    pub const ALL: [EuropeanStyle; 4] = [
        EuropeanStyle::Haussmannian,
        EuropeanStyle::BerlinAltbau,
        EuropeanStyle::VienneseNeoclassical,
        EuropeanStyle::AmsterdamCanal,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Haussmannian => "Parisian Haussmannian",
            Self::BerlinAltbau => "Berlin Gründerzeit Altbau",
            Self::VienneseNeoclassical => "Viennese Neoclassical",
            Self::AmsterdamCanal => "Amsterdam Canal House Block",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Haussmannian => Self::BerlinAltbau,
            Self::BerlinAltbau => Self::VienneseNeoclassical,
            Self::VienneseNeoclassical => Self::AmsterdamCanal,
            Self::AmsterdamCanal => Self::Haussmannian,
        }
    }

    pub fn default_stories(&self) -> u32 {
        match self {
            Self::Haussmannian => 5,
            Self::BerlinAltbau => 4,
            Self::VienneseNeoclassical => 4,
            Self::AmsterdamCanal => 3,
        }
    }
}

/// Roof silhouette type for European city buildings
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum EuropeanRoofType {
    #[default]
    MansardZinc,
    PitchedSlate,
    TerracottaGabled,
    FlatRooftopTerrace,
}

impl EuropeanRoofType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::MansardZinc => "Parisian Mansard (Zinc/Slate)",
            Self::PitchedSlate => "Pitched Dormer Slate",
            Self::TerracottaGabled => "Terracotta Tile Gabled",
            Self::FlatRooftopTerrace => "Flat Balustrade Terrace",
        }
    }
}

/// Functional room classification for interior generation and RTS unit behaviors
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RoomType {
    EntranceLobby,
    Stairwell,
    Corridor,
    LivingRoom,
    Bedroom,
    KitchenDining,
    Balcony,
}

impl RoomType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::EntranceLobby => "Entrance Lobby",
            Self::Stairwell => "Stairwell Core",
            Self::Corridor => "Hallway Corridor",
            Self::LivingRoom => "Apartment Living Room",
            Self::Bedroom => "Apartment Bedroom",
            Self::KitchenDining => "Kitchen / Dining",
            Self::Balcony => "Exterior Balcony",
        }
    }
}

// ---------------------------------------------------------------------------
// ECS COMPONENTS FOR BUILDING HIERARCHY & RTS GAMEPLAY
// ---------------------------------------------------------------------------

/// Root entity component for an instantiated European building
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct Building;

/// Metadata stored on the building root entity
#[derive(Component, Debug, Clone)]
pub struct BuildingMetadata {
    pub seed: u64,
    pub style: EuropeanStyle,
    pub stories: u32,
    pub story_height: f32,
    pub ground_story_height: f32,
    pub footprint: BuildingPolygon,
    pub total_rooms: usize,
    pub total_living_area_sqm: f32,
}

/// Component tagging all entities belonging to a specific floor level
/// (Used for floor cutaway rendering in RTS mode)
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloorLevelMarker(pub u32);

/// Component tagging roof entities (hidden when cutaway is active)
#[derive(Component, Clone, Copy, Debug)]
pub struct RoofMarker;

/// Interior floor level entity component
#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct FloorLevel {
    pub story_index: u32,
    pub elevation: f32,
    pub ceiling_elevation: f32,
    pub rooms: Vec<Entity>,
}

/// Individual room component
#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct Room {
    pub room_id: u32,
    pub floor_index: u32,
    pub room_type: RoomType,
    pub center: Vec3,
    pub bounds_min: Vec2,
    pub bounds_max: Vec2,
}

/// Doorway transition connecting rooms or street to building
#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct Doorway {
    pub from_room: Option<Entity>,
    pub to_room: Option<Entity>,
    pub world_pos: Vec3,
    pub is_exterior: bool,
}

/// Staircase connecting floor N to N+1
#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct Staircase {
    pub from_floor: u32,
    pub to_floor: u32,
    pub bottom_pos: Vec3,
    pub top_pos: Vec3,
}

/// RTS unit tracking component for occupancy inside buildings
#[derive(Component, Debug, Clone)]
pub struct BuildingOccupant {
    pub current_building: Option<Entity>,
    pub current_floor: Option<u32>,
    pub current_room: Option<Entity>,
}

impl Default for BuildingOccupant {
    fn default() -> Self {
        Self {
            current_building: None,
            current_floor: None,
            current_room: None,
        }
    }
}
