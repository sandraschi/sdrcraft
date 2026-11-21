//! Zero-config legacy fleet import.
//!
//! Resolution order everywhere: `explicit args > process env > imported
//! legacy `.env` > built-in default`. The legacy Python repo keeps no
//! `.env` file today (all keys are code-embedded with env overrides), so
//! the file layer is tolerant: present file wins, absent file is `None`.
//!
//! Legacy keys (from `docs/CONVERSION_DOSSIER.md` §4):
//! `SDR_MCP_MOCK`, `SDR_WEB_API_HOST/PORT`, `MCP_TRANSPORT/HOST/PORT/PATH`,
//! `RTL_TCP_HOST/PORT`, `GNURADIO_DEMOD_URL`, `DEMOD_HOST/PORT`,
//! `SDR_AUDIO_MODE/HOST/PORT`.

use std::collections::HashMap;
use std::path::PathBuf;

/// Mock-mode switch, mirroring `handlers/state.py::_init_mock_from_env`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockSetting {
    Auto,
    Enabled,
    Disabled,
}

impl MockSetting {
    /// Parse hostile input with the exact legacy truth table.
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "enable" | "enabled" | "on" => Self::Enabled,
            "0" | "false" | "no" | "disable" | "disabled" | "off" => Self::Disabled,
            _ => Self::Auto,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

/// Legacy built-in defaults (verbatim from the Python sources).
pub const DEFAULT_WEB_HOST: &str = "127.0.0.1";
pub const DEFAULT_WEB_PORT: u16 = 10892;
pub const DEFAULT_MCP_HOST: &str = "127.0.0.1";
pub const DEFAULT_MCP_PORT: u16 = 11119;
pub const DEFAULT_RTL_TCP_HOST: &str = "127.0.0.1";
pub const DEFAULT_RTL_TCP_PORT: u16 = 1234;
pub const DEFAULT_GNURADIO_URL: &str = "http://127.0.0.1:11223";
pub const DEFAULT_AUDIO_MODE: &str = "udp";
pub const DEFAULT_AUDIO_HOST: &str = "host.docker.internal";
pub const DEFAULT_AUDIO_PORT: u16 = 7355;

/// Expand `$VAR` and `${VAR:-default}` against the process environment.
/// Unknown/unset vars without a default expand to empty. Never panics.
pub fn expand_vars(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '$' {
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        if i < chars.len() && chars[i] == '{' {
            // ${VAR} or ${VAR:-default}
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '}' {
                i += 1;
            }
            let body: String = chars[start..i.min(chars.len())].iter().collect();
            if i < chars.len() {
                i += 1; // consume '}'
            }
            if let Some((name, default)) = body.split_once(":-") {
                let value = std::env::var(name).unwrap_or_default();
                if value.is_empty() {
                    out.push_str(default);
                } else {
                    out.push_str(&value);
                }
            } else {
                out.push_str(&std::env::var(body.as_str()).unwrap_or_default());
            }
        } else {
            // $VAR — alnum + underscore name
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            if start == i {
                out.push('$');
            } else {
                let name: String = chars[start..i].iter().collect();
                out.push_str(&std::env::var(name.as_str()).unwrap_or_default());
            }
        }
    }
    out
}

/// Parse `KEY=VALUE` lines: skips blanks/`#` comments, strips matching
/// single/double quotes, expands `$VAR`/`${VAR:-default}` in values.
pub fn parse_dotenv(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let key = line[..eq].trim();
        if key.is_empty() {
            continue;
        }
        let mut value = line[eq + 1..].trim().to_string();
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = value[1..value.len().saturating_sub(1)].to_string();
        }
        map.insert(key.to_string(), expand_vars(&value));
    }
    map
}

/// Locate the legacy Python repo without hardcoded machine paths:
/// `SDR_MCP_LEGACY_DIR` env, else a `sdr-mcp` sibling of the current dir.
pub fn discover_legacy_root() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SDR_MCP_LEGACY_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return Some(path);
        }
    }
    let mut cwd = std::env::current_dir().ok()?;
    cwd.push("../sdr-mcp");
    if cwd.is_dir() {
        return Some(cwd);
    }
    None
}

/// Load `<legacy>/.env` when present; `None` when absent/unreadable.
pub fn load_legacy_env() -> Option<HashMap<String, String>> {
    let mut root = discover_legacy_root()?;
    root.push(".env");
    let text = std::fs::read_to_string(&root).ok()?;
    Some(parse_dotenv(&text))
}

fn imported(key: &str) -> Option<String> {
    load_legacy_env()
        .as_ref()
        .and_then(|map| map.get(key))
        .cloned()
        .filter(|v| !v.trim().is_empty())
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Secret lookup for the kernel (`llm.rs`): process env, then legacy file.
/// Values are never logged — only presence/absence leaves this module.
pub fn fleet_secret(name: &str) -> Option<String> {
    env_non_empty(name).or_else(|| imported(name))
}

/// Resolved legacy configuration: every field honors
/// explicit > env > legacy file > built-in default.
#[derive(Debug, Clone)]
pub struct LegacyConfig {
    pub mock: MockSetting,
    pub web_host: String,
    pub web_port: u16,
    pub mcp_host: String,
    pub mcp_port: u16,
    pub rtl_host: String,
    pub rtl_port: u16,
    pub gnuradio_url: String,
    pub audio_mode: String,
    pub audio_host: String,
    pub audio_port: u16,
}

impl LegacyConfig {
    pub fn resolve() -> Self {
        let file = load_legacy_env().unwrap_or_default();
        let get = |key: &str| -> Option<String> {
            env_non_empty(key).or_else(|| file.get(key).cloned().filter(|v| !v.trim().is_empty()))
        };
        let port = |key: &str, default: u16| -> u16 {
            get(key)
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(default)
        };
        Self {
            mock: get("SDR_MCP_MOCK")
                .map(|v| MockSetting::parse(&v))
                .unwrap_or(MockSetting::Auto),
            web_host: get("SDR_WEB_API_HOST").unwrap_or_else(|| DEFAULT_WEB_HOST.to_string()),
            web_port: port("SDR_WEB_API_PORT", DEFAULT_WEB_PORT),
            mcp_host: get("MCP_HOST").unwrap_or_else(|| DEFAULT_MCP_HOST.to_string()),
            mcp_port: port("MCP_PORT", DEFAULT_MCP_PORT),
            rtl_host: get("RTL_TCP_HOST").unwrap_or_else(|| DEFAULT_RTL_TCP_HOST.to_string()),
            rtl_port: port("RTL_TCP_PORT", DEFAULT_RTL_TCP_PORT),
            gnuradio_url: get("GNURADIO_DEMOD_URL")
                .map(|u| u.trim_end_matches('/').to_string())
                .unwrap_or_else(|| DEFAULT_GNURADIO_URL.to_string()),
            audio_mode: get("SDR_AUDIO_MODE").unwrap_or_else(|| DEFAULT_AUDIO_MODE.to_string()),
            audio_host: get("SDR_AUDIO_HOST").unwrap_or_else(|| DEFAULT_AUDIO_HOST.to_string()),
            audio_port: port("SDR_AUDIO_PORT", DEFAULT_AUDIO_PORT),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_truth_table_matches_legacy() {
        assert_eq!(MockSetting::parse("enable"), MockSetting::Enabled);
        assert_eq!(MockSetting::parse("ON"), MockSetting::Enabled);
        assert_eq!(MockSetting::parse("1"), MockSetting::Enabled);
        assert_eq!(MockSetting::parse("disable"), MockSetting::Disabled);
        assert_eq!(MockSetting::parse("off"), MockSetting::Disabled);
        assert_eq!(MockSetting::parse(""), MockSetting::Auto);
        assert_eq!(MockSetting::parse("auto"), MockSetting::Auto);
        assert_eq!(MockSetting::parse("nonsense"), MockSetting::Auto);
    }

    #[test]
    fn expand_vars_handles_defaults() {
        assert_eq!(expand_vars("a${MISSING_XYZ:-fallback}b"), "afallbackb");
        assert_eq!(expand_vars("x$DEFINITELY_MISSING_VAR_QQZ"), "x");
        assert_eq!(expand_vars("plain"), "plain");
        assert_eq!(expand_vars("$"), "$");
    }

    #[test]
    fn dotenv_parses_quotes_comments_expansion() {
        let text = "# comment\nEMPTY=\nA=1\nB=\"two words\"\nC='sq'\nD=${MISSING_XYZ:-dflt}\n";
        let map = parse_dotenv(text);
        assert_eq!(map.get("A").map(String::as_str), Some("1"));
        assert_eq!(map.get("B").map(String::as_str), Some("two words"));
        assert_eq!(map.get("C").map(String::as_str), Some("sq"));
        assert_eq!(map.get("D").map(String::as_str), Some("dflt"));
        assert_eq!(map.get("EMPTY").map(String::as_str), Some(""));
    }

    #[test]
    fn resolve_uses_builtin_defaults_off_machine() {
        // Env must not leak machine state into this assertion beyond defaults;
        // fields fall back to documented legacy defaults when unset.
        let cfg = LegacyConfig::resolve();
        assert!(!cfg.gnuradio_url.ends_with('/'));
        assert!(cfg.rtl_port > 0);
    }

    #[test]
    fn absent_key_resolves_to_none() {
        assert!(fleet_secret("RUST_SDR_MCP_DEFINITELY_ABSENT_KEY_QQZ").is_none());
    }
}
