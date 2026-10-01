#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CityPreset {
    pub name: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub description: &'static str,
}

pub const CITY_PRESETS: &[CityPreset] = &[
    CityPreset {
        name: "Berlin",
        country: "Germany",
        lat: 52.5163,
        lon: 13.3777,
        description: "Brandenburg Gate, Unter den Linden & Tiergarten",
    },
    CityPreset {
        name: "Paris",
        country: "France",
        lat: 48.8738,
        lon: 2.2950,
        description: "Arc de Triomphe, Champs-Élysées & Haussmann boulevards",
    },
    CityPreset {
        name: "New York",
        country: "USA",
        lat: 40.7580,
        lon: -73.9855,
        description: "Manhattan grid, Broadway diagonal & FDR Drive",
    },
    CityPreset {
        name: "London",
        country: "UK",
        lat: 51.5007,
        lon: -0.1246,
        description: "Westminster, Thames bridges & historic city streets",
    },
    CityPreset {
        name: "Tokyo",
        country: "Japan",
        lat: 35.6895,
        lon: 139.6917,
        description: "Shinjuku, high-density expressways & arterial ring roads",
    },
    CityPreset {
        name: "San Francisco",
        country: "USA",
        lat: 37.7879,
        lon: -122.4075,
        description: "Market Street, Embarcadero & steep grid avenues",
    },
    CityPreset {
        name: "Rome",
        country: "Italy",
        lat: 41.8902,
        lon: 12.4922,
        description: "Colosseum, Tiber riverbanks & ancient radial roads",
    },
    CityPreset {
        name: "Zurich HB",
        country: "Switzerland",
        lat: 47.3779,
        lon: 8.5403,
        description: "Zürich Hauptbahnhof, Limmat riverbanks & Bahnhofstrasse",
    },
];
