# SDRCraft Help

## Get Started

SDRCraft listens. Plug in an RTL-SDR stick, or start with no hardware: mock mode still draws a spectrum and a waterfall. Launch with `start.ps1`. The left sidebar is the page list. View, then Dark or Light, switches the theme. View, then English or Deutsch, switches the few labels that are translated.

## Functions

Spectrum, waterfall, the seeded station list, and Radio Browser search all go through the same command bus the CLI and the MCP server use. Chat sends a prompt only after a model name is filled in. An empty model shows "no model selected" and does not call out.

## Usage

CLI: `sdrcraft-mcp status` prints JSON. MCP: `sdrcraft-mcp mcp-stdio`. The window smoke check is `sdrcraft --smoke`. Commands, then Refresh status, asks the core for liveness without a dongle.

## Tips

A thumb-stick cannot transmit. Larger USB boxes (HackRF, Pluto, LimeSDR Mini) can, and a ham license still applies on amateur bands. That path is not in this binary. If you are going somewhere with no cellular and no satellite terminal, carry a registered 406 MHz personal locator beacon. The note is `docs/TRANSMIT_BOXES_AND_BEACONS.md`.

## Other Crafts

SDRCraft sits beside the other desktop craft apps (DJCraft, GisCraft, and the rest of the workshop). It does not share their crates. Feedback for the community beta belongs on the ArtCraft Discord once the repo is public.
