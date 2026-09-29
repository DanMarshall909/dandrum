## Why

Stateful DSP processors across the engine expose a `reset()` method, but nothing
ever calls it, so the code carries `#[allow(dead_code)]` and stays unreachable.
Two real gaps sit behind that:

- **Voice retrigger bleed.** Per-voice module state is built once and reused across
  notes (fixed voice pool; see `polyphony::build_polyphonic_states_from_compiled`),
  and note-on does not reset it. Voice-scoped stateful modules — notably `filter`
  and `envelope_follower` — therefore carry biquad/detector memory from the previous
  note into the next. The stale state decays within a few samples, but on rapidly
  retriggered high-resonance filters it can produce an audible click.
- **No engine reset/panic.** Global effects (`reverb`, `echo`, `dynamics`,
  `convolution`, `spectral`, `saturator`, `frequency_splitter`) are not voice-scoped,
  so resetting them per note would wrongly destroy their tails. But there is **no**
  engine-level all-notes-off / panic / patch-reload path that clears them either, so
  their `reset()` has no caller at all.

Both are cases of correct, tested behaviour that is simply not wired up.

## What Changes

- **Voice retrigger reset:** when a voice is allocated for a new note, reset that
  voice's stateful per-module state (filter, envelope follower, and any other
  voice-scoped stateful primitive) so each note starts from clean state.
- **Engine reset / panic:** add an engine-level reset that stops active voices and
  cascades `reset()` to every global effect, clearing reverb/echo/delay tails and
  processor state. Exposed for host-driven panic and patch reload.
- Remove the `#[allow(dead_code)]` allowances on `reset()` methods once they are
  reachable, and cover the newly-reachable paths with behaviour tests.

## Capabilities

### New Capabilities
- `state-reset`: deterministic clearing of stateful DSP — per-voice on note retrigger,
  and engine-wide on an explicit reset/panic — without disturbing global effect tails
  during normal note activity.

## Impact

- Engine: the voice-allocation path resets voice-scoped module state on note-on; a new
  engine reset method cascades `reset()` to all global effects.
- FFI: an additive export for host-driven panic/reset (ABI-additive; no changes to
  existing symbols).
- Audio behaviour: notes start from clean voice state (removes rare retrigger clicks);
  engine reset clears tails deterministically. Normal note activity is unchanged.
- Cleanup: `reset()` methods lose their `#[allow(dead_code)]` allowances once wired.
