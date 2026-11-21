//! Online radio databases: `radio-browser.info` station search and the
//! Signal Identification Wiki, ported from `online_db.py`.
//!
//! Semantics kept verbatim: mirror discovery via
//! `https://all.api.radio-browser.info` (5 s) with fallback to
//! `https://de1.api.radio-browser.info`, 1-hour TTL caches, stale-cache
//! fallback when the network fails, `User-Agent: sdrcraft-mcp/0.1.0`,
//! 32 MiB payload cap on every response.

use crate::creds::with_timeout_secs;
use crate::error::DeviceError;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const RADIO_BROWSER_BASE: &str = "https://de1.api.radio-browser.info";
pub const RADIO_BROWSER_MIRRORS: &str = "https://all.api.radio-browser.info";
pub const SIGIDWIKI_BASE: &str = "https://www.sigidwiki.com/api.php";
pub const HTTP_TIMEOUT_SECS: u64 = 15;
pub const CACHE_TTL: Duration = Duration::from_secs(3600);
pub const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
const USER_AGENT: &str = "sdrcraft-mcp/0.1.0";

#[derive(Debug, Clone, Default, Serialize)]
pub struct OnlineStation {
    pub name: String,
    pub country: String,
    pub language: String,
    pub tags: Vec<String>,
    pub codec: String,
    pub bitrate: u32,
    pub url: String,
    pub stationuuid: String,
    pub clicks: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SignalInfo {
    pub title: String,
    pub description: String,
    pub page_url: String,
}

fn str_field(entry: &Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn u32_field(entry: &Value, key: &str) -> u32 {
    entry
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u32::MAX as u64) as u32
}

/// Parse one radio-browser entry. `tags` arrives as a comma string upstream
/// (or, defensively, a JSON list); malformed entries are skipped by the
/// caller — mirrors the legacy `try/except: continue`.
pub fn parse_station(entry: &Value) -> Option<OnlineStation> {
    if !entry.is_object() {
        return None;
    }
    let tags = match entry.get("tags") {
        Some(Value::String(raw)) => raw
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };
    Some(OnlineStation {
        name: {
            let n = str_field(entry, "name");
            if n.trim().is_empty() {
                "Unknown".to_string()
            } else {
                n
            }
        },
        country: str_field(entry, "country"),
        language: str_field(entry, "language"),
        tags,
        codec: str_field(entry, "codec"),
        bitrate: u32_field(entry, "bitrate"),
        url: str_field(entry, "url"),
        stationuuid: str_field(entry, "stationuuid"),
        clicks: u32_field(entry, "clicks"),
    })
}

/// RFC 3986 percent-encoding for single path segments.
pub fn percent_encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for b in segment.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Rough HTML tag strip for SigID snippets (verbatim `_strip_html`).
pub fn strip_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut inside = false;
    for c in text.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn http_client() -> Result<reqwest::Client, DeviceError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| DeviceError::Protocol(format!("online http client: {e}")))
}

async fn get_text(url: &str, query: &[(&str, &str)]) -> Result<String, DeviceError> {
    let http = http_client()?;
    let resp = with_timeout_secs(
        "online.get",
        HTTP_TIMEOUT_SECS,
        http.get(url).query(query).send(),
    )
    .await?
    .map_err(|e| DeviceError::Protocol(format!("online.get: {e}")))?;
    if !resp.status().is_success() {
        return Err(DeviceError::Protocol(format!(
            "online.get HTTP {}",
            resp.status()
        )));
    }
    let text = with_timeout_secs("online.body", HTTP_TIMEOUT_SECS, resp.text())
        .await?
        .map_err(|e| DeviceError::Protocol(format!("online.body: {e}")))?;
    if text.len() > MAX_PAYLOAD_BYTES {
        return Err(DeviceError::Protocol(format!(
            "online payload {} bytes exceeds cap",
            text.len()
        )));
    }
    Ok(text)
}

type RadioCache = Mutex<HashMap<String, (Instant, Vec<OnlineStation>)>>;

fn radio_cache() -> &'static RadioCache {
    static CACHE: OnceLock<RadioCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

type SignalCache = Mutex<HashMap<String, (Instant, Option<SignalInfo>)>>;

fn signal_cache() -> &'static SignalCache {
    static CACHE: OnceLock<SignalCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_get(
    cache: &Mutex<HashMap<String, (Instant, Vec<OnlineStation>)>>,
    key: &str,
) -> Option<Vec<OnlineStation>> {
    let mut guard = cache.lock().ok()?;
    let (expires, value) = guard.get(key)?.clone();
    if Instant::now() > expires {
        guard.remove(key);
        return None;
    }
    Some(value)
}

/// Discover the fastest radio-browser mirror (verbatim fallback chain).
pub async fn radio_browser_base() -> String {
    match get_text(RADIO_BROWSER_MIRRORS, &[]).await {
        Ok(text) => serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .and_then(|mut items| {
                if items.is_empty() {
                    None
                } else {
                    items.remove(0).as_str().map(str::to_string)
                }
            })
            .unwrap_or_else(|| RADIO_BROWSER_BASE.to_string()),
        Err(_) => RADIO_BROWSER_BASE.to_string(),
    }
}

/// Search radio-browser.info. `by` is `name|country|language|tag|search`;
/// on network failure a stale cache entry is served (offline/airplane use).
pub async fn search_radio_browser(
    query: &str,
    limit: u32,
    by: &str,
    country: Option<&str>,
    language: Option<&str>,
    tag: Option<&str>,
) -> Result<Vec<OnlineStation>, DeviceError> {
    let limit = limit.clamp(1, 100);
    let base = radio_browser_base().await;
    let path = match (by, country, language, tag) {
        ("country", Some(c), _, _) if !c.trim().is_empty() => {
            format!(
                "{base}/json/stations/bycountry/{}?limit={limit}",
                percent_encode(c.trim())
            )
        }
        ("language", _, Some(l), _) if !l.trim().is_empty() => {
            format!(
                "{base}/json/stations/bylanguage/{}?limit={limit}",
                percent_encode(l.trim())
            )
        }
        ("tag", _, _, Some(t)) if !t.trim().is_empty() => {
            format!(
                "{base}/json/stations/bytag/{}?limit={limit}",
                percent_encode(t.trim())
            )
        }
        ("search", _, _, _) => format!(
            "{base}/json/stations/search?name={}&limit={limit}",
            percent_encode(query)
        ),
        _ => format!("{base}/json/stations/byname/{}", percent_encode(query)),
    };
    let cache_key = format!("{path}|{limit}");
    if let Some(hit) = cache_get(radio_cache(), &cache_key) {
        return Ok(hit);
    }
    let text = match get_text(&path, &[]).await {
        Ok(text) => text,
        Err(_) => {
            // Stale fallback, even expired — offline use beats empty use.
            let guard = radio_cache().lock().map_err(|_| {
                DeviceError::Protocol("online radio cache lock poisoned".to_string())
            })?;
            return Ok(guard
                .get(&cache_key)
                .cloned()
                .map(|(_, v)| v)
                .unwrap_or_default());
        }
    };
    let data: Value = serde_json::from_str(&text)
        .map_err(|e| DeviceError::Protocol(format!("online decode: {e}")))?;
    let mut results = Vec::new();
    if let Some(items) = data.as_array() {
        for entry in items.iter().take(limit as usize) {
            if let Some(station) = parse_station(entry) {
                results.push(station);
            }
        }
    }
    if let Ok(mut guard) = radio_cache().lock() {
        guard.insert(cache_key, (Instant::now() + CACHE_TTL, results.clone()));
    }
    Ok(results)
}

pub fn station_json(s: &OnlineStation) -> Value {
    json!({
        "name": s.name,
        "country": s.country,
        "language": s.language,
        "tags": s.tags,
        "codec": s.codec,
        "bitrate": s.bitrate,
        "url": s.url,
        "stationuuid": s.stationuuid,
        "clicks": s.clicks,
    })
}

/// Look up a signal type on the Signal Identification Wiki (MediaWiki API).
/// Stale cache served on failure; `None` cached as a negative.
pub async fn get_signal_info(name: &str) -> Result<Option<SignalInfo>, DeviceError> {
    let key = name.trim().to_ascii_lowercase();
    if key.is_empty() {
        return Err(DeviceError::InvalidInput(
            "signal name must not be empty".to_string(),
        ));
    }
    if let Ok(guard) = signal_cache().lock() {
        if let Some((expires, value)) = guard.get(&key).cloned() {
            if Instant::now() <= expires {
                return Ok(value);
            }
        }
    }
    let query = [
        ("action", "query"),
        ("list", "search"),
        ("srsearch", key.as_str()),
        ("srlimit", "3"),
        ("format", "json"),
    ];
    let info = match get_text(SIGIDWIKI_BASE, &query).await {
        Ok(text) => parse_signal_info(&text, name),
        Err(_) => signal_cache()
            .lock()
            .ok()
            .and_then(|g| g.get(&key).cloned().map(|(_, v)| v))
            .unwrap_or(None),
    };
    if let Ok(mut guard) = signal_cache().lock() {
        guard.insert(key, (Instant::now() + CACHE_TTL, info.clone()));
    }
    Ok(info)
}

/// Pure MediaWiki `list=search` parser — fixture-testable without network.
pub fn parse_signal_info(text: &str, fallback_name: &str) -> Option<SignalInfo> {
    let data: Value = serde_json::from_str(text).ok()?;
    let top = data.get("query")?.get("search")?.as_array()?.first()?;
    let title = top
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(fallback_name);
    Some(SignalInfo {
        title: title.to_string(),
        description: strip_html(top.get("snippet").and_then(Value::as_str).unwrap_or("")),
        page_url: format!("https://www.sigidwiki.com/wiki/{}", percent_encode(title)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_leaves_unreserved() {
        assert_eq!(percent_encode("jazz"), "jazz");
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode("RFI/fr"), "RFI%2Ffr");
    }

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<b>FT8</b> is <i>weak</i>"), "FT8 is weak");
    }

    #[test]
    fn parse_station_handles_string_tags() {
        let entry = json!({
            "name": "Test FM", "country": "Austria", "language": "german",
            "tags": "jazz, news ", "codec": "MP3", "bitrate": 128,
            "url": "http://x", "stationuuid": "u1", "clicks": 5,
        });
        let parsed = parse_station(&entry);
        assert!(parsed.is_some());
        if let Some(s) = parsed {
            assert_eq!(s.tags, vec!["jazz".to_string(), "news".to_string()]);
            assert_eq!(s.bitrate, 128);
        }
    }

    #[test]
    fn parse_station_rejects_non_objects() {
        assert!(parse_station(&json!([1, 2])).is_none());
        // Empty name falls back to Unknown, like the legacy default.
        let parsed = parse_station(&json!({}));
        assert!(parsed.is_some());
        if let Some(s) = parsed {
            assert_eq!(s.name, "Unknown");
        }
    }

    #[test]
    fn parse_signal_info_reads_top_hit() {
        let text = r#"{"query":{"search":[{"title":"FT8","snippet":"<b>FT8</b> digital mode"}]}}"#;
        let parsed = parse_signal_info(text, "FT8");
        assert!(parsed.is_some());
        if let Some(info) = parsed {
            assert_eq!(info.title, "FT8");
            assert_eq!(info.description, "FT8 digital mode");
            assert!(info.page_url.ends_with("/FT8"));
        }
        assert!(parse_signal_info(r#"{"query":{"search":[]}}"#, "X").is_none());
    }
}
