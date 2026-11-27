# SDRplay RSP1B

The RSP1B is the receive-only box for this project: one tuner, 1 kHz to 2 GHz, no gaps. Longwave, medium wave, shortwave, FM, airband, and 1090 MHz are the same radio. It is a 14-bit receiver in a painted steel case, about 98 x 94 x 35 mm and 315 g. The antenna socket is one 50 ohm SMA. Power and data are one USB 2.0 cable with a type B plug on the radio.

SDRCraft does not open this radio. The window's hardware path is an RTL-SDR stick via `rtl_tcp`. Hear the RSP1B in SDRplay's own program, SDRconnect. This note is the first-hour procedure and the limits that matter.

Sources for the numbers below: the RSP1B datasheet (January 2024, still the sheet linked from the product page), SDRplay API specification 3.15 (RSP1B support added January 2024, device id 6), and the SDRconnect download page (current build 1.0.10). SDRplay's own site is the authority if a screen label moves.

## What the hardware does

Coverage is continuous from 1 kHz to 2 GHz. Eleven preselection filters switch by themselves as you tune. You do not pick them.

| Automatic filter | Span |
|---|---|
| Low pass | below 2 MHz |
| Band pass | 2-12, 12-30, 30-60, 60-120, 120-250, 250-300, 300-380, 380-420, 420-1000 MHz |
| High pass | above 1000 MHz |

Two notch switches are manual. Leave both off until a strong local broadcast is wrecking some other band.

| Switch in Device Settings | What it cuts | Datasheet depth |
|---|---|---|
| RF Notch | Medium wave and FM broadcast | FM: more than 50 dB from 85 to 100 MHz. MW: more than 30 dB from 660 to 1550 kHz |
| DAB Notch | DAB digital radio | More than 30 dB from 165 to 230 MHz |

Turn RF Notch off to listen to medium wave or FM. Turn DAB Notch off if the station you want sits between 165 and 230 MHz.

The ADC bit depth depends on how wide a slice you ask for:

| Sample rate | ADC |
|---|---|
| 2 to 6.048 MSPS | 14-bit |
| 6.048 to 8.064 MSPS | 12-bit |
| 8.064 to 9.216 MSPS | 10-bit |
| above 9.216 MSPS, up to about 10.66 | 8-bit |

SDRconnect can also display a decimated rate down to 62.5 kSPS. The visible window tracks the rate you pick. Start at 2 MSPS. That stays 14-bit and is wide enough to see a broadcast station and its neighbours. Use 4 to 6 MSPS when you want a wider map and still want 14 bits. Use 10 MSPS only as a finder, then drop back.

Maximum input is 0 dBm continuous, +10 dBm for a short burst. This radio cannot transmit. Do not connect it to a transmitter, a transmit antenna that is live, or the output of a power amplifier.

Bias-T is 4.7 V at up to 100 mA, software selectable, off by default. Turn it on only for an amplifier whose label asks for about 4.5 to 5 V and draws under 100 mA. A passive wire, a dipole, or a whip gets no Bias-T. Enabling it into a DC short can damage the radio.

Typical specs are for 0 to 35 C. The operating range printed on the product page is -10 to +60 C.

## Install on Windows

1. Download SDRconnect for Windows x64 from https://www.sdrplay.com/sdrconnect/ . The current build on that page is 1.0.10. It is 64-bit. It supports every current RSP except the original RSP1.
2. Run the installer before you plug the radio in. It installs the application and the SDRplay API service. The service starts with Windows. Files land in `C:\Program Files\SDRplay`. The service and the DLL must be the same version. A half-updated install is a silent failure.
3. Plug the type B cable into the radio and a USB port on the computer itself. A charge-only cable will power the LED and pass no samples. If the waterfall stutters, move off an unpowered hub.
4. Open SDRconnect. The device list should name an RSP1B and a serial number. If the list is empty, press Refresh. If it stays empty, the API service is not running. Start that service, or unplug the radio, start the service, and plug it in again.
5. Press Play (the green triangle). Press Stop (the red square) before you let another program use the radio. One program owns the stream.

SDRuno is still downloadable and SDRplay has marked it end of life. Use SDRconnect.

Linux is tested by SDRplay on Ubuntu LTS releases up to 24.04. The installer is on the same SDRconnect page. Raspberry Pi 4 and 5 are in the supported platform list. The API there is a daemon in `/usr/local/bin` and a library in `/usr/local/lib`.

## First hour

Leave Bias-T off. Leave both notches off. Leave IF AGC on. Set the sample rate to 2 MSPS.

The RSP1B has one antenna socket. There is no port menu. The wire or the whip must actually be on that SMA.

### Prove the radio on FM

A local FM station is the loudest thing most houses can hear, and it needs no long wire.

1. Mode: WFM.
2. Bandwidth: about 150 to 200 kHz.
3. Tune somewhere in 88 to 108 MHz until a peak fills the middle of the slice. Scroll a digit in the frequency readout, or type the frequency in MHz and press Enter.
4. RF gain: high, until the audio is clean. If the spectrum paints an Overload warning, lower RF gain until the warning goes away and the station is still there. More gain after that point only raises the noise.
5. You should hear speech or music. If the slice is flat, the antenna is still in the laptop's noise. Move it to a window on a short coax. A quarter-wave rod near 100 MHz is about 75 cm.

FM in Europe is often horizontally polarised. A vertical rod still hears strong locals. A weak one gets louder when the rod is horizontal.

### Longwave and medium wave

These are under 2 MHz. The low-pass filter is already in circuit. You need wire. The whip from a dongle kit will not do this job.

Hang 5 to 15 m of wire, as high as the window allows, and as far from the laptop power supply as the coax allows. The SMA centre goes to that wire. The SMA shell goes to a second wire of a few metres, laid out as a counterpoise, or to the shield of the coax if the run is long. A small indoor magnetic loop is the substitute when you cannot hang wire. One antenna will not be good at 198 kHz and at 1090 MHz. Swap it.

| Station | Frequency | Mode | Bandwidth | When |
|---|---|---|---|---|
| DCF77, Mainflingen | 77.5 kHz | AM | about 1 to 2 kHz | Day and night on a wire or a loop. You hear a carrier and a tick. |
| France Inter | 162 kHz | AM | 9 kHz | Longwave broadcast |
| BBC Radio 4 and ORF | 198 kHz | AM | 9 kHz | The loud longwave check in central Europe |
| RTL | 234 kHz | AM | 9 kHz | Longwave broadcast |
| BBC Radio 5 Live | 909 kHz | AM | 9 kHz | Medium wave, much louder after dark |

If a local medium-wave station is so strong that every other frequency pumps, you are in overload. Drop RF gain first. The MW half of RF Notch would also cut the station you are trying to hear, so leave that notch off while you are on medium wave.

### Shortwave

Same wire. Sample rate still 2 MSPS. Mode AM for a broadcast, bandwidth 5 to 10 kHz. Amateur voice below 10 MHz is LSB, bandwidth about 2.4 to 3 kHz. At and above 10 MHz, amateur voice is USB.

Scan these until a voice appears. The band is the target. A named broadcast moves.

| Band | Where to look | Mode |
|---|---|---|
| 49 m | 5.9 to 6.2 MHz | AM |
| 41 m | 7.2 to 7.45 MHz | AM |
| 31 m | 9.4 to 9.9 MHz | AM, often the busy daytime band |
| 40 m amateur | 7.0 to 7.2 MHz, voice near the top | LSB |
| 20 m amateur | 14.0 to 14.35 MHz | USB |

A local FM tower can raise the whole shortwave floor. Turn RF Notch on for that. Turn it off again before you go back to FM or medium wave.

If everything is a mirror of one loud station, the front end is overloaded. Drop RF gain. Shorten the wire. A preamp is the wrong next purchase.

### VHF and UHF

Change the antenna. A vertical of the right length, or a discone, at a window.

| Listen | Frequency | Mode | Bandwidth |
|---|---|---|---|
| Airband | 118 to 137 MHz | AM | about 8 kHz |
| APRS, Europe | 144.800 MHz | NFM | about 12.5 kHz |
| Two-metre calling | 145.500 MHz | NFM | about 12.5 kHz |
| ADS-B | 1090 MHz | none in SDRconnect | look for a narrow pulsing peak |

Airband is open AM. Some countries still treat it as a private communication. ADS-B on 1090 MHz is a beacon. SDRconnect shows the carriers. Turning them into aircraft plots is a different program. A quarter-wave at 1090 MHz is about 6.5 cm. The HF wire is deaf there.

In a city with strong DAB transmitters, the DAB notch cleans up 2 m and the airband. It also dents 165 to 230 MHz, so leave it off when that is the band you want.

## Gain, in the order that matters

1. Antenna and placement. A wire in the clear beats any slider.
2. RF gain. High until Overload appears, then down one step. On this radio the slider walks a table of LNA gain-reduction steps (API 3.15, the RSP1B table). The most sensitive step is 0 dB of reduction. Below 50 MHz there are 7 steps, up to 61 dB of reduction. From 50 to 420 MHz there are 10 steps, up to 62 dB. From 420 to 1000 MHz, 10 steps, up to 64 dB. From 1 to 2 GHz, 9 steps, up to 62 dB.
3. IF AGC stays on for listening. Turn it off only when a specific strong signal is pumping the audio and you already dropped RF gain.
4. Audio volume last. It does not change the radio.

## Morse, RTTY, and FT8

SDRconnect demodulates the audio. A decoder is a second program on that audio.

On Windows, install a virtual audio cable. Point SDRconnect's output at one end and fldigi or WSJT-X at the other.

| Mode | Dial | SDRconnect mode | Bandwidth |
|---|---|---|---|
| Morse, amateur CW | the CW part of the band you can hear | CW or USB | about 500 Hz for CW, about 3 kHz if you are watching a slice in USB |
| RTTY | where you see the two tones | USB | about 3 kHz |
| FT8 | 7.074 MHz, or 14.074 MHz | USB | about 3 kHz |
| WSPR | 14.0956 MHz | USB | about 3 kHz |

WSJT-X expects the radio on the dial frequency and the tones in the audio. If the waterfall in WSJT-X is empty, the cable is the thing to check, then the mode. USB on an LSB contact sounds like the wrong sideband. Broadcast AM into WSJT-X will not decode.

SDRCraft can copy a Morse tape, a RTTY tape, and an APRS practice frame. Those tapes are inside the program. They are not the RSP1B's audio.

## What this window can and cannot do

| Task | Where |
|---|---|
| Hear longwave, medium wave, shortwave, FM, airband | SDRconnect, on the RSP1B |
| See a 10 MHz-wide slice at 14-bit under 6 MSPS | SDRconnect |
| Decode FT8 or PSK31 from the RSP1B | WSJT-X or fldigi, fed by a virtual audio cable |
| Spectrum, waterfall, and practice-tape decoders in SDRCraft | Demo IQ, or a live RTL-SDR via `rtl_tcp` |
| Open the RSP1B from SDRCraft | Not built. The radio does not speak `rtl_tcp`. |

A later build can talk to the API service. Until that exists, do not expect a Tune button in this window to move the RSP1B.

For someone writing that build later: API 3.15 says the RSP1B uses the RSP1A parameter block and the RSP1A update reasons. Device id is 6. Bias-T is `rsp1aTunerParams.biasTEnable`, applied with `sdrplay_api_Update_Rsp1a_BiasTControl`. Open the API first, close it last, or the service is left in a bad state. The header and the service must match.

## When it sounds broken

| What you see | What it usually is |
|---|---|
| Device list empty | API service stopped, cable is charge-only, or the radio is on a dead hub. Refresh, then check the service. |
| Play does nothing and another SDR is open | The other program owns the device. Stop it. |
| FM is loud, longwave is silent | The whip is still attached. Longwave wants the wire or a loop. |
| Every frequency is a copy of one station | Overload. Drop RF gain. Take off any preamp. Shorten the wire. |
| Shortwave is a hiss with a few FM ghosts | Turn RF Notch on. Drop RF gain. |
| Medium wave vanished after a notch experiment | RF Notch is still on. Turn it off. |
| Waterfall stutters at 10 MSPS | The machine is at the wide rate, and the ADC is down at 8 bits there anyway. Use 2 MSPS for listening. |
| Bias-T was on and the band went dead | Turn Bias-T off. A passive antenna does not want 4.7 V. |

## Related notes

- Bands and why a thumb stick stops at 24 MHz: `docs/TUNING_RANGE.md`
- Wire, dipole, loop, Yagi: `docs/ANTENNAS.md`
- What to look for on each band: `docs/SIGNAL_ATLAS.md`
- Product page: https://www.sdrplay.com/product/rsp1b/
- Datasheet: https://www.sdrplay.com/wp-content/uploads/2026/06/RSP1B_Datasheet.pdf
- API 3.15: https://www.sdrplay.com/wp-content/uploads/2026/04/SDRplay_API_Specification_v3.15.pdf
