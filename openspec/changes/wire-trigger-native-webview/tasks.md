## 1. Native transactional boundary

- [x] 1.1 Specify and test asynchronous owned-YAML replacement, generation rejection and real rendered sample preservation before implementation.
- [x] 1.2 Expose prepared session, native file selection and sample preparation through the existing WebView bridge.

## 2. React native adapter

- [x] 2.1 Test and implement pure native document projection and supported sample graph compilation.
- [x] 2.2 Test and implement serialized accepted edits/undo, real notes/parameters and native waveform/meter feedback; report unsupported operations.
- [x] 2.3 Select native or silent preview backend and connect native sample import and feedback refresh.

## 3. Integrated launch and verification

- [x] 3.1 Embed the full shell and local assets, register maintained launcher and tests, document supported capabilities.
- [x] 3.2 Run focused native/JavaScript and launcher checks, validate specs and coverage, build and launch the integrated WebView with actual engine evidence.

## 4. Native connection-status layout regression

- [x] 4.1 Reproduce the connection banner pushing the editor sideways; place backend status in the existing header, fit native viewports, and verify browser and production WebKit geometry before rebuilding/relaunching the sampler.
- [x] 4.2 Connect native size and performance choices to actual JUCE editor/window resizing; verify all preset dimensions, compact/full restoration and rejected requests through the real bridge.

Initial integration verified on Linux/WSL: 80 Trigger unit tests and type/component checks, 33 native
adapter/UI boundary tests, and all seven focused CTest groups (including
production WebKit and launcher/inventory/boundary checks). The four native
adapter modules have 100% source-line coverage; branch/function coverage is
lower. The native API test proves representative normalized control changes,
known signed sample output, edit/inverse and failed/stale replacement retention.
Strict change validation and spec coverage pass with 239 pre-existing todo entries.
The Release standalone was rebuilt with the final local bundle and launched as
a visible integrated editor. Physical audio listening, DAW loading and other
platforms were not verified. Missing engine/editor capabilities are planned in
`docs/trigger/native-engine-roadmap.md`, with their implementation tasks open.

Connection-status follow-up: the production WebKit regression first failed
because backend status was outside the editor header. The fixed bundle passes
all four focused native/UI CTest groups; WebKit verifies editor bounds and no
horizontal page overflow at 1180×860, 820×560 and 1500×1000. All 81 unit tests,
type/component checks, two browser frame regressions and a headed slow native
browser run pass. Native layouts retain unscaled text; the browser's fixed
preview geometries remain unchanged. The rebuilt standalone was reopened with
the compact connection indicator in its header. Strict change/spec-coverage
checks include the new geometry scenario; the existing 239 todo entries remain.

Host-size follow-up: both the missing native resize function and unchanged
window dimensions were reproduced as failing CTest assertions. The real
production WebKit check now clicks Min, Default, Expanded, Collapse and Open
editor, and verifies both JUCE editor and browser viewport dimensions:
820×560, 1200×800, 1600×1000, 900×126 and restored 1600×1000. Invalid presets,
missing arguments and missing editors are rejected. All 83 unit tests,
type/component checks, fast and headed slow browser regressions and four
focused native/UI CTest groups pass. The native adapter's source-line coverage
remains 100%; this is not exhaustive branch or whole-C++ module coverage.
