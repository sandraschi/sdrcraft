# Signal Atlas (10 kHz to 1.7 GHz)

Field guide to real-world signals you can actually receive with SDRCraft, what hardware front-end they need, and which antenna works best.

| Band / Service | Frequency | Mode | Min Hardware | Antenna |
|---|---|---|---|---|
| DCF77 Time Signal (Germany) | 77.5 kHz | AM / Phase | RSP1B or Upconverter (+125 MHz) | Ferrite loop / long wire |
| DDH47 Weatherfax (Hamburg) | 147.3 kHz | FSK (RTTY/FAX) | RSP1B or Upconverter (+125 MHz) | Long wire / active loop |
| European Longwave Broadcast | 162 - 234 kHz | AM | RSP1B or Upconverter (+125 MHz) | Long wire / magnetic loop |
| Medium Wave Broadcast | 531 - 1602 kHz | AM | Blog v4 or RSP1B | Long wire / loop (best at night) |
| 80m Amateur Band | 3.500 - 3.800 MHz | LSB / CW | Blog v4 or RSP1B | Wire dipole (~40m) / loop |
| 49m / 41m Shortwave Broadcast | 5.900 - 7.450 MHz | AM | Blog v4 or RSP1B | Wire between trees |
| 40m Amateur FT8 / CW | 7.074 MHz (FT8) | USB (FT8) / CW | Blog v4 or RSP1B | Wire dipole (~20m) / loop |
| CHU Canada Time Signal | 7.850 / 14.670 MHz | USB / AM | Blog v4 or RSP1B | Wire / loop |
| WWV / WWVH Time Signal | 10.000 / 15.000 MHz | AM | Blog v4 or RSP1B | Wire / loop |
| HF Oceanic Aero / VOLMET | 5.505 / 8.957 / 11.279 MHz | USB | Blog v4 or RSP1B | Wire / loop |
| 20m Amateur FT8 / WSPR | 14.074 / 14.0956 MHz | USB | Blog v4 or RSP1B | Wire dipole (~10m) / loop |
| CB Radio (Europe) | 26.965 - 27.405 MHz | FM / AM | Any RTL-SDR / RSP1B | Vertical whip / dipole |
| FM Broadcast | 87.5 - 108.0 MHz | WFM | Any RTL-SDR / RSP1B | Telescopic dipole (~75 cm/leg) |
| VHF Airband (Tower / ATIS) | 118.0 - 137.0 MHz | AM | Any RTL-SDR / RSP1B | Vertical whip / discone |
| NOAA Weather Satellites (APT) | 137.10 / 137.62 / 137.91 MHz | WFM (APT) | Any RTL-SDR / RSP1B | V-dipole (~53 cm/leg, 120 deg) |
| APRS Packet Text (Europe) | 144.800 MHz | NFM (AX.25) | Any RTL-SDR / RSP1B | Vertical / 2m dipole |
| Amateur 2m Repeaters / Calling | 145.500 MHz | NFM | Any RTL-SDR / RSP1B | Vertical / 2m dipole |
| PMR446 Walkie-Talkies (EU) | 446.00625 - 446.19375 MHz | NFM | Any RTL-SDR / RSP1B | Short UHF whip / discone |
| MeshCom Amateur LoRa (AT) | 433.775 MHz | LoRa | Any RTL-SDR / RSP1B | UHF vertical |
| Meshtastic EU Public | 869.525 MHz | LoRa | Any RTL-SDR / RSP1B | 868 MHz whip |
| Aircraft ADS-B Radar | 1090.000 MHz | PPM | Any RTL-SDR / RSP1B | 6.8 cm vertical / collinear |

## Tips for First-Time Listeners
1. **Move the stick away from the laptop**: USB 3.0 ports and switching power supplies radiate broadband noise across VHF/HF. A 1-2 m shielded USB extension near a window makes a dramatic difference.
2. **Start on strong known carriers**: Tune `98.0 MHz` (FM broadcast) or `1090.0 MHz` (ADS-B) first to confirm your dongle and antenna path before hunting weak HF signals.
3. **Use `MockIq` / Demo Mode offline**: SDRCraft's deterministic synthetic IQ lets you test `rustfft` spectrum peaks, waterfall scrolling, and `cpal` FM audio without hardware.
