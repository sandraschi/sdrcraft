# SDRCraft

Receive-only desktop radio workstation in pure Rust (`sdrcraft-core`, `sdrcraft-mcp`, `sdrcraft-app`). Replaces `rust-sdr-mcp` (which converted the Python `sdr-mcp` server). There is no path dependency on either source repo. The binary does not transmit.

## Feature Matrix

| Area | Status | Notes |
|---|---|---|
| Core command bus (`sdrcraft-core`) | Implemented | 36 `Command` variants, typed errors, no panics |
| CLI + MCP stdio (`sdrcraft-mcp`) | Implemented | 41 MCP tools (ArtCraft + legacy portmanteau aliases) |
| Desktop UI (`sdrcraft-app`) | Implemented | 12-page `eframe 0.37` app, dark/light themes, kittest goldens |
| Mock IQ + 2048-pt spectrum + waterfall | Implemented | Deterministic synthetic IQ (`MockIq`), 100-line waterfall |
| Seeded station DB + schedules | Implemented | 11 stations across LW/MW/SW/VHF/UHF |
| Radio Browser + Signal ID | Implemented | Online search + Signal Identification Wiki lookup |
| Morse / RTTY / APRS decoders | Partial | Real decoders on built-in practice tapes (`live: false`); live air audio pending |
| GNU Radio demod sidecar client | Implemented | HTTP client for `/health`, `/status`, `/devices`, `/demod/start|stop` |
| `rtl_tcp` IQ streaming | Planned | Phase 3 (currently TCP reachability probe only) |
| `realfft` spectrum upgrade | Planned | Phase 3 (currently $O(n^2)$ DFT) |
| `cpal` FM/AM audio relay | Planned | Phase 3 (currently returns explicit `unsupported`) |
| `tokio-tungstenite` WebSocket stream | Planned | Phase 3 (currently returns explicit `unsupported`) |
| Direct USB RTL-SDR (`rusb`) | Planned | Later phase |

## Layout

```text
sdrcraft/
├── Cargo.toml / Cargo.lock / justfile
├── docs/CONVERSION_DOSSIER.md           # 7-point forensic recon + 1:1 tool map
├── docs/CONVERSION_LOG.md               # append-only execution trail
├── docs/TRANSMIT_BOXES_AND_BEACONS.md   # larger TX boxes (docs only) + explorer PLB
├── docs/ANTENNAS.md                     # whip through rhombic, and why the first session is quiet
├── docs/POLICE_AND_TV.md                # scanner myth, TETRA, and why TV is a hump not a picture
├── docs/MORSE_TEXT_AND_HANDHELDS.md     # CW/RTTY, 1seg, PMR446, APRS and MeshCom
├── docs/TUNING_RANGE.md                 # why 0.5/24 MHz, Blog v4, Airspy HF+, SDRplay
├── installer/installer.nsi              # current-user NSIS installer
└── crates/
    ├── sdrcraft-core/  # L1: bridge, llm (gemma4:12b), error, creds, import, decode, sdr, stations, online
    ├── sdrcraft-mcp/   # L2: clap CLI + MCP 2024-11-05 stdio loop
    └── sdrcraft-app/   # L3: eframe 0.37 window + --smoke + vault/registry
```

## Commands

```powershell
cargo run -p sdrcraft-mcp -- status          # liveness, JSON on stdout
cargo run -p sdrcraft-mcp -- mcp-stdio       # MCP server (pipe JSON-RPC)
cargo run -p sdrcraft-app -- --smoke         # headless state boot, exit 0
cargo run -p xtask -- ci                     # fmt, clippy, test, assets, layers, wasm, deny
just check; just test; just fmt-check        # dev gates
```
