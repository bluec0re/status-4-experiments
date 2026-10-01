use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RoadTier {
    Motorway,
    Primary,
    Secondary,
    Residential,
}

impl RoadTier {
    pub fn display_name(&self) -> &'static str {
        match self {
            RoadTier::Motorway => "Motorway & Trunk",
            RoadTier::Primary => "Primary Arterials",
            RoadTier::Secondary => "Secondary & Tertiary",
            RoadTier::Residential => "Residential & Local",
        }
    }

    pub fn default_width(&self) -> f32 {
        match self {
            RoadTier::Motorway => 15.0,
            RoadTier::Primary => 11.0,
            RoadTier::Secondary => 8.5,
            RoadTier::Residential => 6.2,
        }
    }

    pub fn default_lanes(&self) -> usize {
        match self {
            RoadTier::Motorway => 4,
            RoadTier::Primary => 3,
            RoadTier::Secondary => 2,
            RoadTier::Residential => 2,
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct OsmRoad {
    pub id: i64,
    pub name: String,
    pub highway_type: String,
    pub tier: RoadTier,
    pub lanes: usize,
    pub width: f32,
    pub points: Vec<Vec3>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct OsmBBox {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
    pub center_lat: f64,
    pub center_lon: f64,
    pub size_km: f64,
}

impl OsmBBox {
    pub fn from_center(lat: f64, lon: f64, size_km: f64) -> Self {
        let half_km = size_km * 0.5;
        let d_lat = half_km / 111.32;
        let cos_lat = lat.to_radians().cos().abs().max(0.1);
        let d_lon = half_km / (111.32 * cos_lat);

        Self {
            south: lat - d_lat,
            west: lon - d_lon,
            north: lat + d_lat,
            east: lon + d_lon,
            center_lat: lat,
            center_lon: lon,
            size_km,
        }
    }

    pub fn overpass_query(&self) -> String {
        format!(
            "[out:json][timeout:60];\n(\n  way[\"highway\"~\"^(motorway|trunk|primary|secondary|tertiary)\"]({:.5},{:.5},{:.5},{:.5});\n  way[\"waterway\"~\"^(river|canal|stream|dock)\"]({:.5},{:.5},{:.5},{:.5});\n  way[\"natural\"~\"^(water|coastline|bay|strait)\"]({:.5},{:.5},{:.5},{:.5});\n  way[\"water\"~\"^(lake|river|oxbow|canal|pond|reservoir|basin)\"]({:.5},{:.5},{:.5},{:.5});\n  way[\"landuse\"~\"^(reservoir|basin)\"]({:.5},{:.5},{:.5},{:.5});\n  relation[\"natural\"~\"^(water|coastline|bay|strait)\"]({:.5},{:.5},{:.5},{:.5});\n  relation[\"water\"~\"^(lake|river|oxbow|canal|pond|reservoir|basin)\"]({:.5},{:.5},{:.5},{:.5});\n  relation[\"waterway\"~\"^(river|canal|dock)\"]({:.5},{:.5},{:.5},{:.5});\n);\nout geom;\n",
            self.south, self.west, self.north, self.east,
            self.south, self.west, self.north, self.east,
            self.south, self.west, self.north, self.east,
            self.south, self.west, self.north, self.east,
            self.south, self.west, self.north, self.east,
            self.south, self.west, self.north, self.east,
            self.south, self.west, self.north, self.east,
            self.south, self.west, self.north, self.east,
        )
    }
}

#[derive(Clone, Debug)]
pub struct OsmWaterway {
    pub id: i64,
    pub name: String,
    pub waterway_type: String,
    pub width: f32,
    pub points: Vec<Vec3>,
}

#[derive(Clone, Default, Debug)]
pub struct ParsedOsmData {
    pub roads: Vec<OsmRoad>,
    pub waterways: Vec<OsmWaterway>,
}

#[derive(Clone, Debug)]
pub enum FetchStatus {
    Idle,
    Loading(String),
    Success {
        roads_count: usize,
        total_km: f32,
        city_name: String,
    },
    Error(String),
}

#[derive(Resource)]
pub struct OsmManager {
    pub current_bbox: OsmBBox,
    pub current_city_name: String,
    pub status: FetchStatus,
    pub roads: Vec<OsmRoad>,
    pub waterways: Vec<OsmWaterway>,
    pub dirty: bool,
    receiver: Mutex<Receiver<FetchResult>>,
    sender: Sender<FetchResult>,
}

#[allow(dead_code)]
pub enum FetchResult {
    Progress(String),
    Done {
        data: ParsedOsmData,
        city_name: String,
        bbox: OsmBBox,
    },
    Failed(String),
}

impl Default for OsmManager {
    fn default() -> Self {
        let (tx, rx) = channel();
        let default_lat = 52.5163;
        let default_lon = 13.3777;
        let bbox = OsmBBox::from_center(default_lat, default_lon, 10.0);

        let mut manager = Self {
            current_bbox: bbox,
            current_city_name: "Berlin".to_string(),
            status: FetchStatus::Idle,
            roads: Vec::new(),
            waterways: Vec::new(),
            dirty: true,
            receiver: Mutex::new(rx),
            sender: tx,
        };
        manager.load_offline_preset(default_lat, default_lon, "Berlin");
        manager
    }
}

impl OsmManager {
    pub fn trigger_fetch(&mut self, lat: f64, lon: f64, city_name: &str) {
        let bbox = OsmBBox::from_center(lat, lon, 10.0);
        self.current_bbox = bbox.clone();
        self.current_city_name = city_name.to_string();
        self.status = FetchStatus::Loading(format!(
            "Loading {}...",
            city_name
        ));

        let tx = self.sender.clone();
        let city = city_name.to_string();

        #[cfg(not(target_arch = "wasm32"))]
        {
            thread::spawn(move || {
                fetch_osm_in_background(bbox, city, tx);
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            fetch_preset_web(bbox, city, tx);
        }
    }

    pub fn load_offline_preset(&mut self, lat: f64, lon: f64, city_name: &str) {
        let bbox = OsmBBox::from_center(lat, lon, 10.0);
        self.current_bbox = bbox.clone();
        self.current_city_name = city_name.to_string();

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.load_offline_preset_desktop(lat, lon, city_name, bbox);
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.status = FetchStatus::Loading(format!("Fetching {} preset via HTTP...", city_name));
            let tx = self.sender.clone();
            fetch_preset_web(bbox, city_name.to_string(), tx);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_offline_preset_desktop(&mut self, lat: f64, lon: f64, city_name: &str, bbox: OsmBBox) {
        let slug = city_name.to_lowercase().replace(' ', "_");
        let path = format!("assets/presets/{}.json", slug);

        println!(
            "[load_offline_preset] Target: '{}' (lat: {:.4}, lon: {:.4}, slug: '{}')",
            city_name, lat, lon, slug
        );
        println!("[load_offline_preset] Checking disk path: '{}'...", path);

        let mut loaded_data = None;

        if let Ok(content) = std::fs::read_to_string(&path)
            && !content.contains("Beltway Ring Motorway")
        {
            println!(
                "[load_offline_preset] -> Read file '{}' ({} bytes)",
                path,
                content.len()
            );
            match parse_osm_json(&content, &bbox) {
                Ok(data) if !data.roads.is_empty() => {
                    println!(
                        "[load_offline_preset] -> Successfully parsed {} roads and {} waterways from '{}'",
                        data.roads.len(),
                        data.waterways.len(),
                        path
                    );
                    loaded_data = Some(data);
                }
                Ok(_) => {
                    println!("[load_offline_preset] -> Parsed 0 roads from '{}'", path);
                }
                Err(e) => {
                    println!(
                        "[load_offline_preset] -> Error parsing OSM JSON in '{}': {}",
                        path, e
                    );
                }
            }
        }

        if let Some(data) = loaded_data {
            let total_km: f32 = data.roads.iter().map(|r| road_length_km(&r.points)).sum();
            let count = data.roads.len();
            let water_count = data.waterways.len();
            println!(
                "[load_offline_preset] -> Loaded {} roads ({:.1} km) and {} waterways from disk cache for '{}'",
                count, total_km, water_count, city_name
            );
            self.roads = data.roads;
            self.waterways = data.waterways;
            self.dirty = true;
            self.status = FetchStatus::Success {
                roads_count: count,
                total_km,
                city_name: format!("{} (Cached)", city_name),
            };
        } else {
            println!(
                "[load_offline_preset] -> No valid real OSM cache found for '{}'. Triggering fetch from Overpass API...",
                city_name
            );
            self.trigger_fetch(lat, lon, city_name);
        }
    }
}

pub struct OsmPlugin;

impl Plugin for OsmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OsmManager>()
            .add_systems(Update, poll_osm_network_results);
    }
}

fn poll_osm_network_results(mut manager: ResMut<OsmManager>) {
    let mut finished = false;
    let mut new_data = ParsedOsmData::default();
    let mut final_city = String::new();
    let mut final_bbox = None;

    let messages: Vec<FetchResult> = if let Ok(rx) = manager.receiver.lock() {
        let mut msgs = Vec::new();
        while let Ok(res) = rx.try_recv() {
            msgs.push(res);
        }
        msgs
    } else {
        Vec::new()
    };

    for res in messages {
        match res {
            FetchResult::Progress(msg) => {
                println!("[poll_osm_network_results] Progress: {}", msg);
                manager.status = FetchStatus::Loading(msg);
            }
            FetchResult::Done {
                data,
                city_name,
                bbox,
            } => {
                println!(
                    "[poll_osm_network_results] Done: {} roads, {} waterways for '{}'",
                    data.roads.len(),
                    data.waterways.len(),
                    city_name
                );
                finished = true;
                new_data = data;
                final_city = city_name;
                final_bbox = Some(bbox);
            }
            FetchResult::Failed(err_msg) => {
                println!("[poll_osm_network_results] Failed: {}", err_msg);
                manager.status = FetchStatus::Error(err_msg);
            }
        }
    }

    if finished {
        let total_km: f32 = new_data.roads.iter().map(|r| road_length_km(&r.points)).sum();
        let count = new_data.roads.len();
        println!(
            "[poll_osm_network_results] Setting manager: {} roads, {} waterways, dirty: true for '{}'",
            count,
            new_data.waterways.len(),
            final_city
        );
        manager.roads = new_data.roads;
        manager.waterways = new_data.waterways;
        manager.current_city_name = final_city.clone();
        if let Some(bbox) = final_bbox {
            manager.current_bbox = bbox;
        }
        manager.dirty = true;
        manager.status = FetchStatus::Success {
            roads_count: count,
            total_km,
            city_name: final_city,
        };
    }
}

fn road_length_km(pts: &[Vec3]) -> f32 {
    let mut len = 0.0f32;
    for i in 1..pts.len() {
        len += (pts[i] - pts[i - 1]).length();
    }
    len / 1000.0
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_osm_in_background(bbox: OsmBBox, city_name: String, tx: Sender<FetchResult>) {
    println!(
        "[fetch_osm_in_background] Starting fetch for '{}' ({:.4}, {:.4})...",
        city_name, bbox.center_lat, bbox.center_lon
    );
    let _ = tx.send(FetchResult::Progress(format!(
        "Querying Overpass for 10x10km area around {} ({:.4}, {:.4})...",
        city_name, bbox.center_lat, bbox.center_lon
    )));

    let query = bbox.overpass_query();
    let endpoints = [
        "https://overpass-api.de/api/interpreter",
        "https://lz4.overpass-api.de/api/interpreter",
        "https://z.overpass-api.de/api/interpreter",
        "https://overpass.private.coffee/api/interpreter",
        "https://overpass.kumi.systems/api/interpreter",
        "https://maps.mail.ru/osm/tools/overpass/api/interpreter",
        "https://overpass.osm.ch/api/interpreter",
        "https://overpass.openstreetmap.ru/api/interpreter",
        "https://overpass.nchc.org.tw/api/interpreter",
    ];

    let mut response_body: Option<String> = None;

    {
        let agent = ureq::builder()
            .timeout_connect(std::time::Duration::from_secs(8))
            .timeout_read(std::time::Duration::from_secs(90))
            .build();

        for endpoint in endpoints {
            println!("[fetch_osm_in_background] Connecting to {}...", endpoint);
            let _ = tx.send(FetchResult::Progress(format!(
                "Connecting to {}...",
                endpoint
            )));
            let res = agent
                .post(endpoint)
                .set(
                    "User-Agent",
                    "status-4-city/0.1.0 (Bevy 3D Spline Experiments)",
                )
                .send_form(&[("data", query.as_str())]);

            match res {
                Ok(resp) => {
                    println!(
                        "[fetch_osm_in_background] Endpoint {} responded with HTTP {}",
                        endpoint,
                        resp.status()
                    );
                    if resp.status() == 200 {
                        let mut body = String::new();
                        let mut reader = resp.into_reader();
                        match reader.read_to_string(&mut body) {
                            Ok(_) => {
                                println!(
                                    "[fetch_osm_in_background] Read {} bytes body from {}",
                                    body.len(),
                                    endpoint
                                );
                                response_body = Some(body);
                                break;
                            }
                            Err(e) => {
                                println!(
                                    "[fetch_osm_in_background] Error reading body stream from {}: {}",
                                    endpoint, e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "[fetch_osm_in_background] Request to {} failed: {}",
                        endpoint, e
                    );
                }
            }
        }
    }

    let json_text = match response_body {
        Some(text) => text,
        None => {
            println!(
                "[fetch_osm_in_background] All endpoints failed for '{}'",
                city_name
            );
            let _ = tx.send(FetchResult::Failed(
                "Overpass servers busy or offline. Please retry fetching.".to_string(),
            ));
            return;
        }
    };

    println!(
        "[fetch_osm_in_background] Parsing OSM JSON ({} bytes)...",
        json_text.len()
    );
    let _ = tx.send(FetchResult::Progress(
        "Parsing OSM elements and extracting spline geometry...".to_string(),
    ));

    match parse_osm_json(&json_text, &bbox) {
        Ok(data) if !data.roads.is_empty() => {
            println!(
                "[fetch_osm_in_background] Successfully parsed {} roads and {} waterways! Saving to disk...",
                data.roads.len(),
                data.waterways.len(),
            );
            save_preset_to_disk(&city_name, &json_text);
            let _ = tx.send(FetchResult::Done {
                data,
                city_name,
                bbox,
            });
        }
        Ok(_) => {
            println!(
                "[fetch_osm_in_background] Parsed 0 highway ways for '{}'",
                city_name
            );
            let _ = tx.send(FetchResult::Failed(
                "No highway ways returned for this bounding box.".to_string(),
            ));
        }
        Err(err) => {
            println!(
                "[fetch_osm_in_background] Parser error on '{}': {}",
                city_name, err
            );
            let _ = tx.send(FetchResult::Failed(format!("Parser error: {}", err)));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn fetch_preset_web(
    bbox: OsmBBox,
    city_name: String,
    tx: Sender<FetchResult>,
) {
    use wasm_bindgen::JsCast;

    let slug = city_name.to_lowercase().replace(' ', "_");
    let url = format!("assets/presets/{}.json", slug);

    wasm_bindgen_futures::spawn_local(async move {
        let _ = tx.send(FetchResult::Progress(format!(
            "Loading {} preset via HTTP...",
            city_name
        )));

        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                let _ = tx.send(FetchResult::Failed("No browser window found".to_string()));
                return;
            }
        };

        let resp_value = match wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&url)).await {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(FetchResult::Failed(format!("Failed to fetch '{}': {:?}", url, e)));
                return;
            }
        };

        let resp: web_sys::Response = match resp_value.dyn_into() {
            Ok(r) => r,
            Err(_) => {
                let _ = tx.send(FetchResult::Failed("Failed to cast HTTP response".to_string()));
                return;
            }
        };

        if !resp.ok() {
            let _ = tx.send(FetchResult::Failed(format!(
                "HTTP {} error when fetching '{}'",
                resp.status(),
                url
            )));
            return;
        }

        let text_promise = match resp.text() {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.send(FetchResult::Failed(format!("Failed to read response body: {:?}", e)));
                return;
            }
        };

        let text_value = match wasm_bindgen_futures::JsFuture::from(text_promise).await {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(FetchResult::Failed(format!("Failed to resolve response text: {:?}", e)));
                return;
            }
        };

        let content = text_value.as_string().unwrap_or_default();
        if content.is_empty() {
            let _ = tx.send(FetchResult::Failed(format!("Empty preset returned from '{}'", url)));
            return;
        }

        let _ = tx.send(FetchResult::Progress(format!(
            "Parsing {} preset ({} bytes)...",
            city_name,
            content.len()
        )));

        match parse_osm_json(&content, &bbox) {
            Ok(data) if !data.roads.is_empty() => {
                let _ = tx.send(FetchResult::Done {
                    data,
                    city_name,
                    bbox,
                });
            }
            Ok(_) => {
                let _ = tx.send(FetchResult::Failed(format!(
                    "Parsed 0 roads from preset '{}'",
                    url
                )));
            }
            Err(e) => {
                let _ = tx.send(FetchResult::Failed(format!(
                    "Error parsing OSM preset for '{}': {}",
                    city_name, e
                )));
            }
        }
    });
}


// ============================================================================
// OSM JSON PARSER
// ============================================================================

#[derive(Deserialize, Serialize, Clone)]
pub struct RawOsmResponse {
    #[serde(default)]
    pub elements: Vec<RawOsmElement>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RawRelationMember {
    #[serde(rename = "type")]
    pub member_type: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub geometry: Option<Vec<RawLatLon>>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum RawOsmElement {
    #[serde(rename = "node")]
    Node { id: i64, lat: f64, lon: f64 },
    #[serde(rename = "way")]
    Way {
        id: i64,
        #[serde(default)]
        nodes: Vec<i64>,
        #[serde(default)]
        geometry: Option<Vec<RawLatLon>>,
        #[serde(default)]
        tags: Option<HashMap<String, String>>,
    },
    #[serde(rename = "relation")]
    Relation {
        id: i64,
        #[serde(default)]
        members: Vec<RawRelationMember>,
        #[serde(default)]
        tags: Option<HashMap<String, String>>,
    },
    #[serde(other)]
    Ignored,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct RawLatLon {
    pub lat: f64,
    pub lon: f64,
}

#[allow(dead_code)]
pub fn roads_to_osm_json(roads: &[OsmRoad], bbox: &OsmBBox) -> Result<String, String> {
    let r_earth = 6_371_000.0;
    let cos_center_lat = bbox.center_lat.to_radians().cos();
    let mut elements = Vec::with_capacity(roads.len());

    for road in roads {
        let mut geometry = Vec::with_capacity(road.points.len());
        for pt in &road.points {
            let d_lon_rad = (pt.x as f64) / (r_earth * cos_center_lat);
            let d_lat_rad = (-pt.z as f64) / r_earth;
            let lat = bbox.center_lat + d_lat_rad.to_degrees();
            let lon = bbox.center_lon + d_lon_rad.to_degrees();
            geometry.push(RawLatLon { lat, lon });
        }

        let mut tags = HashMap::new();
        tags.insert("highway".to_string(), road.highway_type.clone());
        tags.insert("name".to_string(), road.name.clone());
        tags.insert("lanes".to_string(), road.lanes.to_string());
        tags.insert("width".to_string(), format!("{:.1}", road.width));

        elements.push(RawOsmElement::Way {
            id: road.id,
            nodes: Vec::new(),
            geometry: Some(geometry),
            tags: Some(tags),
        });
    }

    let response = RawOsmResponse { elements };
    serde_json::to_string_pretty(&response).map_err(|e| e.to_string())
}

pub fn parse_osm_json(json_str: &str, bbox: &OsmBBox) -> Result<ParsedOsmData, String> {
    let parsed: RawOsmResponse = serde_json::from_str(json_str).map_err(|e| e.to_string())?;

    let mut node_coords: HashMap<i64, (f64, f64)> = HashMap::new();
    let mut ways = Vec::new();
    let mut relations = Vec::new();

    for elem in parsed.elements {
        match elem {
            RawOsmElement::Node { id, lat, lon } => {
                node_coords.insert(id, (lat, lon));
            }
            RawOsmElement::Way {
                id,
                nodes,
                geometry,
                tags,
            } => {
                ways.push((id, nodes, geometry, tags.unwrap_or_default()));
            }
            RawOsmElement::Relation {
                id,
                members,
                tags,
            } => {
                relations.push((id, members, tags.unwrap_or_default()));
            }
            RawOsmElement::Ignored => {}
        }
    }

    let mut roads = Vec::with_capacity(ways.len());
    let mut waterways = Vec::new();
    let r_earth = 6_371_000.0;
    let cos_center_lat = bbox.center_lat.to_radians().cos();

    for (id, node_ids, geometry, tags) in ways {
        let raw_pts: Vec<(f64, f64)> = if let Some(geom) = geometry {
            geom.into_iter().map(|g| (g.lat, g.lon)).collect()
        } else {
            node_ids
                .into_iter()
                .filter_map(|nid| node_coords.get(&nid).copied())
                .collect()
        };

        if raw_pts.len() < 2 {
            continue;
        }

        if let Some(highway_type) = tags.get("highway") {
            if let Some(road) = parse_road_way(id, highway_type, &tags, &raw_pts, bbox, r_earth, cos_center_lat) {
                roads.push(road);
            }
        } else if let Some(water_type) = tags
            .get("waterway")
            .or_else(|| tags.get("natural"))
            .or_else(|| tags.get("water"))
            .or_else(|| tags.get("landuse"))
        {
            if let Some(waterway) = parse_waterway_way(id, water_type, &tags, &raw_pts, bbox, r_earth, cos_center_lat) {
                waterways.push(waterway);
            }
        }
    }

    // Process lake / sea / waterway multipolygon relations
    for (rel_id, members, tags) in relations {
        if let Some(water_type) = tags
            .get("waterway")
            .or_else(|| tags.get("natural"))
            .or_else(|| tags.get("water"))
            .or_else(|| tags.get("landuse"))
        {
            for (idx, member) in members.into_iter().enumerate() {
                if (member.role == "outer" || member.role.is_empty())
                    && let Some(geom) = member.geometry
                {
                    let raw_pts: Vec<(f64, f64)> = geom.into_iter().map(|g| (g.lat, g.lon)).collect();
                    if raw_pts.len() >= 2 {
                        let member_id = rel_id.wrapping_mul(1000).wrapping_add(idx as i64);
                        if let Some(waterway) = parse_waterway_way(member_id, water_type, &tags, &raw_pts, bbox, r_earth, cos_center_lat) {
                            waterways.push(waterway);
                        }
                    }
                }
            }
        }
    }

    Ok(ParsedOsmData { roads, waterways })
}

fn parse_road_way(
    id: i64,
    highway_type: &str,
    tags: &HashMap<String, String>,
    raw_pts: &[(f64, f64)],
    bbox: &OsmBBox,
    r_earth: f64,
    cos_center_lat: f64,
) -> Option<OsmRoad> {
    let tier = match highway_type {
        "motorway" | "motorway_link" | "trunk" | "trunk_link" => RoadTier::Motorway,
        "primary" | "primary_link" => RoadTier::Primary,
        "secondary" | "secondary_link" | "tertiary" | "tertiary_link" => RoadTier::Secondary,
        _ => RoadTier::Residential,
    };

    let lanes: usize = tags
        .get("lanes")
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| tier.default_lanes());

    let width: f32 = tags
        .get("width")
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| tier.default_width());

    let layer: i32 = tags.get("layer").and_then(|v| v.parse().ok()).unwrap_or(0);
    let is_bridge = tags.get("bridge").map(|v| v == "yes").unwrap_or(false);
    let height_y = if is_bridge || layer > 0 {
        (layer.max(1) as f32) * 5.0
    } else {
        0.0
    };

    let mut points = Vec::with_capacity(raw_pts.len());
    for &(lat, lon) in raw_pts {
        let d_lat = (lat - bbox.center_lat).to_radians();
        let d_lon = (lon - bbox.center_lon).to_radians();
        let x = (r_earth * d_lon * cos_center_lat) as f32;
        let z = (-r_earth * d_lat) as f32; // North is -Z
        points.push(Vec3::new(x, height_y, z));
    }

    let simplified = simplify_spline_points(points, 3.5);
    if simplified.len() < 2 {
        return None;
    }

    let name = tags
        .get("name")
        .cloned()
        .unwrap_or_else(|| format!("{} #{}", tier.display_name(), id));

    Some(OsmRoad {
        id,
        name,
        highway_type: highway_type.to_string(),
        tier,
        lanes,
        width,
        points: simplified,
    })
}

fn parse_waterway_way(
    id: i64,
    water_type: &str,
    tags: &HashMap<String, String>,
    raw_pts: &[(f64, f64)],
    bbox: &OsmBBox,
    r_earth: f64,
    cos_center_lat: f64,
) -> Option<OsmWaterway> {
    let width = match water_type {
        "coastline" | "sea" | "ocean" => 55.0,
        "bay" | "strait" => 50.0,
        "lake" | "reservoir" | "basin" | "dock" | "water" => 45.0,
        "river" => 35.0,
        "canal" => 20.0,
        "stream" | "pond" => 10.0,
        _ => 20.0,
    };

    let mut points = Vec::with_capacity(raw_pts.len());
    for &(lat, lon) in raw_pts {
        let d_lat = (lat - bbox.center_lat).to_radians();
        let d_lon = (lon - bbox.center_lon).to_radians();
        let x = (r_earth * d_lon * cos_center_lat) as f32;
        let z = (-r_earth * d_lat) as f32;
        points.push(Vec3::new(x, 0.0, z));
    }

    let simplified = simplify_spline_points(points, 4.0);
    if simplified.len() < 2 {
        return None;
    }

    let name = tags
        .get("name")
        .cloned()
        .unwrap_or_else(|| format!("Waterway #{}", id));

    Some(OsmWaterway {
        id,
        name,
        waterway_type: water_type.to_string(),
        width,
        points: simplified,
    })
}

fn simplify_spline_points(pts: Vec<Vec3>, min_dist: f32) -> Vec<Vec3> {
    if pts.len() <= 2 {
        return pts;
    }

    let min_dist_sq = min_dist * min_dist;
    let mut result = Vec::with_capacity(pts.len());
    result.push(pts[0]);

    for i in 1..(pts.len() - 1) {
        let last = result.last().unwrap();
        if (pts[i] - *last).length_squared() >= min_dist_sq {
            result.push(pts[i]);
        }
    }

    // Always include end point
    if let Some(&last_pt) = pts.last()
        && let Some(prev) = result.last()
        && (*prev - last_pt).length_squared() > 1e-3
    {
        result.push(last_pt);
    }

    result
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_preset_to_disk(city_name: &str, json_text: &str) {
    let slug = city_name.to_lowercase().replace(' ', "_");
    let asset_paths = [
        format!("assets/presets/{}.json", slug),
    ];
    for path_str in &asset_paths {
        let path = std::path::Path::new(path_str);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(path, json_text) {
            Ok(_) => {
                println!(
                    "[save_preset_to_disk] Successfully cached {} bytes to '{}'",
                    json_text.len(),
                    path_str
                );
            }
            Err(e) => {
                println!(
                    "[save_preset_to_disk] Error writing cache to '{}': {}",
                    path_str, e
                );
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn save_preset_to_disk(_city_name: &str, _json_text: &str) {}

#[allow(dead_code)]
pub fn generate_synthetic_city_network(_size_m: f32, _seed_name: &str) -> Vec<OsmRoad> {
    Vec::new()
}
