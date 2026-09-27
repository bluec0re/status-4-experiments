use bevy::prelude::*;
use crate::spline::SplineSample;
use crate::terrain::{HeightmapData, HALF_MAP, WATER_THRESHOLD};

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct VegetationMarker;

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

/// Computes valid tree spawn positions from the heightmap texture that form natural woodlots,
/// leaving open areas suitable for town centers and farm meadows, and never underwater.
pub fn compute_tree_positions(road_samples: &[SplineSample], heightmap: &HeightmapData) -> Vec<(Vec3, f32)> {
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
                    let mut too_close_to_road = false;
                    for s in road_samples.iter() {
                        let min_clearance = s.width * 0.5 + 4.5;
                        let d2 = (s.pos.x - tx) * (s.pos.x - tx) + (s.pos.z - tz) * (s.pos.z - tz);
                        if d2 < (min_clearance * min_clearance) {
                            too_close_to_road = true;
                            break;
                        }
                    }

                    if !too_close_to_road {
                        let scale = 0.85 + ((tx * 9.0 + tz * 13.0).sin().abs() * 0.4);
                        trees.push((Vec3::new(tx, y, tz), scale));
                    }
                }
            }

            z += step;
        }
        x += step;
    }

    trees
}
