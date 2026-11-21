# CHANGELOG - sdrcraft

## v0.1.0 - 2026-11-21 - Initial SDRCraft release (Phase 3 hardware & DSP)

- **Workspace**: Vendored from `rust-sdr-mcp` as `sdrcraft-core` (L1), `sdrcraft-mcp` (L2 CLI + MCP stdio), and `sdrcraft-app` (L3 `eframe 0.37` desktop UI). Receive-only by construction (no `Tx` command).
- **Phase 3 DSP & Hardware**:
  - `rustfft` 2048-pt Hamming-windowed spectrum in `sdr.rs`.
  - `rtl_tcp` IQ-over-TCP client (`RTL0` header, big-endian tuning commands, u8 IQ decode) in `rtl_tcp.rs`, wired into `spectrum_get` and `device_scan`.
  - `tokio-tungstenite` WebSocket spectrum broadcaster in `ws.rs`.
  - `cpal` FM/AM audio relay with polar-discriminator FM and envelope AM demodulators in `audio.rs`, plus `sdr.audio_start` and `sdr.audio_stop` (43 MCP tools total).
- **Decoders & Docs**:
  - Added `decode.rs` practice-tape decoders (Morse, RTTY, AX.25 APRS, `live: false`) and `sdr.decode_copy`.
  - Lowered Blog v4 direct-sampling floor to `0.5 MHz` with Ham It Up 125 MHz shift guidance.
  - Added `docs/ANTENNAS.md`, `docs/MORSE_TEXT_AND_HANDHELDS.md`, `docs/POLICE_AND_TV.md`, `docs/TUNING_RANGE.md`, and `ATTRIBUTION.md` asset tracking.
- **Release & CI**:
  - Updated `installer/craft-installer.nsi` + `installer/installer.nsi` for `sdrcraft.exe` / `sdrcraft-mcp.exe`.
  - Configured `.cargo/config.toml` (`WGPU_BACKEND=vulkan`) so `egui_kittest` snapshots pass on Windows.
  - Gates: `cargo xtask ci` green, `73/73` workspace tests passing (`60` core, `7` mcp, `6` app).

---

### Prior vendored history (`rust-sdr-mcp`)
- 2026-10-18 Phase 0/1: `New-RustArtcraftRepo.ps1 -Domain sdr` scaffold + `Command::dispatch` + MCP stdio.
- 2026-10-18 Phase 2: `import.rs`, `stations.rs` (11 seeded stations), `online.rs` (Radio Browser + SigID Wiki), `sdr.rs` (MockIq, Waterfall, GNU Radio sidecar client).
- 2026-10-18 Phase 3 (L2): CLI + dual-aliased MCP server (`sdr.*` + legacy portmanteau aliases).
- 2026-10-18 Phase 4 (L3): 12-page `egui 0.37` desktop app + vault/registry + 3 kittest goldens.
