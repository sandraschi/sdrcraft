# v0.1.0 - 2026-11-21

Initial SDRCraft v0.1.0 release (receive-only desktop radio workstation in pure Rust).

- **Core & Hardware**: `rustfft` 2048-pt spectrum, `rtl_tcp` IQ-over-TCP streaming (`RTL0` handshake + u8 IQ), `tokio-tungstenite` WebSocket spectrum server, and `cpal` FM/AM audio relay.
- **CLI & MCP**: 43 MCP tools (`sdrcraft-mcp`) covering device control, spectrum, waterfall, WebSocket/audio relay, offline station DB, Radio Browser, Signal ID Wiki, GNU Radio sidecar, and practice-tape Morse/RTTY/APRS decoders (`sdr.decode_copy`).
- **Desktop App**: 12-page `eframe 0.37` GUI (`sdrcraft`), dark/light themes, Listen quick-tune cards, spectrum/waterfall plots, and Help tabs (`docs/TUNING_RANGE.md`, `docs/ANTENNAS.md`, `docs/POLICE_AND_TV.md`, `docs/MORSE_TEXT_AND_HANDHELDS.md`).
- **Verification**: `cargo xtask ci` green, 73/73 tests passing.
