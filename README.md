# SDRCraft

Receive-only desktop radio workstation in pure Rust (`sdrcraft-core`, `sdrcraft-mcp`, `sdrcraft-app`). Replaces `rust-sdr-mcp` (which converted the Python `sdr-mcp` server). There is no path dependency on either source repo. The binary does not transmit.

## Feature Matrix

| Area | Status | Notes |
|---|---|---|
| Core command bus (`sdrcraft-core`) | Implemented | 36 `Command` variants, typed errors, no panics |
| CLI + MCP stdio (`sdrcraft-mcp`) | Implemented | 43 MCP tools (ArtCraft + legacy portmanteau aliases) |
| Desktop UI (`sdrcraft-app`) | Implemented | 12-page `eframe 0.37` app, dark/light themes, kittest goldens |
| Mock IQ + 2048-pt spectrum + waterfall | Implemented | Deterministic synthetic IQ (`MockIq`), `rustfft` spectrum, 100-line waterfall |
| Seeded station DB + schedules | Implemented | 11 stations across LW/MW/SW/VHF/UHF |
| Radio Browser + Signal ID | Implemented | Online search + Signal Identification Wiki lookup |
| Morse / RTTY / APRS decoders | Partial | Real decoders on built-in practice tapes (`live: false`); live air audio pending |
| GNU Radio demod sidecar client | Implemented | HTTP client for `/health`, `/status`, `/devices`, `/demod/start|stop` |
| `rtl_tcp` IQ streaming | Implemented | `RTL0` handshake, tuning commands, u8 IQ capture (`rtl_tcp.rs`) |
| `rustfft` spectrum upgrade | Implemented | 2048-pt Hamming-windowed `rustfft` forward FFT (`sdr.rs`) |
| `cpal` FM/AM audio relay | Implemented | Polar FM + envelope AM demodulator + `cpal` output stream (`audio.rs`) |
| `tokio-tungstenite` WebSocket stream | Implemented | Live JSON spectrum broadcast server (`ws.rs`) |
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

## Hardware & Buying Guide

Cheap USB sticks built on the **RTL2832U + R820T2** TV-tuner chip (8-bit ADC, ~2.4 MHz bandwidth) work for casual VHF/UHF (broadcast FM, airband, 1090 MHz ADS-B) but overload easily near strong local transmitters and stop at ~24 MHz. For wide coverage and real sensitivity:

| Receiver | Coverage | ADC / Bandwidth | Notes |
|---|---|---|---|
| **SDRplay RSP1B** *(Recommended)* | **1 kHz - 2 GHz** | **14-bit**, up to 10 MHz | Steel shielded case, hardware preselection + AM/FM/DAB notch filters, 4.7V Bias-T. Uses proprietary SDRplay API v3 (`SoapySDRPlay3` / GNU Radio sidecar for `sdrcraft`). |
| **Airspy HF+ Discovery** | 9 kHz - 31 MHz & 60 - 260 MHz | High-DR HF + VHF | Benchmark HF (< 31 MHz) sensitivity and blocking resistance, open drivers. **No UHF** (no 70cm/446 MHz/1090 MHz ADS-B). |
| **RTL-SDR Blog v4** | 0.5 - 1766 MHz | 8-bit, ~2.4 MHz | Best budget stick: built-in HF direct sampling and Bias-T, still 8-bit dynamic range. |
| **Plain R820T2 stick** (e.g. Nooelec NESDR Nano 2 Plus) | ~25 - 1750 MHz | 8-bit, ~2.4 MHz | TV-tuner heritage: deaf below ~24 MHz without a 125 MHz upconverter (Ham It Up). |
| **HackRF One** | 1 MHz - 6 GHz | 8-bit, 20 MHz | Transceiver (TX-capable). High noise floor / poor weak-signal RX compared to RSP1B or Airspy; skip for RX-only listening. |

**Antennas & Setup**:
- **Windows RTL-SDR**: Install the WinUSB driver via Zadig and serve the stick with `rtl_tcp` on port `1234` (or use Demo Mode with no hardware connected).
- **Antennas**: Move any whip off the laptop onto a USB extension at a window. For VHF/UHF use a telescopic dipole or discone; for HF (< 24 MHz) use an outdoor/window long wire or a small indoor magnetic loop. See [docs/TUNING_RANGE.md](docs/TUNING_RANGE.md) and [docs/ANTENNAS.md](docs/ANTENNAS.md).

