# Plugin and integration development experience review

Recorded: 2026-09-27. Reviewed revision: `8cc0823712f92b012bae65dc16c17c7740a2265e`.

This review records findings at the revision above. Since that snapshot, host preparation order has been fixed; developer-build demo assets resolve from the source checkout; and the reusable host bridge, pages, optional Sound Lab, and executable WebView tests have been implemented in `extract-reusable-instrument-host-ui`. Installed-plugin asset packaging, a faster browser asset loop, Rust-owned load metadata/diagnostics, automation meaning, and a unified reload transaction remain open. Reproduced failures below describe the reviewed revision, not current behavior.

The [reusable instrument host and UI change](../openspec/changes/extract-reusable-instrument-host-ui/proposal.md) covers shared WebView controls, explicit demo configuration, optional Sound Lab, and a second instrument as proof of reuse. The remaining lifecycle and automation recommendations need their own scoped tasks and acceptance tests.

## 1. Make loading and preparation follow one lifecycle

**Reproduced:** loading the same patch before versus after preparation produces different audio at the same requested sample rate.

The plugin constructor loads its default patch before the host calls `prepareToPlay`. Rust's `DandrumEngine::prepare_realtime` updates the engine's sample rate, block size, and fallback synth, but does not update the existing graph processor. Instrument replacement follows a different order: it prepares a candidate engine before loading the patch.

Sources: [processor startup and preparation](../src/juce-plugin/PluginProcessor.cpp), [Rust preparation and graph construction](../src/rust-engine/src/synth.rs).

A temporary probe exercised the built C FFI with the synthetic 808 kick, note 36 at velocity 100, 64-frame render calls, and 8,192 output frames. It compared the left-channel samples from two fresh engines:

1. Create → load patch → prepare → note-on → render.
2. Create → prepare → load patch → note-on → render.

| Requested sample rate | Peak of prepare-first output | RMS difference between orders |
|---|---:|---:|
| 44,100 Hz | 0.999932 | 0 |
| 48,000 Hz | 0.999942 | 0.209627 |
| 96,000 Hz | 0.999985 | 0.175614 |

This comparison establishes an observable dependency on call order. It is not a measurement of spectral fidelity.

**Recommended change:** use one preparation path for startup, reload, and state restoration, with explicit host sample rate and maximum block size. Separate the immutable instrument definition from DSP state prepared for those host settings. Handle a later host sample-rate or block-size change through the same lifecycle.

**Acceptance evidence:** test load-before-prepare and prepare-before-load equivalence, re-preparation at 44.1/48/96 kHz, preserved public values, and continued rendering after a failed replacement.

## 2. Resolve instrument assets independently of the working directory

**Reproduced:** `build/dandrum-plugin-construction-test` passed from the repository root and failed when invoked by absolute path with `/tmp` as its working directory:

```text
Failed to load default patch: examples/patches/synthetic-808-kick.yaml
```

`findRepositoryExample` searches upward from `std::filesystem::current_path()`. That works in the checkout/build tree but does not establish a resource location for a plugin opened by a DAW.

Source: [default patch and fixture lookup](../src/juce-wrapper/DefaultPatch.h).

**Recommended change:** provide an explicit resource root for installed instruments, fixtures, and UI assets, plus an explicit development override. Extend loading to accept YAML bytes and a resource base directory. Match acceptance currently stages YAML beside the source patch, requiring that directory to be writable; state restoration stages YAML in the temporary directory. The resource base should remain an explicit input in both cases.

Source: [snapshot loading and state restoration](../src/juce-plugin/PluginProcessor.cpp).

**Acceptance evidence:** launch from an unrelated directory, load from a read-only instrument directory, and restore an instrument with relative sample references. The first failure was reproduced; the latter two cases are proposed coverage for the inspected path-handling risks.

## 3. Expose one prepared instrument result across the Rust boundary

**Code findings:**

- `dandrum_engine_load_patch_with_error` prepares an instrument, then calls the legacy load function, which prepares it again.
- Public parameter enumeration reparses the file for its count and again for every descriptor.
- C++ separately interprets YAML for instrument identity and presets.
- Detailed Rust preparation errors are retained, but processor load failures replace them with generic messages and do not query `dandrum_engine_last_error_message`.
- That retained error is process-global, so it is not associated with a particular plugin instance or load result.

Sources: [FFI loading and descriptors](../src/rust-engine/src/ffi.rs), [C++ metadata and preset handling](../src/juce-plugin/PluginProcessor.cpp). The diagnostic load wrapper described in this historical review was removed during the graph kernel migration.

**Recommended change:** expose the existing Rust preparation result through an owned FFI handle. Obtain validated metadata, identity, asset resolution, and diagnostics from that same snapshot, then create the runtime from it. Reuse Rust's preset parser and validation boundary. Preserve the separation between immutable definitions and mutable public values.

The smallest useful first step is to display the detailed Rust failure in the editor. Follow with errors owned by the load result or engine instance and removal of redundant parsing.

**Acceptance evidence:** metadata and DSP come from the same snapshot even if the source file changes; two instances retain independent diagnostics; valid YAML is interpreted consistently through CLI and plugin loading; a failed load preserves the active instrument.

## 4. Give parameter changes and reloads a clear thread owner

**Code findings:** ordinary parameter changes are applied at an audio block boundary, but `loadPresetFromFile` also calls `applySlotToEngine` directly on the caller's thread. The file watcher uses a JUCE message-thread timer and invokes the full synchronous reload from its callback. File reload and state restoration each implement their own engine publication and teardown sequence, including a five-millisecond sleep.

Sources: [parameter application and replacement paths](../src/juce-plugin/PluginProcessor.cpp), [watcher thread contract](../src/juce-plugin/InstrumentFileWatcher.h), [watcher callback](../src/juce-plugin/InstrumentFileWatcher.cpp).

**Recommended change:** use the existing parameter handoff consistently for presets and UI edits. Have the file watcher submit preparation to one loading worker, then publish the completed candidate through a common replacement transaction. Document and test the actual JUCE callback synchronization guarantee before removing the fixed sleep; elapsed time should not be treated as proof that a callback has finished.

**Acceptance evidence:** render while applying presets and reloading; failed reload preserves the running patch and associated state; editor interaction remains responsive during preparation. The current concurrent reload test waits for all reload threads to finish before rendering, so it does not establish concurrent render/reload behavior.

Source: [existing concurrent reload test](../tests/cpp/PluginConstructionTest.cpp).

## 5. Specify automation meaning as well as stable slot IDs

**Code findings:** the 64 host parameters retain their IDs, but `preparePublicParameterSlots` assigns public parameters by their order in the YAML descriptor list. Reordering that list can change which public parameter a host automation slot controls. The WebView's set-parameter command also begins and ends a host gesture for every value update, including each pointer movement during a drag.

Sources: [host slot assignment](../src/juce-plugin/PluginProcessor.cpp), [WebView parameter gesture](../src/juce-plugin/PluginEditor.cpp), [knob pointer handling](../src/juce-plugin/Tb303WebUi.h).

**Recommended change:** define and persist the mapping between stable public IDs and host slots, with explicit policies for added, removed, and incompatible parameters. Make one knob drag produce one begin/update/end gesture. These are behavior changes that need a separate contract from the extraction's promise to preserve current automation behavior.

**Acceptance evidence:** reorder public declarations while preserving automation targets; add/remove parameters without silently reassigning retained targets; restore the mapping from plugin state; cancel or release a drag with exactly one completed gesture.

## 6. Provide a fast browser development loop and executable integration tests

**Code findings:** HTML, CSS, and JavaScript are embedded in a C++ header. The WebView contract test mostly searches that text and `PluginEditor.cpp` for substrings. Processor test targets also compile/link the browser dependencies.

Sources at the reviewed revision: [embedded page](../src/juce-plugin/Tb303WebUi.h) and [native test targets](../CMakeLists.txt). The former source-text test has since been replaced by the [executable editor bridge test](../tests/cpp/PluginEditorBridgeTest.cpp) and [page behavior test](../tests/js/Tb303PageTest.mjs).

**Recommended change:** keep UI code in ordinary assets, serve those assets directly during development, and embed the same assets for distribution. Exercise browser control behavior and native commands through executable tests. Keep fast processor tests independent of the browser where practical, and add a separate integration test that loads the built VST3 artifact.

**Acceptance evidence:** edit a control without rebuilding C++; test parameter updates, note release, errors, and resource delivery through the real bridge boundaries; load the distributed plugin from an unrelated directory. A successful processor construction or source-string assertion is not proof that the packaged plugin and browser integration work.

## 7. Make development commands reproducible and checks explicit

**Observed:** the build initially failed during the preceding merge because Linuxbrew's linker was selected from `PATH` and could not resolve system library dependencies. Re-running with the system tools first in `PATH` succeeded, and all 10 CTest tests passed.

**Code and checkout findings:**

- CMake always builds Rust with `--release`, including a C++ development build.
- The Rust custom command tracks `Cargo.toml` and Rust sources but omits `Cargo.lock` from its dependencies.
- The README primarily documents the console launcher and omits the plugin's WebKit/GTK build requirements and standalone launch path.
- The agent notes describe older architecture and refer to a headless-engine change that has already been archived.
- `core.hooksPath` is unset and `.git/hooks/pre-commit` and `.git/hooks/pre-push` are absent in the reviewed checkout. The repository contains hooks, but the preceding pushes did not execute them. Merge validation was run manually.

Sources: [build configuration](../CMakeLists.txt), [README](../README.md), [agent notes](../AGENTS.md), [pre-commit hook](../.githooks/pre-commit), [pre-push hook](../.githooks/pre-push).

**Recommended change:** provide documented debug/release configurations and a small set of reliable build, standalone-run, and fast/full-check commands. Add an environment check that reports the selected compiler/linker, required browser dependencies, and hook installation. Track Rust build inputs consistently and align Rust debug settings with the selected native configuration. Keep expensive mutation checks explicit and report which gates actually ran.

**Acceptance evidence:** a fresh checkout can build and launch the standalone plugin from the documented commands; a lockfile change triggers the Rust build; developers can identify the selected tools and active checks without inspecting Git configuration manually.

## Recommended order

1. Correct lifecycle behavior and preserve detailed load diagnostics.
2. Make resource resolution and snapshot loading explicit.
3. Consolidate Rust preparation, metadata, and the replacement transaction.
4. Implement the existing shared-host/UI proposal with the browser development loop and stronger integration tests.
5. Define and verify stable automation mapping and complete drag gestures as a separate behavior change.

Build configuration, hook visibility, and documentation corrections can proceed independently. The review made no production changes and does not claim the proposed acceptance cases already pass.

## Reproducing the load-order comparison

Build the current Rust library through CMake. Save the following temporary probe as `/tmp/dandrum-load-order-probe.cpp`, then compile and run it from the repository root:

```sh
env PATH=/usr/bin:/bin /usr/bin/c++ -std=c++20 \
  -I src/juce-wrapper /tmp/dandrum-load-order-probe.cpp \
  build/rust-target/release/libdandrum_engine.a -ldl -lpthread -lm \
  -o /tmp/dandrum-load-order-probe
/tmp/dandrum-load-order-probe examples/patches/synthetic-808-kick.yaml
```

This is a Linux reproduction recipe for the reviewed build, not a portable development command or a maintained regression test.

```cpp
#include "RustEngineBindings.h"
#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <iostream>
#include <vector>

std::vector<float> render(const char* patch, float rate, bool loadFirst) {
    auto* engine = dandrum_engine_create();
    if (!loadFirst) dandrum_engine_prepare_realtime(engine, rate, 64);
    if (!dandrum_engine_load_patch(engine, patch)) std::abort();
    if (loadFirst) dandrum_engine_prepare_realtime(engine, rate, 64);
    dandrum_engine_note_on(engine, 36, 100);
    std::vector<float> left(8192), right(8192);
    for (std::size_t offset = 0; offset < left.size(); offset += 64)
        dandrum_engine_render(engine, left.data() + offset, right.data() + offset, 64);
    dandrum_engine_destroy(engine);
    return left;
}

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    for (const float rate : {44100.0f, 48000.0f, 96000.0f}) {
        const auto loadFirst = render(argv[1], rate, true);
        const auto prepareFirst = render(argv[1], rate, false);
        double squaredError = 0.0;
        float peak = 0.0f;
        for (std::size_t i = 0; i < loadFirst.size(); ++i) {
            const double difference = loadFirst[i] - prepareFirst[i];
            squaredError += difference * difference;
            peak = std::max(peak, std::abs(prepareFirst[i]));
        }
        std::cout << "rate=" << rate << " signal_peak=" << peak
                  << " order_difference_rms=" << std::sqrt(squaredError / loadFirst.size()) << '\n';
    }
}
```
