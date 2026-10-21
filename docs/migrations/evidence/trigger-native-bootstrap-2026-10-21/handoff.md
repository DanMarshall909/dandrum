# Native bootstrap frontend handoff

Source is frozen in final-source/ and SHA-256 bound by final-manifest.json. Only the four delegated source paths changed; no Git/index, fixture, package, configuration, spec, documentation or dependency change. Root owns integration/review/publication. No task 3.3 completion or native facade acceptance is claimed.

## Contract

Trusted native configuration synchronously installs the existing bounded WebView broker before production native selection. IPC keeps its Wry receiver. The descriptor exposes receiveNativeMessage for trusted JSON-string replies/events and retains four explicit unavailable capabilities. Absent configuration preserves preview/direct/JUCE selection. Invalid present configuration, missing IPC and conflicting ownership fail visibly. Same live installation preserves pending requests and transport cursor; disposal is idempotent and terminal for the document. Deleted/changed configuration or replaced descriptor/IPC cannot silently take ownership. Native attachment cleanup remains the future facade's responsibility.

Main closes store, engine and browser broker in that order on failure/pagehide; HMR first unmounts UI as before, then closes and removes the pagehide listener. A rejection from an initialization already disposed does not render a late alert. The real Rust protocol's code-only error displays its code; message-bearing errors retain their message.

## TDD evidence

- red-entry: genuine unchanged-main browser RED (native banner missing; silent preview rendered), 8.538s. green-entry: 1 browser test, 3.015s. Seven focused Node public-API tests cover installation, exact envelopes, receiver, raw rejection/event delivery, selection, NativeEngine, validation, conflict, idempotence, publication retry and terminal disposal.
- red-disposal: genuine late-render regression after pagehide, 7.792s. green-lifetime: 11 registered browser cases, 8.289s.
- red-removed-bootstrap: ownership-removal assertion failed, 0.251s. coverage-ownership: 7/7, 0.252s; final new module 100% lines, branches, functions, no uncovered source snippets.
- red-code-only: exact Rust protocol rejection rendered blank suffix, 7.791s. green-final-browser: 12/12, 9.043s; final-browser-owned repeat: 12/12, 8.213s.

Each run retains exact command, cwd, relevant environment, status, duration, output and resource samples; RED source snapshots are retained separately. Script configurations and runner sources are preserved beside receipts. First raw RED browser artifacts are in browser-output/; later outputs have unique run directories.

## Final gates

- npm test: 146/146 (139 existing + 7 new), 1.759s guarded wall.
- npm run check: 70 authored React files, 1.506s.
- Registered native-transport browser selection: 12/12, 8.213s final repeat.
- Preview production build: 2.511s, 247 modules.
- Native production build: 2.514s, 241 modules; expected single app.js >500kB warning. External output-directory preservation notices are expected.
- Raw browser V8 capture: 12/12, 17.826s with instrumentation. Browser source records are per test under coverage-browser-browser/; Node raw V8 files under coverage-ownership/ bind the new module. No mutation run, per task authority.

## Coverage limits and refactor review

Browser coverage's authoritative gap report is browser-uncovered-intervals.json: nested ranges were resolved per execution then unioned. browser-uncovered-ranges.json is retained as a raw zero-range inventory only, NOT combined uncovered coverage (a range absent in one test can execute in another). New installer conflicts/validation are covered in the faster Node lane, not all in browser. Whole-main 100% is NOT claimed: successful initialization resolving after disposal, empty native patch selection and HMR callback remain unexecuted by this browser selection; generated React interop is also unexecuted. Pagehide and initialization-failure cleanup are exercised through the actual main module.

Production refactoring centralized disposal to preserve one local lifetime, kept validation/correlation in the existing broker and used one per-document WeakMap; no generic transport abstraction or extra receiver global was needed. HMR preserves UI unmount-before-engine/broker closure. Test refactoring reused session(), grouped equivalent validation variants, asserted literal transport IDs/args and MIDI48/100/127/generation7 cleanup, and uses public behavior rather than private maps. Conflict-test finally blocks settle pending work even if the ownership oracle fails.

Fast lane is the existing Node package driver; browser selection is the existing boundary/slow lane. Focused Node suite remained roughly 0.12s runner time. The real NativeEngine/session parser case is slower than direct installer peers but remains tens of milliseconds and uniquely proves the accepted session reaches production consumer logic; no lane move is warranted. Comparable browser ordinary runs were 9.04s and 8.21s; first browser startup is consistently the larger case. Instrumented 17.83s is not treated as an ordinary timing comparison. No test membership changes.

## Calibration and resources

The code-only rejected envelope was checked against actual integration/src/trigger-host/tests/protocol.rs:672-675 and its serializer contract. The message-bearing rejection is separately allowed by the broker, not claimed to be the Rust core's emitted form. Accepted session fixture/browser IPC tests prove frontend entry/transport contracts only. Actual joined Wry/controller reply/event/CLAP/audio calibration remains open.

Earlier wrappers recorded root process group IDs and resource floors; no floor triggered. The final browser run additionally records exact observed descendant PID/start ticks/PGIDs each sample, including detached browser groups; its final remainingLive is empty. Host process inspection found no dnb/8426 server/browser leftovers. All outputs/caches were created under owned /dev/shm/dnb or this evidence root; no old caches, Rust targets or /run/user/1000 were touched. Scratch (~14MB) and evidence (~8MB before final snapshots) are retained for root review; no deletion/rebuild cost incurred. No mutation, dependency install, commit or push occurred.

Material reflection: a valid native rejection need not be an Error or include message. The actual producer's code-only contract exposed a presentation bug that an enriched fake missed; the added browser regression records this at the existing frontend boundary without changing standing guidance.
