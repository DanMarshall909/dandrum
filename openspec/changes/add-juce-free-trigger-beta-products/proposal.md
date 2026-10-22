## Why

The JUCE-free Trigger CLAP editor foundation is verified and published at `2c7bcaa5`, but it is not the requested beta: VST3 and standalone products, real selected-sample loading and portable restoration, named-DAW acceptance and installed release artifacts remain missing. This increment completes those outcomes over the persistent format-neutral Rust host rather than creating separate instrument implementations for each product.

## What Changes

- Deliver deliberate Trigger VST3 and standalone products with the production React editor, shared session/runtime behavior and selected Linux x86_64 X11/WebKitGTK proof.
- Qualify the VST3 dependency/toolchain and actual loader, parameter, state, callback and native-view boundary before adopting a wrapper or expanding its integration. The cached wrapper candidate is not yet a verified product.
- Let the native editor choose supported WAV files, retain immutable owned samples, publish full patch edits atomically, map and play distinct selected PCM, and retain native edits/undo across editor destruction and recreation.
- Add bounded self-contained patch/sample/parameter state and standalone file operations, including fresh-instance restoration after original selected files disappear; keep existing packaged-kit state readable.
- Resolve the retained fractional-rate interoperability failure through honest format-neutral clock/preparation/playback behavior, without truncating or rounding the requested host rate.
- Prove actual standalone audio/MIDI ownership, native input and lifecycle behavior, named-DAW automation and independent instances, then install offline products outside the checkout with complete no-JUCE build/runtime/linkage and licence evidence.
- Keep normal hooks, behavior-first tests, independent material-boundary review, focused publication, mutation exclusion, disk-backed bounded caches and cleanup of owned disposable output on every terminal path.

## Capabilities

### New Capabilities

- `juce-free-trigger-products`: VST3 and standalone execution, precise supported host clocks, device and callback ownership, native/DAW acceptance and installed offline artifacts.
- `trigger-sample-resources`: native sample selection, owned resource identity, atomic patch publication/history and portable patch/sample/parameter restoration.

### Modified Capabilities

None. Existing CLAP, editor and session requirements remain in force; this increment adds the missing beta contracts and supersedes the earlier design's integral-only host-rate limitation through new implementation work.

## Impact

Affected owners include `src/trigger-host`, `src/instrument-session`, engine render/preparation configuration where precision requires it, the CLAP/Wry edges, production Trigger native import/document handling, new product adapters and build/install/verification fixtures. The format-neutral host stays free of Clack, Wry, GTK, WebKit, VST3 and JUCE types. Product and SDK dependencies must be pinned, audited and exercised at the actual selected boundary before adoption; no JUCE dependency may enter the new product path.

The selected proof platform remains Linux x86_64 X11 with WebKitGTK 2.50.4. Other native platforms, unrestricted patch/plugin architectures and additional sample decoder formats are outside this beta increment. No current limitation or external acceptance gap may be relabelled as completed proof. Main remains clean at `68dd655`; the full beta goal closes only after all remaining required evidence exists.
