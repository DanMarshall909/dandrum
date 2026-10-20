# Accepted native session snapshot tracer — frozen handoff

Scope: task 3.3 real format-neutral snapshot projection. Base `0f02f575ff2419082370f4d35986d086c03b5d0b`, existing `/tmp/dandrum-lanes/protocol` lane. No commits, staging, task/spec/checklist updates, manifests, locks, other crates, browser files or runtime source changed. Root owns integration/review/publication. Exactly three candidate paths: `src/trigger-host/src/parameters.rs`, `src/trigger-host/src/protocol.rs`, new `src/trigger-host/tests/snapshot.rs`.

**Current status: implemented, source frozen, final-source verification pending resource release.** Five snapshot tests and all 67 host tests passed before two final changes: replacing a redundant error-mapping closure with the same two enum constructors, and adding a literal 23-alias assertion to the existing kit test. Formatting passes on the final bytes. Strict Clippy passes with the final production bytes but before the last test-only assertion. Do not claim final-source LLVM/test/real-parser verification until the commands below are rerun. No OpenSpec task completion is claimed.

## Behavior and API

`ParameterController::sampler_session_reply(&AdmittedRequest) -> Result<String, ParameterError>` is the focused future facade entrypoint for admitted `getSamplerSession`. It returns the existing protocol's bounded accepted envelope, containing actual accepted `{yaml,sourcePath,document,connection}`. Other method kinds return `UnsupportedCommand`; this does not implement notes, mapping, jobs or a general dispatcher.

The DTO reads accepted session YAML/prepared assets, copied canonical document provenance and current applied runtime values off audio. Defaults remain separate from current values. It projects real control groups from root port targets to `sample_map_player.group_N_*`, never dotted alias names; source/region/map declarations and prepared resolved indices/effective frame bounds; decimal full-precision session/attachment/revision/structural-request/frame/seed fields. No PCM, fabricated telemetry or browser dependency is introduced. Snapshot reads reuse the editor and preserve session history and structural cursor. UTF-8-unrepresentable Linux provenance returns an explicit host error, without a panic or altered state. Existing 64 KiB reply encoding returns no partial message on overflow.

Protocol-private per-connection `Arc<()>` identity plus retained request ID/command validation binds the operation to this protocol connection. Cross-controller requests with identical public IDs and forged numeric attachment IDs reject. The native attachment used to read the session is recovered from the private connection; public `AdmittedRequest.attachment` is deliberately not used as read authority. Swapping only that unused public field cannot read the swapped owner's state: the original sealed connection remains authoritative. This is narrower than claiming all public-field mutations are rejected. Later parameter/notes dispatch was not redesigned here.

## Tests and RED evidence

All new tests use copied real YAML/WAV with normal Rust/tempfile APIs. They neither load browser source nor run Node, and therefore remain compatible with the existing source-free fixture's copied Rust sources/examples. Timing is fast owning-crate in-process coverage: five tests together 0.07–0.08 seconds after build. No mutation run was performed; user explicitly excluded it.

1. `accepted_snapshot_projects_real_kit_geometry_and_applied_values_without_changing_session`: real identity/provenance, exact instance `9007199254740993`, transport request90 versus structural cursor1, generation1/revision0/history0, all23 literal aliases, target groups, defaults/ranges, 48000Hz mono51000frames, six literal region bounds, seven literal key/velocity/group/resolved-zone geometries and real candidates. Queued level remains .25; after actual audio application current .5/default .25 and literal stereo `-1`; repeated reads preserve accepted snapshot and next attachment remains2.
2. `snapshot_reply_rejects_foreign_retired_and_forged_admission_without_leaking_data`: foreign controller with equal public instance/attachment IDs, forged connection ID, unadmitted request ID and retired editor reject; independent owner stays readable.
3. `snapshot_reply_does_not_dispatch_other_commands_and_returns_no_oversized_message`: unsupported method stays unsupported; a genuine accepted document with a large name cannot produce an oversized reply and its session remains intact.
4. `accepted_authored_metadata_and_target_groups_survive_projection_and_state_replacement`: real authored loop/root/gain/pan/reverse/fades and prepared zone override/weight/tune; misleading alias names prove target-derived grouping; absent authored zone ID stays empty; state replacement yields actual new YAML/mapping/revision1/generation2/history0.
5. Unix `unrepresentable_resource_provenance_returns_an_error_without_changing_the_session`: real accepted package under a non-UTF-8 filename rejects projection with `ParameterError::Host`, preserving native state.

Exact raw logs, test source snapshots, pre-correction production snapshots and SHA-256 records are in `phase-manifests.json`:

- `red-complete-criterion.log`: E0599 missing intended API, exit101, 0.301s; honestly compile/API-absence RED, not a runtime failure. Initial narrower missing API run17.231s is also retained.
- `red-authored-zone-identity.log`: real assertion expected empty ID, actual invented `"0"`, exit101, .908s. Smallest change uses authored optional ID or empty, removing invented index fallback; four tests then pass1.725s.
- `red-provenance.log`: real `json!` panic on valid Linux path containing invalid UTF-8, exit101, .996s. Smallest change validates representability and returns explicit error before DTO construction; five tests pass1.725s.

`friction.json` also retains the unavailable `python` launcher (no Cargo/test execution), initial mistaken `RequestId.get` compile correction (not claimed GREEN), the initial report/receipt filename collision (profiles re-exported unchanged), and failed first one-off CommonJS loader attempt. Failures were not relabelled as passing evidence.

## Measured checks and limits

- `green-provenance`: five tests pass1.725007s.
- `final-llvm-run`: **67 host tests pass**, 5.948524s, all normal host test targets, not just the snapshot test. Includes existing signed audio, parameter/gesture, state and callback allocation/deallocation tests. `final-llvm.json` is the full retained report.
- Measured owner summary: parameters49/50functions,413/415lines,701/714regions; protocol24/24functions,357/357lines,497/513regions. The new unhit closure was the structural-cursor error adapter; final refactor replaces it with constructors and must be remeasured. The other physical annotated zero line is inherited `CommitError::Session` state-restore handling at parameters181. These are not whole-crate100% claims. LLVM region coverage is not branch coverage (branch collection count0).
- All newly executable physical lines in snapshot/protocol methods were hit in that measured candidate; unexercised regions include preparation-proven missing target/node/multi-target branches and structural-cursor exhaustion. No synthetic counter setter or coverage-only seam was added. Sources/maps/optional loop data are covered through actual prepared accepted documents.
- `final-clippy`: strict all-targets Clippy `-D warnings` passes18.467714s on final production bytes. It predates the extra literal alias assertion in the test, so rerun for final-source closure.
- Final formatting and `git diff --check` pass; no extracted production module was added. Refactoring: simplified redundant error mapping, removed invented absent-zone identity and its enumeration; kept the focused DTO in the existing owner instead of adding an unnecessary public module. Test refactoring retained small package/open/request/reply helpers and independent literal metadata tables; separate authority/bounds/provenance scenarios have distinct oracles.
- Existing source-free/build/native/combined frontend integration checks remain root integration gates. No Wry/CLAP/session editor initialization, lifecycle, DAW, installation or full-beta readiness claim is made by this slice.

### Real production parser calibration

One-off `projection.rs` links the actual instrumented host rlib and opens a real copied package; it emits actual initial and applied bounded replies and asserts literal signed stereo -1. `parse-production.mjs` directly imports unchanged production `patchFromSession`/`parseSessionConnection`, with a loader resolving the already installed exact YAML2.9.1 package. It checks precise decimal IDs, voice limit8/oldest, real source geometry/duration/provenance, six regions, five displayed rules, two two-candidate selectors and current/default scalar separation. `calibration.json`, actual replies, binary/source hashes and all stdout/stderr are retained. This passed before the subsequent provenance error handling/refactor and extra literal assertion; final rerun is pending. First loader failure remains in `calibration-attempt-1`. No production web path, dependency or default Cargo test was changed. This is a separate one-off boundary calibration, not a maintained native integration test.

## Frozen source and remaining commands

`source-manifest.json` binds candidate/supporting engine/session/protocol/fixture/parser/spec inputs. `final-source/` copies the three final candidate files. `measured-source/` reconstructs the last LLVM candidate by reversing only the two explicitly documented subsequent edits; it is labelled reconstructed, not a contemporaneous captured manifest. `tracked-candidate.patch` excludes the untracked new test, whose complete bytes are separately in `final-source/snapshot.rs` and the worktree.

Final SHA-256:

- parameters.rs `cfcfed9254885573cd4bf7253865df9d5ab9e5f88c3068492f39d0dc5aabb4b2`
- protocol.rs `1e12b7c163de522478ffe081cff47385209edc7427d8cf8366bc4271c1483a21`
- tests/snapshot.rs `d93cbf1f16216c1413d25027f56d731832b3aad62a20e13603ede69f3179f9be`

**Resource handback:** `cache-runtime` was released to root before focused LLVM. `host-snapshot-cov` was released with no active process after warm Clippy. Root copied it byte/mode/hash-preservingly to `/tmp/dandrum-lanes/cache-host-snapshot-cov`; the original real-host RAM copy is retained. Auto-review rejected removing that duplicate under the earlier different-cache approval, and user confirmation is pending. **Do not mutate either copy or run Cargo/LLVM until root GO.** No cache was deleted by this agent; all evidence, reusable targets and dependencies remain. Actual host last RAM free1,162,846,208B; disk had~10GiB free before root copy. `/dev/shm` under sandbox differs from actual host; use the elevated host tool for cache checks when authorized.

After root confirms relocation/logical symlink and reassigns the cache, run sequentially with jobs2, preserving both LLVM target and build variables:

1. `CARGO_LLVM_COV_TARGET_DIR=/dev/shm/dandrum-beta-20261020/host-snapshot-cov CARGO_LLVM_COV_BUILD_DIR=/dev/shm/dandrum-beta-20261020/host-snapshot-cov cargo llvm-cov --manifest-path src/trigger-host/Cargo.toml --offline --locked --json --output-path <new-final-report.json>`.
2. `CARGO_TARGET_DIR=/dev/shm/dandrum-beta-20261020/host-snapshot-cov cargo clippy --manifest-path src/trigger-host/Cargo.toml --offline --locked --all-targets -- -D warnings`; formatting check.
3. Preserve previous calibration output under a phase directory, then `python3 /tmp/dandrum-lanes/evidence-host-snapshot/calibrate.py` against the freshly measured host rlib. Its hardcoded logical cache path is intentionally stable.
4. Recompute source/evidence hashes, inspect coverage gaps against final bytes, then root combined source-free/default/native integration and independent review. No task checkbox completion before its full integration proof.

The run helper's receipt records the outer default `CARGO_TARGET_DIR=cache-runtime`; post-release commands explicitly override it in their `/usr/bin/env` command or both LLVM target/build variables. Those actual argument lists and binary paths prove post-release runs used only the snapshot cache. There were no post-release Cargo jobs in cache-runtime.
