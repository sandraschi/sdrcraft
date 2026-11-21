# CHANGELOG — rust-sdr-mcp

## v0.1.0 — 2026-10-18 — Phase 0/1: stamp + compiling scaffold

- `New-RustArtcraftRepo.ps1 -Domain sdr`: 70% kernel from `rust-devices-mcp`
  (`bridge`, `llm` @ `gemma4:12b`, `error`, `creds`, `vault`, `registry`).
- `docs/CONVERSION_DOSSIER.md`: Bucket B verdict, port/contention audit,
  27-op + 2-helper tool map, env inventory, `readonly` fence, 11-page oracle.
- 3-crate workspace (no `-store`: waterfall history is in-memory in legacy):
  `sdr-core` (`Status` dispatch + `Send` test), `rust-sdr-mcp` (CLI +
  dual-aliased stdio), `sdr-app` (window + `--smoke`).
- Gates: check / clippy `-D warnings` / fmt / 15 tests green; stdio probe
  (`initialize → tools/list → tools/call`, alias + `-32700` behavior) passes.
- Next: Phase 2 L1 (`sdr.rs`, `stations.rs`, `online.rs`, full `import.rs`).

## v0.1.0 — 2026-10-18 — Phase 2: L1 domain protocol (this repo state)

- `import.rs`: legacy truth tables + `${VAR:-default}` expansion + dotenv
  parser + `LegacyConfig::resolve` (explicit > env > legacy file > default).
- `stations.rs`: 11 stations + 16 LW programs verbatim; pure offline queries.
- `online.rs`: radio-browser + SigID Wiki with TTL + stale fallback + caps.
- `sdr.rs`: validation ranges, presets, 100-step scan planner, deterministic
  mock IQ, Hamming DFT spectrum, 100-line waterfall, sidecar HTTP client,
  `rtl_tcp` probe. Direct USB / WS streaming / audio relay explicitly
  deferred to Phase 3 (`rusb`, `tokio-tungstenite`, `cpal`).
- `lib.rs`: 35-variant `Command` + `dispatch` with typed `kind` errors.
- Gates: clippy `-D warnings` zero · fmt clean · 49/49 tests (L1 46, L2 3).
- Next: Phase 3 L2 (full clap + dual-alias `call_tool`).

## v0.1.0 — 2026-10-18 — Phase 3: L2 CLI + dual-aliased MCP (this repo state)

- 40-tool catalog: 35 ArtCraft (`sdr.*`, `bridge.*`, `llm.*`) + 5 legacy
  portmanteau aliases; dotted/underscore/legacy forms all route to one
  `dispatch_named` table (legacy arg renames + verbatim `valid_operations`).
- 11-group JSON CLI (`device spectrum stations online gnuradio bridge llm
  assist hint status mcp-stdio`).
- Gates: clippy `-D warnings` zero · fmt clean · 53/53 tests (L1 46, L2 7).
- Live probe: stdio (`initialize→list→call`, granular + legacy + underscore
  + missing-param isError) and CLI both green. `127.0.0.1:1234` accepts TCP
  on Goliath (unverified payload; Phase 5 handshakes it).
- Next: Phase 4 L3 (`egui` shell + 11 domain tabs + oracle + goldens).

## v0.1.0 — 2026-10-18 — Phase 4: L3 desktop app (this repo state)

- 12-page `egui` 0.37 app: fleet chrome (themes, vector Sun/Moon, Ctrl+Wheel
  zoom, spawn/mpsc, boot latch, ui.json), 11 legacy pages + Raw JSON,
  custom painters (no new crates), fleet LLM card, vault-backed key,
  registry adopt, bridge quick-send, capped logs.
- `docs/UI_PARITY.md`: code-matched page map + 8 written deviations; pixel
  oracle pending (browser capture checklist included).
- Gates: clippy `-D warnings` zero · fmt clean · 59/59 tests · 3 goldens ·
  glyph scan green · `--smoke` ok.
- Next: Phase 5 (6-gate + live probe + NSIS + GitHub release v0.4.0).
