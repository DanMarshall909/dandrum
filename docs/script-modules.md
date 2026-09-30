# Script Modules

Rhai script modules run once per processing block for event and scalar-control policy: note routing, velocity mapping, probability, and small bits of persistent numeric state. A script's scalar control output fills the per-sample control buffer with the same value for that block; other control sources may change every sample.

Scripts are not audio DSP modules. They cannot declare audio ports, do not receive audio buffers, and must not implement oscillators, filters, convolution, sample-by-sample processing, or other audio-rate behaviour.

Build reusable or performance-critical DSP as Rust primitives, then connect those primitives with YAML. Use scripts to decide which events and controls feed those primitives.

For audio-derived control signals, prefer `envelope_follower` and `curve_mapper` modules. They convert audio energy into deterministic control buffers inside the Rust render path, so patches can route ducking, filter modulation, and dynamics-style control without scripts receiving audio-rate data.
