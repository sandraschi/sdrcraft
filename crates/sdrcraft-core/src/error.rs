//! Typed device errors. No panics, no unwraps: every failure is a value.
use std::fmt;

/// All recoverable device failures. Each variant carries context for logs
/// and maps to a JSON-RPC error string at the L2 boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceError {
    /// Device did not answer in time.
    Timeout(String),
    /// Bytes on the wire did not match the protocol.
    Protocol(String),
    /// Device is known but currently unreachable.
    Offline(String),
    /// Caller-supplied argument failed validation (hostile input fence).
    InvalidInput(String),
    /// Credentials missing or rejected (account login, API key, token).
    /// Never carries secret values — only what is missing and where to set it.
    Auth(String),
    /// Reachable device this driver cannot speak to (wrong generation, model).
    Unsupported(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceError::Timeout(ctx) => write!(f, "device timed out: {ctx}"),
            DeviceError::Protocol(ctx) => write!(f, "protocol error: {ctx}"),
            DeviceError::Offline(ctx) => write!(f, "device offline: {ctx}"),
            DeviceError::InvalidInput(ctx) => write!(f, "invalid input: {ctx}"),
            DeviceError::Auth(ctx) => write!(f, "auth failed: {ctx}"),
            DeviceError::Unsupported(ctx) => write!(f, "unsupported: {ctx}"),
        }
    }
}

impl std::error::Error for DeviceError {}

/// Forensic panic hook (10-point contract §6): unexpected third-party C/GPU
/// panics are appended to `%LOCALAPPDATA%\sdrcraft-mcp\crash.log` and echoed
/// on `stderr` instead of vanishing. Call once at the top of every `main()`.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let when = std::time::SystemTime::now();
        let line = format!("[{when:?}] panic: {info}\n");
        eprint!("{line}");
        let mut log_path = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        log_path.push_str("\\sdrcraft-mcp\\crash.log");
        if let Some(parent) = std::path::Path::new(&log_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            use std::io::Write as _;
            let _ = file.write_all(line.as_bytes());
        }
    }));
}

/// Fence for every external string that names a device.
pub fn require_device_id(device_ip: &str) -> Result<&str, DeviceError> {
    let trimmed = device_ip.trim();
    if trimmed.is_empty() {
        return Err(DeviceError::InvalidInput(
            "device_ip must not be empty".to_string(),
        ));
    }
    if trimmed.chars().count() > 253 {
        return Err(DeviceError::InvalidInput(
            "device_ip exceeds 253 chars".to_string(),
        ));
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_device_id_is_rejected() {
        let result = require_device_id("   ");
        assert_eq!(
            result,
            Err(DeviceError::InvalidInput(
                "device_ip must not be empty".to_string()
            ))
        );
    }

    #[test]
    fn display_carries_context() {
        let e = DeviceError::Timeout("192.168.1.50".to_string());
        assert!(e.to_string().contains("192.168.1.50"));
    }
}
