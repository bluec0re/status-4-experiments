use crate::spline::{RoadWaypoint, SplineSample};
use crate::terrain::HeightmapData;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct RoadMeshMarker;

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct RoadPylonMarker;

/// Linearly interpolates all properties between two spline samples.
pub fn lerp_spline_sample(a: &SplineSample, b: &SplineSample, t: f32) -> SplineSample {
    let t_clamped = t.clamp(0.0, 1.0);
    let mut normal = a.normal.lerp(b.normal, t_clamped);
    if normal.length_squared() > 1e-6 {
        normal = normal.normalize();
    } else {
        normal = a.normal;
    }

    let mut binormal = a.binormal.lerp(b.binormal, t_clamped);
    if binormal.length_squared() > 1e-6 {
        binormal = binormal.normalize();
    } else {
        binormal = a.binormal;
    }

    let mut tangent = a.tangent.lerp(b.tangent, t_clamped);
    if tangent.length_squared() > 1e-6 {
        tangent = tangent.normalize();
    } else {
        tangent = a.tangent;
    }

    SplineSample {
        pos: a.pos.lerp(b.pos, t_clamped),
        tangent,
        normal,
        binormal,
        distance: a.distance + (b.distance - a.distance) * t_clamped,
        width: a.width + (b.width - a.width) * t_clamped,
        banking: a.banking + (b.banking - a.banking) * t_clamped,
        grade: a.grade + (b.grade - a.grade) * t_clamped,
    }
}

/// Evaluates the exact 3D coordinates for a road cross section's deck:
/// (left_edge, crown_center, right_edge).
/// Both road ribbons and junction mouths share this exact definition to eliminate gaps.
pub fn compute_road_deck_points(
    sample: &SplineSample,
    heightmap: &HeightmapData,
) -> (Vec3, Vec3, Vec3) {
    let center = sample.pos;
    let right = sample.normal;
    let up = sample.binormal;
    let half_w = sample.width * 0.5;

    let center_ground_y = heightmap.sample(center.x, center.z);
    let is_bridge = (center.y - center_ground_y) > 2.8;

    if is_bridge {
        (
            center - right * half_w,
            center + up * 0.04,
            center + right * half_w,
        )
    } else {
        let mut left = center - right * half_w;
        let mut crown = center + up * 0.05;
        let mut right_pt = center + right * half_w;

        let gl = heightmap.sample(left.x, left.z);
        let gc = heightmap.sample(crown.x, crown.z);
        let gr = heightmap.sample(right_pt.x, right_pt.z);

        left.y = left.y.max(gl + 0.12);
        crown.y = crown.y.max(gc + 0.16);
        right_pt.y = right_pt.y.max(gr + 0.12);

        (left, crown, right_pt)
    }
}

/// Builds the 3D road ribbon mesh along the spline samples,
/// conforming strictly to the terrain texture to eliminate ground clipping.
pub fn build_road_mesh(samples: &[SplineSample], heightmap: &HeightmapData) -> Mesh {
    if samples.len() < 2 {
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
    }

    // Cross-section has 9 points across:
    // 0: Left underground skirt anchor (buried -0.8m into terrain)
    // 1: Left surface embankment verge
    // 2: Left gravel shoulder
    // 3: Left asphalt edge (u = 0.15)
    // 4: Center crown (u = 0.50)
    // 5: Right asphalt edge (u = 0.85)
    // 6: Right gravel shoulder
    // 7: Right surface embankment verge
    // 8: Right underground skirt anchor (buried -0.8m into terrain)
    let num_slices = samples.len();
    let num_verts_per_slice = 9;
    let total_verts = num_slices * num_verts_per_slice;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_verts);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(total_verts);
    let mut indices: Vec<u32> =
        Vec::with_capacity((num_slices - 1) * 6 * (num_verts_per_slice - 1));

    let v_scale = 0.25; // Repeats texture every 4 meters

    for s in samples.iter() {
        let center = s.pos;
        let right = s.normal;
        let up = s.binormal;
        let half_w = s.width * 0.5;
        let shoulder_w = 1.0;
        let embankment_w = 2.8;

        let v = s.distance * v_scale;

        // Elevation difference relative to ground
        let center_ground_y = heightmap.sample(center.x, center.z);
        let is_bridge = (center.y - center_ground_y) > 2.8;

        // Slice profiles:
        // Position offsets: (lateral_offset, height_offset, u_coord, vertex_color)
        let pts: [(f32, f32, f32, [f32; 4]); 9] = if is_bridge {
            // Elevated viaduct bridge profile with safety parapets/railings
            [
                (-(half_w + 0.4), -1.2, 0.0, [0.35, 0.35, 0.38, 1.0]), // Bridge bottom-left
                (-(half_w + 0.4), -0.6, 0.04, [0.45, 0.45, 0.48, 1.0]), // Bridge girder-left
                (-(half_w + 0.3), 0.4, 0.08, [0.72, 0.72, 0.76, 1.0]), // Railing top-left
                (-half_w, 0.0, 0.15, [0.35, 0.35, 0.38, 1.0]),         // Deck road left
                (0.0, 0.04, 0.5, [0.35, 0.35, 0.38, 1.0]),             // Crown center
                (half_w, 0.0, 0.85, [0.35, 0.35, 0.38, 1.0]),          // Deck road right
                (half_w + 0.3, 0.4, 0.92, [0.72, 0.72, 0.76, 1.0]),    // Railing top-right
                (half_w + 0.4, -0.6, 0.96, [0.45, 0.45, 0.48, 1.0]),   // Bridge girder-right
                (half_w + 0.4, -1.2, 1.0, [0.35, 0.35, 0.38, 1.0]),    // Bridge bottom-right
            ]
        } else {
            // Embankment cross-section smoothly blended into hillside
            [
                (
                    -(half_w + shoulder_w + embankment_w + 0.4),
                    0.0,
                    0.0,
                    [0.26, 0.32, 0.20, 1.0],
                ), // Subterranean left
                (
                    -(half_w + shoulder_w + embankment_w),
                    0.0,
                    0.02,
                    [0.28, 0.36, 0.22, 1.0],
                ), // Surface embankment left
                (-(half_w + shoulder_w), -0.05, 0.08, [0.42, 0.40, 0.36, 1.0]), // Shoulder left
                (-half_w, 0.0, 0.15, [0.32, 0.32, 0.34, 1.0]),                  // Road edge left
                (0.0, 0.05, 0.5, [0.32, 0.32, 0.34, 1.0]),                      // Crown center
                (half_w, 0.0, 0.85, [0.32, 0.32, 0.34, 1.0]),                   // Road edge right
                (half_w + shoulder_w, -0.05, 0.92, [0.42, 0.40, 0.36, 1.0]),    // Shoulder right
                (
                    half_w + shoulder_w + embankment_w,
                    0.0,
                    0.98,
                    [0.28, 0.36, 0.22, 1.0],
                ), // Surface embankment right
                (
                    half_w + shoulder_w + embankment_w + 0.4,
                    0.0,
                    1.0,
                    [0.26, 0.32, 0.20, 1.0],
                ), // Subterranean right
            ]
        };

        let (deck_left, deck_crown, deck_right) = compute_road_deck_points(s, heightmap);

        for (idx, &(lat_off, h_off, u_coord, vertex_color)) in pts.iter().enumerate() {
            let pt = match idx {
                3 => deck_left,
                4 => deck_crown,
                5 => deck_right,
                _ => {
                    let mut p = center + right * lat_off + up * h_off;
                    if !is_bridge {
                        let local_ground = heightmap.sample(p.x, p.z);
                        match idx {
                            0 | 8 => p.y = local_ground - 0.75,
                            1 | 7 => p.y = local_ground - 0.03,
                            2 | 6 => p.y = p.y.max(local_ground + 0.06),
                            _ => {}
                        }
                    }
                    p
                }
            };

            positions.push([pt.x, pt.y, pt.z]);
            normals.push([up.x, up.y, up.z]);
            uvs.push([u_coord, v]);
            colors.push(vertex_color);
        }
    }

    // Build triangle index grid
    for i in 0..(num_slices - 1) {
        let row_curr = (i * num_verts_per_slice) as u32;
        let row_next = ((i + 1) * num_verts_per_slice) as u32;

        for j in 0..(num_verts_per_slice - 1) {
            let j_u32 = j as u32;
            let c0 = row_curr + j_u32;
            let c1 = row_next + j_u32;
            let c2 = row_curr + j_u32 + 1;
            let c3 = row_next + j_u32 + 1;

            indices.push(c0);
            indices.push(c1);
            indices.push(c2);

            indices.push(c2);
            indices.push(c1);
            indices.push(c3);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Generates bridge pillar positions where road is elevated high above terrain
pub fn generate_bridge_pylons(
    samples: &[SplineSample],
    heightmap: &HeightmapData,
) -> Vec<(Vec3, f32)> {
    let mut pylons = Vec::new();
    let min_spacing = 18.0;
    let mut last_pylon_dist = -100.0;

    for s in samples.iter() {
        let ground_y = heightmap.sample(s.pos.x, s.pos.z);
        let height_above_ground = s.pos.y - ground_y;

        if height_above_ground > 3.0 && (s.distance - last_pylon_dist) >= min_spacing {
            let base_pos = Vec3::new(s.pos.x, ground_y + height_above_ground * 0.5, s.pos.z);
            pylons.push((base_pos, height_above_ground));
            last_pylon_dist = s.distance;
        }
    }

    pylons
}

/// Generates roadside delineators / reflectors along curves and edges
#[allow(dead_code)]
pub fn generate_road_posts(samples: &[SplineSample]) -> Vec<(Vec3, Quat)> {
    let mut posts = Vec::new();
    let step = 10.0;
    let mut next_dist = 5.0;

    for s in samples.iter() {
        if s.distance >= next_dist {
            let half_w = s.width * 0.5 + 0.5;
            let post_pos_left = s.pos - s.normal * half_w;
            let post_pos_right = s.pos + s.normal * half_w;

            let rot = Quat::from_rotation_arc(Vec3::Z, s.tangent);
            posts.push((post_pos_left, rot));
            posts.push((post_pos_right, rot));

            next_dist += step;
        }
    }

    posts
}

/// Generates realistic procedural PBR road asphalt texture with markings adapted to road width and lanes
pub fn create_road_texture(road_width: f32, lanes: usize) -> Image {
    let width = 512;
    let height = 512;
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    let rw = road_width.max(3.0);
    let effective_lanes = match lanes {
        1 => 1,
        2 => 2,
        4 => 4,
        _ => {
            if rw < 5.8 {
                1
            } else if rw < 11.2 {
                2
            } else {
                4
            }
        }
    };

    for y in 0..height {
        let v = y as f32 / height as f32;
        // Seamless periodic repeat of dashes along V (4 complete cycles per texture height)
        let dash_phase = (v * 4.0).fract();
        let is_dash = dash_phase < 0.55;

        for x in 0..width {
            let u = x as f32 / width as f32;

            let noise_val = (((x * 97 + y * 131) ^ (x * 17)) & 0x1F) as f32 / 31.0;
            let mut r = 42.0 + noise_val * 14.0;
            let mut g = 42.0 + noise_val * 14.0;
            let mut b = 45.0 + noise_val * 14.0;

            if u < 0.15 || u > 0.85 {
                // Shoulders gravel
                let gravel_noise = (((x * 67 + y * 43) * 73) & 0x3F) as f32 / 63.0;
                r = 85.0 + gravel_noise * 30.0;
                g = 80.0 + gravel_noise * 25.0;
                b = 70.0 + gravel_noise * 25.0;
            } else {
                // Asphalt road surface
                let u_road = (u - 0.15) / 0.70;
                let x_m = u_road * rw;

                // 1. Solid white outer edge lines (16cm wide, inset 6cm from asphalt edge)
                let is_left_edge = (0.06..=0.22).contains(&x_m);
                let is_right_edge = x_m >= (rw - 0.22) && x_m <= (rw - 0.06);

                if is_left_edge || is_right_edge {
                    let edge_noise = (noise_val - 0.5) * 20.0;
                    r = (235.0 + edge_noise).clamp(180.0, 255.0);
                    g = (235.0 + edge_noise).clamp(180.0, 255.0);
                    b = (230.0 + edge_noise).clamp(175.0, 250.0);
                } else {
                    let mut is_line_painted = false;
                    let mut in_wheel_track = false;

                    match effective_lanes {
                        1 => {
                            // Single lane road: No center line.
                            // Wheel tracks for single vehicle centered in lane
                            let center = rw * 0.5;
                            if (x_m - (center - 0.85)).abs() < 0.26
                                || (x_m - (center + 0.85)).abs() < 0.26
                            {
                                in_wheel_track = true;
                            }
                        }
                        2 => {
                            // Two lanes: Center dashed line
                            let center = rw * 0.5;
                            if (x_m - center).abs() <= 0.08 && is_dash {
                                let line_noise = (noise_val - 0.5) * 15.0;
                                r = (242.0 + line_noise).clamp(180.0, 255.0);
                                g = (205.0 + line_noise).clamp(160.0, 230.0);
                                b = (40.0 + line_noise).clamp(30.0, 80.0);
                                is_line_painted = true;
                            }

                            // Two sets of wheel tracks
                            let lane1_c = rw * 0.25;
                            let lane2_c = rw * 0.75;
                            if (x_m - (lane1_c - 0.80)).abs() < 0.24
                                || (x_m - (lane1_c + 0.80)).abs() < 0.24
                                || (x_m - (lane2_c - 0.80)).abs() < 0.24
                                || (x_m - (lane2_c + 0.80)).abs() < 0.24
                            {
                                in_wheel_track = true;
                            }
                        }
                        4 => {
                            // Four lanes: Double yellow center line + dashed white lane dividers
                            let center = rw * 0.5;
                            // Center double yellow line:
                            let is_yellow1 = x_m >= (center - 0.24) && x_m <= (center - 0.08);
                            let is_yellow2 = x_m >= (center + 0.08) && x_m <= (center + 0.24);

                            if is_yellow1 || is_yellow2 {
                                let line_noise = (noise_val - 0.5) * 15.0;
                                r = (245.0 + line_noise).clamp(190.0, 255.0);
                                g = (200.0 + line_noise).clamp(150.0, 225.0);
                                b = (35.0 + line_noise).clamp(20.0, 70.0);
                                is_line_painted = true;
                            } else {
                                // Dashed white lane dividers at 1/4 and 3/4
                                let div1 = rw * 0.25;
                                let div2 = rw * 0.75;
                                if ((x_m - div1).abs() <= 0.075 || (x_m - div2).abs() <= 0.075)
                                    && is_dash
                                {
                                    let line_noise = (noise_val - 0.5) * 15.0;
                                    r = (235.0 + line_noise).clamp(180.0, 255.0);
                                    g = (235.0 + line_noise).clamp(180.0, 255.0);
                                    b = (230.0 + line_noise).clamp(175.0, 250.0);
                                    is_line_painted = true;
                                }
                            }

                            // Four sets of wheel tracks
                            let centers = [rw * 0.125, rw * 0.375, rw * 0.625, rw * 0.875];
                            for lc in centers {
                                if (x_m - (lc - 0.75)).abs() < 0.22
                                    || (x_m - (lc + 0.75)).abs() < 0.22
                                {
                                    in_wheel_track = true;
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }

                    if !is_line_painted && in_wheel_track {
                        r *= 0.86;
                        g *= 0.86;
                        b *= 0.88;
                    }
                }
            }

            data.push(r as u8);
            data.push(g as u8);
            data.push(b as u8);
            data.push(255);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    // Set sampler addressing to Repeat so texture tiles seamlessly along spline V axis
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..default()
    });

    image
}

/// Visual marking and asphalt style for road junctions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JunctionStyle {
    #[default]
    BoxMarking, // Yellow criss-cross intersection box with stop bars & rubber swept paths
    TurningCircle, // Central dashed turning roundabout guide circle & stop bars
    Continental,   // Pedestrian zebra crossings along entrance mouths & clean asphalt
}

impl JunctionStyle {
    pub fn next(&self) -> Self {
        match self {
            Self::BoxMarking => Self::TurningCircle,
            Self::TurningCircle => Self::Continental,
            Self::Continental => Self::BoxMarking,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BoxMarking => "Box",
            Self::TurningCircle => "Circle",
            Self::Continental => "Zebra",
        }
    }
}

/// Controls whether crossings (junctions) warp their deck to match incoming road elevations,
/// or remain a rigid horizontal plane with incoming roads ramping smoothly to attach to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JunctionElevationMode {
    #[default]
    Warped, // Junction deck warps smoothly to match incoming road elevations
    Planar, // Junction deck is always a flat plane (y = junction.pos.y) and incoming roads ramp/attach to it
}

impl JunctionElevationMode {
    pub fn toggle(&self) -> Self {
        match self {
            Self::Warped => Self::Planar,
            Self::Planar => Self::Warped,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Warped => "Warped",
            Self::Planar => "Planar",
        }
    }
}

/// Generates a dedicated PBR intersection texture featuring specialized road markings,
/// realistic curved rubber scrub wear tracks, and entrance stop lines.
pub fn create_junction_texture(style: JunctionStyle) -> Image {
    let width = 512;
    let height = 512;
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        let v = y as f32 / height as f32;
        let cy = v - 0.5;

        for x in 0..width {
            let u = x as f32 / width as f32;
            let cx = u - 0.5;
            let dist = (cx * cx + cy * cy).sqrt();
            let angle = cy.atan2(cx);

            // Multi-frequency noise for realistic aggregate
            let noise_fine = (((x * 127 + y * 311) ^ (x * 37)) & 0x1F) as f32 / 31.0;
            let noise_coarse = (((x * 53 + y * 89) * 19) & 0x3F) as f32 / 63.0;

            // Dedicated UV corner for white road markings (zebra crossings, stop lines)
            if u < 0.03 && v < 0.03 {
                let line_noise = (noise_fine - 0.5) * 10.0;
                let wr = (248.0 + line_noise).clamp(210.0, 255.0) as u8;
                let wg = (248.0 + line_noise).clamp(210.0, 255.0) as u8;
                let wb = (242.0 + line_noise).clamp(205.0, 255.0) as u8;
                data.push(wr);
                data.push(wg);
                data.push(wb);
                data.push(255);
                continue;
            }

            // 1. Skirt / Shoulder outer gravel
            if dist > 0.46 {
                let gravel_noise = (((x * 67 + y * 43) * 73) & 0x3F) as f32 / 63.0;
                let r = (82.0 + gravel_noise * 28.0).clamp(0.0, 255.0);
                let g = (78.0 + gravel_noise * 24.0).clamp(0.0, 255.0);
                let b = (68.0 + gravel_noise * 24.0).clamp(0.0, 255.0);
                data.push(r as u8);
                data.push(g as u8);
                data.push(b as u8);
                data.push(255);
                continue;
            }

            // 2. Base high-grade asphalt
            let mut r = 40.0 + noise_fine * 12.0 + (noise_coarse - 0.5) * 6.0;
            let mut g = 40.0 + noise_fine * 12.0 + (noise_coarse - 0.5) * 6.0;
            let mut b = 43.0 + noise_fine * 12.0 + (noise_coarse - 0.5) * 6.0;

            // 3. Curved rubber turning tire scuff marks / swept paths
            // Hyperbolic and circular wear tracks between orthogonal directions
            let turn_hyp = (cx * cy).abs();
            let is_turn_track = (0.025..=0.065).contains(&turn_hyp) && dist < 0.38;
            let is_ring_track = (dist - 0.28).abs() < 0.04 || (dist - 0.16).abs() < 0.035;
            if is_turn_track || is_ring_track {
                // Darkened bitumen from compacted rubber deposits
                r *= 0.78;
                g *= 0.78;
                b *= 0.76;
            }

            // Center engine oil / idle stain
            if dist < 0.12 {
                let oil_factor = 1.0 - (dist / 0.12);
                let oil_darken = 1.0 - (oil_factor * 0.22);
                r *= oil_darken;
                g *= oil_darken;
                b *= oil_darken * 0.95;
            }

            // 4. Center markings according to JunctionStyle
            // (Entrance markings like zebra crosswalks & stop lines are generated per road arm in build_junction_mesh)
            match style {
                JunctionStyle::BoxMarking => {
                    // Yellow criss-cross box junction in center square
                    let box_half_w = 0.24;
                    let in_box = cx.abs() < box_half_w && cy.abs() < box_half_w;
                    if in_box {
                        let is_box_border =
                            cx.abs() > (box_half_w - 0.018) || cy.abs() > (box_half_w - 0.018);
                        let diag1 = ((cx + cy) * 26.0).sin().abs() < 0.18;
                        let diag2 = ((cx - cy) * 26.0).sin().abs() < 0.18;
                        if is_box_border || diag1 || diag2 {
                            let yellow_noise = (noise_fine - 0.5) * 15.0;
                            r = (246.0 + yellow_noise).clamp(180.0, 255.0);
                            g = (205.0 + yellow_noise).clamp(150.0, 230.0);
                            b = (38.0 + yellow_noise).clamp(25.0, 75.0);
                        }
                    }
                }
                JunctionStyle::TurningCircle => {
                    if dist >= 0.195 && dist <= 0.225 {
                        // Dashed guidance circle
                        let dash = (angle * 12.0).sin() > 0.0;
                        if dash {
                            let line_noise = (noise_fine - 0.5) * 15.0;
                            r = (242.0 + line_noise).clamp(180.0, 255.0);
                            g = (242.0 + line_noise).clamp(180.0, 255.0);
                            b = (238.0 + line_noise).clamp(175.0, 255.0);
                        }
                    } else if dist < 0.075 {
                        // Central brick / cobblestone paved medallion
                        let tile_x = ((cx * 60.0).sin().abs() < 0.15) as i32;
                        let tile_y = ((cy * 60.0).sin().abs() < 0.15) as i32;
                        if tile_x + tile_y > 0 {
                            r = 75.0;
                            g = 72.0;
                            b = 70.0;
                        } else {
                            r = 135.0 + noise_fine * 20.0;
                            g = 95.0 + noise_fine * 18.0;
                            b = 75.0 + noise_fine * 18.0;
                        }
                    }
                }
                JunctionStyle::Continental => {
                    // Clean central asphalt with aggregate and tire wear marks
                }
            }

            // Transition blend towards outer curb
            if dist > 0.44 && dist <= 0.46 {
                let t = (dist - 0.44) / 0.02;
                r = r * (1.0 - t) + 75.0 * t;
                g = g * (1.0 - t) + 72.0 * t;
                b = b * (1.0 - t) + 65.0 * t;
            }

            data.push(r as u8);
            data.push(g as u8);
            data.push(b as u8);
            data.push(255);
        }
    }

    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        ..default()
    });

    image
}

/// Represents a single street / road corridor composed of ordered waypoint indices
#[derive(Clone, Debug, PartialEq)]
pub struct Street {
    pub id: usize,
    pub name: String,
    pub node_indices: Vec<usize>,
    pub road_width: f32,
    pub lanes: usize,
}

impl Street {
    pub fn new(id: usize, name: impl Into<String>, road_width: f32, lanes: usize) -> Self {
        Self {
            id,
            name: name.into(),
            node_indices: Vec::new(),
            road_width,
            lanes,
        }
    }
}

/// A road arm meeting at a junction
#[derive(Clone, Debug)]
pub struct JunctionArm {
    pub street_idx: usize,
    pub neighbor_node_idx: usize,
    pub dir: Vec3,  // XZ direction pointing away from junction
    pub angle: f32, // atan2(dir.z, dir.x) in radians
    pub width: f32,
    pub mouth_sample: Option<SplineSample>,
}

/// A junction / intersection joining multiple streets or road branches
#[derive(Clone, Debug)]
pub struct Junction {
    pub node_idx: usize,
    pub pos: Vec3,
    pub connected_arms: Vec<JunctionArm>,
    pub radius: f32,
}

/// Detects all junctions across waypoints and streets.
/// A node forms a junction if it connects 3+ road arms (T-junction, crossroads, etc.)
/// or connects 2+ arms from different streets (joining two distinct streets).
pub fn detect_junctions(waypoints: &[RoadWaypoint], streets: &[Street]) -> Vec<Junction> {
    if waypoints.is_empty() || streets.is_empty() {
        return Vec::new();
    }

    let mut junctions = Vec::new();

    for node_idx in 0..waypoints.len() {
        let node_pos = waypoints[node_idx].pos;
        let mut arms = Vec::new();

        for (street_idx, street) in streets.iter().enumerate() {
            let indices = &street.node_indices;
            for (pos_in_street, &idx) in indices.iter().enumerate() {
                if idx != node_idx {
                    continue;
                }

                // Preceding node in this street
                if pos_in_street > 0 {
                    let prev_idx = indices[pos_in_street - 1];
                    let prev_pos = waypoints[prev_idx].pos;
                    let dir = (prev_pos - node_pos).normalize_or_zero();
                    let angle = dir.z.atan2(dir.x);
                    arms.push(JunctionArm {
                        street_idx,
                        neighbor_node_idx: prev_idx,
                        dir,
                        angle,
                        width: street.road_width,
                        mouth_sample: None,
                    });
                }

                // Succeeding node in this street
                if pos_in_street + 1 < indices.len() {
                    let next_idx = indices[pos_in_street + 1];
                    let next_pos = waypoints[next_idx].pos;
                    let dir = (next_pos - node_pos).normalize_or_zero();
                    let angle = dir.z.atan2(dir.x);
                    arms.push(JunctionArm {
                        street_idx,
                        neighbor_node_idx: next_idx,
                        dir,
                        angle,
                        width: street.road_width,
                        mouth_sample: None,
                    });
                }
            }
        }

        arms.dedup_by(|a, b| {
            a.neighbor_node_idx == b.neighbor_node_idx && a.street_idx == b.street_idx
        });

        let unique_streets_count = {
            let mut s_ids: Vec<usize> = arms.iter().map(|a| a.street_idx).collect();
            s_ids.sort_unstable();
            s_ids.dedup();
            s_ids.len()
        };

        if arms.len() >= 3 || (arms.len() >= 2 && unique_streets_count >= 2) {
            let max_w = arms.iter().map(|a| a.width).fold(0.0f32, f32::max);
            let radius = (max_w * 0.5 * 1.35).clamp(4.0, 16.0);

            arms.sort_by(|a, b| {
                a.angle
                    .partial_cmp(&b.angle)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            junctions.push(Junction {
                node_idx,
                pos: node_pos,
                connected_arms: arms,
                radius,
            });
        }
    }

    junctions
}

/// Builds a 3D intersection mesh for a junction, rounding corner curbs with smooth fillets
/// and warping perfectly to connected road arm profiles to eliminate gaps and glitching.
pub fn build_junction_mesh(
    junction: &Junction,
    waypoints: &[RoadWaypoint],
    heightmap: &HeightmapData,
    style: JunctionStyle,
    elevation_mode: JunctionElevationMode,
) -> Option<Mesh> {
    let k = junction.connected_arms.len();
    if k < 2 {
        return None;
    }

    let center_pos = junction.pos;
    let ground_center = heightmap.sample(center_pos.x, center_pos.z);
    let is_bridge = (center_pos.y - ground_center) > 2.8;

    let r = junction.radius;
    let max_extent = junction
        .connected_arms
        .iter()
        .map(|a| a.width * 0.5 + r)
        .fold(r * 1.35, f32::max);

    // Helper to evaluate exact road elevation at arm mouth from connected street waypoint slope (fallback)
    let get_arm_elevation = |arm: &JunctionArm, mouth_xz: Vec2| -> f32 {
        let neighbor_pos = waypoints
            .get(arm.neighbor_node_idx)
            .map(|w| w.pos)
            .unwrap_or(center_pos);
        let dist = Vec2::new(neighbor_pos.x - center_pos.x, neighbor_pos.z - center_pos.z)
            .length()
            .max(1.0);
        let t = (r / dist).clamp(0.0, 1.0);
        let waypoint_y = center_pos.y + (neighbor_pos.y - center_pos.y) * t;

        if is_bridge {
            waypoint_y + 0.04
        } else {
            let g = heightmap.sample(mouth_xz.x, mouth_xz.y);
            waypoint_y.max(g + 0.12)
        }
    };

    // Compute exact deck points (left, center_crown, right) for each arm mouth
    let (center_elev, arm_mouth_pts) = match elevation_mode {
        JunctionElevationMode::Warped => {
            let mut pts: Vec<(Vec3, Vec3, Vec3)> = Vec::with_capacity(k);
            for arm in &junction.connected_arms {
                let d = Vec3::new(arm.dir.x, 0.0, arm.dir.z).normalize_or_zero();
                let right_arm = Vec3::new(-d.z, 0.0, d.x);
                let half_w = arm.width * 0.5;

                let mouth_pts = if let Some(ref sample) = arm.mouth_sample {
                    let (l, c, r_pt) = compute_road_deck_points(sample, heightmap);
                    let outward = Vec3::new(
                        sample.pos.x - center_pos.x,
                        0.0,
                        sample.pos.z - center_pos.z,
                    )
                    .normalize_or_zero();
                    let sample_right = Vec3::new(-outward.z, 0.0, outward.x);
                    if sample.normal.dot(sample_right) >= 0.0 {
                        (l, c, r_pt)
                    } else {
                        (r_pt, c, l)
                    }
                } else {
                    let mouth_center = center_pos + d * r;
                    let y_c = get_arm_elevation(arm, Vec2::new(mouth_center.x, mouth_center.z));
                    (
                        Vec3::new(
                            mouth_center.x - right_arm.x * half_w,
                            y_c,
                            mouth_center.z - right_arm.z * half_w,
                        ),
                        Vec3::new(mouth_center.x, y_c + 0.04, mouth_center.z),
                        Vec3::new(
                            mouth_center.x + right_arm.x * half_w,
                            y_c,
                            mouth_center.z + right_arm.z * half_w,
                        ),
                    )
                };
                pts.push(mouth_pts);
            }
            let avg_arm_y = pts.iter().map(|(_, c, _)| c.y).sum::<f32>() / (k as f32);
            let c_elev = if is_bridge {
                avg_arm_y.max(center_pos.y + 0.04)
            } else {
                avg_arm_y.max(ground_center + 0.16)
            };
            (c_elev, pts)
        }
        JunctionElevationMode::Planar => {
            let plane_y = center_pos.y;
            let c_elev = plane_y + 0.04;
            let mut pts: Vec<(Vec3, Vec3, Vec3)> = Vec::with_capacity(k);
            for arm in &junction.connected_arms {
                let d = Vec3::new(arm.dir.x, 0.0, arm.dir.z).normalize_or_zero();
                let right_arm = Vec3::new(-d.z, 0.0, d.x);
                let half_w = arm.width * 0.5;

                let mouth_center = if let Some(ref sample) = arm.mouth_sample {
                    Vec3::new(sample.pos.x, plane_y, sample.pos.z)
                } else {
                    center_pos + d * r
                };
                pts.push((
                    Vec3::new(
                        mouth_center.x - right_arm.x * half_w,
                        plane_y,
                        mouth_center.z - right_arm.z * half_w,
                    ),
                    Vec3::new(mouth_center.x, plane_y + 0.04, mouth_center.z),
                    Vec3::new(
                        mouth_center.x + right_arm.x * half_w,
                        plane_y,
                        mouth_center.z + right_arm.z * half_w,
                    ),
                ));
            }
            (c_elev, pts)
        }
    };

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Center vertex (index 0)
    positions.push([center_pos.x, center_elev, center_pos.z]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);
    let deck_color = if is_bridge {
        [0.35, 0.35, 0.38, 1.0]
    } else {
        [0.32, 0.32, 0.34, 1.0]
    };
    colors.push(deck_color);

    let mut perimeter_pts: Vec<Vec3> = Vec::new();

    for i in 0..k {
        let (pt_arm_left, pt_arm_center, pt_arm_right) = arm_mouth_pts[i];
        let (pt_next_left, _, _) = arm_mouth_pts[(i + 1) % k];

        perimeter_pts.push(pt_arm_left);
        perimeter_pts.push(pt_arm_center);
        perimeter_pts.push(pt_arm_right);

        // Smooth quadratic Bézier fillet rounding the corner between streets
        let arm_curr = &junction.connected_arms[i];
        let arm_next = &junction.connected_arms[(i + 1) % k];
        let d_curr = Vec3::new(arm_curr.dir.x, 0.0, arm_curr.dir.z).normalize_or_zero();
        let d_next = Vec3::new(arm_next.dir.x, 0.0, arm_next.dir.z).normalize_or_zero();

        let corner_dir_sum = d_curr + d_next;
        let corner_apex_y = match elevation_mode {
            JunctionElevationMode::Warped => (pt_arm_right.y + pt_next_left.y) * 0.5,
            JunctionElevationMode::Planar => center_pos.y,
        };
        let corner_apex = if corner_dir_sum.length_squared() < 0.05 {
            // Straight through-street (180 degrees)
            (pt_arm_right + pt_next_left) * 0.5
        } else {
            let corner_dir = corner_dir_sum.normalize();
            let dist_curr =
                Vec2::new(pt_arm_right.x - center_pos.x, pt_arm_right.z - center_pos.z).length();
            let dist_next =
                Vec2::new(pt_next_left.x - center_pos.x, pt_next_left.z - center_pos.z).length();
            let corner_dist = dist_curr.max(dist_next);
            let corner_apex_xz = center_pos + corner_dir * corner_dist;
            Vec3::new(corner_apex_xz.x, corner_apex_y, corner_apex_xz.z)
        };

        for step in 1..=3 {
            let t = step as f32 / 4.0;
            let one_minus_t = 1.0 - t;
            let mut pt_fillet = pt_arm_right * (one_minus_t * one_minus_t)
                + corner_apex * (2.0 * one_minus_t * t)
                + pt_next_left * (t * t);

            if elevation_mode == JunctionElevationMode::Planar {
                pt_fillet.y = center_pos.y;
            } else if !is_bridge {
                let g = heightmap.sample(pt_fillet.x, pt_fillet.z);
                pt_fillet.y = pt_fillet.y.max(g + 0.12);
            }
            perimeter_pts.push(pt_fillet);
        }
    }

    let num_perim = perimeter_pts.len();
    if num_perim < 3 {
        return None;
    }

    // Add perimeter points to positions
    for pt in perimeter_pts.iter() {
        positions.push([pt.x, pt.y, pt.z]);
        normals.push([0.0, 1.0, 0.0]);
        let u = (0.5 + (pt.x - center_pos.x) / (2.0 * max_extent)).clamp(0.02, 0.98);
        let v = (0.5 + (pt.z - center_pos.z) / (2.0 * max_extent)).clamp(0.02, 0.98);
        uvs.push([u, v]);
        colors.push(deck_color);
    }

    // Triangulate asphalt surface with triangle fan from center (wound counter-clockwise for upward normal)
    for p in 0..num_perim {
        let p0 = 0;
        let p1 = 1 + p as u32;
        let p2 = 1 + ((p + 1) % num_perim) as u32;
        indices.push(p0);
        indices.push(p2);
        indices.push(p1);
    }

    // Subterranean / girder skirt dropping down ONLY along corner fillets (leaving road mouths open!)
    let skirt_color = if is_bridge {
        [0.45, 0.45, 0.48, 1.0]
    } else {
        [0.26, 0.32, 0.20, 1.0]
    };

    for i in 0..k {
        // Fillet arc vertices: pt_arm_right (6*i + 2) through 3 fillet points to pt_next_left ((6*(i+1)) % num_perim)
        let corner_deck_indices = [
            6 * i + 2,
            6 * i + 3,
            6 * i + 4,
            6 * i + 5,
            (6 * (i + 1)) % num_perim,
        ];

        let mut corner_skirt_indices = Vec::with_capacity(5);
        for &deck_p_idx in &corner_deck_indices {
            let pt = perimeter_pts[deck_p_idx];
            let outward =
                Vec3::new(pt.x - center_pos.x, 0.0, pt.z - center_pos.z).normalize_or_zero();
            let skirt_x = pt.x + outward.x * 3.8;
            let skirt_z = pt.z + outward.z * 3.8;
            let skirt_y = if is_bridge {
                pt.y - 1.4
            } else {
                heightmap.sample(skirt_x, skirt_z) - 0.75
            };

            let s_idx = positions.len() as u32;
            positions.push([skirt_x, skirt_y, skirt_z]);
            normals.push([outward.x, 0.2, outward.z]);
            let u = (0.5 + (skirt_x - center_pos.x) / (2.0 * max_extent)).clamp(0.0, 1.0);
            let v = (0.5 + (skirt_z - center_pos.z) / (2.0 * max_extent)).clamp(0.0, 1.0);
            uvs.push([u, v]);
            colors.push(skirt_color);
            corner_skirt_indices.push(s_idx);
        }

        // Quads connecting fillet deck arc to skirt
        for step in 0..4 {
            let d0 = 1 + corner_deck_indices[step] as u32;
            let d1 = 1 + corner_deck_indices[step + 1] as u32;
            let s0 = corner_skirt_indices[step];
            let s1 = corner_skirt_indices[step + 1];

            indices.push(d0);
            indices.push(s0);
            indices.push(d1);

            indices.push(d1);
            indices.push(s0);
            indices.push(s1);
        }
    }

    // Road entrance markings (Zebra pedestrian crossings, stop bars, yield lines)
    // placed exactly across each incoming road arm corridor
    for i in 0..k {
        let arm = &junction.connected_arms[i];
        let (pt_arm_left, pt_arm_center, pt_arm_right) = arm_mouth_pts[i];
        let d = Vec3::new(arm.dir.x, 0.0, arm.dir.z).normalize_or_zero();
        let inward = -d;
        let road_w = (pt_arm_right - pt_arm_left).length().max(2.0);

        let calc_marking_pt = |s: f32, t: f32| -> Vec3 {
            let pt_base = if t <= 0.5 {
                let f = t * 2.0;
                pt_arm_left * (1.0 - f) + pt_arm_center * f
            } else {
                let f = (t - 0.5) * 2.0;
                pt_arm_center * (1.0 - f) + pt_arm_right * f
            };
            let blend = (s / r).clamp(0.0, 1.0);
            let y_surface = pt_base.y * (1.0 - blend) + center_elev * blend;
            Vec3::new(
                pt_base.x + inward.x * s,
                y_surface + 0.018,
                pt_base.z + inward.z * s,
            )
        };

        let mut add_marking_quad = |s0: f32, s1: f32, t0: f32, t1: f32| {
            let c0 = calc_marking_pt(s0, t0);
            let c1 = calc_marking_pt(s1, t0);
            let c2 = calc_marking_pt(s1, t1);
            let c3 = calc_marking_pt(s0, t1);

            let base = positions.len() as u32;
            positions.push([c0.x, c0.y, c0.z]);
            positions.push([c1.x, c1.y, c1.z]);
            positions.push([c2.x, c2.y, c2.z]);
            positions.push([c3.x, c3.y, c3.z]);

            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            normals.push([0.0, 1.0, 0.0]);

            uvs.push([0.015, 0.015]);
            uvs.push([0.015, 0.015]);
            uvs.push([0.015, 0.015]);
            uvs.push([0.015, 0.015]);

            colors.push([1.0, 1.0, 1.0, 1.0]);
            colors.push([1.0, 1.0, 1.0, 1.0]);
            colors.push([1.0, 1.0, 1.0, 1.0]);
            colors.push([1.0, 1.0, 1.0, 1.0]);

            // CCW winding so normal faces UP
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 3);

            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base + 3);
        };

        match style {
            JunctionStyle::Continental => {
                // Zebra pedestrian crossing across full road width
                let margin = 0.40;
                let stripe_w = 0.45;
                let gap_w = 0.45;
                let period = stripe_w + gap_w;
                let avail_w = (road_w - 2.0 * margin).max(0.6);
                let num_stripes = ((avail_w + gap_w) / period).floor() as usize;
                let num_stripes = num_stripes.max(1);
                let total_stripes_w =
                    num_stripes as f32 * stripe_w + (num_stripes - 1) as f32 * gap_w;
                let start_w = margin + (avail_w - total_stripes_w) * 0.5;

                let crossing_len = (r * 0.35).clamp(2.4, 3.0);
                let s0 = 0.4;
                let s1 = s0 + crossing_len;

                for s_idx in 0..num_stripes {
                    let w0 = start_w + s_idx as f32 * period;
                    let w1 = w0 + stripe_w;
                    let t0 = (w0 / road_w).clamp(0.0, 1.0);
                    let t1 = (w1 / road_w).clamp(0.0, 1.0);
                    add_marking_quad(s0, s1, t0, t1);
                }

                // Solid white transverse stop bar across incoming traffic lane
                let stop_s0 = (s1 + 0.45).min(r - 0.8);
                let stop_s1 = (stop_s0 + 0.45).min(r - 0.35);
                let (stop_t0, stop_t1) = if road_w > 5.0 {
                    (0.52, 0.96)
                } else {
                    (0.15, 0.85)
                };
                if stop_s1 > stop_s0 {
                    add_marking_quad(stop_s0, stop_s1, stop_t0, stop_t1);
                }
            }
            JunctionStyle::BoxMarking => {
                // Solid white stop bar across incoming traffic lane
                let stop_s0: f32 = 0.5;
                let stop_s1 = (stop_s0 + 0.45).min(r - 0.5);
                let (stop_t0, stop_t1) = if road_w > 5.0 {
                    (0.52, 0.96)
                } else {
                    (0.15, 0.85)
                };
                if stop_s1 > stop_s0 {
                    add_marking_quad(stop_s0, stop_s1, stop_t0, stop_t1);
                }
            }
            JunctionStyle::TurningCircle => {
                // Dashed yield bar across incoming traffic lane
                let (stop_t0, stop_t1) = if road_w > 5.0 {
                    (0.52, 0.96)
                } else {
                    (0.15, 0.85)
                };
                let dash_len = (stop_t1 - stop_t0) / 5.0;
                let stop_s0: f32 = 0.5;
                let stop_s1 = (stop_s0 + 0.40).min(r - 0.5);
                for d_i in 0..3 {
                    let dt0 = stop_t0 + (d_i as f32 * 2.0) * dash_len;
                    let dt1 = (dt0 + dash_len).min(stop_t1);
                    if stop_s1 > stop_s0 {
                        add_marking_quad(stop_s0, stop_s1, dt0, dt1);
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    Some(mesh)
}

/// Smoothly transitions a road segment's elevation, banking, and slope
/// so it attaches flush to a planar horizontal junction deck.
pub fn smooth_attach_segment_to_junctions(
    segment: &mut [SplineSample],
    start_junc_y: Option<f32>,
    end_junc_y: Option<f32>,
) {
    let n = segment.len();
    if n < 2 {
        return;
    }

    let total_len = (segment[n - 1].distance - segment[0].distance)
        .abs()
        .max(0.1);
    let ramp_dist = 22.0f32.min(total_len * 0.45);

    // Smooth ramp connecting to start junction
    if let Some(y_start) = start_junc_y
        && ramp_dist > 0.5
    {
        segment[0].pos.y = y_start;
        segment[0].tangent =
            Vec3::new(segment[0].tangent.x, 0.0, segment[0].tangent.z).normalize_or_zero();
        segment[0].normal =
            Vec3::new(segment[0].normal.x, 0.0, segment[0].normal.z).normalize_or_zero();
        segment[0].binormal = Vec3::Y;
        segment[0].banking = 0.0;
        segment[0].grade = 0.0;

        let start_d = segment[0].distance;
        for s in segment.iter_mut().skip(1) {
            let d = s.distance - start_d;
            if d >= ramp_dist {
                break;
            }
            let u = (d / ramp_dist).clamp(0.0, 1.0);
            let w = 1.0 - (u * u * (3.0 - 2.0 * u));
            s.pos.y = s.pos.y * (1.0 - w) + y_start * w;
            s.banking *= 1.0 - w;
            s.tangent.y *= 1.0 - w;
            s.tangent = s.tangent.normalize_or_zero();
            s.normal.y *= 1.0 - w;
            s.normal = s.normal.normalize_or_zero();
            s.binormal = s.binormal.lerp(Vec3::Y, w).normalize_or_zero();
        }
    }

    // Smooth ramp connecting to end junction
    if let Some(y_end) = end_junc_y
        && ramp_dist > 0.5
    {
        segment[n - 1].pos.y = y_end;
        segment[n - 1].tangent =
            Vec3::new(segment[n - 1].tangent.x, 0.0, segment[n - 1].tangent.z).normalize_or_zero();
        segment[n - 1].normal =
            Vec3::new(segment[n - 1].normal.x, 0.0, segment[n - 1].normal.z).normalize_or_zero();
        segment[n - 1].binormal = Vec3::Y;
        segment[n - 1].banking = 0.0;
        segment[n - 1].grade = 0.0;

        let end_d = segment[n - 1].distance;
        for s in segment.iter_mut().rev().skip(1) {
            let d = end_d - s.distance;
            if d >= ramp_dist {
                break;
            }
            let u = (d / ramp_dist).clamp(0.0, 1.0);
            let w = 1.0 - (u * u * (3.0 - 2.0 * u));
            s.pos.y = s.pos.y * (1.0 - w) + y_end * w;
            s.banking *= 1.0 - w;
            s.tangent.y *= 1.0 - w;
            s.tangent = s.tangent.normalize_or_zero();
            s.normal.y *= 1.0 - w;
            s.normal = s.normal.normalize_or_zero();
            s.binormal = s.binormal.lerp(Vec3::Y, w).normalize_or_zero();
        }
    }
}

/// Splits a street's spline samples into contiguous segments that lie strictly outside junctions.
/// When approaching or leaving a junction, accurately calculates the boundary SplineSample at the junction radius
/// and attaches it to the junction arm, guaranteeing a 100% gapless, perfectly height-aligned seam.
pub fn split_street_samples(
    street_idx: usize,
    samples: &[SplineSample],
    junctions: &mut [Junction],
    elevation_mode: JunctionElevationMode,
) -> Vec<Vec<SplineSample>> {
    if samples.is_empty() {
        return Vec::new();
    }
    if junctions.is_empty() {
        return vec![samples.to_vec()];
    }

    let mut segments: Vec<Vec<SplineSample>> = Vec::new();
    let mut current_segment: Vec<SplineSample> = Vec::new();
    let mut current_start_junc_y: Option<f32> = None;

    let find_containing_junction = |pt: Vec3, juncs: &[Junction]| -> Option<(usize, f32)> {
        for (j_idx, j) in juncs.iter().enumerate() {
            let d = Vec2::new(pt.x - j.pos.x, pt.z - j.pos.z).length();
            if d < j.radius {
                return Some((j_idx, d));
            }
        }
        None
    };

    let attach_arm_sample = |junc: &mut Junction, sample: SplineSample| {
        let outward = Vec3::new(sample.pos.x - junc.pos.x, 0.0, sample.pos.z - junc.pos.z)
            .normalize_or_zero();
        let mut best_idx: Option<usize> = None;
        let mut best_dot = -2.0f32;

        for (arm_idx, arm) in junc.connected_arms.iter().enumerate() {
            if arm.street_idx == street_idx {
                let dot = arm.dir.dot(outward);
                if dot > best_dot {
                    best_dot = dot;
                    best_idx = Some(arm_idx);
                }
            }
        }

        if let Some(idx) = best_idx
            && best_dot > -0.2
        {
            junc.connected_arms[idx].mouth_sample = Some(sample);
        }
    };

    let n = samples.len();
    let mut i = 0;

    while i < n {
        let s_curr = samples[i];
        let curr_junc = find_containing_junction(s_curr.pos, junctions);

        if let Some((j_idx, _)) = curr_junc {
            // Road is entering or inside junction j_idx
            if !current_segment.is_empty() {
                let s_prev = *current_segment.last().unwrap();
                let j = &junctions[j_idx];
                let d_prev = Vec2::new(s_prev.pos.x - j.pos.x, s_prev.pos.z - j.pos.z).length();
                let d_curr = Vec2::new(s_curr.pos.x - j.pos.x, s_curr.pos.z - j.pos.z).length();
                let denom = d_prev - d_curr;
                let t = if denom.abs() > 1e-4 {
                    ((d_prev - j.radius) / denom).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let mut boundary_sample = lerp_spline_sample(&s_prev, &s_curr, t);
                let end_junc_y = if elevation_mode == JunctionElevationMode::Planar {
                    boundary_sample.pos.y = j.pos.y;
                    boundary_sample.tangent =
                        Vec3::new(boundary_sample.tangent.x, 0.0, boundary_sample.tangent.z)
                            .normalize_or_zero();
                    boundary_sample.normal =
                        Vec3::new(boundary_sample.normal.x, 0.0, boundary_sample.normal.z)
                            .normalize_or_zero();
                    boundary_sample.binormal = Vec3::Y;
                    boundary_sample.banking = 0.0;
                    boundary_sample.grade = 0.0;
                    Some(j.pos.y)
                } else {
                    None
                };

                current_segment.push(boundary_sample);

                if current_segment.len() >= 2 {
                    if elevation_mode == JunctionElevationMode::Planar {
                        smooth_attach_segment_to_junctions(
                            &mut current_segment,
                            current_start_junc_y,
                            end_junc_y,
                        );
                    }
                    segments.push(current_segment);
                }
                current_segment = Vec::new();
                current_start_junc_y = None;

                attach_arm_sample(&mut junctions[j_idx], boundary_sample);
            }

            // Skip samples until we exit this junction
            let mut next_i = i + 1;
            while next_i < n {
                let s_test = samples[next_i];
                let d_test = Vec2::new(
                    s_test.pos.x - junctions[j_idx].pos.x,
                    s_test.pos.z - junctions[j_idx].pos.z,
                )
                .length();
                if d_test >= junctions[j_idx].radius {
                    break;
                }
                next_i += 1;
            }

            if next_i < n {
                let s_in = samples[next_i - 1];
                let s_out = samples[next_i];
                let j = &junctions[j_idx];
                let d_in = Vec2::new(s_in.pos.x - j.pos.x, s_in.pos.z - j.pos.z).length();
                let d_out = Vec2::new(s_out.pos.x - j.pos.x, s_out.pos.z - j.pos.z).length();
                let denom = d_out - d_in;
                let t = if denom.abs() > 1e-4 {
                    ((j.radius - d_in) / denom).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let mut boundary_sample = lerp_spline_sample(&s_in, &s_out, t);
                if elevation_mode == JunctionElevationMode::Planar {
                    boundary_sample.pos.y = j.pos.y;
                    boundary_sample.tangent =
                        Vec3::new(boundary_sample.tangent.x, 0.0, boundary_sample.tangent.z)
                            .normalize_or_zero();
                    boundary_sample.normal =
                        Vec3::new(boundary_sample.normal.x, 0.0, boundary_sample.normal.z)
                            .normalize_or_zero();
                    boundary_sample.binormal = Vec3::Y;
                    boundary_sample.banking = 0.0;
                    boundary_sample.grade = 0.0;
                    current_start_junc_y = Some(j.pos.y);
                } else {
                    current_start_junc_y = None;
                }

                attach_arm_sample(&mut junctions[j_idx], boundary_sample);
                current_segment.push(boundary_sample);
            }

            i = next_i;
        } else {
            current_segment.push(s_curr);
            i += 1;
        }
    }

    if current_segment.len() >= 2 {
        if elevation_mode == JunctionElevationMode::Planar {
            smooth_attach_segment_to_junctions(&mut current_segment, current_start_junc_y, None);
        }
        segments.push(current_segment);
    }

    segments
}

/// Partitions street spline samples into contiguous segments that lie outside junctions.
pub fn split_samples_by_junctions(
    samples: &[SplineSample],
    junctions: &[Junction],
) -> Vec<Vec<SplineSample>> {
    let mut j_cloned = junctions.to_vec();
    split_street_samples(0, samples, &mut j_cloned, JunctionElevationMode::default())
}

/// Generates roadside delineators along road samples, excluding samples inside or near junctions
pub fn generate_road_posts_filtered(
    samples: &[SplineSample],
    junctions: &[Junction],
) -> Vec<(Vec3, Quat)> {
    let mut posts = Vec::new();
    let step = 10.0;
    let mut next_dist = 5.0;

    for s in samples.iter() {
        if s.distance >= next_dist {
            let in_junction = junctions.iter().any(|j| {
                let d2 = (s.pos.x - j.pos.x) * (s.pos.x - j.pos.x)
                    + (s.pos.z - j.pos.z) * (s.pos.z - j.pos.z);
                d2 < (j.radius * 1.15) * (j.radius * 1.15)
            });

            if !in_junction {
                let half_w = s.width * 0.5 + 0.5;
                let post_pos_left = s.pos - s.normal * half_w;
                let post_pos_right = s.pos + s.normal * half_w;

                let rot = Quat::from_rotation_arc(Vec3::Z, s.tangent);
                posts.push((post_pos_left, rot));
                posts.push((post_pos_right, rot));
            }

            next_dist += step;
        }
    }

    posts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_road_texture_lanes() {
        for &(width, lanes) in &[(4.0, 1), (8.0, 2), (15.0, 4), (6.5, 0)] {
            let img = create_road_texture(width, lanes);
            assert_eq!(img.texture_descriptor.size.width, 512);
            assert_eq!(img.texture_descriptor.size.height, 512);
            assert_eq!(img.data.as_ref().unwrap().len(), 512 * 512 * 4);
        }
    }

    #[test]
    fn test_detect_junctions_t_junction_and_crossroads() {
        let waypoints = vec![
            RoadWaypoint::new(Vec3::new(0.0, 0.0, -20.0), 8.0), // 0
            RoadWaypoint::new(Vec3::new(0.0, 0.0, 0.0), 8.0),   // 1 (Junction)
            RoadWaypoint::new(Vec3::new(0.0, 0.0, 20.0), 8.0),  // 2
            RoadWaypoint::new(Vec3::new(-20.0, 0.0, 0.0), 8.0), // 3
            RoadWaypoint::new(Vec3::new(20.0, 0.0, 0.0), 8.0),  // 4
        ];

        // 1. Street 1 (North-South) and Street 2 (West-East forming crossroads at node 1)
        let streets = vec![
            Street {
                id: 1,
                name: "Main St".to_string(),
                node_indices: vec![0, 1, 2],
                road_width: 8.0,
                lanes: 2,
            },
            Street {
                id: 2,
                name: "Cross St".to_string(),
                node_indices: vec![3, 1, 4],
                road_width: 8.0,
                lanes: 2,
            },
        ];

        let junctions = detect_junctions(&waypoints, &streets);
        assert_eq!(
            junctions.len(),
            1,
            "Should detect exactly one crossroads junction"
        );
        assert_eq!(junctions[0].node_idx, 1);
        assert_eq!(
            junctions[0].connected_arms.len(),
            4,
            "Crossroads junction must have 4 arms"
        );

        // 2. T-junction test: side street branches from node 1 eastward only
        let t_streets = vec![
            Street {
                id: 1,
                name: "Main St".to_string(),
                node_indices: vec![0, 1, 2],
                road_width: 8.0,
                lanes: 2,
            },
            Street {
                id: 2,
                name: "Side Spur".to_string(),
                node_indices: vec![1, 4],
                road_width: 8.0,
                lanes: 2,
            },
        ];

        let t_junctions = detect_junctions(&waypoints, &t_streets);
        assert_eq!(t_junctions.len(), 1, "Should detect exactly one T-junction");
        assert_eq!(t_junctions[0].node_idx, 1);
        assert_eq!(
            t_junctions[0].connected_arms.len(),
            3,
            "T-junction must have 3 arms"
        );
    }

    #[test]
    fn test_detect_junctions_joining_two_streets() {
        let waypoints = vec![
            RoadWaypoint::new(Vec3::new(-30.0, 0.0, 0.0), 8.0), // 0
            RoadWaypoint::new(Vec3::new(0.0, 0.0, 0.0), 8.0),   // 1 (Join node)
            RoadWaypoint::new(Vec3::new(30.0, 0.0, 0.0), 8.0),  // 2
        ];

        let streets = vec![
            Street {
                id: 1,
                name: "Street A".to_string(),
                node_indices: vec![0, 1],
                road_width: 15.0,
                lanes: 4,
            },
            Street {
                id: 2,
                name: "Street B".to_string(),
                node_indices: vec![1, 2],
                road_width: 8.0,
                lanes: 2,
            },
        ];

        let junctions = detect_junctions(&waypoints, &streets);
        assert_eq!(junctions.len(), 1, "Should detect a street join junction");
        assert_eq!(junctions[0].node_idx, 1);
        assert_eq!(junctions[0].connected_arms.len(), 2);
    }

    #[test]
    fn test_create_junction_texture() {
        for style in [
            JunctionStyle::BoxMarking,
            JunctionStyle::TurningCircle,
            JunctionStyle::Continental,
        ] {
            let img = create_junction_texture(style);
            assert_eq!(img.texture_descriptor.size.width, 512);
            assert_eq!(img.texture_descriptor.size.height, 512);
            assert_eq!(img.data.as_ref().unwrap().len(), 512 * 512 * 4);
        }
    }

    #[test]
    fn test_split_samples_by_junctions() {
        let dummy_junction = Junction {
            node_idx: 1,
            pos: Vec3::new(0.0, 0.0, 0.0),
            connected_arms: Vec::new(),
            radius: 8.0,
        };

        let mut samples = Vec::new();
        for x in -20..=20 {
            samples.push(SplineSample {
                pos: Vec3::new(x as f32, 0.0, 0.0),
                tangent: Vec3::X,
                normal: Vec3::Z,
                binormal: Vec3::Y,
                distance: (x + 20) as f32,
                grade: 0.0,
                banking: 0.0,
                width: 8.0,
            });
        }

        let segments = split_samples_by_junctions(&samples, &[dummy_junction]);
        assert_eq!(
            segments.len(),
            2,
            "Should split line crossing junction into 2 segments"
        );
        assert!(segments[0].last().unwrap().pos.x <= -7.0);
        assert!(segments[1].first().unwrap().pos.x >= 7.0);
    }
}
