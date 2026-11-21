# Why the window used to stop at 24 MHz

24 MHz is the bottom of the R820T2 tuner inside a plain RTL-SDR stick. That chip was a television tuner. It does not hear medium wave or longwave. The old floor in this program copied that limit and applied it to every tune, including sticks that can go lower.

## What each front end actually hears

| Front end | About | Medium wave | Shortwave | Longwave |
|-----------|--------|-------------|-----------|----------|
| RTL-SDR, R820T2 (the common thumb stick) | 24-1766 MHz | No | No | No |
| RTL-SDR Blog v4 | 0.5-1766 MHz | Yes | Yes | No |
| Nooelec NESDR Nano 2 Plus (RTL2832U + R820T2, 0.5 ppm TCXO) | about 25-1750 MHz | No | No | No |
| Ham It Up v1.3 or Nano, in front of that stick | 100 kHz-65 MHz, shifted up by 125 MHz | Yes | Yes | Yes |
| Ham It Up Plus, same 125 MHz shift | 300 Hz-65 MHz | Yes | Yes | Yes |
| Airspy HF+ Discovery | 9 kHz-31 MHz, and 60-260 MHz | Yes | Yes | Yes |
| SDRplay RSP1B / RSPdx | 1 kHz-2 GHz | Yes | Yes | Yes |
| HackRF One | 1 MHz-6 GHz | Barely, and it is deaf | Poor | No |

Medium wave in Europe is about 531-1602 kHz (0.531-1.602 MHz). That is above 0.5 MHz, so a Blog v4 can sit on it. Nighttime carries more of it.

Longwave in Europe is about 148.5-283.5 kHz. The seeded stations are in that band: France Inter 162 kHz, BBC and ORF 198 kHz, RTL 234 kHz. All of them are under 0.5 MHz. A Blog v4 still will not hear them.

A Ham It Up v1.3 or Nano will. Nooelec rates those two from 100 kHz to 65 MHz, and the Plus from 300 Hz. The local oscillator is 125 MHz on all three, so the stick is tuned to 125 MHz plus the station, in AM. BBC and ORF on 198 kHz are 125.198 MHz in this window. France Inter is 125.162. Medium wave at 909 kHz is 125.909. Shortwave at 5.975 MHz is 130.975. The upconverter has its own USB power lead, and a switch: upconvert for HF, passthrough for FM, airband, and 1090 MHz. Leave it in upconvert and the VHF stations disappear. Conversion loss is about 10 dB, so the same station is quieter than on a native HF radio. The antenna plugs into the upconverter, not into the stick.

The window now tunes from 0.5 MHz. Below that, Set Frequency returns a typed error. A longwave preset still stores the frequency and reports `front_end: hf_receiver`, because the station is real even when this stick cannot hear it.

A plain R820T2 stick will accept a tune under 24 MHz in this window and then hear nothing there. The hardware floor did not move. Only the Blog v4 (direct sampling, or its built-in HF path) covers 0.5-24 MHz.

## The whip is still the wrong antenna

Opening the tune control does not make 1 MHz readable on the antenna that came in the box. Medium wave and shortwave want wire. See `docs/ANTENNAS.md`.
