# HANDOVER - `sdrcraft`

**Last Updated:** 2026-11-21
**Current Phase:** Phase 3 (Hardware & Real-Time DSP: `realfft`, `rtl_tcp` IQ streaming, `cpal` audio, `tokio-tungstenite` WebSocket)

## Context
- `sdrcraft` replaces `rust-sdr-mcp` (vendored from `rust-sdr-mcp`, originally converted from Python `sdr-mcp`).
- Receive-only desktop radio workstation (no `Tx` command by construction).

## Recent Changes
- Took over Cursor WIP (`decode.rs` practice tapes for Morse/RTTY/APRS, Blog v4 0.5 MHz floor, Listen page + Help tabs + 4 band/antenna docs).
- Fixed CI gates: `ATTRIBUTION.md` for all PNGs, `WGPU_BACKEND=vulkan` for Windows `egui_kittest` snapshots, `installer/installer.nsi` renamed to `sdrcraft`, `README.md` feature matrix added.
