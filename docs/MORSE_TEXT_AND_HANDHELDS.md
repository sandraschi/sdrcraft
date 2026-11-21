# Morse, teletype, 1seg, handhelds, and the Austrian text nets

## Morse and teletype

These are supposed to be read by a machine.

- **Morse (CW)** is still daily amateur traffic. A tone turns on and off. A decoder measures the dits and dahs.
- **RTTY** is radioteletype: two tones, Baudot, about 45 baud. It looks like a slow teleprinter.
- **PSK31, FT8, JS8** are the later slow-text modes. FT8 is the crowded one. A computer copies the message. A person usually cannot.

The Listen page runs three decoders on practice tapes built in the program:

- Morse envelope timing, round-trip of `CQ DE SDR`.
- RTTY as Baudot bits (start, 5 data, stop), round-trip of `SDR`.
- APRS as an AX.25 UI frame with a CRC check, round-trip of `SDR>APRS:>hi`.

The report says the copy is not live. The stick's samples are not in the window, and the demo spectrum is four steady tones, so feeding it to these decoders would invent letters. The same functions will read the air when IQ arrives. fldigi and WSJT-X remain the tools for PSK31, FT8, and JS8. This program does not pretend to copy those.

A Blog v4 can tune the HF bands where Morse and RTTY actually live. A plain R820T2 stick cannot. See `docs/TUNING_RANGE.md`.

**APRS** at 144.800 MHz in Europe is the text mode already inside every stick's range. The Listen card centers it. You see short bursts. The APRS line in the decoder report is the practice frame, not a station on the air.

## A garakei in Tokyo

The stubby TV antenna on an old Japanese flip phone is for **1seg** (one segment). Japanese digital terrestrial TV, ISDB-T, splits each 6 MHz channel into 13 segments. Twelve carry the living-room picture. One narrow segment, about 430 kHz, is the phone picture.

Tokyo still transmits that. Analog TV ended on 24 July 2011 (the disaster prefectures a year later). The stations come off Tokyo Skytree: NHK, the commercial key stations, and TOKYO MX. Through 2025 and 2026 the broadcasters turned off the extra 1seg data pages (news and weather text). TOKYO MX said on 13 May 2026 that its 1seg data pages would end on 28 May 2026, and that watching the program on 1seg continues. The phone can still show the picture in the Tokyo coverage area if the tuner and battery work.

A European RTL-SDR is a poor 1seg set. The segment is narrow enough to fit in the stick's sample rate, which a full European DVB-T channel is not, but this program has no ISDB-T demodulator. Spectrum at UHF will show the television humps. It will not show the program.

## Walkie-talkies

In Austria the license-free handheld is **PMR446**, a narrow band around 446 MHz. Analog sets are ordinary FM. Range is short: a worksite, a ski slope, a family in the same park. Digital sets (dPMR and similar) are not intelligible speech. These are private conversations. The Listen page does not scan them.

## The slow text web in Austria

People are describing three nets, and they are not the same net.

| Net | Where | Who it is for | Encrypted? |
|-----|--------|----------------|------------|
| APRS | 144.800 MHz, classic packet | Licensed amateurs. Positions and one-line text, drawn on public maps. | No. It is published. |
| MeshCom | 70 cm LoRa, ÖVSV and ICSSW | Licensed amateurs. Text, positions, and sensor values, hopped across nodes, bridged to APRS and HAMNET. | No. Amateur rules want a callsign and no cipher. |
| Meshtastic | 868 MHz, license-free | Anyone with a small LoRa board. The Austrian public preset is MediumFast. | The default public channel is readable by every node on that preset. Extra channels, and MeshCore, are actually encrypted. |

MeshCom is the Austrian amateur one: a mesh of short texts that still works if the phone network does not. Meshtastic is the larger license-free cousin on 868 MHz. MeshCore is a third 868 MHz mesh, growing in Austria, and its messages are encrypted.

The APRS decoder prints a practice frame. It will not recover private Meshtastic channels or MeshCore.
