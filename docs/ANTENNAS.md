# Antennas: why the little whip disappoints

The dongle is not the radio you hear. The antenna is. A notebook, an RTL-SDR
stick, and the short whip from the bag will show a spectrum. A few strong
local FM stations may even sound clean. Most of the rest looks like noise,
and that is the antenna, not a broken stick.

## The first afternoon

1. Screw the whip onto the SMA jack. Extend it all the way.
2. Put the stick on a short USB extension at a window, not in a hub on the
   laptop. The laptop is a bag of hash (USB, CPU, display) sitting a few
   centimetres from the whip.
3. Tune a known local FM station, then airband or 1090 MHz if you care about
   aircraft. If those are dead indoors, the stick is still probably fine.

What you should expect from that whip, indoors:

| Band | What the whip actually does |
|------|-----------------------------|
| FM broadcast, about 88-108 MHz | A handful of strong local stations, often noisy. Wavelength is about 3 m. A 20 cm whip is a small slice of that. |
| Airband and 2 m, about 118-174 MHz | Nearby towers if you are at a window and the whip is vertical. Weaker the moment you sit back down. |
| ADS-B, 1090 MHz | Often the best surprise. Wavelength is about 27 cm, so the short whip is finally a plausible size. |
| Shortwave and HF, under 30 MHz | Almost deaf. Those wavelengths are 10 m to 100 m. The whip hears the laptop more than the ionosphere. |

Two more disappointments that look like "no signals":

- **Polarization.** A whip is vertical. Many European FM broadcasts are
  horizontal, so you throw away a lot of the station before the stick sees it.
  Airband and most amateur repeaters are vertical, which is why the same whip
  feels better there.
- **Overload.** One nearby FM tower can splatter the whole screen with copies
  of itself. That is not "more signals." It is one signal too strong. Less
  gain, a window that does not face the tower, or a filter in front of the
  stick.

## Kinds, from the bag to the field

### The whip in the box

A telescopic rod, sometimes only a few centimetres when collapsed. It is a
starter, not a design. Fully extended, at a window, on a USB extension: that
is the whole upgrade that costs nothing. Still expect local strong signals
only.

### The dipole kit (two telescopic rods)

Many RTL-SDR kits include a small mount and two whips in a V. That is a
dipole. Each side wants to be about a quarter of a wavelength for the band
you are listening to, so the pair is about half a wavelength end to end.
A rough length for one side is `75 / frequency_in_MHz` metres. At 100 MHz
that is 75 cm per side. At 1090 MHz it is about 7 cm. Set it, aim the flat
of the V at the station, and it already beats the single stub for that band.
It is still an indoor antenna.

### Outdoor vertical and the discone

A vertical with a few drooping wires at the bottom (a ground plane) is the
ordinary scanner and amateur receive antenna for VHF and UHF. A discone is
the same idea made broadband: a disc on top, a cone of rods underneath. One
of those on a mast or a balcony rail is how a stick starts hearing airband,
marine, amateur repeaters, and ADS-B without you retuning the metal every
time you change band. Keep the coax run short once you are above 400 MHz.
Coax eats UHF.

### Wire between trees

This is the shortwave antenna. Pick two supports (trees, a mast, the house
and a shed) and hang copper wire.

- **Dipole.** Two equal wires, fed in the middle. Total length about half a
  wavelength: roughly `150 / frequency_in_MHz` metres. A dipole cut near
  7 MHz is on the order of 20 m of wire. It hears that part of HF and a good
  deal around it. For receive it does not have to be perfect.
- **Inverted-V.** The same dipole with the middle held high and the ends
  lower. Easier when you have one tall point instead of two.
- **One long wire.** A single wire from the window out to a tree, the longer
  and higher the better. Give it a second wire as a counterpoise (along the
  ground, or the coax shield treated as the other half) or it is still half
  an antenna. This is the classic "wire between trees." It is how shortwave
  stops being a blank waterfall.

These are receive sizes, so you can be approximate. The stick will not
transmit through them. Do not run the wire across a power line or a path
where someone can walk into it.

### Directional: Yagi and log-periodic

A **Yagi** is a boom with several rods across it. One rod is driven (connected
to the coax). A slightly longer rod behind it is the reflector. Shorter rods
in front are directors. Energy arrives best from the front. Point it at a
city, a repeater site, or the airport, and stations off the side and the back
drop. A household TV antenna is usually this shape. A small VHF Yagi is about
a metre long. A serious HF Yagi is a tower and a rotator, which is the "ham
contraption": aluminium, traps, and a mast, because the elements for 20 m are
themselves about 10 m across.

A **log-periodic** looks similar, but every element is driven and they get
steadily shorter toward the front. It covers a wider band than a Yagi and is
a bit less sharp. Roof TV antennas that claim a wide UHF range are often this.

### Rhombic (the diamond)

A **rhombic** is four long wires in a diamond. The shape is a rhombus, which
is why it is sometimes called a rhomboid. Each leg is several wavelengths, so
on HF the whole thing is a field, not a balcony: think tens of metres on a
side, aimed at a continent. It is strongly directional off one end. A resistor
at the far corner soaks up the energy that would otherwise reflect back and
blur the direction. Almost nobody in a city should build one. It belongs in
this list so the whip and the rhombic are visibly not the same object. Hams
with land, and old shortwave stations, used them because a wire diamond hears
a chosen direction and ignores a lot of the rest of the world.

### Small loop, for a noisy room

A loop about a metre across (coax or copper pipe) hears the magnetic part of
the wave and shrugs off a lot of electrical hash from the house. It is
narrow: you retune it when you move frequency. It is the indoor HF answer
when there is no tree and no permission to hang wire.

### Active mini-whip

A tiny powered antenna, often fed by the stick's bias tee. It can hear HF
from a window where a passive whip cannot. It also amplifies every nearby
broadcast tower, and the stick then overloads. Turn the gain down. Prefer a
real wire if you can hang one.

## What to buy, in order

1. USB extension and the whip or dipole kit you already have, at a window.
2. For aircraft, FM, and UHF: an outdoor vertical, a discone, or a small Yagi.
3. For shortwave: wire between trees, or a small loop if the wire is impossible.
4. One antenna will not be good at 7 MHz and at 1090 MHz. Plan on two if you
   care about both.

The SMA jack on an RTL-SDR Blog v4 takes the kit whip directly. A balcony
antenna usually arrives on TV coax or with a different plug. Use one adapter,
not a chain of them.
