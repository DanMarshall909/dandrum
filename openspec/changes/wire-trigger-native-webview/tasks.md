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
- [x] 4.3 Investigate the reported immovable standalone title bar under WSLg; use system window decorations while preserving host Options actions and exact editor presets, then verify real title-bar dragging and the standalone wrapper.
- [x] 4.4 Display `trigger - {name of patch}` in the standalone title bar; verify initial name, accepted rename/undo and external replacement, and preserve the title after rejected edits or malformed title requests.

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

Window movement and title follow-up: the standalone uses a system title bar
and an Options menu that delegates to JUCE's existing audio/settings and state
actions. Chrome installation waits until window construction completes and
preserves the editor size; the regression first reproduced a startup shrink
to 760×126 and now verifies 1180×860 initially and after Reset, plus all presets.
The title API and native adapter first failed their new tests, then passed for
accepted names, rename/undo, external replacement, Unicode/quotes, malformed
requests, early title requests and host-owned window isolation. Production
WebKit verifies the actual `trigger - Advanced Drum Kit` editor title.

All eight focused native/UI and launcher CTest groups, 85 unit tests,
type/component checks and fast/headed SLOW_MO=700 browser regressions pass.
The native adapter retains 100% source-line coverage, with lower branch/function
coverage. Spec coverage includes both new scenarios and keeps the 239 existing
todo entries. The rebuilt standalone was relaunched with its full editor and
correct title; Windows input dragged the actual system title bar 100 pixels
right and 50 pixels down while retaining its dimensions. The GUI fixture uses
no hardware stream and avoids PulseAudio enumeration; real API-to-DSP checks
remain separate. WSLg's stuck supervised PulseAudio process required restarting
during runtime verification. No physical audio listening, DAW loading or other
platform verification is claimed.
