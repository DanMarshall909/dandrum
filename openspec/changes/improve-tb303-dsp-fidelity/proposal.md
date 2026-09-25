## Why

The TB-303 acid acceptance patch currently opens much too brightly, collapses to a nearly static fundamental within roughly 250 ms, and leaves its resonant filter input un-driven. Fixing that patch is a useful proof of concept, but the durable result should be a repeatable workflow for implementing any sound through controlled stimuli, deterministic renders, listening, and spectral measurements. Better controlled recordings from a real TB-303 with a Devil Fish modification can be added later without redesigning the workflow.

## What Changes

- Tune the existing TB-303 patch's cutoff mapping, filter-envelope trajectory, resonance, and voice gain staging so a held note retains a useful evolving spectrum through its middle decay.
- Preserve and extend render-level coverage for velocity accent and legato slide while later calibration fixtures are acquired.
- Add reusable Hann-windowed spectral and level analysis that emits a frame-by-frame metrics report.
- Add a declarative 48 kHz sound fixture and command-line workbench that render an audition WAV and metrics CSV from the same stimulus, or analyze an aligned external WAV with the same settings.
- Document an iterative sound-implementation workflow and how later hardware or clone references fit into it without committing third-party audio, proprietary binaries, or licence material.
- Keep the voice implementation within YAML composition and existing general-purpose primitives; it does not add a product-specific Rust primitive or UI.

## Capabilities

### New Capabilities

- `sound-design-workflow`: Render a versioned sound fixture and emit repeatable audition and analysis artifacts for implementation work.

### Modified Capabilities

- `acceptance-examples`: Add measurable spectral-evolution and resonant-response requirements for the TB-303 acid acceptance patch while retaining its accent and slide behaviours.

## Impact

- Updates `examples/patches/tb303-acid.yaml` and its render-level Rust tests.
- Adds a reusable sound-analysis module, a sound-workbench command, and a checked-in 303 fixture that can later be replicated for instruments such as the MS-20.
- Adds documentation for iterating from listening and metrics to a maintained behavioral test.
- Adds a host-side sound-analysis/workbench API but does not change the patch schema, built-in module registry, plugin UI, realtime DSP path, or third-party dependencies.
