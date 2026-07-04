# Script Modules

Rhai script modules are for block-rate event and scalar-control policy: note routing, velocity mapping, probability, and small bits of persistent numeric state.

Scripts are not audio DSP modules. They cannot declare audio ports, do not receive audio buffers, and must not implement oscillators, filters, convolution, sample-by-sample processing, or other audio-rate behaviour.

Build reusable or performance-critical DSP as Rust primitives, then connect those primitives with YAML. Use scripts to decide which events and controls feed those primitives.
