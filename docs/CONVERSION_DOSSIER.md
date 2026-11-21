# Conversion Dossier: `rust-sdr-mcp` (from `sdr-mcp`)

**Generated:** 2026-10-18 (Europe/Vienna, UTC+02:00)
**Legacy Repo Path (100% READ-ONLY):** `D:\Dev\repos\sdr-mcp`
**Legacy Git Baseline:** branch `master` @ commit `51ba1c1` (stamp-probed, untouched)
**Target Rust Repo:** `D:\Dev\repos\rust-sdr-mcp`
**Blueprint Kernel:** `D:\Dev\repos\rust-devices-mcp` (`v0.4.0+`, 70% kernel)
**SOP:** `mcp-central-docs/patterns/REPCONV_SOP.md` + `standards/rust-artcraft/FLEET_CONVERSION_STANDARD.md`

Legacy repo was NOT modified during this reconnaissance (read-only `Get-Content` / `Get-ChildItem` only, `node_modules/` and `.venv/` never entered).

---

## 1. Bucket & Privilege-Split / Hidden Binary Audit

**Verdict: Pure-Rust `Bucket B` — Clean Hardware / Bench Instrument / Local IoT. No privilege split, no host-plugin, no defer.**

| Probe | Result |
|---|---|
| `service/` (LocalSystem C++ over named pipe, fastsearch-style) | ABSENT |
| `native/` | PRESENT but thin Tauri webview shell only: `tauri.conf.json`, `build.ps1`, small `Cargo.toml` (481 B blueprint-era), `src/`, `icons/`, `windows/`, `capabilities/`, `binaries/`, `resources/`, `gen/`. No elevated service, no `$MFT` handle, no `\\.\pipe\` server. Retired by L3 `egui` (zero WebView2). |
| `bin/` | ABSENT |
| `src-tauri/` | ABSENT (lives as `native/`, see above) |
| `zed-extension/` / root `Cargo.toml` | ABSENT |
| `docker/gnuradio/` + `docker-compose.yml` | PRESENT: `gnuradio-demod` sidecar (container `8000` -> host `11223`), `RTL_TCP_HOST/HOST`, NOT a privilege boundary — plain TCP/HTTP demod service. Kept as external sidecar, driven from Rust via `reqwest`. |
| `scripts/start-rtl-tcp.ps1` | Host-side `rtl_tcp` launcher (USB dongle -> TCP `:1234`). Userland, no elevation. |
| Host-plugin (`import bpy`, `pcbnew`, FreeCAD) | ABSENT |
| CUDA/PyTorch (`.safetensors`, `triton`, `diffusers`) | ABSENT |

**Triage reference:** `FLEET_TRIAGE_AND_PYPI_TRANSPILATION.md` §3.2 Group 2 explicitly lists `sdr-mcp, openbci-mcp, lidar-mcp` as prime Bucket B candidates: high-throughput binary byte streams (IQ samples) where Python GIL/GC drops packets; Rust + `egui` renders live waterfall at 60+ FPS zero-copy. Target crates: `tokio::net::UdpSocket`, `tokio-serial`, `egui TextureHandle::set`. For SDR specifically: `rtl_tcp` is a trivial length-prefixed IQ-over-TCP protocol — ~150 lines of `tokio::net::TcpStream` replaces `pyrtlsdr`; 2048-pt FFT via `realfft`/`rustfft` replaces `numpy/scipy`; `tokio-tungstenite` replaces `websockets`; `cpal` replaces `sounddevice`.

---

## 2. Live Service, Port & Database Contention Audit

No shared SQLite database. Contention surface is USB hardware + TCP ports.

| Legacy endpoint | Port | Rust shadow strategy (zero contention) |
|---|---|---|
| Frontend Vite React (`fleet-start.config.ps1` FrontendPort) | `11118` | NOT claimed. L3 `egui` is in-process, zero sockets. |
| MCP HTTP (`MCP_PORT`, `fleet-start BackendPort`) | `11119` | NOT claimed. L2 MCP runs over `stdio` only. |
| GNU Radio demod sidecar (`docker-compose.yml gnuradio-demod`) | host `11223` -> container `8000` | Shared read-only HTTP client (`reqwest` + `with_timeout_secs` + 32 MiB cap). Never stops a running demod during probes; `status`/`health` only. |
| Legacy REST bridge (`web_api.py`, `sdr-mcp serve --http` only) | `10892` (`SDR_WEB_API_PORT`, NOT served by `start.ps1`) | Ignored. Dead path, no port taken. |
| `rtl_tcp` IQ stream (host dongle / `scripts/start-rtl-tcp.ps1`) | `1234` (`RTL_TCP_HOST/PORT`; compose uses `host.docker.internal:1234`) | Shared read-only TCP reader with timeout; Demo IQ generator when dongle absent. Never exclusive-opens USB while legacy holds it. |
| FM audio relay UDP | `7355` (`SDR_AUDIO_HOST/PORT`, mode `udp`) | Read-only subscriber; never binds exclusively during parity probes. |
| Spectrum WebSocket (`sdr_spectrum start_websocket`) | `8765` (`localhost`) | Rust serves its own stream only on explicit user action; probes use `websocket_status`, never `start`. |

**USB dongle (RTL2832U/R820T2, 24 MHz–1.766 GHz):** single-opener device. Rule: if legacy `sdr-mcp` holds the dongle, Rust uses `Vendor::Demo` deterministic IQ + recorded fixtures. Live USB probe only after legacy released it.
**Legacy running check:** `fleet-start.config.ps1` Kind `uvicorn-web-app` + `vite-npm`; before Phase 5 live probe, verify with port probe whether `:11119`/`:11118` are up. Either way legacy files stay untouched (`git -C ../sdr-mcp status -s` must stay clean).

---

## 3. Complete 1:1 MCP Tool & Portmanteau Operation Mapping Table

Source of truth: `src/sdr_mcp/server.py` (7 `@mcp.tool`) + `src/sdr_mcp/tools/*.py` + `handlers/gnuradio_ops.py`. Total: **27 portmanteau operations + 2 sampling helpers = 29 endpoints.**

| Legacy Python Tool (`@mcp.tool`) | Operation variants (exact) | Rust `Command` Variant | CLI Subcommand | Rust MCP Tool + Legacy Alias |
|---|---|---|---|---|
| `sdr_device` (`tools/device.py`, 9 ops) | `list` | `DeviceList` | `device list` | `sdr.list_devices` + alias `sdr_device(operation="list")` |
| | `initialize` (device_index) | `DeviceInit { device_index }` | `device init --index` | `sdr.init_device` + alias |
| | `status` | `DeviceStatus` | `device status` | `sdr.device_status` + alias |
| | `health` (RTL-SDR + sidecar) | `DeviceHealth` | `device health` | `sdr.device_health` + alias |
| | `set_frequency` (frequency_mhz, required) | `DeviceSetFreq { mhz }` | `device tune --mhz` | `sdr.set_frequency` + alias |
| | `set_gain` (gain auto\|dB) | `DeviceSetGain { gain }` | `device gain --gain` | `sdr.set_gain` + alias |
| | `tune_preset` (preset_name, required) | `DeviceTunePreset { name }` | `device preset --name` | `sdr.tune_preset` + alias |
| | `scan` (start_freq, end_freq required, step_size) | `DeviceScan { start, end, step }` | `device scan --start --end --step` | `sdr.scan` + alias |
| | `mock_mode` (mock_enabled?) | `DeviceMockMode { enabled }` | `device mock --on/--off` | `sdr.mock_mode` + alias |
| `sdr_spectrum` (`tools/spectrum.py`, 6 ops) | `spectrum` | `SpectrumGet` | `spectrum get` | `sdr.spectrum` + alias |
| | `waterfall` (100-line history) | `SpectrumWaterfall` | `spectrum waterfall` | `sdr.waterfall` + alias |
| | `start_websocket` (host, port=8765) | `SpectrumWsStart { host, port }` | `spectrum ws-start` | `sdr.ws_start` + alias |
| | `stop_websocket` | `SpectrumWsStop` | `spectrum ws-stop` | `sdr.ws_stop` + alias |
| | `websocket_status` | `SpectrumWsStatus` | `spectrum ws-status` | `sdr.ws_status` + alias |
| | `audio_status` | `SpectrumAudioStatus` | `spectrum audio-status` | `sdr.audio_status` + alias |
| `sdr_stations` (`tools/stations.py`, 5 ops, 11 seeded stations LW/MW/SW/VHF) | `search` (query, band?, country?) | `StationsSearch { query, band, country }` | `stations search --query` | `sdr.stations_search` + alias |
| | `by_band` (band required) | `StationsByBand { band }` | `stations by-band` | `sdr.stations_by_band` + alias |
| | `by_country` (country required) | `StationsByCountry { country }` | `stations by-country` | `sdr.stations_by_country` + alias |
| | `schedule` (station_callsign required, day?) | `StationsSchedule { callsign, day }` | `stations schedule` | `sdr.stations_schedule` + alias |
| | `stats` | `StationsStats` | `stations stats` | `sdr.stations_stats` + alias |
| `sdr_online` (`tools/online.py`, 2 ops, `api.radio-browser.info`, 25k+ stations) | `search` (query/by=name, country, language, tag, limit=25) | `OnlineSearch { query, country, language, tag, limit }` | `online search` | `sdr.online_search` + alias `sdr_online(operation="search")` |
| | `signal_id` (query e.g. FT8) | `OnlineSignalId { query }` | `online signal-id` | `sdr.online_signal_id` + alias |
| `sdr_gnuradio` (`tools/gnuradio.py` + `handlers/gnuradio_ops.py`, 5 ops, modes fm/am/usb/lsb, sources rtl_tcp/hackrf) | `health` | `GnuradioHealth` | `gnuradio health` | `sdr.gnuradio_health` + alias |
| | `status` | `GnuradioStatus` | `gnuradio status` | `sdr.gnuradio_status` + alias |
| | `start` (frequency_mhz required, range-checked, mode, source, rtl host/port, gain=20.0) | `GnuradioStart { mhz, mode, source, host, port, gain }` | `gnuradio start --mhz --mode` | `sdr.gnuradio_start` + alias |
| | `stop` (also stops audio relay) | `GnuradioStop` | `gnuradio stop` | `sdr.gnuradio_stop` + alias |
| | `list_devices` | `GnuradioDevices { source }` | `gnuradio devices` | `sdr.gnuradio_devices` + alias |
| `sdr_agentic_assist` (SEP-1577 `ctx.sample`) | `goal: str` | — (L2 sampling helper, port as thin prompt wrapper or drop; NOT a hardware command) | `assist --goal` | `sdr.agentic_assist` + alias |
| `sdr_sampling_hint` (SEP-1577) | `topic: str` | — (same) | `hint --topic` | `sdr.sampling_hint` + alias |

**Intentionally excluded / delegated:** `audio_stream.py` + `fm_demod.py` native-Python FM path is dead code (ASSESSMENT.md §5 — `sdr_gnuradio` sidecar is the live path). Rust implements FM/AM/SSB via sidecar client + optional `rustfft` native demod later; no `pyrtlsdr`/`numpy` vendored. Local LanceDB/fastembed: none present, nothing to delegate. `web_api.py` regex NLU `parse_chat_command`: NOT ported (replaced by L3 Chat via `llm.rs` `gemma4:12b`).
**Fleet-standard additions:** `status`, `bridge.{event,speak,webhook}`, `llm.{chat,models}` (from kernel, verbatim).

---

## 4. Credential, `.env`, Config & Data-Path Inventory (`import.rs` Spec)

No `.env` / `.env.example` at legacy root (stamp scan found only `SDR_MCP_MOCK`, `SDR_WEB_API_PORT` via code grep). Full key list extracted from `cli.py`, `transport.py`, `web_api.py`, `gnuradio_client.py`, `docker-compose.yml`, `fleet-start.config.ps1` (values redacted; resolution order `explicit args > process env > OS vault > imported legacy config`):

| Key | Source file | Default | `import.rs` handling |
|---|---|---|---|
| `SDR_MCP_MOCK` | `handlers/state.py`, `mock_capture.py`, `mock_iq.py` | `auto` (auto-fallback, no dongle → synthetic IQ) | `mock: Auto\|On\|Off`; Demo oracle when On/Auto-without-dongle |
| `SDR_WEB_API_HOST` / `SDR_WEB_API_PORT` | `web_api.py` | `127.0.0.1` / `10892` | Imported but IGNORED (legacy bridge retired; documented only) |
| `MCP_TRANSPORT` | `transport.py`, `cli.py` | `stdio` | Ignored (Rust is always stdio; no HTTP sidecar) |
| `MCP_HOST` / `MCP_PORT` / `MCP_PATH` | `transport.py` | `127.0.0.1` / `11119` / `/mcp` | Imported for diagnostics banner only |
| `RTL_TCP_HOST` / `RTL_TCP_PORT` | `gnuradio_client.py`, compose | `127.0.0.1` / `1234` (compose: `host.docker.internal:1234`) | Core L1 `sdr.rs` connection target |
| `GNURADIO_URL` (`ENV_GNURADIO_URL`) / `service_url` | `gnuradio_client.py` | sidecar `http://127.0.0.1:11223` | Core L1 `gnuradio.rs` base URL |
| `DEMOD_HOST` / `DEMOD_PORT` | `docker-compose.yml` | `0.0.0.0` / `8000` (container) | Container-internal; host uses `11223` |
| `SDR_AUDIO_MODE` / `SDR_AUDIO_HOST` / `SDR_AUDIO_PORT` | compose | `udp` / `host.docker.internal` / `7355` | Audio relay target |
| `WEB_PORT` | `fleet-start.config.ps1` Backend Env | `11119` | Ignored (zero-port architecture) |
| Frequency DB | `frequency_db.py` (11 seeded stations + schedules) | in-code | Ported as `stations.rs` static table + JSON fixture tests |
| `radio-browser.info` API | `online_db.py` | `https://api.radio-browser.info` | `online.rs` base URL, 25s timeout, offline → structured `Offline` diagnostic |

**Data paths:** USB RTL-SDR (WinUSB via Zadig) or `rtl_tcp://HOST:1234`; GNU Radio `http://HOST:11223`; WS `ws://localhost:8765` (user-started only). No local DB files, no secrets on disk. OS vault `SERVICE="rust-sdr-mcp"` holds nothing initially (no API keys in legacy); reserved for future `X-` tokens.

---

## 5. Legacy Python Source Files (wire-spec surface for `PyPI-as-Oracle`)

`src/sdr_mcp/` (20 files; `__pycache__` ignored):

- Core wire spec (read for Phase 2): `capture.py` (pyrtlsdr wrap + `asyncio.to_thread`), `mock_capture.py` + `mock_iq.py` (Demo oracle vectors), `processor.py` (2048-pt Hamming FFT, peak detect, 100-line waterfall), `fm_demod.py` (dead native path — see §3), `frequency_db.py` (11 stations + schedules), `online_db.py` (radio-browser shapes), `gnuradio_client.py` (`start_demod/stop/status/is_reachable` shapes), `websocket_server.py` (ws protocol + remote tune/gain commands), `audio_stream.py` (UDP relay), `handlers/device.py` (tune/scan semantics, `MIN/MAX_FREQ_MHZ`), `handlers/gnuradio_ops.py` (mode/source whitelists, range checks), `handlers/spectrum.py`, `handlers/stations.py`, `handlers/online.py`, `handlers/state.py` (`should_use_mock`, semaphore), `handlers/health.py`.
- Ceremony (DO NOT port): `server.py` (FastMCP reg), `transport.py` (stdio/http switch), `web_api.py` (`asyncio.run` per-request anti-pattern + regex NLU), `cli.py` (click), `async_bridge.py`.
- Upstream PyPI oracles to read (never vendor): `pyrtlsdr` (rtl_tcp framing), `numpy/scipy` FFT params only.

---

## 6. Write-Safety & Destructive-Action Fence Design

No file deletes, no SQL writes, no purchases. All mutations are hardware/process state:

| Mutating command | Effect | Fence |
|---|---|---|
| `DeviceSetFreq` / `DeviceSetGain` / `DeviceTunePreset` / `DeviceScan` | Retunes USB dongle / rtl_tcp; scan does N tune-dwell-read cycles (blocks seconds) | `readonly: bool` flag (default false); when `readonly=true`, retune/scan return `isError` diagnostic instead of touching hardware. Scan runs off UI thread via `spawn`, cancellable, dwell-capped. Frequency range-checked (`24–1766 MHz`) before dispatch. |
| `DeviceMockMode` | Flips live↔synthetic IQ globally | Requires explicit `--on/--off`; never auto-flips during probes. |
| `SpectrumWsStart` / `WsStop` | Binds `host:port` (default `8765`) | Start requires explicit user action in CLI/UI; `stop` is always safe. Port-in-use → structured `Offline` error, no panic. |
| `GnuradioStart` / `GnuradioStop` | POSTs demod start to sidecar `:11223`, starts UDP audio relay | `readonly` blocks both; `start` requires `frequency_mhz` + whitelist (`fm/am/usb/lsb`, `rtl_tcp/hackrf`); unknown op → `valid_operations` error, never panic. |
| TX safety | HackRF TX path is DOCS ONLY (`HACKRF.md` buying guide + licensing). No TX command exists in the mapping table and none will be added. RX-only binary by construction. | Compile-time: no `Tx` variant in `Command`. |

Shared-DB guards (`SQLITE_OPEN_READ_ONLY`) are N/A (no SQLite in this repo). All I/O wrapped in `with_timeout_secs` + payload caps per the 10-point contract.

---

## 7. L3 `egui` Desktop Oracle Plan (near-identical UI and UX)

Legacy webapp: `web_sota/src/` — 11 routes + chrome. Oracle screenshots go to `docs/parity/legacy/<page>.png` (+ `-<gesture>.png`) at fixed 1440×900 BEFORE Phase 1 code; Rust pairs to `docs/parity/rust/`. Layout source per route: `components/layout/sidebar.tsx` + `pages/<route>.tsx` + the placing component. `node_modules/` never read.

| # | Legacy route (`pages/`) | Groups / controls to replicate (order + labels) | Gestures to capture | Golden snapshot name |
|---|---|---|---|---|
| 1 | `dashboard.tsx` | Hero KPIs (device, freq, gain, sidecar), quick tune box | tune submit | `dashboard.png`, `dashboard-tune.png` |
| 2 | `spectrum.tsx` | Spectrum canvas, freq/gain controls, peak list | drag-freq, gain change | `spectrum.png`, `spectrum-tune.png` |
| 3 | `waterfall.tsx` | Waterfall canvas (100 lines), time controls | scroll history | `waterfall.png` |
| 4 | `stations.tsx` | Search box, band filter, station table, schedule panel | search `BBC`, by_band `FM` | `stations.png`, `stations-search.png` |
| 5 | `tools.tsx` | Tool cards per portmanteau op | invoke `status` | `tools.png` |
| 6 | `apps.tsx` (`common/apps-catalog.ts`) | App catalog grid | open entry | `apps.png` |
| 7 | `chat.tsx` | NLU chat log + input | ask `what's on 89.5` (L3 answers via `llm.rs`, legacy regex may fail — record both) | `chat.png` |
| 8 | `settings.tsx` | **Fleet standard LLM card (mandatory):** provider select (Ollama `http://127.0.0.1:11434/v1` / LM Studio `http://127.0.0.1:1234/v1` / OpenAI / Anthropic / Custom), editable Base URL + default models (`gemma4:12b`, `gpt-4o-mini`, `claude-sonnet-4-20250514`), API key via `keyring` (never disk), live model elicitation (`/api/tags`, `/v1/models` → dropdown), connection test + status pill; plus SDR targets (`RTL_TCP_HOST/PORT`, sidecar URL) | switch provider, test connection | `settings.png`, `settings-llm-test.png` |
| 9 | `status.tsx` | Device + sidecar health pills | refresh | `status.png` |
| 10 | `help.tsx` (`common/help-content.tsx` horizontal tabs) | SDR primer tabs | tab switch | `help.png` |
| 11 | `Logging.tsx` | Log stream view | filter | `logging.png` |

Chrome (from kernel shell, not re-read): `harsh_visuals` dark + `crisp_light_visuals` light, vector Sun/Moon toggle, `Ctrl+Wheel` zoom + reset pill, `Crossconnect`, `Chat` (`gemma4:12b` default, zero VRAM scrub on 24 GB 4090), `Settings` (vault off UI thread), `Help`, `Raw JSON`. Domain tabs route every action via `self.spawn(... dispatch(Command::...) ...)` over `mpsc`; offline/unconfigured shows diagnostic banner + `Load Demo Data` where legacy shows empty state. Exceptions (font raster, CSS-only effects) recorded in `docs/UI_PARITY.md` one per item. Goldens locked ONLY after pair matches; `glyph_source_scan` allowlist `°×·…--`.
