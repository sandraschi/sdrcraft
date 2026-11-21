# SDRCraft

Receive-only desktop radio workstation. This repo vendors the Rust workspace
from `rust-sdr-mcp` (the conversion of the Python `sdr-mcp` server) under the
crate names `sdrcraft-core`, `sdrcraft-mcp`, and `sdrcraft-app`. There is no
path dependency on either source repo. The binary does not transmit.

# sdrcraft-mcp - vendored workspace notes

Shadow conversion of [`sdr-mcp`](../sdr-mcp) (Python/FastMCP + React, ports
`11118/11119`) to a single-language Rust workspace: L1 `sdrcraft-core` drivers +
`Command::dispatch`, L2 `sdrcraft-mcp` CLI + MCP stdio server, L3 `sdrcraft-app`
`egui` desktop app. Zero ports claimed (stdio + in-process calls).

> **Stage: Phase 4 L3 (v0.1.0).** 12-page `egui` app (11 legacy + Raw),
> fleet chrome + LLM card + vault/registry, 3 kittest goldens, 59/59 tests.
> Legacy-pixel oracle diff pending (no browser this session). Next: Phase 5
> release. Spec: `docs/CONVERSION_DOSSIER.md`, parity: `docs/UI_PARITY.md`.

## Layout

```text
sdrcraft-mcp/
├── Cargo.toml / Cargo.lock / justfile
├── docs/CONVERSION_DOSSIER.md  # 7-point forensic recon + 1:1 tool map
├── docs/CONVERSION_LOG.md      # append-only execution trail
├── docs/TRANSMIT_BOXES_AND_BEACONS.md  # larger TX boxes (docs only) + explorer PLB
├── installer/installer.nsi     # current-user NSIS (Phase 5)
└── crates/
    ├── sdrcraft-core/  # bridge, llm (gemma4:12b), error, creds, import, lib (dispatch)
    ├── sdrcraft-mcp/   # clap CLI + MCP 2024-11-05 stdio loop
    └── sdrcraft-app/   # eframe 0.37 window + --smoke (+ vault/registry in Phase 4)
```

## Commands

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo run -p sdrcraft-mcp -- status          # liveness, JSON on stdout
cargo run -p sdrcraft-mcp -- mcp-stdio       # MCP server (pipe JSON-RPC)
cargo run -p sdrcraft-app -- --smoke              # headless state boot, exit 0
just check; just test; just fmt-check        # the 3 dev gates
```

## Live verification (Phase 1, 2026-10-18)

| Probe | Result |
|---|---|
| `cargo check --workspace` | green (553 locked deps) |
| `cargo clippy --all-targets -- -D warnings` | zero warnings |
| `cargo test --workspace` | 15/15 pass (12 kernel + 3 scaffold) |
| `cargo fmt --all --check` | clean |
| stdio `initialize → tools/list → tools/call` | `2024-11-05`, 1 tool, legacy `sdr_device(status)` alias routes, unknown tool → `isError`, garbage → `-32700`, session survives |
| Legacy `../sdr-mcp@master@51ba1c1` | untouched (`git status` clean) |
