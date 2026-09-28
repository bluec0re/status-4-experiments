#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use crate::catalog::ModelCatalog;
use crate::floorplan::generate_internal_floors;
use crate::house::{
    Building, BuildingMetadata, BuildingOccupant, EuropeanStyle, FloorLevelMarker,
    RoofMarker,
};
use crate::polygon::{BuildingPolygon, PolygonPreset};
use crate::rts::{plan_path_to_room, plan_path_to_street, RtsSelectionState, RtsUnit};
use crate::textures::BuildingMaterials;

/// Cutaway visualization mode for interior inspection and RTS gameplay
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum CutawayMode {
    #[default]
    FullExterior,
    RoofOff,
    Floor0,
    Floor1,
    Floor2,
    Floor3,
    FollowSelectedUnit,
}

impl CutawayMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FullExterior => "Full Exterior",
            Self::RoofOff => "Roof Cutaway",
            Self::Floor0 => "Floor 0 (Ground)",
            Self::Floor1 => "Floor 1",
            Self::Floor2 => "Floor 2",
            Self::Floor3 => "Floor 3",
            Self::FollowSelectedUnit => "Auto-Follow Unit",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::FullExterior => Self::RoofOff,
            Self::RoofOff => Self::Floor0,
            Self::Floor0 => Self::Floor1,
            Self::Floor1 => Self::Floor2,
            Self::Floor2 => Self::Floor3,
            Self::Floor3 => Self::FollowSelectedUnit,
            Self::FollowSelectedUnit => Self::FullExterior,
        }
    }
}

/// Unit movement command dispatched from UI or shortcuts
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnitOrder {
    EnterLobby,
    GoToFloor(u32),
    ExitToStreet,
}

/// Generation actions decoupled from input sources
#[derive(Message, Clone, Debug)]
pub enum HouseAction {
    GenerateRandom,
    SetSeed(u64),
    CycleStyle,
    SetStyle(EuropeanStyle),
    CyclePreset,
    SetPreset(PolygonPreset),
    AdjustStories(i32),
    SetStories(u32),
    ToggleInterior,
    CycleCutaway,
    SetCutaway(CutawayMode),
    DispatchUnitOrder(UnitOrder),
    Rebuild,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum HouseSet {
    Input,
    ApplyActions,
    Generate,
    VisibilityUpdate,
    PostUpdate,
}

pub struct HouseGeneratorPlugin;

impl Plugin for HouseGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HouseAction>()
            .init_resource::<BuildingMaterials>()
            .init_resource::<ModelCatalog>()
            .init_resource::<GeneratorSettings>()
            .init_resource::<GeneratorStats>()
            .init_resource::<ActiveBuildingNavigation>()
            .configure_sets(
                Update,
                (
                    HouseSet::Input,
                    HouseSet::ApplyActions.after(HouseSet::Input),
                    HouseSet::Generate.after(HouseSet::ApplyActions),
                    HouseSet::VisibilityUpdate.after(HouseSet::Generate),
                    HouseSet::PostUpdate.after(HouseSet::VisibilityUpdate),
                ),
            )
            .add_systems(
                Update,
                (
                    handle_generator_hotkeys.in_set(HouseSet::Input),
                    apply_house_actions.in_set(HouseSet::ApplyActions),
                    generate_building_system.in_set(HouseSet::Generate),
                    update_cutaway_visibility.in_set(HouseSet::VisibilityUpdate),
                    draw_building_gizmos.in_set(HouseSet::PostUpdate),
                ),
            );
    }
}

/// Current active generator configuration
#[derive(Resource, Debug, Clone)]
pub struct GeneratorSettings {
    pub seed: u64,
    pub style: EuropeanStyle,
    pub preset: PolygonPreset,
    pub polygon_scale: f32,
    pub stories: u32,
    pub ground_story_h: f32,
    pub upper_story_h: f32,
    pub has_interior: bool,
    pub cutaway: CutawayMode,
    pub needs_rebuild: bool,
}

impl Default for GeneratorSettings {
    fn default() -> Self {
        Self {
            seed: 1848,
            style: EuropeanStyle::Haussmannian,
            preset: PolygonPreset::HaussmannBlock,
            polygon_scale: 1.0,
            stories: 4,
            ground_story_h: 3.6,
            upper_story_h: 3.2,
            has_interior: true,
            cutaway: CutawayMode::FullExterior,
            needs_rebuild: true,
        }
    }
}

/// Summary stats regarding generated building
#[derive(Resource, Debug, Clone)]
pub struct GeneratorStats {
    pub seed: u64,
    pub style: EuropeanStyle,
    pub preset: PolygonPreset,
    pub stories: u32,
    pub total_rooms: usize,
    pub total_living_area_sqm: f32,
    pub building_height: f32,
    pub windows_count: usize,
    pub doorways_count: usize,
    pub active_occupants: usize,
}

impl Default for GeneratorStats {
    fn default() -> Self {
        Self {
            seed: 1848,
            style: EuropeanStyle::Haussmannian,
            preset: PolygonPreset::HaussmannBlock,
            stories: 4,
            total_rooms: 0,
            total_living_area_sqm: 0.0,
            building_height: 0.0,
            windows_count: 0,
            doorways_count: 0,
            active_occupants: 0,
        }
    }
}

/// Cached building navigation positions for RTS unit commands
#[derive(Resource, Default)]
pub struct ActiveBuildingNavigation {
    pub street_entrance: Vec3,
    pub lobby_pos: Vec3,
    pub staircases: Vec<(u32, Vec3, Vec3)>,
    pub floor_room_centers: Vec<(u32, Vec3)>,
}

pub fn handle_generator_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut action_writer: MessageWriter<HouseAction>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        action_writer.write(HouseAction::GenerateRandom);
    }
    if keys.just_pressed(KeyCode::KeyP) {
        action_writer.write(HouseAction::CyclePreset);
    }
    if keys.just_pressed(KeyCode::KeyT) {
        action_writer.write(HouseAction::CycleStyle);
    }
    if keys.just_pressed(KeyCode::KeyI) {
        action_writer.write(HouseAction::ToggleInterior);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        action_writer.write(HouseAction::CycleCutaway);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        action_writer.write(HouseAction::AdjustStories(1));
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        action_writer.write(HouseAction::AdjustStories(-1));
    }
}

pub fn apply_house_actions(
    mut action_reader: MessageReader<HouseAction>,
    mut settings: ResMut<GeneratorSettings>,
    selection: Res<RtsSelectionState>,
    nav: Res<ActiveBuildingNavigation>,
    mut units: Query<(&Transform, &mut RtsUnit)>,
) {
    for action in action_reader.read() {
        match action {
            HouseAction::GenerateRandom => {
                settings.seed = settings.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                settings.needs_rebuild = true;
            }
            HouseAction::SetSeed(s) => {
                settings.seed = *s;
                settings.needs_rebuild = true;
            }
            HouseAction::CycleStyle => {
                settings.style = settings.style.next();
                settings.needs_rebuild = true;
            }
            HouseAction::SetStyle(st) => {
                settings.style = *st;
                settings.needs_rebuild = true;
            }
            HouseAction::CyclePreset => {
                settings.preset = settings.preset.next();
                settings.needs_rebuild = true;
            }
            HouseAction::SetPreset(p) => {
                settings.preset = *p;
                settings.needs_rebuild = true;
            }
            HouseAction::AdjustStories(delta) => {
                let s = (settings.stories as i32 + delta).clamp(2, 6) as u32;
                if s != settings.stories {
                    settings.stories = s;
                    settings.needs_rebuild = true;
                }
            }
            HouseAction::SetStories(s) => {
                settings.stories = (*s).clamp(2, 6);
                settings.needs_rebuild = true;
            }
            HouseAction::ToggleInterior => {
                settings.has_interior = !settings.has_interior;
                settings.needs_rebuild = true;
            }
            HouseAction::CycleCutaway => {
                settings.cutaway = settings.cutaway.next();
            }
            HouseAction::SetCutaway(c) => {
                settings.cutaway = *c;
            }
            HouseAction::DispatchUnitOrder(order) => {
                if let Some(selected_ent) = selection.selected_unit
                    && let Ok((tf, mut unit)) = units.get_mut(selected_ent)
                {
                    match order {
                        UnitOrder::EnterLobby => {
                            let path = plan_path_to_room(
                                tf.translation,
                                0,
                                nav.lobby_pos,
                                nav.street_entrance,
                                nav.lobby_pos,
                                &nav.staircases,
                            );
                            unit.waypoints = path;
                        }
                        UnitOrder::GoToFloor(target_f) => {
                            // Find a room on this floor
                            let mut target_center = nav.lobby_pos;
                            for &(f, c) in &nav.floor_room_centers {
                                if f == *target_f {
                                    target_center = c;
                                    break;
                                }
                            }
                            let path = plan_path_to_room(
                                tf.translation,
                                *target_f,
                                target_center,
                                nav.street_entrance,
                                nav.lobby_pos,
                                &nav.staircases,
                            );
                            unit.waypoints = path;
                        }
                        UnitOrder::ExitToStreet => {
                            let current_f = if tf.translation.y > 1.0 {
                                (tf.translation.y / 3.2).floor() as u32
                            } else {
                                0
                            };
                            let path = plan_path_to_street(
                                tf.translation,
                                current_f,
                                nav.street_entrance,
                                nav.lobby_pos,
                                &nav.staircases,
                            );
                            unit.waypoints = path;
                        }
                    }
                }
            }
            HouseAction::Rebuild => {
                settings.needs_rebuild = true;
            }
        }
    }
}

/// Procedural European Building Synthesis Pipeline System
pub fn generate_building_system(
    mut commands: Commands,
    mut settings: ResMut<GeneratorSettings>,
    mut stats: ResMut<GeneratorStats>,
    mut nav: ResMut<ActiveBuildingNavigation>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<BuildingMaterials>,
    catalog: Res<ModelCatalog>,
    existing: Query<Entity, With<Building>>,
) {
    if !settings.needs_rebuild {
        return;
    }
    settings.needs_rebuild = false;

    // 1. Despawn previous building hierarchy
    for ent in &existing {
        commands.entity(ent).despawn();
    }

    // 2. Build footprint polygon from preset
    let polygon = settings.preset.build_polygon(settings.polygon_scale);
    let total_height = settings.ground_story_h + (settings.stories - 1) as f32 * settings.upper_story_h + 3.0;

    // 3. Spawn Building root entity
    let building_root = commands
        .spawn((
            Building,
            BuildingMetadata {
                seed: settings.seed,
                style: settings.style,
                stories: settings.stories,
                story_height: settings.upper_story_h,
                ground_story_height: settings.ground_story_h,
                footprint: polygon.clone(),
                total_rooms: 0,
                total_living_area_sqm: polygon.area() * settings.stories as f32,
            },
            Transform::IDENTITY,
        ))
        .id();

    // 4. Generate Road Enclosure Pavement around polygon
    spawn_road_enclosure(
        &mut commands,
        building_root,
        &polygon,
        &mut meshes,
        &materials,
    );

    // 5. Generate Multi-Story Facade Walls & Premade Windows/Doors along Polygon Edges
    let mut total_windows = 0;
    let mut total_doorways = 1; // Front entrance portal

    let edge_count = polygon.edge_count();
    let mut current_y = 0.0;

    for story in 0..settings.stories {
        let story_h = if story == 0 {
            settings.ground_story_h
        } else {
            settings.upper_story_h
        };
        let wall_mat = materials.wall_material_for_story(settings.style, story);

        for edge_idx in 0..edge_count {
            let (p1, p2) = polygon.edge(edge_idx);
            let edge_len = polygon.edge_length(edge_idx);
            let normal = polygon.edge_outward_normal(edge_idx);
            let wall_center_2d = (p1 + p2) * 0.5;

            // Rotation aligning wall length along the edge and facade normal outward
            let wall_rotation = Quat::from_rotation_y(normal.x.atan2(normal.y));

            // Wall quad mesh
            let wall_mesh = meshes.add(Cuboid::new(edge_len, story_h, 0.35).mesh().build());
            let wall_pos = Vec3::new(wall_center_2d.x, current_y + story_h * 0.5, wall_center_2d.y);

            commands.entity(building_root).with_children(|b| {
                b.spawn((
                    Mesh3d(wall_mesh),
                    MeshMaterial3d(wall_mat.clone()),
                    Transform::from_translation(wall_pos) * Transform::from_rotation(wall_rotation),
                    FloorLevelMarker(story),
                ));
            });

            // Algorithmic placement of premade Windows & Doors from catalog
            let bay_spacing = 3.6;
            let bays = ((edge_len - 1.5) / bay_spacing).floor() as usize;

            if bays > 0 {
                let start_offset = (edge_len - (bays - 1) as f32 * bay_spacing) * 0.5;
                for b_idx in 0..bays {
                    let t = (start_offset + b_idx as f32 * bay_spacing) - edge_len * 0.5;
                    let socket_pos = wall_pos + wall_rotation * Vec3::new(t, 0.0, 0.20);

                    let socket_transform = Transform::from_translation(socket_pos)
                        * Transform::from_rotation(wall_rotation);

                    if story == 0 && edge_idx == 0 && b_idx == bays / 2 {
                        // Place Grand Entrance Portal at center of front facade!
                        catalog.spawn_grand_portal(
                            &mut commands,
                            building_root,
                            socket_transform,
                            &materials,
                        );
                    } else if story == 0 {
                        // Ground floor commercial / arched window
                        catalog.spawn_window(
                            &mut commands,
                            building_root,
                            socket_transform,
                            &materials,
                            story,
                        );
                        total_windows += 1;
                    } else if story == 1 && settings.style == EuropeanStyle::Haussmannian {
                        // Piano Nobile French Balcony with wrought iron railing!
                        catalog.spawn_french_window(
                            &mut commands,
                            building_root,
                            socket_transform,
                            &materials,
                            story,
                        );
                        total_windows += 1;
                    } else {
                        // Standard upper casement window
                        catalog.spawn_window(
                            &mut commands,
                            building_root,
                            socket_transform,
                            &materials,
                            story,
                        );
                        total_windows += 1;
                    }
                }
            }

            // Decorative cornice string course between stories
            if story > 0 {
                let cornice_mesh = meshes.add(Cuboid::new(edge_len + 0.1, 0.25, 0.45).mesh().build());
                commands.entity(building_root).with_children(|b| {
                    b.spawn((
                        Mesh3d(cornice_mesh),
                        MeshMaterial3d(materials.ground_wall_rusticated.clone()),
                        Transform::from_translation(Vec3::new(wall_center_2d.x, current_y, wall_center_2d.y))
                            * Transform::from_rotation(wall_rotation),
                        FloorLevelMarker(story),
                    ));
                });
            }
        }

        current_y += story_h;
    }

    // 6. Generate European Roof (Mansard / Pitched with Zinc/Slate/Tile material)
    spawn_european_roof(
        &mut commands,
        building_root,
        &polygon,
        current_y,
        settings.style,
        &mut meshes,
        &materials,
        &catalog,
    );

    // 7. Dynamic Internal Floor Generation (Rooms, Stairwell Core, Corridor, Doors, Furniture)
    let mut total_rooms = 0;
    nav.staircases.clear();
    nav.floor_room_centers.clear();

    if settings.has_interior {
        let floorplan = generate_internal_floors(
            &mut commands,
            building_root,
            &polygon,
            settings.seed,
            settings.stories,
            settings.ground_story_h,
            settings.upper_story_h,
            &mut meshes,
            &materials,
            &catalog,
        );

        total_rooms = floorplan.total_rooms;
        total_doorways += floorplan.doorway_entities.len();
        nav.street_entrance = floorplan.street_entrance_pos;
        nav.lobby_pos = floorplan.lobby_pos;

        // Register navigation waypoints for floors
        for s in 0..settings.stories {
            let story_y = if s == 0 { 0.0 } else { settings.ground_story_h + (s - 1) as f32 * settings.upper_story_h };
            let centroid = polygon.centroid();
            nav.floor_room_centers.push((s, Vec3::new(centroid.x, story_y, centroid.y)));
        }

        // Cache staircases
        let stair_pos = polygon.centroid() + Vec2::new(0.0, 1.5);
        for s in 0..(settings.stories - 1) {
            let y0 = if s == 0 { 0.0 } else { settings.ground_story_h + (s - 1) as f32 * settings.upper_story_h };
            let y1 = settings.ground_story_h + s as f32 * settings.upper_story_h;
            nav.staircases.push((
                s,
                Vec3::new(stair_pos.x, y0, stair_pos.y),
                Vec3::new(stair_pos.x, y1, stair_pos.y + 2.8),
            ));
        }
    } else {
        let (p1, p2) = polygon.edge(0);
        let front_center = (p1 + p2) * 0.5;
        let normal = polygon.edge_outward_normal(0);
        nav.street_entrance = Vec3::new(front_center.x + normal.x * 0.5, 0.0, front_center.y + normal.y * 0.5);
        nav.lobby_pos = Vec3::new(front_center.x - normal.x * 2.0, 0.0, front_center.y - normal.y * 2.0);
    }

    // 8. Update statistics
    stats.seed = settings.seed;
    stats.style = settings.style;
    stats.preset = settings.preset;
    stats.stories = settings.stories;
    stats.total_rooms = total_rooms;
    stats.total_living_area_sqm = polygon.area() * settings.stories as f32;
    stats.building_height = total_height;
    stats.windows_count = total_windows;
    stats.doorways_count = total_doorways;
}

// ---------------------------------------------------------------------------
// ROOF & ROAD ENCLOSURE HELPERS
// ---------------------------------------------------------------------------

fn spawn_european_roof(
    commands: &mut Commands,
    building: Entity,
    polygon: &BuildingPolygon,
    roof_base_y: f32,
    style: EuropeanStyle,
    meshes: &mut Assets<Mesh>,
    materials: &BuildingMaterials,
    catalog: &ModelCatalog,
) {
    let roof_mat = materials.roof_material_for_style(style);
    let inset_poly = polygon.inset(1.8);
    let edge_count = polygon.edge_count();

    // Mansard roof inclined side panels
    for i in 0..edge_count {
        let (p1, p2) = polygon.edge(i);
        let (ip1, ip2) = inset_poly.edge(i);

        let positions = vec![
            [p1.x, roof_base_y, p1.y],
            [p2.x, roof_base_y, p2.y],
            [ip2.x, roof_base_y + 2.4, ip2.y],
            [ip1.x, roof_base_y + 2.4, ip1.y],
        ];
        let normal = polygon.edge_outward_normal(i);
        let normals = vec![[normal.x, 0.7, normal.y]; 4];
        let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let indices = vec![0, 1, 2, 0, 2, 3];

        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

        commands.entity(building).with_children(|b| {
            b.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(roof_mat.clone()),
                Transform::IDENTITY,
                RoofMarker,
            ));
        });

        // Dormer windows on roof slopes
        let dormer_pos = Vec3::new(
            (p1.x + p2.x) * 0.5 * 0.8 + (ip1.x + ip2.x) * 0.5 * 0.2,
            roof_base_y + 1.2,
            (p1.y + p2.y) * 0.5 * 0.8 + (ip1.y + ip2.y) * 0.5 * 0.2,
        );
        let dormer_rotation = Quat::from_rotation_y(normal.x.atan2(normal.y));

        commands.entity(building).with_children(|b| {
            b.spawn((
                Mesh3d(catalog.window_dormer_roof.clone()),
                MeshMaterial3d(roof_mat.clone()),
                Transform::from_translation(dormer_pos) * Transform::from_rotation(dormer_rotation),
                RoofMarker,
            ));
        });
    }

    // Top flat / terrace roof surface
    let top_slab_mesh = create_flat_top_roof_mesh(&inset_poly, roof_base_y + 2.4);
    commands.entity(building).with_children(|b| {
        b.spawn((
            Mesh3d(meshes.add(top_slab_mesh)),
            MeshMaterial3d(roof_mat),
            Transform::IDENTITY,
            RoofMarker,
        ));
    });

    // Roof Chimneys from catalog
    let centroid = polygon.centroid();
    let chimney_pos = Vec3::new(centroid.x - 3.0, roof_base_y + 2.8, centroid.y - 2.0);
    commands.entity(building).with_children(|b| {
        b.spawn((
            Mesh3d(catalog.chimney_stack.clone()),
            MeshMaterial3d(materials.upper_wall_brick.clone()),
            Transform::from_translation(chimney_pos),
            RoofMarker,
        ));
    });
}

fn create_flat_top_roof_mesh(polygon: &BuildingPolygon, y: f32) -> Mesh {
    let tri_indices = polygon.triangulate();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for &v in &polygon.vertices {
        positions.push([v.x, y, v.y]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([v.x * 0.2, v.y * 0.2]);
    }
    for tri in &tri_indices {
        indices.push(tri[0]);
        indices.push(tri[1]);
        indices.push(tri[2]);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

fn spawn_road_enclosure(
    commands: &mut Commands,
    building: Entity,
    polygon: &BuildingPolygon,
    meshes: &mut Assets<Mesh>,
    mats: &BuildingMaterials,
) {
    let edge_count = polygon.edge_count();

    // Sidewalk strip immediately bordering building polygon
    for i in 0..edge_count {
        let (p1, p2) = polygon.edge(i);
        let normal = polygon.edge_outward_normal(i);

        let positions = vec![
            [p1.x, 0.02, p1.y],
            [p2.x, 0.02, p2.y],
            [p2.x + normal.x * 2.5, 0.02, p2.y + normal.y * 2.5],
            [p1.x + normal.x * 2.5, 0.02, p1.y + normal.y * 2.5],
        ];
        let normals = vec![[0.0, 1.0, 0.0]; 4];
        let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let indices = vec![0, 1, 2, 0, 2, 3];

        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

        commands.entity(building).with_children(|b| {
            b.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(mats.sidewalk_stone.clone()),
                Transform::IDENTITY,
            ));
        });

        // Surrounding Asphalt Street strip
        let road_p1 = p1 + normal * 2.5;
        let road_p2 = p2 + normal * 2.5;
        let road_outer_1 = road_p1 + normal * 7.0;
        let road_outer_2 = road_p2 + normal * 7.0;

        let r_positions = vec![
            [road_p1.x, 0.01, road_p1.y],
            [road_p2.x, 0.01, road_p2.y],
            [road_outer_2.x, 0.01, road_outer_2.y],
            [road_outer_1.x, 0.01, road_outer_1.y],
        ];
        let r_indices = vec![0, 1, 2, 0, 2, 3];

        let mut r_mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        r_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, r_positions);
        r_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4]);
        r_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        r_mesh.insert_indices(bevy::render::mesh::Indices::U32(r_indices));

        commands.entity(building).with_children(|b| {
            b.spawn((
                Mesh3d(meshes.add(r_mesh)),
                MeshMaterial3d(mats.road_asphalt.clone()),
                Transform::IDENTITY,
            ));
        });
    }
}

// ---------------------------------------------------------------------------
// VISIBILITY CUTAWAY SYSTEM
// ---------------------------------------------------------------------------

pub fn update_cutaway_visibility(
    settings: Res<GeneratorSettings>,
    selection: Res<RtsSelectionState>,
    units: Query<&BuildingOccupant, With<RtsUnit>>,
    mut vis_query: Query<(&mut Visibility, Option<&RoofMarker>, Option<&FloorLevelMarker>)>,
) {
    let active_cutaway = match settings.cutaway {
        CutawayMode::FollowSelectedUnit => {
            if let Some(sel_ent) = selection.selected_unit
                && let Ok(occ) = units.get(sel_ent)
                && let Some(floor) = occ.current_floor
            {
                Some(floor)
            } else {
                None
            }
        }
        CutawayMode::RoofOff => None,
        CutawayMode::Floor0 => Some(0),
        CutawayMode::Floor1 => Some(1),
        CutawayMode::Floor2 => Some(2),
        CutawayMode::Floor3 => Some(3),
        CutawayMode::FullExterior => {
            for (mut vis, _, _) in &mut vis_query {
                *vis = Visibility::Visible;
            }
            return;
        }
    };

    for (mut vis, is_roof, floor_marker) in &mut vis_query {
        if is_roof.is_some() {
            *vis = Visibility::Hidden;
        } else if let Some(marker) = floor_marker {
            if let Some(cut_floor) = active_cutaway {
                *vis = if marker.0 <= cut_floor {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            } else {
                *vis = Visibility::Visible;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GIZMOS FOR ROAD SPLINES & PARCEL ENCLOSURE
// ---------------------------------------------------------------------------

pub fn draw_building_gizmos(
    mut gizmos: Gizmos,
    settings: Res<GeneratorSettings>,
    nav: Res<ActiveBuildingNavigation>,
) {
    let polygon = settings.preset.build_polygon(settings.polygon_scale);
    let road_spline = settings.preset.build_road_spline(&polygon, 8.5);

    // 1. Road Splines Enclosure (Gold / Orange Road Lines)
    let n_road = road_spline.len();
    for i in 0..n_road {
        let p1 = road_spline[i];
        let p2 = road_spline[(i + 1) % n_road];
        gizmos.line(
            Vec3::new(p1.x, 0.08, p1.y),
            Vec3::new(p2.x, 0.08, p2.y),
            Color::srgb(1.0, 0.8, 0.2),
        );
    }

    // 2. Front Street Entrance Doorway Marker (Bright Cyan Pillar)
    gizmos.sphere(
        Isometry3d::new(nav.street_entrance + Vec3::Y * 0.4, Quat::IDENTITY),
        0.3,
        Color::srgb(0.2, 0.9, 1.0),
    );

    // 3. Lobby Waypoint Marker (Emerald Green)
    gizmos.sphere(
        Isometry3d::new(nav.lobby_pos + Vec3::Y * 0.4, Quat::IDENTITY),
        0.25,
        Color::srgb(0.2, 1.0, 0.5),
    );
}
