# Native note lease handoff

N1 authenticated heartbeat renewal and N2 autonomous native wall-time expiry with a live attachment are complete and ready for root review/publication. The production change routes the existing heartbeat command to the note controller and calls its existing bounded expiry service from the existing GLib editor service. The unmodified production app and public CLAP/WebKit/authenticated transport are used throughout.

## Ownership return

I release all write/build ownership of these four paths under `/tmp/dandrum-lanes/runtime` and both caches to root immediately with this handoff:

- `src/clap-adapter/src/trigger/gui.rs`
- `src/clap-adapter/tests/internal/trigger_gui_dispatch.rs`
- `src/clap-adapter/tests/trigger_parameters.rs`
- `src/clap-adapter/tests/internal/trigger_note_native.rs` (new)
- `/tmp/dandrum-lanes/cache-editor-guard`
- `/tmp/dandrum-lanes/cache-host-snapshot-cov`

Base remains `0dc1f7a9814a238edca21c22b67ca81e411d1994`, branch `lane/trigger-runtime-20261020`. No Git mutations or commits. No maintained edits outside those four paths. Root retains docs/spec/tracker/CMake/Python/integration and cache-runtime. The unrelated nested Runtime `cache-editor-guard/` is byte-preserved, proven by `untracked-cache-before.json` and `untracked-cache-preserved.json`. No reusable cache deleted. Final source hashes are in `candidate-sources.sha256`.

## Behavioral proof and genuine RED

The composed sample fixture uses an actual gated sample_player child and independent signed stereo PCM expectations. It has 480000 frames, while every test enforces processed frames below half that count. Qualification proves held output beyond the old 0.341-second fixture and actual authenticated NoteOff silence. Setup/X11/observer/PCM calibration failures are separately labeled in `checkpoints.md`, not counted as behavior RED.

- Qualification: [qualification-pcm-green.stdout.log](qualification-pcm-green.stdout.log).
- N1 genuine native RED: [n1-native-red.stdout.log](n1-native-red.stdout.log); immutable source/DSO/test binding [n1-red-build-binding.json](n1-red-build-binding.json). Heartbeat is rejected capabilityUnavailable. Matching fast RED: [n1-owner-red.stdout.log](n1-owner-red.stdout.log). GREEN: [n1-native-green.stdout.log](n1-native-green.stdout.log). A never-held-pitch NoteOff forces the old public native-time sweep after the original deadline, so retained signed output proves renewal rather than an unswept old lease.
- N2 genuine native RED: [n2-native-red.stdout.log](n2-native-red.stdout.log); immutable binding [n2-red-build-binding.json](n2-red-build-binding.json). Editor and independent host voices remain -0.25 after renewed expiry instead of host-only -0.125. Host release leaves -0.125, independently locating the leaked editor voice. GREEN: [n2-native-green.stdout.log](n2-native-green.stdout.log), -0.25 -> -0.125 -> zero. No further note command triggers expiry; output is captured before later browser/CLAP callbacks. Final authenticated snapshot confirms the same attachment is still operational.

Direct authenticated backend calls and passive unchanged forwarding establish native method/lease behavior; physical keyboard/pointer behavior is not claimed. The frontend desired-note map is not populated, and exact observed note traffic excludes automatic renewals. Real native time, bounded GLib service, finite sample, no fake clock, new protocol or private product seam.

## Final validation

- `final-native-regression`: all 15 current trigger_parameters native cases PASS, 23.40 seconds overall. Case inventory/timing: `final-native-cases.json`. This does not claim the separate trigger_gui binary or all repo-native tests.
- `final-owner-regression`: exact 13 adapter owner tests PASS, 2.01 seconds overall.
- `final-core-notes`: 14 core editor_note_controller tests PASS, 0.73 seconds test execution, 75.40 seconds overall including distinct standalone dependency build.
- `final-fmt`, `final-editor-clippy`, `final-default-clippy`, `final-trigger-clippy`: PASS. Strict release Clippy covers lib/tests with warnings denied. OpenSpec strict validation and git diff check PASS. Root owns task mapping/registration and publication.
- `final-native-llvm`: 3 final note cases PASS, 12.21 seconds, 3 raw profiles. `final-owner-llvm`: exact 13 owner cases PASS, 2.01 seconds, 27 raw profiles including exact-selector list invocations.
- `final-normal-note-regression`: after restoring the ordinary DSO, all 3 note cases PASS, 11.20 seconds, without LLVM runtime stack/preload override.

All commands, exits, elapsed times, process identities and tool sessions are retained in per-run JSON/stdout/stderr, samples and `sessions.json`. Builds used absolute roots, jobs=1, offline/locked dependencies and serialized execution. Mutation was excluded by assignment and was not run.

## Coverage, artifacts and provenance

Final source GUI SHA256: `eb878a98a1acfee7a5ad86683d96c54a45992dbdd0ba69959a9cad6d83e4e9b5`.

Final normal DSO SHA256: `17251daa8ecbcc4ec3da600e04056e2338426926c0a274ebb84b70e72e810df3`, uniquely selected from Cargo JSON at `/tmp/dandrum-lanes/cache-editor-guard/release/deps/libdandrum_clap_adapter.so`, retained at `artifacts/final-product-build/libdandrum_clap_adapter.so`, installed at `installed/DandrumTrigger.clap`. See `final-product-build-binding.json` and `final-normal-restored.json`.

Final native host SHA256: `c4eafe278c51658bc9a860188dda3b94d78861424434f5893062b2b47a75de3a`, retained at `artifacts/final-product-build/native-host`. Its Cargo-selected test origin is `final-native-build`; executable reuse and product DSO provenance are bound separately. Production app.js SHA256 remains `4b9ce5a356fea5f22703f9046c77a80ac616c00242348a12fe81f58fe4f760f2`.

Final instrumented native DSO SHA256: `c01945a5291f7f1df1e6af5ec9851aad61371c1de1f0c154a6227b27763396d9`. Final instrumented owner ELF SHA256: `aa4e83e197e568794ac4518c38e814f76295988c04750a716711e571d138dad0`. Immutable portable ELF/profile/profdata bindings are `final-native-portable-coverage-binding.json` and `final-owner-portable-coverage-binding.json`; N1/N2 equivalents also retained under `artifacts/coverage/`. Native and owner raw exports are retained. `final-service-region-union.json` identifies the exact GUI file and region coordinates: new service lines145-147 have 5 code regions, native 4/5, owner 5/5, union 5/5. Region [147,9,147,10] (busy-borrow/closing-brace path) is native zero, owner one. No blanket whole-GUI/dependency coverage claim; other unchanged parameter/error/structural branches are not fully exercised by note-specific native tests.

N1 normal product DSO and N2 RED DSO differ; no byte-reuse claim. N2 RED SHA256 is `08d771fc083a906e4a6fccbfb690f33aa1a3cc96c202a5e8b3cfaa214ee75477`. Cargo test unifies the clack-host dev feature in clack_extensions, unlike product build. Actual N2 RED rustc argv was recovered from retained process samples into `n2-red-observed-rustc-commands.json`; comparison `n2-red-rustc-attribution.diff` supports metadata and clack_extensions extern differences. Full historical compiler environment is unavailable, and the exact byte-difference cause beyond these observed invocation differences is not asserted. Later compiler capture is append-only in `compiler-captures/`. Original instrumented staging bindings retain a normal-origin retained_library field; portable coverage bindings unambiguously identify the actual instrumented ELFs.

Refactor decisions and source-bound checkpoint details are in `checkpoints.md`. Production policy remains shared; no newly extracted production module. Test helpers centralize bounded native progress and fixture cleanup. Normal DSO is restored, copied fixture YAML/WAV restored by RAII, and app.js unchanged.

## Resource/process state and limits

`final-process-check.json`: all 44 run receipts completed; no owned native group members, pending run receipts or observed cargo/rustc/linker builders. Tool sessions are completed; namespace-local sandbox PGIDs must not be interpreted as host PGIDs. Latest measured disk free 3393892352 bytes and MemAvailable 11821867008 bytes, above 1GiB/4GiB floors. The final standalone core check added roughly 2GB of reusable ordinary target data; this was disclosed and preserved. No pending command or process remains with this agent.

This checkpoint does not claim held-note/gesture destruction, creation failure, two native instances, continuous GUI/audio lifecycle, physical UI input, or completed beta acceptance. OpenSpec tasks3.3/4.2/4.3 remain open. Root must perform normal independent review, registration, integration, commit and publication before the next material boundary. No next criterion has started.

`evidence.SHA256SUMS` binds the final evidence tree (excluding itself); `candidate-sources.sha256` binds maintained source files.
