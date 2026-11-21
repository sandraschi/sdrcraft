# Conversion Audit Log: `rust-sdr-mcp`

## [2026-10-18T20:10:04+02:00] Phase 0 & Phase 1 (Automated `New-RustArtcraftRepo.ps1` Stamp)
- Verified legacy repo `D:\Dev\repos\sdr-mcp` at `master@51ba1c1` (kept 100% read-only).
- Copied reusable kernel modules from `D:\Dev\repos\rust-devices-mcp`:
  - `crates/sdr-core/src/{bridge.rs, llm.rs (gemma4:12b default), error.rs, creds.rs}`
  - `crates/sdr-app/src/{vault.rs, registry.rs}`
- Generated pre-populated `docs/CONVERSION_DOSSIER.md` with 0 discovered `@mcp.tool` definitions and 2 `.env` keys.
- Next step: Complete `docs/CONVERSION_DOSSIER.md` review, then implement Phase 2 (`crates/sdr-core`).

## [2026-10-18T20:11Z] Phase 0 Finalize (7-point forensic reconnaissance, agent pass)
- Bucket verdict: Pure-Rust Bucket B (Group 2 high-throughput IQ; `native/` is thin Tauri shell, no privilege split, no host-plugin, no CUDA). Triage ref `FLEET_TRIAGE_AND_PYPI_TRANSPILATION.md` §3.2.
- Ports inventoried (legacy READ-ONLY): frontend `11118`, MCP HTTP `11119`, sidecar `11223->8000`, legacy REST `10892`, `rtl_tcp 1234`, audio UDP `7355`, WS `8765`. No SQLite. Rust claims zero ports (stdio + in-process).
- Tool map locked: 7 `@mcp.tool` → 27 operations + 2 sampling helpers (device 9, spectrum 6, stations 5, online 2, gnuradio 5). Dead `fm_demod.py` native path excluded; HackRF TX stays docs-only (RX-only binary).
- Env inventory: `SDR_MCP_MOCK`, `SDR_WEB_API_HOST/PORT`, `MCP_TRANSPORT/HOST/PORT/PATH`, `RTL_TCP_HOST/PORT`, `GNURADIO_URL/service_url`, `DEMOD_HOST/PORT`, `SDR_AUDIO_MODE/HOST/PORT`, `WEB_PORT`; 11-station `frequency_db.py` → `stations.rs` fixtures.
- Fence: `readonly: bool` on retune/scan/websocket/gnuradio-start; range check 24–1766 MHz; scan cancellable off UI thread.
- Oracle plan: 11 legacy routes mapped to `Page` variants + golden pairs in `docs/parity/{legacy,rust}/`; Settings carries mandatory fleet LLM card (`gemma4:12b` default).
- Legacy `sdr-mcp@master@51ba1c1` verified untouched. Next: Phase 1 checkpoint (`cargo check --workspace`) then Phase 2 L1 `sdr.rs` + `import.rs`.

## [2026-10-18T20:30Z] Phase 1 Checkpoint (mechanical workspace stamp → compiling)
- Added manifests + scaffold sources: workspace `Cargo.toml` (deny lints, blessed stack minus hardware), `sdr-core` (`lib.rs` Status dispatch + Send test, `import.rs` Phase-1 `fleet_secret()->None`), L2 `main.rs` (clap `status`/`mcp-stdio`, dual-alias `sdr_device`, panic hook), L3 `main.rs`/`app.rs` (window + `--smoke`), `justfile`, `AGENTS.md`, `README.md`, `CHANGELOG.md` (v0.1.0).
- Stamp gaps found & fixed: `llm.rs` needs `crate::import` (added Phase-1 skeleton); eframe 0.37 `App::ui` (not `update`) + `use eframe::egui`.
- Gates: `cargo check` green (553 locked deps) · clippy `-D warnings` zero · fmt clean · `cargo test` 15/15 · stdio probe (`initialize→list→call`, alias routes, unknown→isError, garbage→-32700, no-reply to notifications, EOF clean). First-pipe failure was PS UTF-16 pipe encoding, not server code — file-redirect probe is the recorded one.
- Omitted per triage: `-store` crate (legacy waterfall is in-memory; historian = new scope, revisit in Phase 4 if wanted).
- Next: Phase 2 L1 domain protocol (`sdr.rs` rtl_tcp + sidecar, `stations.rs`, `online.rs`, full `import.rs`) — FRONTLINE session recommended (strict-Rust synthesis).

## [2026-10-18T21:04Z] Phase 2 Checkpoint (L1 domain core, cheap-tier pass)
- `import.rs` full: `MockSetting` legacy truth table, `${VAR:-default}`/`$VAR` expansion, `KEY=VALUE` parser (quotes/comments), `discover_legacy_root` (env `SDR_MCP_LEGACY_DIR` else `../sdr-mcp` sibling — zero hardcoded machine paths), `LegacyConfig::resolve` (explicit > env > legacy file > defaults).
- `stations.rs`: 11 seeded stations + 16 LW programs verbatim, `Band`/`StationKind`, pure queries (search/by_band/by_country/schedule/`current_program_at` + `chrono::Local` wrapper), `station_json` (100-char desc trim like legacy).
- `online.rs`: mirror discovery + `search_radio_browser` (limit clamp 1–100, 1h TTL, stale-on-failure) + `get_signal_info` (negative caching), 32 MiB caps, `parse_station`/`parse_signal_info` fixture-testable, UA `rust-sdr-mcp/0.1.0`.
- `sdr.rs`: `MIN/MAX_FREQ` 24–1766, gain `auto|0–49.6`, 4 LW presets + callsign/substring resolution, `scan_plan` (100-step cap verbatim), `MockIq` (xorshift64* + Box-Muller, 4 tones + 50 kHz drift — no `rand` crate), Hamming DFT + fftshift + dB (naive O(n²), `realfft` upgrade slated Phase 3), 100-line `Waterfall`, sidecar client (`/health|/status|/devices`, `/demod/start|stop` verbatim payload), `rtl_tcp_probe`, process singletons.
- `lib.rs`: 35-variant `Command` (27 dossier ops + Status + bridge/llm) + `dispatch` → typed `kind` errors. Deliberately deferred with named Phase-3 targets: direct USB (`rusb`), WS streaming (`tokio-tungstenite`), audio relay (`cpal`).
- Deny-lint lessons: no `expect`/`unwrap` even in tests (`assert!(x.is_ok())` + `if let` idiom), no `assert!(false)`, no `assert_eq!(bool, true)`, type aliases for deep generics.
- Gates: `cargo clippy -p sdr-core --all-targets -- -D warnings` zero · fmt clean · `cargo test --workspace` 49/49 (L1 46 incl. 31 new, L2 3) · mock-tone E2E (synthetic → spectrum peak @ ~200 kHz) green.
- Next: Phase 3 L2 (clap subcommands per `Command`, full dual-alias `call_tool`, catalog tests) — mechanical, cheap-tier suitable.

## [2026-10-18T21:12Z] Phase 3 Checkpoint (L2 CLI + dual-aliased MCP)
- `main.rs` rewritten around ONE table: `tools_list()` = 40 entries (35 ArtCraft `sdr.*`/`bridge.*`/`llm.*` + 5 legacy portmanteau), `call_tool` normalizes dotted/underscore/legacy to `dispatch_named`. Legacy arg renames verbatim (`frequency_mhz`→`mhz`, `preset_name`→`name`, `start_freq`→`start_mhz`, `station_callsign`→`callsign`) + verbatim `valid_operations` errors. Refactor lesson: first draft tripled the table across two fns — collapsed before commit.
- CLI: 11 groups (`device spectrum stations online gnuradio bridge llm assist hint status mcp-stdio`), all-JSON output; `assist`/`hint` take positionals.
- SOP tests in `main.rs`: `catalog_lists_40_tools`, `unknown_tool_becomes_error_content_not_panic`, `legacy_portmanteau_alias_routes_correctly` (offline-safe trio; online/gnuradio aliases covered by error-path tests, no network in unit tests), `underscore_form_routes_like_dotted`, `demo_round_trip` (search+spectrum+tune, zero hardware), `required_params_reject_missing_values` (5 cases), `unknown_legacy_operation_lists_valid_operations`.
- Gates: clippy `-D warnings` zero · fmt clean · `cargo test --workspace` 53/53 (L1 46, L2 7).
- Live probe (file-redirect, release-debug binary): `initialize`→2024-11-05, `tools/list`, granular `stations_by_band(LW)`→4 stations, legacy `sdr_device(tune_preset BBC LW)`→0.198 MHz, underscore `sdr_list_devices`, missing-`mhz`→isError. CLI: `stations search --query BBC`→3, `device tune-preset`→0.198, `assist` runbook, `spectrum get`→2048 bins.
- OBSERVED: TCP connect to `127.0.0.1:1234` SUCCEEDS on Goliath (`Test-NetConnection` True) — `DeviceList` reports live `rtl-tcp` device, `Auto` mock resolves to hardware path. Payload unverified (connect-only probe); Phase 5 will handshake the stream. Possible leftover `rtl_tcp` from the legacy setup — do NOT kill it (read-only fence; another owner's process).
- Next: Phase 4 L3 (`app.rs` full shell + 11 domain tabs from `docs/parity/legacy/`, `vault`/`registry` wiring, oracle screenshots, kittest goldens).

## [2026-10-18T21:26Z] Phase 4 Checkpoint (L3 egui app, no-browser session)
- `app.rs` (~2000 lines): fleet chrome (harsh/crisp themes via `style_of`/`set_style_of`, vector Sun/Moon, Ctrl+Wheel zoom + reset pill, `mpsc` spawn/drain, boot latch, ui.json persistence) + 12 pages (11 legacy + Raw JSON) + custom painters (spectrum polyline + peak, rect waterfall, heat ramp, pills) + fleet LLM card (5 providers, elicit, vault key, test) + registry adopt + bridge quick-send + capped log buffer.
- Fresh-write, not shell-copy (deviation logged): adapting the 1900-line device-specific blueprint by edit would cost more tokens and risk leftover device semantics; chrome requirements implemented 1:1, verified against blueprint seams by grep.
- 0.37 API corrections from compiler + registry source (ground truth, not memory): `Panel::top/left` (no SidePanel/TopBottomPanel), panels take `&mut Ui`, `style_of`/`set_style_of` (no `style()`/`set_style`), `App::ui` root Ui. Blueprint's `style_mut_of` does NOT exist in 0.37 — its app.rs cannot compile as-is; flagged for the fleet (blueprint drift).
- Deny-lint fixes: runtime builds via match+exit(1) (no expect), E0500 clone-before-closure, `step_by(0)` panic guard, dead-code wiring (scan results, detail card, demod Start, RED log lines, vault slot trim).
- `glyph_source_scan` caught my own `§`/`→` in comments — purged; file pure ASCII.
- kittest: harness renders headless fine; "Missing snapshot" blessed via `UPDATE_SNAPSHOTS=1` → `hero_kpi.png`, `pills.png`, `spectrum_demo.png` (Demo IQ to FFT to canvas E2E). Re-run green without the flag.
- Gates: clippy `-D warnings` zero · fmt clean · `cargo test --workspace` 59/59 (L1 46, L2 7, L3 6: scan+glyph+3 goldens+2 registry) · `--smoke` ok.
- `docs/UI_PARITY.md`: 11-row page map (code-matched), 8 written deviations, oracle capture checklist. Legacy-pixel diff PENDING (no browser tooling this session).
- Next: Phase 5 (6-gate run, read-only live probe incl. `:1234` handshake, NSIS + GitHub release v0.4.0).