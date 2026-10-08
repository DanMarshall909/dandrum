# Slint design library verification

Verified on **2026-10-08** for the `feat/trigger-slint-library-2026-10-08`
candidate. The implementation follows the maintained design guide and its
prepared-data contracts. This record covers the reusable UI library and its
silent catalog.

## Environment and result

| Item | Verified value |
| --- | --- |
| Platform | Linux, x86-64 |
| Slint compiler, C++ SDK, viewer | 1.18.1 |
| Renderer | `winit-software`, authenticated Xvfb display |
| C++ compiler | GCC 13.3.0, C++20 |
| CMake | 4.4.4 |
| Node / Python | 24.19.0 / 3.12.14 |
| Public guide vocabulary | All 29 guide components plus Scrollbar |
| Shared assets | 146 generated tokens, 33 original SVGs, eight licensed font binaries |
| Slint compilation | All 10 entrypoints passed: nine fixtures and the complete catalog |
| Runtime verification | Nine contract suites and 249 independent input/render-geometry checks passed |
| Catalog dimensions | Actual windows measured at 1200×800 and 820×560 |
| Repository/native CTest | All eight test groups passed |

The machine-readable [runtime receipt](catalog/verification/runtime-results.json)
contains every fixture result, requested catalog size, renderer, platform and
viewer version. The [CTest receipt](catalog/verification/ctest-results.txt)
records the separate repository and compiled C++ tests.

## Runtime behavior

Contract checks run in a fresh Slint window by activating a test button and
reading its resulting report. Independent checks then use another fresh window
and real pointer, keyboard, typing, wheel or accessibility actions, observing
the resulting value, callback report or rendered element geometry. Contract
mutations cannot supply the independent input test's result.

| Fixture | Contract result | Independent assertions |
| --- | --- | ---: |
| Controls | 29 behavior contracts | 23 |
| Displays | Composite display contract passed | 10 |
| Layout | 45 layout, selection, capability and overlay contracts | 37 |
| Menus | 11 menu and lifecycle contracts | 19 |
| Waveform | 14 source-relative modulation-rail contracts | 4 |
| DisplayEdges | Composite edge-case contract passed | 29 |
| LayerDetails | 10 detail capability and model contracts | 13 |
| Readouts | Nine caption/value/unit/alignment contracts | 9 |
| DisplayBindings | Composite persistent-model contract passed | 9 |
| Catalog, 1200×800 | Compiled catalog; actual size checked | 48 |
| Catalog, 820×560 | Same catalog; actual size checked | 48 |
| **Total independent assertions** | | **249** |

Representative behavior includes bounded value edits, Shift precision, reset,
invalid input and cancellation, balanced gestures, host updates without edit
echoes, disabled/prepared capability gates, panel actions independent of
collapse, keyboard selection, native context menus, delayed tooltips, shared
scroll state, and release of editor-owned audition notes.

The display checks also exercise arbitrary output channel counts, valid silence
versus missing measurements, clip acknowledgement tickets, persistent layer/send
refresh after user edits, supplied source/module parameters and choices, source
inspection focus return, zone corner edits, selected slice-label ordering, and
signed region-relative modulation geometry.

The catalog checks navigate all six pages, change presentation state, edit
controls, scroll content, reset scrolling on page changes and collapse panels.
LEVEL, FREE and SEND popups are measured against the actual content clip at both
window sizes. Their arrow centers must continue to point to the owning control
after placement is clamped or flipped above it.

## Compiled C++ and repository checks

The final Release build generates and compiles the Slint catalog and embeds its
fonts/icons. The compiled executable is launched separately from the viewer;
the [native startup receipt](catalog/verification/native-startup.json) records
its SHA-256, renderer and observed startup, and the
[native screenshot](catalog/screenshots/native-catalog.png) shows that process.
The process remained alive through the three-second capture. Its only stderr
message was the container's unavailable desktop D-Bus settings watcher; the
window and embedded resources rendered successfully.

| CTest group | Evidence |
| --- | --- |
| `slint-token-drift` | Generated CSS, C++ and Slint outputs match the maintained token source |
| `slint-token-contracts` | 11 Node contracts, including aliases, lengths, typography and Slint drift rejection |
| `slint-library-inventory` | Three Node contracts covering the independent guide inventory, composition, assets and catalog coverage |
| `demo-launcher` | Launcher behavior, including the real Python command entrypoint |
| `demo-launcher-inventory` | Maintained demo registration and native artifact discovery |
| `demo-launcher-boundaries` | Native-only catalog boundary and existing launcher calibration |
| `slint-value-codec` | 43 standalone C++ parsing/formatting checks |
| `slint-codec-binding` | Generated Slint controls bound to the real C++ codec; unit-bearing and Unicode-minus typing plus the control contracts |

The three launcher groups contain **31 Python tests** in total. The two Node
groups contain **14 tests**. The native codec accepts declared units, Unicode
minus and supported pan notation, rejects invalid/nonfinite values, and avoids
coupling the standalone parser to generated Slint types.

Both the root `DANDRUM_SLINT_LIBRARY_ONLY` configuration and direct
`cmake -S ui/slint` configuration were checked. The Release catalog was built
through the root configuration. It returns before configuring JUCE, Rust or
Node; repository Node/Python tests are an explicit optional CMake setting.

The preserved design reference archive, original font files, generated CSS and
generated C++ token output were also compared with the base commit and are
unchanged. No JUCE vendor, engine or React production source is part of this
change. An independent read-only review checked scope, host API consistency,
imports, manifest entries and documentation links.

## Visual review

All six catalog pages were rendered at the default and compact dimensions and
at taller inspection sizes. The review included scrolling, source details,
header actions, focused controls and open popups. Retained examples are:

- [Native C++ catalog at startup](catalog/screenshots/native-catalog.png).
- [Waveforms and analysis, full-page inspection](catalog/screenshots/waveforms.png).
- [Compact control popup after placement and arrow correction](catalog/screenshots/controls-popup.png).
- [Compact layout and feedback after scrolling](catalog/screenshots/layout-compact.png).

Visual/input review found and corrected layout height overflow, feedback action
alignment, icon centering, selected slice-label stacking, persistent scroll
bindings and scrollbar gutters, popup clipping, popup paint order and arrow anchoring. These checks
complement compilation; a successful compiler run alone does not establish
usable layout or input behavior.

## Acceptance criteria

The active OpenSpec delta is
[`add-slint-design-library`](../../openspec/changes/add-slint-design-library/).
Its six scenarios map to these concrete tests and evidence:

| Scenario | Proof |
| --- | --- |
| A consumer imports the guide vocabulary | `SlintLibraryInventoryTest.mjs`, public-barrel catalog compilation, six-page renders |
| Semantic tokens change | `DesignTokensTest.mjs`, `SlintDesignTokensTest.mjs`, generator `--check` |
| An editable value is committed or cancelled | Controls contracts/input checks, C++ value codec and generated binding test, LayerDetails contracts/input checks |
| A header action is activated | Layout and Menus contract/input checks; catalog header-action/collapse assertions |
| A prepared display has no editing capability | Displays, DisplayEdges, DisplayBindings, Waveform and LayerDetails contracts/input checks |
| The catalog runs at compact and default sizes | Two actual-size catalog runs, screenshots, native Release build/startup |

Strict OpenSpec validation passes. The existing main-spec coverage gate also
passes: **584 acceptance criteria, 345 mapped to tests, 239 existing ratchet
backlog entries**. This change remains an active delta on its feature branch;
it has not been synced into or archived over the main specifications.

## Reproduce

Install the matching Slint 1.18.1 SDK and viewer and expose the SDK through
`CMAKE_PREFIX_PATH`. Follow the platform runtime setup in the
[library README](README.md). To build and run the catalog:

```sh
./demo slint-library
```

To include the repository development gates:

```sh
cmake -S . -B build/slint-library -DCMAKE_BUILD_TYPE=Release -DDANDRUM_SLINT_LIBRARY_ONLY=ON -DDANDRUM_SLINT_TEST_REPOSITORY=ON
cmake --build build/slint-library --config Release --parallel
ctest --test-dir build/slint-library -C Release --output-on-failure
python scripts/check-slint-library.py --evidence build/slint-library-evidence/final
scripts/check-spec-coverage
openspec validate add-slint-design-library --strict
```

The live viewer runner needs a display server. Headless setup, fixture selection
and the distinction between `--check-only` and live input verification are in
[the harness README](../../tests/slint/README.md). Raw logs and additional
screenshots remain under the ignored build evidence directory; the compact
receipts and representative images above are committed with the library.

## Verification boundaries

This is behavioral and render-geometry evidence, not a measured line/branch
coverage percentage. Rust mutation testing is outside this UI-only change.
Windows/macOS execution, physical-device rendering, DAW host transport, audio
engine wiring and plugin lifecycle were not exercised in this Linux run.

The catalog is silent and displays explicitly supplied fixtures. Consumer code
owns audio analysis, parameter transport, prepared generations and capability
decisions. Native `ContextMenu` uses Slint's platform menu style; the separate
`MenuSurface` provides the guide's themed rows, icons and modulation controls.
Consumers with clipped ancestors must supply overlay bounds and appropriate
parent paint order as described in the README.
