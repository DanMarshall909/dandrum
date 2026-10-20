# Frozen browser request-broker handoff

Owner returned to root. Frontend lane HEAD `29b8e90e9074aa667b2d7443376ba7aab5376fcb`; only two untracked/new source paths changed:
- `web/trigger/engine/native-webview-transport.mjs` SHA256 `b0e0ecb976d79b012487216c16216d1cc33b4f811d5c0f05b1a0a506075f0ed1`
- `web/trigger/tests/native-webview-transport.test.mjs` SHA256 `dca1d7cb887047dbd5614f16f15434683382813286fac30e5ef44d29d7792570`

Frozen copies: `frozen/`. Exact dependency/protocol/parser/artifact binding: `manifest.json`. Per-phase source snapshots, SHA256, command/cwd, start time, exit and elapsed time: each phase's `run.json`, with complete `output.log`; coverage raw V8 under coverage phase dirs. Final host phases also assert identical before/after source hashes.

## Behavior and proof

`createWebViewTransport({instanceId,attachmentId,postMessage,nextRequestId='1'})` returns descriptor, receive, dispose. Descriptor is consumed by the existing selector and declares all four optional services false. Each request preserves positional JSON arguments, canonical decimal u64 IDs and independent transport cursor. Only actual matching native replies settle successful calls. Native rejection retains its actual error object; local failures carry stable codes. Capacity128 pending and128 total listeners;16KiB outgoing/64KiB incoming UTF8 bounds. Post exceptions free retained state. Reentrant native replies can resolve before post returns/throws. Serialization callbacks are snapshotted before ID allocation and final lifetime/capacity admission; regression tests prove duplicate-ID/capacity/disposal/exhaustion hazards cannot recur.

`receive` returns `reply`, `event`, or `ignored`. Invalid, foreign, oversized, unknown, duplicate and retired replies cannot settle unrelated calls. Events support only parameterStateChanged and instrumentSessionChanged, retain payloads and increasing attachment-local u64 sequences with gaps. Listener tokens are opaque Symbols. Listener exceptions propagate after consuming the sequence; reentrant newer delivery supersedes the remaining older delivery, preserving monotonicity with permitted gaps. Registration changes during callbacks are bounded by a snapshot, removal applies immediately, additions start next event. `dispose` returns true once, clears listeners/pending, rejects outstanding calls, and makes later callbacks/calls inert without affecting another broker.

Nine TDD loops are detailed in `assessment.md`. C1 and C7 RED are honest missing module/API failures. C2 also includes an initial pending-promise cancellation; C3 initial cancellation RED was replaced before production changes with immediate independent assertion RED. Later behavior regressions include explicit assertion RED. No mutation/fault-injection run performed per scope.

## Final verification

- `final-host-fast`: ordinary `npm test`,131 tests pass,1.254s wall.
- `final-host-check`: ordinary `npm run check`, TypeScript and70 React component checks pass,1.120s wall.
- `final-host-focused-coverage`: ordinary Node coverage runner,22 broker tests pass,0.138s wall; broker100% lines/branches/functions reported. `coverage-detail.json` records13 executed functions and105 V8 regions with no zero-count regions. Offsets directly map to the exact untransformed ASCII source. Regions overlap and do not prove each boolean operand or assertion; coverage is execution evidence, not semantic correctness proof.
- `final-real-replies`: separate external calibration,0.082s. Retained actual Rust accepted initial/applied snapshot envelopes pass through frozen broker and unchanged production session parsers, preserving exact instance9007199254740993, transport request90 versus structural cursor1,23 parameters, six regions, five rules and level0.25/0.5. Original artifact SHA256 `556a660e40061706c585a6481be9013c490ed62899f680105166f4dc14cda71e`. Rejection/event wire cases are literal protocol-authority unit tests, not actual emitted rejection/event calibration; no native execution is claimed.
- Scoped OpenSpec strict validation passed; no tasks/specs/tracker changes.

Earlier sandbox package failures were Node `spawnSync EPERM`, not broker failures. Published component-guard/test source hashes match29b8; Python-launched guard detected all three real fault diagnostics, while Node child launch returned EPERM with empty stderr. Exact commands/PIDs/env/results retained in `guard-diagnosis`. Authorized host runs resolved the setup limitation without source edits or skipped guards.

## Limits and resource ownership

This is an isolated unwired browser component. No bootstrap/Wry/CLAP/native methods/UI/build/assets/dependencies/locks/Rust/spec/docs/tracker/Git/index changed. No browser, native, Cargo or server process/build/cache was started. No new worktree. All Node subprocesses completed synchronously; no owned background workers remain. Evidence is about4.7MiB, retained without deletion; initial disk preflight3.8GiB available. No durable guidance edits. Root owns independent review, integration, normal commit/publication and remaining actual bootstrap/native lifecycle integration. No full task3.3, native cleanup, rendering, deployed editor or native readiness claim.

Final refactor findings: keep this single104-line owner, since correlation and disposal share lifetime. JSON serialization is a user-code callback boundary and must precede final admission; this discovered invariant is pinned by tests. Scope does not justify a new exported helper or extracted module. The normal package suite remains about1.25s; the largest new focused bound tests are milliseconds, with no evidence supporting lane migration. Existing real browser/packaging gates remain root integration work.
