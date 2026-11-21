# Transmit-capable SDR boxes, and explorer locator beacons

The app binary stays receive-only. This page is the buying and field note for
hardware that can send, and for the distress beacon an explorer actually carries.
Austria and EU license detail for HackRF stays in the legacy guide:
[sdr-mcp HACKRF.md](../../sdr-mcp/docs/HACKRF.md).

## Two sizes

| You hold | Typical box | Transmit? |
|----------|-------------|-----------|
| Thumb stick | RTL-SDR Blog v4, NooElec NESDR SMArt | No. Receive only. |
| Slightly larger USB box | HackRF One (and PortaPack bundles), ADALM-Pluto, LimeSDR Mini | Yes. A license still applies. |

The stick is the receiver for this app. The larger boxes are lab and amateur
radios that happen to speak IQ samples over USB. Owning one does not add a
transmit command to the binary.

### What those larger boxes are

| Box | Duplex | Official span (order of magnitude) | On-air use |
|-----|--------|--------------------------------------|------------|
| HackRF One, and HackRF-class boards such as the R9 with a PortaPack | Half duplex (send or receive, not both at once) | About 1 MHz to 6 GHz | Wideband lab tool. Output is low milliwatts and band-dependent. |
| ADALM-Pluto | Full duplex | 325 MHz to 3.8 GHz as sold | Small amateur and lab transceiver. Unofficial firmware that claims a wider span is outside this note. |
| LimeSDR Mini class | Full duplex | About 10 MHz to 3.5 GHz | Same role as Pluto, different chip. |

A real handheld or mobile amateur transceiver is still the radio you use for a
contact. These boxes are milliwatts, with harmonics a proper rig filters out.
Put a band filter between the box and the antenna before any on-air test.

### If you are a licensed amateur

A license covers your own callsign, on the amateur bands your class includes,
in the country that issued it, inside that license's power and identification
rules. Some countries recognize a foreign license for a short visit. Assume
yours stops at the border until you have checked that country.

The app does not arm a transmitter for you. There is no emergency switch.
A person in grave and imminent danger may use any radio they already have to
call for help. That duty sits on the person. A false distress call is a crime.
This program does not decide that you qualify, and it does not synthesize
distress signals.

Leave aviation, marine working channels, pagers, cellular, and anyone else's
equipment alone. Unlicensed ISM bands allow tiny power and a limited duty
cycle. These boxes exceed that if you turn the gain up.

## Explorer locator beacon

Cellular and a satellite terminal both go dark in the same places: no tower,
no sky view for the dish, no subscription. A personal locator beacon does not
use either network.

Buy a **406 MHz personal locator beacon (PLB)** before the trip. Register it
in the country that issued it, in your name, with a reachable emergency
contact. Write the beacon ID into the trip plan you leave with that contact.

| Beacon | Who it is for | Path |
|--------|----------------|------|
| PLB | One person on foot: jungle, desert, mountains, open ocean in a raft | COSPAS-SARSAT |
| EPIRB | A boat | COSPAS-SARSAT, registered to the vessel |
| ELT | An aircraft | COSPAS-SARSAT, registered to the airframe |
| Garmin inReach, ZOLEO, SPOT, and similar messengers | Two-way text on a paid plan | Commercial satellites (Iridium or Globalstar). Useful. Not a substitute for a registered 406 MHz beacon. |

### What happens when you trigger a PLB

1. The beacon transmits a short digital burst on 406 MHz. The burst carries
   the registered ID and, on current models, a GNSS position.
2. COSPAS-SARSAT satellites (the MEOSAR constellation on the navigation
   satellites) relay that burst to a ground station, then to a rescue
   coordination center.
3. Most current PLBs also send a weak homing signal on 121.5 MHz so a search
   aircraft or ground team can walk the last kilometers. Satellites no longer
   treat 121.5 MHz as the alert. An old beacon that is only 121.5 MHz does
   not call the rescue system.

The beacon is one-way. It says who you are and where you are. It does not
carry a conversation. Under thick canopy the GNSS fix can wait. The ID still
goes out. Trigger it when you are in grave and imminent danger, then leave it
on, outside, with a clear view of the sky.

### Before you go

- Register the beacon, then check the battery replacement date.
- Run the manufacturer's self-test only. A live transmission starts a rescue.
- If you set it off by mistake, switch it off and call the authority named in
  your registration paperwork so the mission can be cancelled.
- Tell someone the route, the return time, and the beacon ID.

### What this app will not do

An SDR box pointed at 406 MHz is not a beacon. It is not registered, rescue
centers do not accept it, and transmitting there without being the distress
beacon you bought is unlawful. The receive-only binary does not emulate a
PLB, an EPIRB, or an ELT.
