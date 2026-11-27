# SDRCraft Help

## Get Started

SDRCraft listens. Plug in an RTL-SDR stick, or start with no hardware: mock mode still draws a spectrum and a waterfall. Launch with `start.ps1`. The left sidebar is the page list. View, then Dark or Light, switches the theme. View, then English or Deutsch, switches the few labels that are translated.

## Functions

Spectrum, waterfall, the seeded station list, and Radio Browser search all go through the same command bus the CLI and the MCP server use. Chat sends a prompt only after a model name is filled in. An empty model shows "no model selected" and does not call out.

## Usage

CLI: `sdrcraft-mcp status` prints JSON. MCP: `sdrcraft-mcp mcp-stdio`. The window smoke check is `sdrcraft --smoke`. Commands, then Refresh status, asks the core for liveness without a dongle.

## Below 24 MHz

A plain RTL-SDR stick stops near 25 MHz because its R820T2 tuner was built for television. The Nooelec NESDR Nano 2 Plus is that kind of stick: about 24 x 21 x 8 mm, 0.5 ppm TCXO, MCX jack, roughly 25-1750 MHz. Medium wave and shortwave on the stick itself need an RTL-SDR Blog v4, from about 0.5 MHz. Longwave (BBC and ORF on 198 kHz) is still below that. A Ham It Up v1.3 or Nano hears from 100 kHz to 65 MHz. The Plus hears from 300 Hz. All three shift by 125 MHz, so this window tunes BBC 198 kHz as 125.198 MHz, AM. The upconverter needs its own USB socket, and passthrough mode when you go back to FM. Detail is in `docs/TUNING_RANGE.md`.

## Hardware & Buying Guide

Cheap mini sticks use an 8-bit RTL2832U + R820T2 TV tuner chip: fine for casual FM, airband, or 1090 MHz ADS-B, but they overload near strong local transmitters and lack sensitivity on HF.

- **SDRplay RSP1B (Recommended)**: 1 kHz to 2 GHz continuous, 14-bit ADC, up to 10 MHz bandwidth, steel case, automatic preselection, MW/FM and DAB notch filters, and a 4.7 V Bias-T that stays off unless the antenna asks for it. Listen in SDRconnect. This window does not open the radio yet. The procedure is `docs/RSP1B.md`.
- **Airspy HF+ Discovery**: Benchmark HF (< 31 MHz) sensitivity with open drivers, covering 9 kHz-31 MHz and 60-260 MHz. No UHF (no 70cm, PMR446, or 1090 MHz ADS-B).
- **RTL-SDR Blog v4**: Best budget stick (0.5-1766 MHz with built-in HF direct sampling and Bias-T), still 8-bit dynamic range.
- **Plain R820T2 stick** (e.g. Nooelec NESDR Nano 2 Plus): ~25-1750 MHz only; needs a Ham It Up upconverter (+125 MHz) for HF/MW/LW.
- **HackRF One**: 1 MHz-6 GHz 8-bit transceiver. High noise floor / weak RX sensitivity compared to RSP1B or Airspy; skip for RX-only listening.

## Using the RSP1B

Install SDRconnect from sdrplay.com before you plug the radio in. The installer also installs the API service. Use a USB cable that carries data. The socket on the radio is type B. Press Play. Set the sample rate to 2 MSPS so the ADC stays 14-bit. Leave IF AGC on and Bias-T off.

Prove it on a local FM station: mode WFM, bandwidth about 200 kHz, somewhere in 88-108 MHz. Then hang a wire for longwave. BBC and ORF on 198 kHz are AM, 9 kHz bandwidth. The whip will not hear that. RF Notch cuts medium wave and FM, so turn it off while you are listening to those bands, and on when a local FM tower is burying shortwave. One program owns the radio. Stop SDRconnect before anything else opens it.

SDRCraft's Tune button still moves an RTL-SDR stick, or the demo spectrum. It does not move the RSP1B. Morse and FT8 from this radio go out of SDRconnect into fldigi or WSJT-X through a virtual audio cable. Detail, gain steps, and the station table are in `docs/RSP1B.md`.

## Morse, text, and handhelds

The Listen page copies a Morse tape, a RTTY tape, and an APRS practice frame. The report says the copy is not live: the stick's audio is not in the window yet. PSK31, FT8, and JS8 stay with fldigi and WSJT-X. APRS at 144.800 MHz is the text net you can already tune. You see bursts. The printed APRS line is the practice frame.

A Japanese flip phone's TV antenna is 1seg. Tokyo terrestrial TV is still on from Skytree. The 1seg program picture continues after the 2026 shutdown of the extra data pages. This app will not show it.

Austrian license-free walkie-talkies are PMR446 near 446 MHz, short range, and private. The slow text webs are APRS, the amateur MeshCom mesh on 70 cm, and license-free Meshtastic on 868 MHz. Private Meshtastic channels and MeshCore are encrypted. Detail is in `docs/MORSE_TEXT_AND_HANDHELDS.md`.

## Police and TV

Police scanner chatter is mostly over. In the analog years a pursuit really was on ordinary FM. In Austria, Germany, and most of Europe that voice moved to encrypted TETRA. A whip hears nothing, and removing the encryption is a crime. This app will not try. Parts of the United States still have unencrypted fire, EMS, and some small-town police; big-city police channels are often encrypted digital. The lawful, still-audible listens are broadcast FM, shortwave broadcasts, and amateur radio.

Terrestrial TV is digital now (DVB-T/T2 in Europe, ATSC in North America). Analog TV is gone. A channel is wider than this stick can sample, so the spectrum can show a hump around 470-694 MHz and cannot show a picture. The same chip used to be a TV tuner before the SDR driver replaced that mode. Pay TV is encrypted on top of the broadcast. Detail is in `docs/POLICE_AND_TV.md`.

## Antennas

The whip in the box is why the first session feels empty. It is long enough for a few strong local FM stations and, sometimes, nearby aircraft on 1090 MHz. It is the wrong size for shortwave, it sits in the laptop's electrical noise, and a vertical rod mismatches many horizontal FM broadcasts. Put the stick on a USB extension at a window before you blame the dongle.

After that, antennas scale with the wavelength you care about. A two-rod dipole kit is the indoor upgrade. An outdoor vertical or a discone opens VHF and UHF (airband, repeaters, ADS-B). A wire between trees, usually a dipole or a single long wire, is what makes shortwave readable. A Yagi or a log-periodic points at one direction and ignores the rest. A rhombic is that idea stretched into a wire diamond the size of a field. A small loop is the indoor HF answer when you cannot hang wire. The long version, with sizes, is `docs/ANTENNAS.md`.

## Tips

A thumb-stick cannot transmit. Larger USB boxes (HackRF, Pluto, LimeSDR Mini) can, and a ham license still applies on amateur bands. That path is not in this binary. If you are going somewhere with no cellular and no satellite terminal, carry a registered 406 MHz personal locator beacon. The note is `docs/TRANSMIT_BOXES_AND_BEACONS.md`.

## Other Crafts

SDRCraft sits beside the other desktop craft apps (DJCraft, GisCraft, and the rest of the workshop). It does not share their crates. Feedback for the community beta belongs on the ArtCraft Discord once the repo is public.
