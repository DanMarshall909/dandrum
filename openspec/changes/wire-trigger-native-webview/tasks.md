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

Verified on Linux/WSL: 80 Trigger unit tests and type/component checks, 33 native
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
