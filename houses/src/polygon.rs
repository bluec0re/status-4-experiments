#![allow(dead_code)]

use bevy::prelude::*;

/// 2D Polygon representing a parcel or building footprint
#[derive(Clone, Debug)]
pub struct BuildingPolygon {
    pub vertices: Vec<Vec2>,
}

impl BuildingPolygon {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        let mut poly = Self { vertices };
        poly.ensure_ccw();
        poly
    }

    /// Ensure polygon vertices are wound in Counter-Clockwise (CCW) order
    pub fn ensure_ccw(&mut self) {
        if self.signed_area() < 0.0 {
            self.vertices.reverse();
        }
    }

    /// Calculate signed area (Shoelace formula). Positive = CCW, Negative = CW.
    pub fn signed_area(&self) -> f32 {
        let n = self.vertices.len();
        if n < 3 {
            return 0.0;
        }
        let mut area = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            area += self.vertices[i].x * self.vertices[j].y;
            area -= self.vertices[j].x * self.vertices[i].y;
        }
        area * 0.5
    }

    pub fn area(&self) -> f32 {
        self.signed_area().abs()
    }

    /// Compute geometric centroid
    pub fn centroid(&self) -> Vec2 {
        let n = self.vertices.len();
        if n == 0 {
            return Vec2::ZERO;
        }
        let signed_a = self.signed_area();
        if signed_a.abs() < 1e-4 {
            let sum: Vec2 = self.vertices.iter().copied().sum();
            return sum / n as f32;
        }

        let mut cx = 0.0;
        let mut cy = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            let factor = self.vertices[i].x * self.vertices[j].y - self.vertices[j].x * self.vertices[i].y;
            cx += (self.vertices[i].x + self.vertices[j].x) * factor;
            cy += (self.vertices[i].y + self.vertices[j].y) * factor;
        }
        let inv_6a = 1.0 / (6.0 * signed_a);
        Vec2::new(cx * inv_6a, cy * inv_6a)
    }

    /// Number of edges / vertices
    pub fn edge_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get edge segment (start, end)
    pub fn edge(&self, index: usize) -> (Vec2, Vec2) {
        let n = self.vertices.len();
        (self.vertices[index % n], self.vertices[(index + 1) % n])
    }

    /// Outward facing normal vector for edge (assuming CCW)
    pub fn edge_outward_normal(&self, index: usize) -> Vec2 {
        let (p1, p2) = self.edge(index);
        let edge_vec = p2 - p1;
        // In CCW polygon in XZ plane (where X is right and Z is forward),
        // edge normal pointing outward to the right is (edge_vec.y, -edge_vec.x)
        Vec2::new(edge_vec.y, -edge_vec.x).normalize_or_zero()
    }

    /// Edge length
    pub fn edge_length(&self, index: usize) -> f32 {
        let (p1, p2) = self.edge(index);
        p1.distance(p2)
    }

    /// Inset polygon inward by a distance (e.g. for courtyard or interior wall perimeter)
    pub fn inset(&self, distance: f32) -> Self {
        let n = self.vertices.len();
        if n < 3 {
            return self.clone();
        }

        let mut offset_lines = Vec::with_capacity(n);
        for i in 0..n {
            let (p1, p2) = self.edge(i);
            let normal = self.edge_outward_normal(i);
            let offset_p1 = p1 - normal * distance;
            let offset_p2 = p2 - normal * distance;
            offset_lines.push((offset_p1, offset_p2));
        }

        let mut inset_verts = Vec::with_capacity(n);
        for i in 0..n {
            let prev = (i + n - 1) % n;
            let (p1, p2) = offset_lines[prev];
            let (p3, p4) = offset_lines[i];

            if let Some(intersection) = line_intersection(p1, p2, p3, p4) {
                inset_verts.push(intersection);
            } else {
                inset_verts.push(p2);
            }
        }

        let mut res = Self::new(inset_verts);
        if res.signed_area() <= 0.1 {
            // If inset collapsed, scale inward from centroid
            let center = self.centroid();
            let scaled: Vec<Vec2> = self.vertices.iter().map(|&v| center + (v - center) * 0.4).collect();
            res = Self::new(scaled);
        }
        res
    }

    /// Triangulate polygon into indices for flat mesh creation (Ear clipping or fan)
    pub fn triangulate(&self) -> Vec<[u32; 3]> {
        let n = self.vertices.len();
        if n < 3 {
            return Vec::new();
        }

        // For convex / mildly concave building footprints, robust fan triangulation from centroid or ear clipping
        // We'll use simple ear-clipping algorithm
        let mut indices = Vec::new();
        let mut vert_indices: Vec<usize> = (0..n).collect();

        let mut iterations = 0;
        while vert_indices.len() > 3 && iterations < 100 {
            iterations += 1;
            let mut ear_found = false;
            let count = vert_indices.len();

            for i in 0..count {
                let prev = vert_indices[(i + count - 1) % count];
                let curr = vert_indices[i];
                let next = vert_indices[(i + 1) % count];

                let a = self.vertices[prev];
                let b = self.vertices[curr];
                let c = self.vertices[next];

                // Check if (a, b, c) forms a CCW turn
                let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
                if cross <= 1e-5 {
                    continue;
                }

                // Check if any other vertex lies inside triangle abc
                let mut contains_other = false;
                for &other_idx in &vert_indices {
                    if other_idx == prev || other_idx == curr || other_idx == next {
                        continue;
                    }
                    if point_in_triangle(self.vertices[other_idx], a, b, c) {
                        contains_other = true;
                        break;
                    }
                }

                if !contains_other {
                    indices.push([prev as u32, curr as u32, next as u32]);
                    vert_indices.remove(i);
                    ear_found = true;
                    break;
                }
            }

            if !ear_found {
                // Fallback to fan from first vertex
                break;
            }
        }

        if vert_indices.len() == 3 {
            indices.push([vert_indices[0] as u32, vert_indices[1] as u32, vert_indices[2] as u32]);
        } else if vert_indices.len() > 3 {
            // Fallback fan
            for i in 1..(vert_indices.len() - 1) {
                indices.push([vert_indices[0] as u32, vert_indices[i] as u32, vert_indices[i + 1] as u32]);
            }
        }

        indices
    }
}

/// Compute 2D infinite line intersection
fn line_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let d = (p1.x - p2.x) * (p3.y - p4.y) - (p1.y - p2.y) * (p3.x - p4.x);
    if d.abs() < 1e-6 {
        return None;
    }
    let t = ((p1.x - p3.x) * (p3.y - p4.y) - (p1.y - p3.y) * (p3.x - p4.x)) / d;
    Some(p1 + t * (p2 - p1))
}

/// Check if point p lies inside 2D triangle abc
fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let sign1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
    let sign2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
    let sign3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);

    let has_neg = sign1 < -1e-5 || sign2 < -1e-5 || sign3 < -1e-5;
    let has_pos = sign1 > 1e-5 || sign2 > 1e-5 || sign3 > 1e-5;

    !(has_neg && has_pos)
}

/// Road block preset polygons matching European city parcel geometries
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum PolygonPreset {
    #[default]
    HaussmannBlock,
    CanalTrapezoid,
    ParisCornerFlatiron,
    CurvedBoulevard,
    LShapeCourtyard,
    CompactUrbanPlot,
}

impl PolygonPreset {
    pub const ALL: [PolygonPreset; 6] = [
        PolygonPreset::HaussmannBlock,
        PolygonPreset::CanalTrapezoid,
        PolygonPreset::ParisCornerFlatiron,
        PolygonPreset::CurvedBoulevard,
        PolygonPreset::LShapeCourtyard,
        PolygonPreset::CompactUrbanPlot,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::HaussmannBlock => "Haussmann Perimeter Block",
            Self::CanalTrapezoid => "Amsterdam Canal Trapezoid",
            Self::ParisCornerFlatiron => "Parisian Flatiron Corner",
            Self::CurvedBoulevard => "Curved Spline Boulevard",
            Self::LShapeCourtyard => "L-Shaped Courtyard Block",
            Self::CompactUrbanPlot => "Compact Urban Quarter",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::HaussmannBlock => Self::CanalTrapezoid,
            Self::CanalTrapezoid => Self::ParisCornerFlatiron,
            Self::ParisCornerFlatiron => Self::CurvedBoulevard,
            Self::CurvedBoulevard => Self::LShapeCourtyard,
            Self::LShapeCourtyard => Self::CompactUrbanPlot,
            Self::CompactUrbanPlot => Self::HaussmannBlock,
        }
    }

    /// Generate base polygon vertices for the preset
    pub fn build_polygon(&self, scale: f32) -> BuildingPolygon {
        let s = scale;
        let verts = match self {
            Self::HaussmannBlock => vec![
                Vec2::new(-14.0 * s, -10.0 * s),
                Vec2::new(14.0 * s, -10.0 * s),
                Vec2::new(12.0 * s, 10.0 * s),
                Vec2::new(-12.0 * s, 10.0 * s),
            ],
            Self::CanalTrapezoid => vec![
                Vec2::new(-12.0 * s, -8.0 * s),
                Vec2::new(16.0 * s, -6.0 * s),
                Vec2::new(11.0 * s, 11.0 * s),
                Vec2::new(-10.0 * s, 9.0 * s),
            ],
            Self::ParisCornerFlatiron => vec![
                Vec2::new(-14.0 * s, -10.0 * s),
                Vec2::new(14.0 * s, -10.0 * s),
                Vec2::new(1.0 * s, 14.0 * s),
            ],
            Self::CurvedBoulevard => vec![
                Vec2::new(-16.0 * s, -9.0 * s),
                Vec2::new(0.0 * s, -11.0 * s),
                Vec2::new(16.0 * s, -9.0 * s),
                Vec2::new(13.0 * s, 9.0 * s),
                Vec2::new(0.0 * s, 11.0 * s),
                Vec2::new(-13.0 * s, 9.0 * s),
            ],
            Self::LShapeCourtyard => vec![
                Vec2::new(-14.0 * s, -12.0 * s),
                Vec2::new(14.0 * s, -12.0 * s),
                Vec2::new(14.0 * s, 0.0 * s),
                Vec2::new(2.0 * s, 0.0 * s),
                Vec2::new(2.0 * s, 12.0 * s),
                Vec2::new(-14.0 * s, 12.0 * s),
            ],
            Self::CompactUrbanPlot => vec![
                Vec2::new(-10.0 * s, -8.0 * s),
                Vec2::new(10.0 * s, -8.0 * s),
                Vec2::new(10.0 * s, 8.0 * s),
                Vec2::new(-10.0 * s, 8.0 * s),
            ],
        };
        BuildingPolygon::new(verts)
    }

    /// Generate enclosing road spline loop around the polygon
    pub fn build_road_spline(&self, polygon: &BuildingPolygon, road_offset: f32) -> Vec<Vec2> {
        let n = polygon.vertices.len();
        let mut road_points = Vec::with_capacity(n);
        for i in 0..n {
            let v = polygon.vertices[i];
            let normal_prev = polygon.edge_outward_normal((i + n - 1) % n);
            let normal_curr = polygon.edge_outward_normal(i);
            let avg_normal = (normal_prev + normal_curr).normalize_or_zero();
            road_points.push(v + avg_normal * road_offset);
        }
        road_points
    }
}
