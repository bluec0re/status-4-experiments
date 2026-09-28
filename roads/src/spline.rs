use bevy::prelude::*;
use crate::terrain::HeightmapData;

#[derive(Clone, Copy, Debug)]
pub struct RoadWaypoint {
    pub pos: Vec3,
    pub width: f32,
    pub banking: f32, // in radians
}

impl RoadWaypoint {
    pub fn new(pos: Vec3, width: f32) -> Self {
        Self {
            pos,
            width,
            banking: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SplineSample {
    pub pos: Vec3,
    pub tangent: Vec3,
    pub normal: Vec3, // Right vector
    pub binormal: Vec3, // Road Up vector
    pub width: f32,
    #[allow(dead_code)]
    pub banking: f32,
    pub distance: f32,
    pub grade: f32, // Slope percentage (dy / horizontal_dx) * 100
}

/// Evaluates Catmull-Rom spline position at parameter t in [0, 1]
pub fn catmull_rom_pos(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;

    0.5 * (
        (2.0 * p1) +
        (-p0 + p2) * t +
        (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
        (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
    )
}

/// Evaluates Catmull-Rom spline first derivative (tangent) at parameter t
pub fn catmull_rom_tangent(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;

    0.5 * (
        (-p0 + p2) +
        (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * (2.0 * t) +
        (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * (3.0 * t2)
    )
}

struct DensePoint {
    pos: Vec3,
    tangent: Vec3,
    width: f32,
    banking: f32,
    dist: f32,
}

/// Discretizes a list of waypoints into a dense set of uniform arc-length road samples,
/// conformed to the terrain texture to eliminate any road clipping through the ground.
pub fn sample_spline(
    waypoints: &[RoadWaypoint],
    sample_step: f32,
    heightmap: &HeightmapData,
) -> Vec<SplineSample> {
    if waypoints.len() < 2 {
        return Vec::new();
    }

    let n = waypoints.len();

    // 1. Dense pre-sampling of each Catmull-Rom segment
    let mut dense_points: Vec<DensePoint> = Vec::new();
    let mut total_len = 0.0f32;

    for i in 0..(n - 1) {
        let p0 = if i == 0 {
            2.0 * waypoints[0].pos - waypoints[1].pos
        } else {
            waypoints[i - 1].pos
        };
        let p1 = waypoints[i].pos;
        let p2 = waypoints[i + 1].pos;
        let p3 = if i + 2 < n {
            waypoints[i + 2].pos
        } else {
            2.0 * waypoints[n - 1].pos - waypoints[n - 2].pos
        };

        let w1 = waypoints[i].width;
        let w2 = waypoints[i + 1].width;
        let b1 = waypoints[i].banking;
        let b2 = waypoints[i + 1].banking;

        // Determine if this segment is an intentionally elevated bridge
        let g1 = heightmap.sample(p1.x, p1.z);
        let g2 = heightmap.sample(p2.x, p2.z);
        let is_elevated_bridge = (p1.y - g1) > 2.6 || (p2.y - g2) > 2.6;

        let steps_per_seg = 32;
        let start_step = if i == 0 { 0 } else { 1 };

        for s in start_step..=steps_per_seg {
            let t = s as f32 / steps_per_seg as f32;
            let mut pos = catmull_rom_pos(p0, p1, p2, p3, t);
            let mut tangent = catmull_rom_tangent(p0, p1, p2, p3, t);
            if tangent.length_squared() > 1e-6 {
                tangent = tangent.normalize();
            } else {
                tangent = (p2 - p1).normalize_or_zero();
            }
            let width = w1 + (w2 - w1) * t;

            // Ground clearance check to prevent clipping through rising ground between waypoints
            let right_approx = Vec3::new(-tangent.z, 0.0, tangent.x).normalize_or_zero();
            let half_footprint = width * 0.5 + 2.0;

            let g_center = heightmap.sample(pos.x, pos.z);
            let g_left = heightmap.sample(pos.x - right_approx.x * half_footprint, pos.z - right_approx.z * half_footprint);
            let g_right = heightmap.sample(pos.x + right_approx.x * half_footprint, pos.z + right_approx.z * half_footprint);
            let max_ground_under_road = g_center.max(g_left).max(g_right);

            if !is_elevated_bridge {
                // Surface road: ensure the road follows the terrain without dipping beneath mounds
                let min_clearance_y = max_ground_under_road + 0.18;
                pos.y = pos.y.max(min_clearance_y);
            } else {
                // Even on bridges, never allow clipping into peaks
                pos.y = pos.y.max(max_ground_under_road + 0.18);
            }

            // Automatic banking into turns
            let t_next = ((s + 1) as f32 / steps_per_seg as f32).min(1.0);
            let next_tangent = catmull_rom_tangent(p0, p1, p2, p3, t_next).normalize_or_zero();
            let turn_rate = tangent.cross(next_tangent).y;
            let auto_bank = (-turn_rate * 3.2).clamp(-0.10, 0.10);

            let manual_bank = b1 + (b2 - b1) * t;
            let banking = manual_bank + auto_bank;

            if let Some(prev) = dense_points.last() {
                total_len += (pos - prev.pos).length();
            }

            dense_points.push(DensePoint {
                pos,
                tangent,
                width,
                banking,
                dist: total_len,
            });
        }
    }

    if dense_points.is_empty() || total_len <= 1e-4 {
        return Vec::new();
    }

    // 2. Resample at uniform distance intervals
    let num_samples = ((total_len / sample_step).floor() as usize).max(2);
    let mut samples = Vec::with_capacity(num_samples + 1);

    let mut current_idx = 0;
    for k in 0..=num_samples {
        let target_dist = if k == num_samples {
            total_len
        } else {
            (k as f32) * sample_step
        };

        while current_idx + 1 < dense_points.len() && dense_points[current_idx + 1].dist < target_dist {
            current_idx += 1;
        }

        let p_a = &dense_points[current_idx];
        let (pos, tangent, width, banking) = if current_idx + 1 < dense_points.len() {
            let p_b = &dense_points[current_idx + 1];
            let seg_len = p_b.dist - p_a.dist;
            let alpha = if seg_len > 1e-5 {
                ((target_dist - p_a.dist) / seg_len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (
                p_a.pos.lerp(p_b.pos, alpha),
                p_a.tangent.lerp(p_b.tangent, alpha).normalize_or_zero(),
                p_a.width + (p_b.width - p_a.width) * alpha,
                p_a.banking + (p_b.banking - p_a.banking) * alpha,
            )
        } else {
            (p_a.pos, p_a.tangent, p_a.width, p_a.banking)
        };

        // Compute orthonormal road orientation frame
        let world_up = Vec3::Y;
        let mut right = tangent.cross(world_up);
        if right.length_squared() < 1e-5 {
            right = Vec3::X;
        } else {
            right = right.normalize();
        }

        let mut road_up = right.cross(tangent).normalize();

        // Apply banking rotation around tangent
        if banking.abs() > 1e-4 {
            let bank_rot = Quat::from_axis_angle(tangent, banking);
            right = bank_rot * right;
            road_up = bank_rot * road_up;
        }

        // Compute slope gradient
        let horiz_tangent_len = (tangent.x * tangent.x + tangent.z * tangent.z).sqrt();
        let grade = if horiz_tangent_len > 1e-4 {
            (tangent.y / horiz_tangent_len) * 100.0
        } else {
            0.0
        };

        samples.push(SplineSample {
            pos,
            tangent,
            normal: right,
            binormal: road_up,
            width,
            banking,
            distance: target_dist,
            grade,
        });
    }

    samples
}
