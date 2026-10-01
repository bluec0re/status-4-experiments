#[cfg(not(target_arch = "wasm32"))]
use status_4_city::osm::{OsmBBox, parse_osm_json};
#[cfg(not(target_arch = "wasm32"))]
use status_4_city::presets::CITY_PRESETS;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("============================================================");
    println!("Status 4 City: Bundling 10x10km Presets for Web Deployment");
    println!("============================================================");

    // Determine output directory: assets/presets
    let target_dirs = [PathBuf::from("assets/presets")];

    for dir in &target_dirs {
        let _ = fs::create_dir_all(dir);
    }

    println!("Target bundle directory: assets/presets");

    let args: Vec<String> = std::env::args().collect();
    let mut city_filter: Option<String> = None;

    let mut arg_idx = 1;
    while arg_idx < args.len() {
        match args[arg_idx].as_str() {
            "-h" | "--help" => {
                println!("Usage: bundle_presets [OPTIONS] [CITY_NAME]");
                println!();
                println!("Options:");
                println!("  -c, --city <NAME>    Bundle only presets matching NAME (e.g. 'New York' or 'new_york')");
                println!("  -h, --help           Print help");
                println!();
                println!("Available presets:");
                for p in CITY_PRESETS {
                    println!("  - {} ({})", p.name, p.name.to_lowercase().replace(' ', "_"));
                }
                return;
            }
            "-c" | "--city" | "--preset" => {
                if arg_idx + 1 < args.len() {
                    city_filter = Some(args[arg_idx + 1].clone());
                    arg_idx += 1;
                }
            }
            arg if !arg.starts_with('-') => {
                city_filter = Some(arg.to_string());
            }
            _ => {}
        }
        arg_idx += 1;
    }

    let presets_to_bundle: Vec<_> = if let Some(filter) = &city_filter {
        let needle = filter.to_lowercase().replace([' ', '_', '-'], "");
        let filtered: Vec<_> = CITY_PRESETS
            .iter()
            .filter(|p| {
                let p_slug = p.name.to_lowercase().replace([' ', '_', '-'], "");
                p_slug.contains(&needle)
            })
            .collect();
        if filtered.is_empty() {
            eprintln!("Error: No preset found matching '{}'.", filter);
            println!("Available presets:");
            for p in CITY_PRESETS {
                println!("  - {} ({})", p.name, p.name.to_lowercase().replace(' ', "_"));
            }
            std::process::exit(1);
        }
        println!("Filtering presets by '{}' -> matched {} preset(s)", filter, filtered.len());
        filtered
    } else {
        CITY_PRESETS.iter().collect()
    };

    let mut success_count = 0;
    let total = presets_to_bundle.len();

    // Read existing manifest if present to preserve other bundled cities
    let manifest_path = target_dirs[0].join("manifest.json");
    let mut manifest_map: std::collections::BTreeMap<String, serde_json::Value> = std::collections::BTreeMap::new();
    if let Ok(content) = fs::read_to_string(&manifest_path)
        && let Ok(existing) = serde_json::from_str::<Vec<serde_json::Value>>(&content)
    {
        for item in existing {
            if let Some(slug) = item.get("slug").and_then(|s| s.as_str()) {
                manifest_map.insert(slug.to_string(), item);
            }
        }
    }

    for (i, preset) in presets_to_bundle.iter().enumerate() {
        let slug = preset.name.to_lowercase().replace(' ', "_");

        println!(
            "[{}/{}] Bundling {} ({:.4}, {:.4})...",
            i + 1,
            total,
            preset.name,
            preset.lat,
            preset.lon
        );

        let bbox = OsmBBox::from_center(preset.lat, preset.lon, 10.0);
        let mut bundled_json = None;

        // 1. Try querying Overpass API
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

        let agent = ureq::builder()
            .timeout_connect(std::time::Duration::from_secs(8))
            .timeout_read(std::time::Duration::from_secs(90))
            .build();

        for endpoint in endpoints {
            println!("  -> Trying {}...", endpoint);
            let res = agent
                .post(endpoint)
                .set(
                    "User-Agent",
                    "status-4-city-bundler/0.1.0 (Bevy 3D Spline Experiments)",
                )
                .send_form(&[("data", query.as_str())]);

            match res {
                Ok(resp) if resp.status() == 200 => {
                    let mut body = String::new();
                    let mut reader = resp.into_reader();
                    if let Err(e) = reader.read_to_string(&mut body) {
                        eprintln!("  -> Warning: Failed to read response stream: {}", e);
                        continue;
                    }
                    match parse_osm_json(&body, &bbox) {
                        Ok(data) if !data.roads.is_empty() => {
                            println!(
                                "  -> Fetched {} real OSM roads and {} waterways from {}",
                                data.roads.len(),
                                data.waterways.len(),
                                endpoint
                            );
                            bundled_json = Some(body);
                            break;
                        }
                        Ok(_) => {
                            eprintln!("  -> Warning: 0 roads parsed from {}", endpoint);
                        }
                        Err(err) => {
                            let preview = if body.len() > 300 {
                                &body[..300]
                            } else {
                                &body
                            };
                            eprintln!(
                                "  -> Warning: Parse error on {}: {}\n     Preview: {}",
                                endpoint, err, preview
                            );
                        }
                    }
                }
                Ok(resp) => {
                    eprintln!("  -> HTTP {} from {}", resp.status(), endpoint);
                }
                Err(e) => {
                    eprintln!("  -> Connection error from {}: {}", endpoint, e);
                }
            }
        }

        let Some(final_json) = bundled_json else {
            eprintln!(
                "  -> Warning: Could not fetch real OSM data for {}. Skipping bundling.",
                preset.name
            );
            continue;
        };

        let size_kb = final_json.len() as f32 / 1024.0;
        let mut wrote_any = false;

        for dir in &target_dirs {
            let out_file = dir.join(format!("{}.json", slug));
            if fs::write(&out_file, &final_json).is_ok() {
                wrote_any = true;
            }
        }

        if wrote_any {
            println!("  -> Saved {}.json ({:.1} KB)", slug, size_kb);
            success_count += 1;

            manifest_map.insert(
                slug.clone(),
                serde_json::json!({
                    "name": preset.name,
                    "country": preset.country,
                    "slug": slug,
                    "lat": preset.lat,
                    "lon": preset.lon,
                    "description": preset.description,
                    "file": format!("{}.json", slug),
                    "size_bytes": final_json.len(),
                }),
            );
        }

        // Polite delay between requests to avoid Overpass rate-limiting
        std::thread::sleep(std::time::Duration::from_millis(600));
    }

    // Write updated manifest index
    let all_manifest_items: Vec<_> = manifest_map.into_values().collect();
    if let Ok(manifest_str) = serde_json::to_string_pretty(&all_manifest_items) {
        for dir in &target_dirs {
            let manifest_path = dir.join("manifest.json");
            let _ = fs::write(&manifest_path, &manifest_str);
        }
        println!("  -> Updated bundle manifest ({} total cities) in target directories", all_manifest_items.len());
    }

    println!("============================================================");
    println!(
        "Successfully bundled {}/{} presets into target directories",
        success_count, total
    );
    println!("============================================================");
}
