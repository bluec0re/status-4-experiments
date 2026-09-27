use bevy::prelude::*;
use crate::editor::{EditorSet, EditorState, update_road_mesh_system};
use crate::spline::SplineSample;
use crate::terrain::{HeightmapData, HALF_MAP, WATER_THRESHOLD};

pub struct VegetationPlugin;

impl Plugin for VegetationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_vegetation)
            .add_systems(
                Update,
                (
                    update_tree_targets_system
                        .in_set(EditorSet::MeshRebuild)
                        .after(update_road_mesh_system),
                    animate_dynamic_trees_system.in_set(EditorSet::PostUpdate),
                ),
            );
    }
}

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct VegetationMarker;

/// Dynamic tree component holding its natural home position on the landscape,
/// target clearance position away from roads, and physics state for smooth dynamic avoidance.
#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility)]
pub struct DynamicTree {
    pub home_pos: Vec3,         // Base natural resting position in the landscape
    pub target_pos: Vec3,       // Target position (either home_pos or cleared roadside position)
    pub velocity: Vec3,         // Dynamic velocity for lively spring-damper movement
    pub base_scale: f32,        // Scale of the tree
    pub push_offset: f32,       // Individual roadside margin variance (0.5..2.5m)
    pub side_preference: f32,   // -1.0 or 1.0 (preferred displacement side when road is centered)
    pub sway_phase: f32,        // Phase offset for organic breeze sway
    pub is_displaced: bool,     // Whether the tree is currently pushed out of the way of a road
}

/// Builds a realistic procedural conifer/deciduous-style pine tree mesh with trunk and needle canopy
pub fn create_pine_tree_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let trunk_col = [0.28, 0.20, 0.14, 1.0];
    let foliage_col = [0.12, 0.26, 0.13, 1.0];

    // 1. Trunk (Octagonal prism)
    let trunk_radius = 0.24;
    let trunk_height = 2.6;
    let sides = 8;

    let base_idx = positions.len() as u32;
    for i in 0..sides {
        let angle = (i as f32) / (sides as f32) * std::f32::consts::TAU;
        let x = angle.cos() * trunk_radius;
        let z = angle.sin() * trunk_radius;

        // Bottom
        positions.push([x, 0.0, z]);
        normals.push([angle.cos(), 0.0, angle.sin()]);
        colors.push(trunk_col);

        // Top
        positions.push([x * 0.75, trunk_height, z * 0.75]);
        normals.push([angle.cos(), 0.0, angle.sin()]);
        colors.push(trunk_col);
    }

    for i in 0..sides {
        let next = (i + 1) % sides;
        let b0 = base_idx + (i * 2) as u32;
        let t0 = base_idx + (i * 2 + 1) as u32;
        let b1 = base_idx + (next * 2) as u32;
        let t1 = base_idx + (next * 2 + 1) as u32;

        indices.push(b0);
        indices.push(t0);
        indices.push(b1);

        indices.push(b1);
        indices.push(t0);
        indices.push(t1);
    }

    // 2. Three stacked foliage cones
    let tiers = [
        (1.4, 4.2, 2.0),
        (2.8, 5.6, 1.5),
        (4.2, 6.8, 0.95),
    ];

    for (y_base, y_tip, r) in tiers {
        let tip_idx = positions.len() as u32;
        positions.push([0.0, y_tip, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        colors.push(foliage_col);

        let ring_start = positions.len() as u32;
        for i in 0..sides {
            let angle = (i as f32) / (sides as f32) * std::f32::consts::TAU;
            let x = angle.cos() * r;
            let z = angle.sin() * r;

            let norm = Vec3::new(angle.cos() * 0.7, 0.4, angle.sin() * 0.7).normalize();
            positions.push([x, y_base, z]);
            normals.push([norm.x, norm.y, norm.z]);
            colors.push(foliage_col);
        }

        for i in 0..sides {
            let next = (i + 1) % sides;
            let r0 = ring_start + i as u32;
            let r1 = ring_start + next as u32;

            indices.push(tip_idx);
            indices.push(r0);
            indices.push(r1);
        }
    }

    let mut mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// Generates all persistent natural tree entities across the landscape (groves, rolling hills, meadows)
/// strictly on dry gentle terrain, away from the river bed.
pub fn compute_world_tree_homes(heightmap: &HeightmapData) -> Vec<DynamicTree> {
    let mut trees = Vec::new();
    let step = 8.0;

    let mut x = -HALF_MAP + 20.0;
    while x <= HALF_MAP - 20.0 {
        let mut z = -HALF_MAP + 20.0;
        while z <= HALF_MAP - 20.0 {
            let grove_val = ((x * 0.02).sin() * (z * 0.02 + 0.5).cos())
                + ((x * 0.045 + 1.2).cos() * (z * 0.045 - 0.7).sin()) * 0.5;

            if grove_val > 0.15 {
                let jx = (x * 17.0 + z * 19.0).sin() * 3.8;
                let jz = (x * 29.0 - z * 23.0).cos() * 3.8;
                let tx = x + jx;
                let tz = z + jz;

                let y = heightmap.sample(tx, tz);
                let norm = heightmap.sample_normal(tx, tz);

                let river_x = (tz * 0.016).sin() * 40.0 - 15.0;
                let dist_to_river = (tx - river_x).abs();

                // Only plant on gentle slopes, away from the river water, and strictly above water threshold
                if norm.y > 0.88 && dist_to_river > 15.0 && y > (WATER_THRESHOLD + 0.9) {
                    let scale = 0.85 + ((tx * 9.0 + tz * 13.0).sin().abs() * 0.4);
                    let push_offset = 0.5 + ((tx * 13.7 + tz * 7.9).sin().abs() * 2.0);
                    let side_pref = if (tx * 31.0 + tz * 17.0).sin() >= 0.0 { 1.0 } else { -1.0 };
                    let sway_phase = (tx * 3.1 + tz * 5.7).abs();
                    let home = Vec3::new(tx, y, tz);

                    trees.push(DynamicTree {
                        home_pos: home,
                        target_pos: home,
                        velocity: Vec3::ZERO,
                        base_scale: scale,
                        push_offset,
                        side_preference: side_pref,
                        sway_phase,
                        is_displaced: false,
                    });
                }
            }

            z += step;
        }
        x += step;
    }

    trees
}

/// Spawns the entire forest of persistent dynamic tree entities at startup
pub fn setup_vegetation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    heightmap: Res<HeightmapData>,
) {
    let pine_mesh = create_pine_tree_mesh();
    let pine_mesh_handle = meshes.add(pine_mesh);

    let tree_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });

    let tree_homes = compute_world_tree_homes(&heightmap);

    commands.spawn_batch(tree_homes.into_iter().map(move |tree| {
        let pos = tree.home_pos;
        let scale = tree.base_scale;
        (
            Mesh3d(pine_mesh_handle.clone()),
            MeshMaterial3d(tree_mat.clone()),
            Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
            tree,
            VegetationMarker,
        )
    }));
}

/// Validates whether a potential 2D coordinate on the heightmap is safe, dry ground suitable for a tree
pub fn is_valid_tree_ground(pos: Vec2, heightmap: &HeightmapData) -> bool {
    if pos.x.abs() > (HALF_MAP - 12.0) || pos.y.abs() > (HALF_MAP - 12.0) {
        return false;
    }

    let y = heightmap.sample(pos.x, pos.y);
    if y <= (WATER_THRESHOLD + 0.9) {
        return false;
    }

    let river_x = (pos.y * 0.016).sin() * 40.0 - 15.0;
    if (pos.x - river_x).abs() <= 12.0 && y <= (WATER_THRESHOLD + 1.8) {
        return false;
    }

    let norm = heightmap.sample_normal(pos.x, pos.y);
    if norm.y < 0.72 {
        return false;
    }

    true
}

/// Computes the closest point, normal, and road width along the road polyline for a given 2D query position
pub fn find_closest_road_point(
    pos: Vec2,
    road_samples: &[SplineSample],
) -> Option<(Vec2, Vec2, f32, f32)> {
    if road_samples.len() < 2 {
        return None;
    }

    let mut min_dist_sq = f32::MAX;
    let mut best_pt = Vec2::ZERO;
    let mut best_normal = Vec2::ZERO;
    let mut best_width = 8.0;

    for window in road_samples.windows(2) {
        let s0 = &window[0];
        let s1 = &window[1];
        let a = Vec2::new(s0.pos.x, s0.pos.z);
        let b = Vec2::new(s1.pos.x, s1.pos.z);
        let ab = b - a;
        let ab_sq = ab.length_squared();

        let t = if ab_sq > 1e-6 {
            ((pos - a).dot(ab) / ab_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let pt = a + ab * t;
        let dist_sq = (pos - pt).length_squared();

        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            best_pt = pt;
            let n0 = Vec2::new(s0.normal.x, s0.normal.z);
            let n1 = Vec2::new(s1.normal.x, s1.normal.z);
            best_normal = n0.lerp(n1, t).normalize_or_zero();
            best_width = s0.width + (s1.width - s0.width) * t;
        }
    }

    Some((best_pt, best_normal, best_width, min_dist_sq.sqrt()))
}

/// Computes the minimum distance from a query point to any point along the road samples polyline
pub fn min_dist_to_road(pos: Vec2, road_samples: &[SplineSample]) -> f32 {
    let mut min_sq = f32::MAX;
    for window in road_samples.windows(2) {
        let a = Vec2::new(window[0].pos.x, window[0].pos.z);
        let b = Vec2::new(window[1].pos.x, window[1].pos.z);
        let ab = b - a;
        let ab_sq = ab.length_squared();
        let t = if ab_sq > 1e-6 {
            ((pos - a).dot(ab) / ab_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let pt = a + ab * t;
        let d_sq = (pos - pt).length_squared();
        if d_sq < min_sq {
            min_sq = d_sq;
        }
    }
    min_sq.sqrt()
}

/// Evaluates dynamic avoidance for an individual tree against the road spline
pub fn calculate_tree_target(
    tree: &DynamicTree,
    road_samples: &[SplineSample],
    heightmap: &HeightmapData,
    road_bbox: Option<(Vec2, Vec2)>,
) -> (Vec3, bool) {
    if road_samples.len() < 2 {
        return (tree.home_pos, false);
    }

    let tree_xz = Vec2::new(tree.home_pos.x, tree.home_pos.z);

    // Fast rejection: check road AABB
    if let Some((min_p, max_p)) = road_bbox {
        if tree_xz.x < min_p.x || tree_xz.x > max_p.x || tree_xz.y < min_p.y || tree_xz.y > max_p.y {
            return (tree.home_pos, false);
        }
    }

    let Some((road_pt, road_normal, road_width, dist_to_road)) = find_closest_road_point(tree_xz, road_samples) else {
        return (tree.home_pos, false);
    };

    // Clearance buffer: half-width + shoulder (1.0m) + embankment verge (2.8m) + canopy margin (1.0m) + individual jitter
    let required_clearance = road_width * 0.5 + 4.8 + tree.push_offset;

    if dist_to_road >= required_clearance {
        // Tree's natural home is safely outside the road corridor
        return (tree.home_pos, false);
    }

    // Tree conflicts with road! Calculate direction to push tree out of the way
    let away = tree_xz - road_pt;
    let mut push_dir = if away.length_squared() > 1e-4 {
        away.normalize()
    } else {
        // Right on centerline: use road normal along preferred side
        let n = if road_normal.length_squared() > 1e-4 { road_normal } else { Vec2::X };
        if tree.side_preference >= 0.0 { n } else { -n }
    };
    if push_dir.length_squared() < 1e-4 {
        push_dir = Vec2::new(1.0, 0.0);
    }

    // Candidate 1: Preferred roadside
    let cand_1 = road_pt + push_dir * required_clearance;
    let safe_1 = is_valid_tree_ground(cand_1, heightmap);
    let d1 = min_dist_to_road(cand_1, road_samples);

    // Candidate 2: Opposite roadside
    let cand_2 = road_pt - push_dir * required_clearance;
    let safe_2 = is_valid_tree_ground(cand_2, heightmap);
    let d2 = min_dist_to_road(cand_2, road_samples);

    let chosen_xz = if safe_1 && d1 >= (required_clearance - 0.4) {
        cand_1
    } else if safe_2 && d2 >= (required_clearance - 0.4) {
        cand_2
    } else if safe_1 {
        cand_1
    } else if safe_2 {
        cand_2
    } else {
        let y1 = heightmap.sample(cand_1.x, cand_1.y);
        let y2 = heightmap.sample(cand_2.x, cand_2.y);
        if y2 > y1 { cand_2 } else { cand_1 }
    };

    let target_y = heightmap.sample(chosen_xz.x, chosen_xz.y);
    (Vec3::new(chosen_xz.x, target_y, chosen_xz.y), true)
}

/// Whenever road samples change or version increments, updates target positions for all trees
pub fn update_tree_targets_system(
    mut state: ResMut<EditorState>,
    heightmap: Res<HeightmapData>,
    mut trees_query: Query<&mut DynamicTree>,
    mut last_version: Local<usize>,
    mut initialized: Local<bool>,
) {
    if *initialized && *last_version == state.road_version {
        return;
    }
    *initialized = true;
    *last_version = state.road_version;

    let samples = &state.samples;

    // Compute road bounding box for rapid tree pruning
    let road_bbox = if samples.len() >= 2 {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        let mut max_w = 0.0f32;

        for s in samples.iter() {
            if s.pos.x < min_x { min_x = s.pos.x; }
            if s.pos.x > max_x { max_x = s.pos.x; }
            if s.pos.z < min_z { min_z = s.pos.z; }
            if s.pos.z > max_z { max_z = s.pos.z; }
            if s.width > max_w { max_w = s.width; }
        }

        let margin = max_w * 0.5 + 8.5;
        Some((
            Vec2::new(min_x - margin, min_z - margin),
            Vec2::new(max_x + margin, max_z + margin),
        ))
    } else {
        None
    };

    let mut displaced_count = 0usize;
    let mut total_count = 0usize;

    for mut tree in trees_query.iter_mut() {
        total_count += 1;
        let (target, is_displaced) = calculate_tree_target(&tree, samples, &heightmap, road_bbox);
        tree.target_pos = target;
        tree.is_displaced = is_displaced;
        if is_displaced {
            displaced_count += 1;
        }
    }

    state.displaced_trees_count = displaced_count;
    state.total_trees_count = total_count;
}

/// Runs every frame, animating trees toward their target clearance positions using smooth spring-damper dynamics,
/// conformed strictly to the heightmap terrain surface, with lively movement tilt, squash & stretch, and wind sway.
pub fn animate_dynamic_trees_system(
    time: Res<Time>,
    heightmap: Res<HeightmapData>,
    mut query: Query<(&mut DynamicTree, &mut Transform)>,
) {
    let dt = time.delta_secs().clamp(0.0, 0.05);
    if dt <= 0.0 {
        return;
    }
    let t = time.elapsed_secs();

    for (mut tree, mut transform) in query.iter_mut() {
        let current_xz = Vec2::new(transform.translation.x, transform.translation.z);
        let target_xz = Vec2::new(tree.target_pos.x, tree.target_pos.z);
        let diff_xz = target_xz - current_xz;
        let dist = diff_xz.length();

        let is_moving = dist > 0.02 || tree.velocity.length_squared() > 0.001;

        if is_moving {
            // Spring-damper physics for responsive, lively scurrying
            let spring_k = 28.0;
            let damping = 8.8;

            let diff_3d = Vec3::new(diff_xz.x, 0.0, diff_xz.y);
            let horiz_vel = Vec3::new(tree.velocity.x, 0.0, tree.velocity.z);

            let force = diff_3d * spring_k - horiz_vel * damping;
            tree.velocity += force * dt;

            // Cap max speed
            let max_speed = 22.0;
            if tree.velocity.length() > max_speed {
                tree.velocity = tree.velocity.normalize() * max_speed;
            }

            let new_x = transform.translation.x + tree.velocity.x * dt;
            let new_z = transform.translation.z + tree.velocity.z * dt;
            let ground_y = heightmap.sample(new_x, new_z);

            transform.translation = Vec3::new(new_x, ground_y, new_z);

            // Dynamic tilt based on horizontal velocity (trees lean forward as they move out of the way)
            let current_speed = Vec2::new(tree.velocity.x, tree.velocity.z).length();
            let mut lean_rot = Quat::IDENTITY;
            if current_speed > 0.15 {
                let dir_x = tree.velocity.x / current_speed;
                let dir_z = tree.velocity.z / current_speed;
                let travel_dir = Vec3::new(dir_x, 0.0, dir_z);
                let tilt_axis = Vec3::Y.cross(travel_dir).normalize_or_zero();
                let tilt_angle = (current_speed * 0.016).clamp(0.0, 0.20); // up to ~11.5 degrees
                if tilt_axis.length_squared() > 0.5 {
                    lean_rot = Quat::from_axis_angle(tilt_axis, tilt_angle);
                }
            }

            // Squash & stretch: elongated while moving fast, returns to normal upon landing
            let stretch = 1.0 + (current_speed * 0.006).min(0.10);
            let squash = 1.0 / stretch.sqrt();
            let base_s = tree.base_scale;
            transform.scale = Vec3::new(base_s * squash, base_s * stretch, base_s * squash);

            // Ambient wind sway combined with movement tilt
            let wind_sway = ((t * 2.5 + tree.sway_phase).sin() * 0.02)
                + ((t * 5.0 + tree.sway_phase * 1.7).cos() * 0.008);
            let sway_rot = Quat::from_rotation_z(wind_sway);

            transform.rotation = lean_rot * sway_rot;
        } else {
            // Settled stably at roadside target
            let ground_y = heightmap.sample(tree.target_pos.x, tree.target_pos.z);
            transform.translation = Vec3::new(tree.target_pos.x, ground_y, tree.target_pos.z);
            tree.velocity = Vec3::ZERO;
            transform.scale = Vec3::splat(tree.base_scale);

            // Gentle idle breeze swaying
            let wind_sway = ((t * 1.5 + tree.sway_phase).sin() * 0.025)
                + ((t * 3.1 + tree.sway_phase * 2.1).cos() * 0.010);
            transform.rotation = Quat::from_rotation_z(wind_sway);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_world_tree_homes() {
        let hm = HeightmapData::load_or_generate();
        let trees = compute_world_tree_homes(&hm);
        assert!(!trees.is_empty(), "World should generate persistent trees");
        println!("Generated {} persistent natural trees", trees.len());

        for tree in &trees {
            assert!(tree.home_pos.x.abs() <= HALF_MAP);
            assert!(tree.home_pos.z.abs() <= HALF_MAP);
            assert!(tree.home_pos.y > (WATER_THRESHOLD + 0.9), "Tree cannot be spawned underwater");
            assert_eq!(tree.home_pos, tree.target_pos);
            assert!(!tree.is_displaced);
        }
    }

    #[test]
    fn test_tree_dynamic_avoidance_and_return() {
        let hm = HeightmapData::load_or_generate();

        // 1. Create a road segment along Z axis from Z=0 to Z=50 at X=0
        let samples = vec![
            SplineSample {
                pos: Vec3::new(0.0, 10.0, 0.0),
                tangent: Vec3::Z,
                normal: Vec3::X,
                binormal: Vec3::Y,
                width: 8.0,
                banking: 0.0,
                distance: 0.0,
                grade: 0.0,
            },
            SplineSample {
                pos: Vec3::new(0.0, 10.0, 50.0),
                tangent: Vec3::Z,
                normal: Vec3::X,
                binormal: Vec3::Y,
                width: 8.0,
                banking: 0.0,
                distance: 50.0,
                grade: 0.0,
            },
        ];

        // 2. Tree positioned at (1.0, 10.0, 25.0) - right on the road corridor!
        let tree = DynamicTree {
            home_pos: Vec3::new(1.0, 10.0, 25.0),
            target_pos: Vec3::new(1.0, 10.0, 25.0),
            velocity: Vec3::ZERO,
            base_scale: 1.0,
            push_offset: 1.0,
            side_preference: 1.0,
            sway_phase: 0.0,
            is_displaced: false,
        };

        // 3. Calculate target - must move out of the way!
        let (displaced_target, is_displaced) = calculate_tree_target(&tree, &samples, &hm, None);
        assert!(is_displaced, "Tree on the road must be marked displaced");

        let dist_from_centerline = (displaced_target.x * displaced_target.x).sqrt();
        let expected_min_clearance = 8.0 * 0.5 + 4.8 + tree.push_offset - 0.5;
        assert!(
            dist_from_centerline >= expected_min_clearance,
            "Tree must be pushed outside road footprint: got {:.2}m, expected >= {:.2}m",
            dist_from_centerline,
            expected_min_clearance
        );

        // 4. Clear the road: tree must return to home
        let empty_samples: Vec<SplineSample> = Vec::new();
        let (cleared_target, cleared_displaced) = calculate_tree_target(&tree, &empty_samples, &hm, None);
        assert!(!cleared_displaced, "Tree must not be displaced when no road exists");
        assert_eq!(cleared_target, tree.home_pos, "Tree must return home when road is cleared");
    }

    #[test]
    fn test_tree_spring_simulation_convergence() {
        let hm = HeightmapData::load_or_generate();
        let mut tree = DynamicTree {
            home_pos: Vec3::new(0.0, 10.0, 0.0),
            target_pos: Vec3::new(12.0, 10.0, 0.0),
            velocity: Vec3::ZERO,
            base_scale: 1.0,
            push_offset: 1.0,
            side_preference: 1.0,
            sway_phase: 0.0,
            is_displaced: true,
        };

        let mut current_pos = tree.home_pos;
        let dt = 0.016;

        // Simulate 2 seconds of spring physics
        for _ in 0..120 {
            let diff_xz = Vec2::new(tree.target_pos.x - current_pos.x, tree.target_pos.z - current_pos.z);
            let force = Vec3::new(diff_xz.x, 0.0, diff_xz.y) * 28.0 - tree.velocity * 8.8;
            tree.velocity += force * dt;
            current_pos += tree.velocity * dt;
            current_pos.y = hm.sample(current_pos.x, current_pos.z);
        }

        let err = (current_pos.x - tree.target_pos.x).abs();
        assert!(err < 0.1, "Tree should converge to target position, err: {:.3}", err);
        assert!(tree.velocity.length() < 0.2, "Tree velocity should settle, got: {:.3}", tree.velocity.length());
    }
}
