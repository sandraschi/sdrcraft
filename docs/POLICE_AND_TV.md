# Police chatter, and terrestrial television

Two things films taught people an SDR would do. Neither is what this stick is for.

## Is police radio still a thing?

Partly, and only in some places.

The old picture was analog FM. Fire, ambulance, and police shared a few channels. A scanner on the kitchen counter followed the city. That still exists in pieces:

- Many US fire and EMS departments are still unencrypted, and some smaller police departments still use analog FM or unencrypted digital (P25).
- Large US cities have moved police to trunked digital systems, and a growing share of those channels are encrypted. The speaker stays quiet. You do not get the movie mix of every pursuit.
- In Austria, Germany, and most of the EU, police voice is TETRA (the BOS networks). It is encrypted. There is no "unit respond" to hear on a whip.

What is still ordinary to receive, and what this Listen page tunes:

- Broadcast FM.
- Shortwave and medium-wave broadcasts, on an RTL-SDR Blog v4 (0.5 MHz and up). A plain stick still starts at 24 MHz. Longwave needs a different front end. See `docs/TUNING_RANGE.md`.
- Amateur radio, receive only.
- ADS-B at 1090 MHz, which is a beacon, not a conversation.

Airband (118-137 MHz) is still plain AM. Some countries treat it as a private communication even though it is not scrambled. The Listen card says so. It does not pretend the answer is the same in every country.

## Is it legal?

Receiving a broadcast is legal. Receiving encrypted police traffic, or stripping that encryption, is not. This program has no TETRA decoder and no encrypted-P25 decoder, and it will not grow one.

Unencrypted public-safety radio is a local question. In parts of the United States, listening at home to unencrypted police and fire is a long-standing hobby, and some states restrict a scanner in a car or during a crime. In Austria and Germany, police radio is not a broadcast, and intercepting non-public radio communications is a criminal matter. Do not point this app at it.

The app also does not ship a list of police frequencies. A frequency list would be the feature the law is about.

## Is it all encrypted?

No. Fire and EMS in the US are often still open. Some rural police are still open. European police voice, and a lot of big-city US police tactical channels, are encrypted. "All" would be wrong. "The movie is over for European police, and unreliable for American police" is the useful sentence.

## Terrestrial TV

Analog television has been switched off in Europe and in the United States. What remains is digital:

| Region | System | About how wide |
|--------|--------|----------------|
| Europe | DVB-T / DVB-T2 | 7-8 MHz |
| North America | ATSC | 6 MHz |

An RTL-SDR stably samples about 2.4 million samples per second. That is a slice of a TV channel, not the channel. On the spectrum, a mux in the UHF band (roughly 470-694 MHz after the 700 MHz mobile clearance in much of Europe) looks like a wide raised hump. The Listen card centers 490 MHz so you can look for that hump. It will not become a picture.

The RTL2832 chip was sold as a DVB-T tuner. Installing the WinUSB driver, which this kind of app needs, replaces the television driver. Watching free-to-air TV means that TV mode, or a separate tuner stick. A wider software radio can capture a whole channel, and a separate demodulator can turn a free-to-air mux into video. That is not this program.

Pay television is encrypted again, on top of the digital broadcast. This app does not remove conditional access.
