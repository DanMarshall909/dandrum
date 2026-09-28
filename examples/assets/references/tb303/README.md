# Din Sync TB-303 reference recordings

The local `x0x-reference.zip` archive comes from Din Sync's [TB-303 reference
recordings for x0xb0x builders](http://www.dinsync.info/2010/02/tb-303-reference-recordings-for-x0xb0x.html).
The [download link](http://privat.bahnhof.se/wb447909/dinsync/files/x0x-reference.zip)
is on that page. Use these recordings as external listening and analysis
references when tuning the 303 patch.

The downloaded archive is 76,492,567 bytes and contains 25 stereo, 16-bit,
44.1 kHz WAV sets. Its SHA-256 is
`f6aaeb3361f87ed1f215af1e17905562a11f978b8ed350a8e9e2be514917a450`.

The source page documents each set's approximate cutoff, resonance, envelope
modulation, decay, and accent knob positions. Each set has four positions with
four low-C notes per position: unaccented saw, accented saw, unaccented square,
then accented square. Preserve the recordings' relative levels when comparing
accented and unaccented notes.

The archive is ignored by Git because the page offers the recordings for
reference use but does not specify redistribution terms. Download it from the
source link to reproduce the local asset.

The WAVs have been extracted locally into `wav/`, which is also ignored by
Git. See [ANALYSIS.md](ANALYSIS.md) for measurement methods and findings and
[analysis.csv](analysis.csv) for per-note results.
