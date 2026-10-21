# G3 held activity destruction handoff

Author scope complete; ready for root integration and independent review. No Git mutations, task checkboxes or beta-completion claim.

## Candidate

Runtime base: `16a4dafa1994211ddad2d80e22038f20e3e61f19`.
Only maintained change: `src/clap-adapter/tests/internal/trigger_note_native.rs`.
Final SHA256: `edb8ff581897df35ed3f70415b8ea5037d17770145563e387827bf6656266aee`. See `final-source.diff`, `final-test.rs`, and per-run source snapshots.
Selector: `note_native::copied_native_destroy_releases_held_activity_once_and_reconnects`.

The existing production cleanup already works. The initial run failed because the proposed ID3 oracle was copied from the full kit while HeldSample declares one alias, drums.level, whose public ID is1. This is an oracle/setup failure, not product RED. The corrected test independently checks public count1/name/ID1 and passed unchanged production on its first qualified execution. Exact initial failure, source preimage, Cargo artifacts and qualified PASS are retained. Production source was never modified.

## Checked behavior

The actual copied ordinary DSO and production app execute authenticated beginGesture/setParameter/noteOn without sending End or noteOff before complete GUI destruction. The cumulative Recorder retains every process/flush output, so a render cannot hide a duplicate. Public metadata binds ID1 to the fixture alias. Actual accepted ledger is `[Begin(1), Value(1,0.5), End(1), Begin(1), Value(1,1.5), End(1)]`, exactly6 attempts, with old attachment1 and fresh attachment2. The final native log prints these observations.

Signed expectations derive from PCM16 input and physical gain: normalized0.125 gives physical0.5, editor onset-0.25, tail-0.0625, host onset with editor tail-0.3125, combined tail-0.125, post-destroy host-only-0.0625, host NoteOff gives exact0. Cleanup is observed before1800ms, below native expiry2s; the fixture independently bounds total rendered frames below half of480000, preventing natural sample exhaustion. Final ordinary run rendered12032 frames.

The actual XID disappears and WebView weak reference expires. Repeated destroy, real flush and bounded GLib/main service offer no additional automation. Fresh attachment authenticates getSamplerSession, retains physical0.5, begins silent, plays/releases a new signed note and accepts the balanced new gesture. Its later destruction also adds nothing.

## Verification

- `final-candidate-native-regression`: all16 exact cases PASS,25.406s. Existing parameter/structural/note/callback-audit regression plus this case. Each selected case is discovered and executed in its own subprocess; zero-test success is rejected.
- `owner-tests`: all13 adapter fast owner cases PASS,2.030s controller wall; includes detach/End mirror ownership and scheduler lifetime. Owner sources are unchanged by subsequent test-only strengthening.
- Final fmt and release `trigger-editor` lib/tests Clippy with `-D warnings` PASS.
- `final-llvm-dso-test`, `final-llvm-host-test`, and `final-restored-normal-test`: PASS. Final ordinary case2.36s test body in full regression; retained exact run receipts distinguish test and controller wall time.
- New case remains in the explicit ignored native/slow lane. Its real copied DSO/WebKit join is the unique contract; fast owner laws are retained. No lane membership change or mutation run.

Ordinary DSO SHA256 `17251daa8ecbcc4ec3da600e04056e2338426926c0a274ebb84b70e72e810df3`.
Final ordinary host ELF SHA256 `51035832bb31d8381ceaf193aba09cbc020b368ddda9faf2870b40669f60659a`.
Production app SHA256 `4b9ce5a356fea5f22703f9046c77a80ac616c00242348a12fe81f58fe4f760f2`.
Cargo test graph DSO differs (`e9200f...`) from the ordinary product graph; both bindings are explicit, and native acceptance uses the ordinary DSO. `final-product-build-binding.json` retains separate actual Cargo-selected DSO/test provenance and hashes. Compilation/instrumentation recipes retain compiler/source/extern identities, output paths and resource bounds. Only builders use the existing16MiB compiler stack override; native runs have no runtime stack override/preload except the deliberately isolated existing allocator-audit child.

## Coverage and refactor checkpoint

`held-dso-final-coverage-binding.json` binds fresh profiles to the final normal host and unchanged instrumented ordinary DSO recipe. `final-focused-production-coverage.json`: destroy_editor13/13 code regions; CLAP GUI destroy7/7; cleanup_attachment17/20. Its three error-propagation spans and their closures do not execute in this normal public lifecycle; no whole-owner, branch100% or error-injection proof is claimed. No production bytes changed and no production refactor is warranted.

`held-host-final-coverage-binding.json` and `final-focused-test-coverage.json` bind the final source test to its separate instrumented host with ordinary DSO. The new test body261/261 regions plus wrapper6/6 execute. Every HeldAudio function/closure region executes, including the newly retained output. No newly extracted production module.

Test refactor: reuse the existing actual-parent/copied-plugin/long-sample/audio helpers and retain Recorder in HeldAudio; keep the lifecycle phases and independent literal oracles visible. An extra helper abstraction would hide this one linear scenario. Coverage review strengthened the independent pre-expiry bound and raw successful ledger; final native/strict/coverage checks all bind that strengthened source. `qualified-test.rs` preserves the earlier qualified PASS preimage. Raw exports/profiles remain separate for initial and final test versions; final raw profiles and merged data also copied to `retained-profiles/`.

## Scope limits and prior calibration

Per-block cross-thread processing is not continuous concurrent lifecycle audio. Weak-view disposal and finite late dispatch do not prove universal callback quiescence. Reuse the actual handler-release calibration in `docs/migrations/evidence/trigger-native-handler-2026-10-21/README.md` and `root-checks.json`: current production SHAa2803e24488a04350d951fc2809c7afe2025a191fb38995efe9610181e2aea73 and test SHA73ce40cf06504dfb596eeb47022aeabefc982c118a753ec82f333dcc26a41e35 match that proof exactly. This case adds the native held-activity join, not a broader callback policy.

Physical keyboard/pointer, injected creation failure, continuously running audio lifecycle, two edited native instances and all later beta gates remain open. Root retains registration, specs/docs/tracker, independent review and publication. No next criterion started.

## Explicit resource/source handback

All source ownership is returned to root, including the sole changed test and unchanged scoped production files. Both exclusive leases are returned idle:
- `/tmp/dandrum-lanes/cache-editor-guard`
- `/tmp/dandrum-lanes/cache-host-snapshot-cov`

`final-process-cache-audit.json` is a host-namespace audit: no remaining processes in any recorded owned PGID; no live host builders. All tool sessions finished. Source status contains only the intended test change and the known untracked `cache-editor-guard/`, which remains1766 files and was never used as a target or modified. Root cache-runtime was not used for builds. No caches, dependencies or other data were deleted, downloaded or relocated. All commands were offline/locked where applicable and used jobs1. Minimum sampled disk3508068352 bytes; minimum MemAvailable11967778816 bytes; no floor stops. Final disk3509825536 bytes.

Ordinary selected DSO and copied DSO both match the ordinary hash after LLVM; `final-ordinary-restoration.json` and successful `final-restored-normal-test` verify restoration. `resources-restored.json` verifies all17 copied resource files against their original fixture tree. Retain the179MiB evidence directory and the focused instrumented artifacts/profiles under the coverage cache for review; no cleanup authority was assumed.
