//! WebSocket spectrum broadcaster (`tokio-tungstenite`).
//!
//! Manages an optional background `TcpListener` that accepts WebSocket clients
//! and streams periodic JSON spectrum frames (`frequencies`, `spectrum`, `source`).

use crate::error::DeviceError;
use crate::sdr;
use futures_util::SinkExt;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

struct WsRuntime {
    host: String,
    port: u16,
    clients: Arc<AtomicUsize>,
    stop_tx: watch::Sender<bool>,
}

static WS_STATE: OnceLock<Mutex<Option<WsRuntime>>> = OnceLock::new();

fn ws_lock() -> &'static Mutex<Option<WsRuntime>> {
    WS_STATE.get_or_init(|| Mutex::new(None))
}

pub fn status() -> Value {
    let guard = match ws_lock().lock() {
        Ok(g) => g,
        Err(_) => {
            return json!({ "success": false, "status": "error", "error": "ws state lock poisoned" });
        }
    };
    match guard.as_ref() {
        Some(rt) => json!({
            "success": true,
            "running": true,
            "host": rt.host,
            "port": rt.port,
            "url": format!("ws://{}:{}", rt.host, rt.port),
            "clients": rt.clients.load(Ordering::Relaxed),
        }),
        None => json!({
            "success": true,
            "running": false,
            "clients": 0,
        }),
    }
}

pub async fn start(host: String, port: u16) -> Result<Value, DeviceError> {
    {
        let guard = ws_lock()
            .lock()
            .map_err(|_| DeviceError::Protocol("ws state lock poisoned".to_string()))?;
        if let Some(rt) = guard.as_ref() {
            return Ok(json!({
                "success": true,
                "running": true,
                "already_running": true,
                "host": rt.host,
                "port": rt.port,
                "url": format!("ws://{}:{}", rt.host, rt.port),
                "clients": rt.clients.load(Ordering::Relaxed),
            }));
        }
    }

    let bind_addr = format!("{}:{}", host.trim(), port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| DeviceError::Protocol(format!("ws bind {bind_addr}: {e}")))?;
    let bound_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);

    let (stop_tx, mut stop_rx) = watch::channel(false);
    let clients = Arc::new(AtomicUsize::new(0));
    let clients_accept = Arc::clone(&clients);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    if *stop_rx.borrow() {
                        break;
                    }
                }
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else {
                        break;
                    };
                    let mut client_stop = stop_rx.clone();
                    let client_count = Arc::clone(&clients_accept);
                    tokio::spawn(async move {
                        let Ok(mut ws_stream) = tokio_tungstenite::accept_async(stream).await else {
                            return;
                        };
                        client_count.fetch_add(1, Ordering::Relaxed);
                        let mut interval = tokio::time::interval(Duration::from_millis(200));
                        loop {
                            tokio::select! {
                                _ = client_stop.changed() => {
                                    if *client_stop.borrow() {
                                        let _ = ws_stream.close(None).await;
                                        break;
                                    }
                                }
                                _ = interval.tick() => {
                                    let frame = build_spectrum_frame();
                                    let text = frame.to_string();
                                    if ws_stream.send(Message::Text(text.into())).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                        client_count.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }
        }
    });

    let mut guard = ws_lock()
        .lock()
        .map_err(|_| DeviceError::Protocol("ws state lock poisoned".to_string()))?;
    *guard = Some(WsRuntime {
        host: host.clone(),
        port: bound_port,
        clients: Arc::clone(&clients),
        stop_tx,
    });

    Ok(json!({
        "success": true,
        "running": true,
        "host": host,
        "port": bound_port,
        "url": format!("ws://{host}:{bound_port}"),
        "clients": 0,
    }))
}

pub fn stop() -> Result<Value, DeviceError> {
    let mut guard = ws_lock()
        .lock()
        .map_err(|_| DeviceError::Protocol("ws state lock poisoned".to_string()))?;
    if let Some(rt) = guard.take() {
        let _ = rt.stop_tx.send(true);
        Ok(json!({
            "success": true,
            "running": false,
            "stopped_port": rt.port,
        }))
    } else {
        Ok(json!({
            "success": true,
            "running": false,
            "message": "websocket server was not running",
        }))
    }
}

fn build_spectrum_frame() -> Value {
    let samples = match sdr::mock_iq() {
        Ok(mut gen) => gen.generate(sdr::FFT_SIZE * 2),
        Err(_) => Vec::new(),
    };
    let (freqs, power) = sdr::compute_spectrum(&samples, sdr::DEFAULT_SAMPLE_RATE);
    json!({
        "type": "spectrum",
        "frequencies": freqs,
        "spectrum": power,
        "source": "synthetic_iq",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn ws_start_stream_and_stop_roundtrip() {
        let started = start("127.0.0.1".to_string(), 0).await;
        assert!(started.is_ok());
        let Ok(info) = started else { return };
        let url = info
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(url.starts_with("ws://127.0.0.1:"));

        let conn = tokio_tungstenite::connect_async(&url).await;
        assert!(conn.is_ok());
        if let Ok((mut client, _)) = conn {
            if let Some(Ok(Message::Text(txt))) = client.next().await {
                let parsed: Result<Value, _> = serde_json::from_str(&txt);
                assert!(parsed.is_ok());
                if let Ok(v) = parsed {
                    assert_eq!(v.get("type").and_then(Value::as_str), Some("spectrum"));
                }
            }
            let _ = client.close(None).await;
        }

        let stopped = stop();
        assert!(stopped.is_ok());
        assert_eq!(
            status().get("running").and_then(Value::as_bool),
            Some(false)
        );
    }
}
