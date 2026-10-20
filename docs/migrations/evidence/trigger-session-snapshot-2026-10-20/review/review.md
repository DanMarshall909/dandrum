# Independent completion review: accepted native snapshot component

**PASS.** No in-scope correction remains in the exact integrated candidate.
Publication is bounded by the five deterministic metadata transformations in
`approved-envelope.json`, the required final manifest addendum, named validations,
normal hooks and unchanged-upstream check. This verdict does not complete task
3.3, the editor, CLAP acceptance or the full beta.

## Candidate and independent reconstruction

Reviewed worktree: `/tmp/dandrum-lanes/protocol`.
Current branch: `lane/trigger-protocol-20261020`.
Base/current/integration object and intended remote publication base:
`dd47ec4635d9bd1e9af6de2f4f82bc77a88083ac`.
Intended upstream: `origin/feat/juce-free-host-adapters`.
Initial manifest SHA-256:
`e8fe36ad712ffdc6cd4b9f0436151c6523f8d0fa6635dc140b227dcffd8651dd`.

I independently compared the complete 14-path status inventory, index and
worktree bytes, canonical Git modes, SHA-256 digests and staged object IDs against
the manifest. All match; no unrelated dirty or untracked path is present.
HEAD, the explicit upstream ref and their merge base match the declared object.
The protocol lane has **no configured tracking upstream**; the reference above is
the explicitly selected publication upstream, not a successful `@{u}` resolution.
Normal publication must target the feature ref named in the envelope.

The three source hashes remain:

| Path | SHA-256 |
| --- | --- |
| `src/trigger-host/src/parameters.rs` | `cfcfed9254885573cd4bf7253865df9d5ab9e5f88c3068492f39d0dc5aabb4b2` |
| `src/trigger-host/src/protocol.rs` | `1e12b7c163de522478ffe081cff47385209edc7427d8cf8366bc4271c1483a21` |
| `src/trigger-host/tests/snapshot.rs` | `d93cbf1f16216c1413d25027f56d731832b3aad62a20e13603ede69f3179f9be` |

The other paths are the progress record and ten portable evidence files. I
verified every portable checksum, all 114 raw archive payload digests and all 20
integration archive payload digests. All 587 integration source-manifest entries
match the current candidate. Five omitted rebuildable binary/profile payloads
are explicitly inventoried externally; they are not represented as portable.

I read the applicable project/shared rules, review reference, current proposal,
design, tasks and delta specifications, production source and consumers, tests,
raw RED/final/integration logs and receipts. The retained preliminary review
provided source analysis; I reassessed integration and the whole staged packet.
No repository mutation, test/build/native execution, cache operation or delegated
approval was performed by the reviewer.

## Task-to-implementation matrix

| Scope | Code or record | Finding |
| --- | --- | --- |
| Task 3.3 accepted native snapshot component | `ParameterController::sampler_session_reply` | Real accepted YAML, prepared geometry, resource provenance and applied public state projected off audio |
| Native read admission authority | `EditorProtocol::admitted_snapshot`, private `admission_owner` | Connection identity and retained request/command validate access before native session read |
| Bounded reply | Existing `encode_reply` / `encode_bounded` | 64 KiB limit remains enforced; failure returns no partial reply |
| Format-neutral/source-free boundary | Existing host modules; snapshot test; recorded affected fixture verification | No new dependency or adapter/browser type; normal Rust tests need no Node or web tree |
| Durable evidence and status | Portable evidence and migration progress | Records original limitations and integration results without overwriting failed attempts |
| Remaining task 3.3 and editor/CLAP acceptance | Current unchecked task list | No general facade, full required method/lifecycle integration or task completion claimed |

Tasks 3.2 and 3.3 remain unchecked; seven of fourteen tasks are checked. No
canonical specification synchronization, acceptance-map change or new full-scenario
completion claim is made. The broader plugin scenarios still require their real
host/editor gates.

## Criterion-to-test matrix

All named tests are in `src/trigger-host/tests/snapshot.rs` unless stated otherwise.

| Observable criterion | Executable evidence and oracle |
| --- | --- |
| Actual copied kit identity and public controls | `accepted_snapshot_projects_real_kit_geometry_and_applied_values_without_changing_session`: literal 23 aliases and instrument ID, exact copied source path and accepted YAML |
| Prepared source/region/map geometry | Same test: 48 kHz, mono, 51000 frames, six literal region bounds, seven literal resolved zone mappings, round-robin seed `"2026"` and authored groups |
| Full-precision identities and distinct cursors | Same test: instance `"9007199254740993"`, transport request `"90"`, structural request `"1"`, revision `"0"`, exact limits/history object |
| Defaults separate from applied values | Same test: queued level stays .25; actual audio processing changes current to .5, retains default .25 and renders literal stereo (-1, -1) |
| Snapshot reads preserve attachment/history | Same test: native snapshots remain equal and next attachment is `"2"` after repeated reads |
| Foreign/forged/retired admission cannot read | `snapshot_reply_rejects_foreign_retired_and_forged_admission_without_leaking_data`: different controller with equal numeric IDs, forged numeric attachment/request and retired connection reject; other owner remains readable |
| Unsupported dispatch and oversized failure | `snapshot_reply_does_not_dispatch_other_commands_and_returns_no_oversized_message`: exact errors and native state retention after a real oversized accepted document |
| Grouping follows real targets, authored metadata survives | `accepted_authored_metadata_and_target_groups_survive_projection_and_state_replacement`: deliberately misleading aliases, exact loop/fade/root/gain/pan/reverse data, effective zone bounds/weight/tuning and empty absent authored identity |
| Snapshot follows accepted replacement | Same test: actual loaded replacement YAML, revision 1, generation 2, new key mapping and retained empty history |
| Valid Linux non-UTF-8 provenance fails explicitly | `unrepresentable_resource_provenance_returns_an_error_without_changing_the_session`: real filesystem path, host error and unchanged native snapshot |
| Production frontend accepts actual native DTO | Separate retained final `projection.rs` + `parse-production.mjs`: actual host/controller/audio output is consumed by unchanged production `patchFromSession` and `parseSessionConnection`; five displayed rules, two real selectors, identities, geometry and default/current values asserted |
| Integrated default Rust path remains source-free | Recorded CMake release build, dependency tree and three CTests: all 67 host tests, four editor asset tests and dynamic fixture tests run with web/ui/third_party absent from the copied fixture |

## Acceptance evidence, TDD and test effectiveness

The original API-absence RED is a genuine compile failure for the intended
method, explicitly not a runtime assertion. Its preserved tests already specify
the initial data, bounds and authority contracts. Later RED logs show the actual
invented zone ID assertion and non-UTF-8 serialization panic; final tests retain
both protections. Source/log hashes for those phases were independently checked.
The final literal alias list strengthens the existing data contract.

The tests use real accepted packages, preparation, runtime application and
replacement. They assert fixed values, including signed audio, rather than only
comparing outputs with each other. The authority test deliberately collides
numeric identities across owners. Tests remain at the owning crate boundary,
with distinct failure cases and small shared setup helpers. Five snapshot tests
take .07–.08 seconds in retained runs. The separate parser calibration uses the
actual consumer, not a more permissive substitute.

Final isolated source verification at planning base
`0f02f575ff2419082370f4d35986d086c03b5d0b` records 67 host tests/LLVM export,
strict all-targets Clippy, formatting and actual Rust-to-JS parsing passing.
The recorded elapsed times are 6.282 s, 1.258 s, .252 s and .754 s.
Those checks are not retroactively described as runs at the later integrated base.

The integration changes among the original 20 bound inputs are runtime lease
admission behavior and an instrument-session trait comment. I inspected those
diffs: snapshot readers/parsers and scalar application used by the calibration
are unchanged. The new integrated source-free run directly retests the host,
including replacement and all five snapshot cases.

The initial integration CTest failure genuinely reports missing
`sampler_session_reply` in a stale cached debug dependency. The preserved retry
script changes only two fixture source timestamps and checks unchanged source
bytes. Its raw output shows the host recompiling, then all 67 host tests passing
and all three CTests passing. Release build and dependency checks also pass.
I verified receipt-to-log hashes and summed the host test groups independently.
The same 587 candidate hashes are checked before and after by the retained
driver. This is sufficient affected-boundary integration evidence, not a fresh
complete default-profile guard or actual Trigger WebView/CLAP acceptance.

Scoped strict OpenSpec validation and the acceptance map check pass in retained
logs: 618 scenarios, 379 mapped, 239 existing todo entries. No new ratchet entry
or checkbox completion is concealed. Final validations remain in the envelope.

## Coverage, architecture and operational safety

Raw LLVM file summaries confirm parameters 49/49 source functions, 414/415 lines,
702/714 regions; protocol 24/24, 357/357, 497/513. The inherited parameter
state-restore fallback at line 181 is the sole wholly unhit physical line in
that owner. Partial regions and unexecuted generic instances remain disclosed;
branch data was not collected. These are not whole-host or exhaustive branch
claims. No new module, dependency, coverage-only seam or exclusion was added.
Mutation is excluded by recorded user direction.

The private allocation identity binds requests to their original live protocol
connection; matching numeric IDs alone are insufficient. Retained request and
command validation precede native session access through the connection's own
attachment. The public `request.attachment` field is deliberately not read
authority. Mutating only that ignored field is not claimed rejected and cannot
select a different owner. Existing state-loading validation preserves the alias,
root-index, range and default invariants required by the projection.

The seal is allocated/cloned on protocol control paths; DTO parsing, allocation
and encoding occur off audio. No Clack/Wry/GTK/X11/JUCE type or browser dependency
enters the format-neutral owner. No PCM or fabricated measurement is emitted.
Oversize and provenance failures preserve native state. Resource receipts retain
the declared floors and completed owned-group checks; no cache deletion is part
of this candidate or its approved envelope.

## Alternatives and reusable lessons

The focused controller method with one protocol-private authority check is
smaller than introducing a general dispatcher or separate public DTO module.
A numeric-only check would be weaker; checking only the public native attachment
would not bind request admission to its original protocol connection. A broader
API immutability redesign is unnecessary for the accepted read operation.

The retained stale-timestamp diagnosis is useful local operational evidence:
byte equality alone does not force timestamp-based build invalidation when
reusing copied fixtures. The retry directly addresses it without deleting
reusable caches. No new mandatory standing guidance is proposed in this review.

## Publication soundness and remaining boundaries

The candidate plus exact five-file metadata envelope is ready for normal
publication. Root must return the complete final staged manifest for this same
reviewer's external final addendum before committing. The envelope carries exact
resulting hashes, required validation commands, normal-hook policy, explicit
feature publication ref and unchanged-upstream object. Any unexpected content,
validation, upstream or push drift invalidates this verdict.

I inspected the active `.githooks` setting and entrypoints: pre-commit invokes
`scripts/check-rust-coverage`; pre-push runs engine Cargo tests and selects changed
engine files for mutation. This candidate changes no engine source. The absent
lane tracking configuration makes that hook use its documented last-commit
fallback if invoked there; publication must still explicitly check the intended
feature upstream. No hook bypass is approved.

No source correction, acceptance gap or unresolved scope decision blocks this
component. Remaining required-method wiring, notes/heartbeat, mapping recovery,
native undo, complete destruction/reconnection, real Wry/CLAP parent acceptance,
DAW, installed-release and wider beta gates retain their existing owners and
open status.
