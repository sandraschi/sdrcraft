# UI Parity: `sdr-app` vs legacy `web_sota` (11 routes)

Oracle sources read (never `node_modules/`): `components/layout/sidebar.tsx`
(nav order + labels), `App.tsx` (routes), `pages/*.tsx` (groups, order,
labels, gestures). Legacy-pixel screenshots (`docs/parity/legacy/`) do NOT
exist yet — no browser tooling in this session — so the kittest goldens below
are **implementation snapshots**, and each row's `oracle` column records what
was matched from code reading vs what still needs a pixel diff.

## Page map (legacy order)

| # | Legacy route + label | Rust `Page` | Groups / controls ported | Oracle |
|---|---|---|---|---|
| 1 | `/` Overview | Overview | Hero eyebrow/H1/explainer, 3 CTAs (Open Spectrum / Read Help / AI Command), 3 feature cards (Listen / See signals / Ask AI), Mock-IQ banner + dongle-help link, Live status 4 KPIs (RTL-SDR Detected/Mock/Not found + count, GNU Radio Online/Offline, Center Freq MHz, MCP Bridge Online), Refresh | code-matched; pixel diff pending |
| 2 | `/spectrum` Spectrum | Spectrum | Canvas + peak readout, center-freq tune, gain auto/dB, scan (start/end/step, 100-step note), results list | code-matched; pixel diff pending |
| 3 | `/waterfall` Waterfall | Waterfall | History view newest-bottom, Refresh-adds-row, Clear, row count | code-matched; pixel diff pending |
| 4 | `/stations` Stations | Stations | Search box, band combo (All/LW/MW/SW/VHF/UHF), results + Tune/Schedule per row, Quick Presets (4 LW), schedule (callsign+day, now-playing), manual tune, online search, signal-ID | code-matched; pixel diff pending |
| 5 | `/tools` Tools | Tools | Per-group cards (Device/Spectrum/Stations/GNU Radio) with buttons, Active Capabilities card | code-matched; pixel diff pending |
| 6 | `/status` Status | Status | 4 telemetry cards (RTL-SDR/Mock IQ/GNU Radio/Activity), Health Check Details raw card, Refresh | code-matched; pixel diff pending |
| 7 | `/apps` App Hub | AppHub | Fleet Registry (adopted targets + adopt form), bridge quick-send (event/speak/webhook) | code-matched; pixel diff pending |
| 8 | `/chat` AI Command | Chat | Log, input + Send/Enter, provider/model line, Clear | code-matched; pixel diff pending |
| 9 | `/help` Help | Help | 6 primer tabs (tune/spectrum/stations/mock/dongle/HackRF-TX) | code-matched; pixel diff pending |
| 10 | `/settings` Settings | Settings | Fleet LLM card (5 providers, base URL, model + elicit, vault key, test pill, save), SDR targets (read-only import + env hint) | code-matched; pixel diff pending |
| 11 | `/logs` Logs | Logs | Search filter, Clear all, capped 200-line buffer, error lines in red | code-matched; pixel diff pending |
| + | — (fleet chrome) | Raw | Latest sdr-core payload as JSON | fleet addition, no legacy counterpart |

## Deliberate deviations (not fails)

1. **Text nav, no Lucide icons** — egui has no icon font; labels are verbatim.
2. **Stations sections instead of tabs** — same groups, same order, stacked scroll (tabs add state for zero information gain).
3. **Blueprint Crossconnect merged into App Hub** — adopted targets + bridge quick-send in one place; no separate page.
4. **No 5 s auto-poll on Overview** — boot latch loads once + Refresh button (per-frame polling violates the no-task-storm law); pending-job counter shows activity instead.
5. **Chat answers via `llm.rs`** — legacy regex NLU (`web_api.py`) is retired; the page keeps input/log/clear gestures with a real model behind them.
6. **Settings drops the Web API Bridge URL card** — no HTTP bridge exists anymore (in-process `dispatch`); SDR targets card shows the resolved import instead.
7. **Waterfall/spectrum are painted vectors** — no canvas/texture crates; heat ramp + polyline match the information content, not Tailwind pixels.
8. **Font rasterization + CSS glow** — one written exception each (egui bundled fonts; no blur-backdrop).

## Goldens (kittest, `crates/sdr-app/tests/snapshots/`)

- `hero_kpi.png`, `pills.png`, `spectrum_demo.png` (L1 Demo IQ to FFT to canvas E2E).
- Blessed 2026-10-18 via `UPDATE_SNAPSHOTS=1` after code-review of the renders.
- `glyph_source_scan` green: `app.rs` is pure ASCII (allowlist kept for parity with the fleet shell).

## To close the oracle (needs a browser session)

Capture at 1440x900 into `docs/parity/legacy/<page>.png` (+ `-<gesture>.png`
for tune, gain, scan, preset, tab-switch, chat-send, elicitation), then flip
each row above to `pixel-matched` and re-bless goldens only where they differ.
