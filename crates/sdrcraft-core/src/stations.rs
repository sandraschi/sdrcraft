//! Seeded frequency database: the 11 legacy stations (LW/MW/SW/VHF) with
//! program schedules, ported 1:1 from `frequency_db.py`.
//!
//! Pure data + pure queries — no I/O, no clock inside the query core
//! (`current_program_at` takes weekday/minutes so tests are deterministic;
//! `current_program_now` reads `chrono::Local` like `datetime.now()` did).

use chrono::Timelike;
use serde_json::{json, Value};

/// Broadcast band, wire values verbatim (`LW`, `MW`, `SW`, `VHF`, `UHF`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    LW,
    MW,
    SW,
    VHF,
    UHF,
}

impl Band {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "LW" | "LONGWAVE" => Some(Self::LW),
            "MW" | "MEDIUMWAVE" => Some(Self::MW),
            "SW" | "SHORTWAVE" => Some(Self::SW),
            "VHF" => Some(Self::VHF),
            "UHF" => Some(Self::UHF),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LW => "LW",
            Self::MW => "MW",
            Self::SW => "SW",
            Self::VHF => "VHF",
            Self::UHF => "UHF",
        }
    }

    pub fn range(&self) -> &'static str {
        match self {
            Self::LW => "150-300 kHz",
            Self::MW => "530-1700 kHz",
            Self::SW => "3-30 MHz",
            Self::VHF => "30-300 MHz",
            Self::UHF => "300-3000 MHz",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationKind {
    Public,
    Commercial,
    Community,
    Religious,
    Educational,
    Military,
    International,
}

impl StationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Commercial => "commercial",
            Self::Community => "community",
            Self::Religious => "religious",
            Self::Educational => "educational",
            Self::Military => "military",
            Self::International => "international",
        }
    }
}

/// One program entry. Times are (hour, minute) UTC; days are lowercase
/// English names exactly as in the legacy DB.
#[derive(Debug, Clone, Copy)]
pub struct Program {
    pub name: &'static str,
    pub description: &'static str,
    pub start: (u8, u8),
    pub end: (u8, u8),
    pub days: &'static [&'static str],
    pub language: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Station {
    pub name: &'static str,
    pub callsign: &'static str,
    pub frequency_hz: f64,
    pub band: Band,
    pub country: &'static str,
    pub city: &'static str,
    pub kind: StationKind,
    pub power_kw: u32,
    pub language: &'static str,
    pub website: Option<&'static str>,
    pub description: &'static str,
    pub programs: &'static [Program],
}

impl Station {
    pub fn frequency_mhz(&self) -> f64 {
        self.frequency_hz / 1e6
    }
    pub fn frequency_khz(&self) -> f64 {
        self.frequency_hz / 1e3
    }
}

const WEEKDAYS: &[&str] = &["monday", "tuesday", "wednesday", "thursday", "friday"];

/// The 11 seeded stations, band-grouped exactly like `_load_database`.
pub static STATIONS: &[Station] = &[
    Station {
        name: "BBC Radio 4",
        callsign: "BBC LW",
        frequency_hz: 198000.0,
        band: Band::LW,
        country: "United Kingdom",
        city: "Droitwich",
        kind: StationKind::Public,
        power_kw: 500,
        language: "English",
        website: Some("https://www.bbc.co.uk/radio4"),
        description: "BBC's flagship speech radio station featuring news, drama, comedy, and cultural programming",
        programs: &[
            Program { name: "Today", description: "News and current affairs", start: (6, 0), end: (9, 0), days: WEEKDAYS, language: "English" },
            Program { name: "Book of the Week", description: "Literary discussion", start: (9, 45), end: (10, 0), days: WEEKDAYS, language: "English" },
            Program { name: "The World at One", description: "Lunchtime news", start: (13, 0), end: (13, 45), days: WEEKDAYS, language: "English" },
            Program { name: "PM", description: "Evening news and analysis", start: (17, 0), end: (18, 0), days: WEEKDAYS, language: "English" },
            Program { name: "The Archers", description: "Long-running radio soap opera", start: (19, 15), end: (19, 45), days: WEEKDAYS, language: "English" },
        ],
    },
    Station {
        name: "ORF Radio Österreich 1",
        callsign: "ORF LW",
        frequency_hz: 198000.0,
        band: Band::LW,
        country: "Austria",
        city: "Moosbrunn",
        kind: StationKind::Public,
        power_kw: 100,
        language: "German",
        website: Some("https://orf.at"),
        description: "ORF's flagship cultural and information station with classical music and comprehensive news",
        programs: &[
            Program { name: "Ö1 Morgenjournal", description: "Morning news", start: (5, 0), end: (9, 0), days: WEEKDAYS, language: "German" },
            Program { name: "Ö1 Mittagsjournal", description: "Lunch news", start: (12, 0), end: (13, 0), days: WEEKDAYS, language: "German" },
            Program { name: "Ö1 Abendjournal", description: "Evening news", start: (18, 30), end: (19, 30), days: WEEKDAYS, language: "German" },
            Program { name: "Klassik-Treffpunkt", description: "Classical music program", start: (19, 30), end: (21, 0), days: WEEKDAYS, language: "German" },
        ],
    },
    Station {
        name: "France Inter",
        callsign: "France Inter LW",
        frequency_hz: 162000.0,
        band: Band::LW,
        country: "France",
        city: "Allouis",
        kind: StationKind::Public,
        power_kw: 2000,
        language: "French",
        website: Some("https://www.franceinter.fr"),
        description: "France's main public radio station featuring news, culture, music, and intellectual programming",
        programs: &[
            Program { name: "Le Journal de 7h", description: "Morning news", start: (7, 0), end: (9, 0), days: WEEKDAYS, language: "French" },
            Program { name: "Le Journal de 8h", description: "News update", start: (8, 0), end: (8, 30), days: WEEKDAYS, language: "French" },
            Program { name: "Le Journal de 12h", description: "Midday news", start: (12, 0), end: (12, 30), days: WEEKDAYS, language: "French" },
            Program { name: "Le Journal de 13h", description: "Afternoon news", start: (13, 0), end: (13, 30), days: WEEKDAYS, language: "French" },
            Program { name: "Le Journal de 18h", description: "Evening news", start: (18, 0), end: (18, 30), days: WEEKDAYS, language: "French" },
        ],
    },
    Station {
        name: "RTL Radio",
        callsign: "RTL LW",
        frequency_hz: 234000.0,
        band: Band::LW,
        country: "Luxembourg",
        city: "Junglinster",
        kind: StationKind::Commercial,
        power_kw: 300,
        language: "French",
        website: Some("https://www.rtl.lu"),
        description: "Popular commercial radio station broadcasting entertainment, music, and information across Europe",
        programs: &[
            Program { name: "RTL Matin", description: "Morning show", start: (6, 0), end: (9, 0), days: WEEKDAYS, language: "French" },
            Program { name: "RTL Soir", description: "Evening entertainment", start: (16, 0), end: (19, 0), days: WEEKDAYS, language: "French" },
        ],
    },
    Station {
        name: "BBC Radio 5 Live",
        callsign: "BBC 5L",
        frequency_hz: 909000.0,
        band: Band::MW,
        country: "United Kingdom",
        city: "Wrotham",
        kind: StationKind::Public,
        power_kw: 50,
        language: "English",
        website: None,
        description: "BBC's sports and live events station",
        programs: &[],
    },
    Station {
        name: "France Info",
        callsign: "France Info",
        frequency_hz: 837000.0,
        band: Band::MW,
        country: "France",
        city: "Paris",
        kind: StationKind::Public,
        power_kw: 100,
        language: "French",
        website: None,
        description: "France's 24/7 news and information station",
        programs: &[],
    },
    Station {
        name: "Deutsche Welle",
        callsign: "DW",
        frequency_hz: 943000.0,
        band: Band::MW,
        country: "Germany",
        city: "Berlin",
        kind: StationKind::International,
        power_kw: 150,
        language: "German",
        website: None,
        description: "Germany's international broadcaster",
        programs: &[],
    },
    Station {
        name: "BBC World Service",
        callsign: "BBC WS",
        frequency_hz: 5975000.0,
        band: Band::SW,
        country: "United Kingdom",
        city: "Woofferton",
        kind: StationKind::International,
        power_kw: 250,
        language: "English",
        website: None,
        description: "BBC's international news and information service",
        programs: &[],
    },
    Station {
        name: "Voice of America",
        callsign: "VOA",
        frequency_hz: 5955000.0,
        band: Band::SW,
        country: "United States",
        city: "Delano",
        kind: StationKind::International,
        power_kw: 250,
        language: "English",
        website: None,
        description: "US international broadcasting service",
        programs: &[],
    },
    Station {
        name: "Radio France Internationale",
        callsign: "RFI",
        frequency_hz: 6165000.0,
        band: Band::SW,
        country: "France",
        city: "Issoudun",
        kind: StationKind::International,
        power_kw: 500,
        language: "French",
        website: None,
        description: "France's international radio service",
        programs: &[],
    },
    Station {
        name: "BFBS Radio 1",
        callsign: "BFBS 1",
        frequency_hz: 89000000.0,
        band: Band::VHF,
        country: "United Kingdom",
        city: "Various (Europe)",
        kind: StationKind::Military,
        power_kw: 10,
        language: "English",
        website: None,
        description: "British Forces Broadcasting Service",
        programs: &[],
    },
];

fn matches(haystack: &str, needle: &str) -> bool {
    haystack.to_ascii_lowercase().contains(needle)
}

/// Verbatim `FrequencyDatabase.search_stations` semantics: substring over
/// name/callsign/description/country, optional band + country filters.
/// Empty query matches everything (substring of everything).
pub fn search(query: &str, band: Option<Band>, country: Option<&str>) -> Vec<&'static Station> {
    let q = query.to_ascii_lowercase();
    let country = country.unwrap_or("").trim().to_ascii_lowercase();
    STATIONS
        .iter()
        .filter(|s| band.is_none_or(|b| s.band == b))
        .filter(|s| country.is_empty() || s.country.to_ascii_lowercase() == country)
        .filter(|s| {
            matches(s.name, &q)
                || matches(s.callsign, &q)
                || matches(s.description, &q)
                || matches(s.country, &q)
        })
        .collect()
}

pub fn by_band(band: Band) -> Vec<&'static Station> {
    STATIONS.iter().filter(|s| s.band == band).collect()
}

pub fn by_country(country: &str) -> Vec<&'static Station> {
    let want = country.trim().to_ascii_lowercase();
    STATIONS
        .iter()
        .filter(|s| s.country.to_ascii_lowercase() == want)
        .collect()
}

pub fn lookup_callsign(callsign: &str) -> Option<&'static Station> {
    STATIONS.iter().find(|s| s.callsign == callsign)
}

pub fn schedule(callsign: &str, day: Option<&str>) -> Vec<&'static Program> {
    let Some(station) = lookup_callsign(callsign) else {
        return Vec::new();
    };
    match day.map(|d| d.trim().to_ascii_lowercase()) {
        None => station.programs.iter().collect(),
        Some(want) => {
            let want_ref: &str = want.as_str();
            station
                .programs
                .iter()
                .filter(|p| p.days.contains(&want_ref))
                .collect()
        }
    }
}

/// Deterministic core of `get_current_program`: weekday + minutes-since-
/// midnight are parameters so tests never depend on the wall clock.
pub fn current_program_at(
    callsign: &str,
    weekday_lower: &str,
    minutes: u16,
) -> Option<&'static Program> {
    let station = lookup_callsign(callsign)?;
    station.programs.iter().find(|p| {
        p.days.contains(&weekday_lower)
            && (p.start.0 as u16) * 60 + p.start.1 as u16 <= minutes
            && minutes <= (p.end.0 as u16) * 60 + p.end.1 as u16
    })
}

/// Wall-clock wrapper (`datetime.now()` parity via `chrono::Local`).
pub fn current_program_now(callsign: &str) -> Option<&'static Program> {
    let now = chrono::Local::now();
    let weekday = now.format("%A").to_string().to_ascii_lowercase();
    let minutes = now.time().hour() as u16 * 60 + now.time().minute() as u16;
    current_program_at(callsign, &weekday, minutes)
}

pub fn station_json(s: &Station) -> Value {
    let desc = if s.description.chars().count() > 100 {
        format!("{}...", s.description.chars().take(100).collect::<String>())
    } else {
        s.description.to_string()
    };
    json!({
        "name": s.name,
        "callsign": s.callsign,
        "frequency_mhz": s.frequency_mhz(),
        "frequency_khz": s.frequency_khz(),
        "band": s.band.as_str(),
        "country": s.country,
        "city": s.city,
        "type": s.kind.as_str(),
        "power_kw": s.power_kw,
        "language": s.language,
        "description": desc,
        "website": s.website,
    })
}

pub fn stats_json() -> Value {
    let total_power: u32 = STATIONS.iter().map(|s| s.power_kw).sum();
    let count = STATIONS.len() as u32;
    json!({
        "status": "success",
        "database_stats": {
            "total_stations": STATIONS.len(),
            "total_transmitter_power": total_power,
            "average_power_per_station": if count > 0 {
                (total_power as f64 / count as f64 * 10.0).round() / 10.0
            } else { 0.0 },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn eleven_seeded_stations() {
        assert_eq!(STATIONS.len(), 11);
    }

    #[test]
    fn search_bbc_finds_three() {
        assert_eq!(search("BBC", None, None).len(), 3);
    }

    #[test]
    fn search_empty_matches_all() {
        assert_eq!(search("", None, None).len(), 11);
    }

    #[test]
    fn band_filter_longwave_has_four() {
        assert_eq!(by_band(Band::LW).len(), 4);
        assert!(by_band(Band::UHF).is_empty());
    }

    #[test]
    fn country_lookup_is_case_insensitive() {
        assert_eq!(by_country("france").len(), 3);
        assert_eq!(by_country("Austria").len(), 1);
    }

    #[test]
    fn schedule_filters_by_day() {
        assert_eq!(schedule("BBC LW", Some("monday")).len(), 5);
        assert!(schedule("BBC LW", Some("sunday")).is_empty());
        assert!(schedule("DW", None).is_empty());
        assert!(schedule("NOPE", None).is_empty());
    }

    #[test]
    fn current_program_is_deterministic() {
        // Wednesday 08:00 -> Today (06:00-09:00).
        let hit = current_program_at("BBC LW", "wednesday", 480);
        assert_eq!(hit.map(|p| p.name), Some("Today"));
        assert!(current_program_at("BBC LW", "sunday", 480).is_none());
        assert!(current_program_at("DW", "wednesday", 480).is_none());
    }

    #[test]
    fn stats_counts_match_table() {
        let stats = stats_json();
        assert_eq!(stats["database_stats"]["total_stations"].as_u64(), Some(11));
    }

    #[test]
    fn band_parse_accepts_names() {
        assert_eq!(Band::parse("lw"), Some(Band::LW));
        assert_eq!(Band::parse("Longwave"), Some(Band::LW));
        assert_eq!(Band::parse("FM"), None);
    }

    #[test]
    fn chrono_import_is_used() {
        // Keeps the chrono dependency honest: wall-clock wrapper compiles.
        let _ = chrono::Local::now().time().hour();
    }
}
