# Independent completion review: packaged resource origins

Verdict: **PASS**

The bounded resource CSP slice has no required in-scope correction. This verdict covers the frozen 27-path candidate and the finite administrative envelope below. It does not complete task 3.2, the owning OpenSpec change, or the JUCE-free beta. Publication still requires the same-reviewer final-envelope check and normal hooks.

## Identity and method

I independently reconstructed the candidate from Git status, the stage-zero index, worktree bytes, canonical executable modes, staged blobs, bytewise path ordering, branch, HEAD, upstream and merge-base. The resulting serialized manifest exactly matches SHA-256 `d16f61190213a6742bbf252b51639ebe98c40bbc9102c4dd78a6598b7f9f2698`: 27 paths, 23 additions and 4 modifications, all regular mode 100644 files, with no unstaged, untracked, conflicted or excluded edits. Branch is `lane/trigger-runtime-20261020`; base/current/explicit `origin/feat/juce-free-host-adapters`/integration-base are all `c3f6ca8adf93089dd08630151d1a4e70b11408be`. Main independently reads clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937`.

I read project/shared instructions, completion-review, coverage-and-mutation, security-by-design and reflection guidance; the captured OpenSpec status/apply results and all five returned context files; changed code; adjacent owner/resolver/tests; and original raw evidence. No implementation, build, test, native process, endpoint, index/ref operation, cache operation or delegated approval was performed. Hashing, archive inspection and source reads are review checks, not rerun test evidence. Root retains exclusive mutable-resource ownership. Network/upstream freshness remains root's pre-freeze/post-review publication responsibility; the local identities match the packet.

All 391 archived members match their complete size/mode/SHA index and the original files in `/tmp/dandrum-lanes/evidence-editor-resource-csp`. Portable SHA256SUMS validates. All 15 phase snapshots independently match each phase's own complete 19-path manifest, command cwd, exit metadata and log digest. Every one of the eight final check bindings matches all 19 current paths. Initial native coverage is therefore valid exact-source reuse, not a newly named execution. Earlier baseline/oracle snapshots remain distinct. Historical policy-choice and decision artifacts explicitly retain their earlier base and unimplemented status; current receipt/bindings establish the final state.

## Task-to-implementation matrix

| Accepted work | Implementation and disposition |
| --- | --- |
| Task 3.2 bounded local-only resource slice | One fixed `Content-Security-Policy` response-header literal in `NativeEditor::create`; script/style/font/connect self permissions, style inline allowance, default denial. Complete only for the four calibrated external classes. |
| Real browser resource regression protection | New `resource_policy.rs`, ordinary packaged `resource-policy.mjs`, valid 70-byte PNG; existing test Endpoint extended for binary bytes, MIME, no-store and test-only CORS. Registered in the existing native CTest path. |
| Preserve packaged production page | Unchanged resolver, bundle observer and native owner tests remain exercised. No frontend/backend/session functionality is claimed. |
| Default build boundary | Unchanged cfg and manifest wiring; qualified older source-free guard reuse with complete expanded input comparison, plus separately compiled/exercised native candidate. |
| Record bounded progress | Portable evidence and one progress section. Task 3.2 remains unchecked; 7/14 tasks complete. No canonical spec sync, scenario fingerprint change or premature acceptance mapping. |
| Remaining product work | Popup/download characterization, actual CLAP main-thread/API/order/timer proof, task 3.3, 4.x, 5.x and subsequent VST3/standalone/DAW/installed/offline/non-Linux increments remain open. |

The larger copied-bundle/backend/session and complete-host-lifecycle scenarios are not fulfilled by this slice. Their open status is correct. Scoped strict OpenSpec validation and spec coverage logs pass; the canonical map still reports 379 mapped of 618 criteria with 239 existing ratchet entries, not full acceptance coverage.

## Criterion-to-test matrix and causal evidence

All new negative attempts and violation listeners run in the ordinary packaged module loaded by WebKit. Native evaluation is used only by the preexisting positive production observer. For each denial the final test requires effectiveDirective and violatedDirective, `enforce`, the exact known external URL and fixed `dandrum://localhost/index.html` documentURI, together with the negative effect and unchanged endpoint request ledger.

| Criterion | Independently calibrated old-policy browser success | Final asserted denial/continuation |
| --- | --- | --- |
| External classic script | Unique executed marker and GET `/external.js` | `script-src-elem` event; rejected load, marker absent, no browser GET |
| External stylesheet | Computed height 61px and GET `/external.css` | `style-src-elem` event; rejected load, control remains 7px, no browser GET |
| External font | Real packaged TTF loads as ResourcePolicyExternal/500 and enters FontFaceSet; GET `/external.ttf` | `font-src` event; error, exact family/weight, absent from set, no browser GET |
| External image | Valid PNG loads with natural 1×1 dimensions and GET `/external.png` | `img-src` event; rejected, natural 0×0, no browser GET; header uses default-none fallback |
| Local module/CSS/style/fetch | Ordinary local module executes and reports 404/43px/37px | Same exact local receipt after enforcement |
| Unmodified production page | React Silent preview, geometry, computed CSS, rendered inline SVG, eight loaded family/weight sets | Same positive assertions plus exact bytes/status/MIME for all eleven consumed non-notice resources |
| Adjacent native behavior | Existing local fetch/navigation, show/hide/fixed-size, immediate disposal and rollback tests | Existing native test remains passing; these are regression controls, not new resource-boundary calibration |

Five independent TCP health/body/MIME calibrations precede browser attempts. The old-policy baseline asserts all four browser effects, exactly the multiset of four subsequent GETs and no violation; concurrent request order is deliberately irrelevant. The valid font is byte-bound to the declared production input. The PNG's independent IHDR/CRC/zlib evidence and browser dimensions avoid a malformed-fixture false positive.

The initial aggregate RED (`red-four-resource-denials`, exit 101) records four real effects/GETs and zero violations, then all nine expected failure entries. `green-header-only` also exits 101, but its raw events show every resource blocked and no extra GET: the only failures are the earlier origin-only blockedURI expectation. Actual resource events include full URLs. This is a calibrated oracle correction, not a second production defect. Corrected final tests fail against the old header and pass against the changed header. I compared `final-test-old-header-red` owner bytes directly to Git HEAD: identical. Its complete 19-path snapshot differs from final only by that single production header line. The additional already-launched comparable pair is retained with separate timings and is not hidden or represented as necessary new work.

Final aggregate assertions wait for every attempt, require positive local continuation, inspect exact policy identities and effects, then require the request ledger to equal independent calibration. A generic timeout, invalid resource, network outage, CORS failure, no-op browser shim or synthetic native-evaluated probe cannot satisfy these combined oracles. Test-level native timeout is 15 seconds and the maintained CTest timeout bounds the full native test. The boundary lane is appropriately slower than the fast asset lane: three final CTests pass in 4.034962 seconds (approximately .08/.06/3.88 seconds). No new blanket repetition or test-lane reclassification is warranted.

## Coverage, scope and operational limits

I read actual LLVM exports and annotated owner output. The native owner remains 148/153 lines, 14/16 functions and 208/229 regions; the changed header executes 37 times in the annotation (physical source line 78). Fast assets separately reach 52/52 lines, 5/5 functions and 86/87 regions. Native and fast profiles remain separate, with the same recorded instrumented binary SHA `9831bd524a5eee53f8f9aba253b7b567fe6da9be63f166fe4d2b39cc2148e43a`. Branch and MC/DC counts are zero, not proof of complete branch coverage.

The five inherited unhit owner lines and 21 regions remain disclosed: IPC registration refusal, non-X11 backend, connection/reparent transport failures; partial error directions include display/cast failures, parent-validation transport/race/root/depth paths, nonvalue IPC and dependency errors. Both visibility guard propagation directions at 118/131 remain unexercised; thread-affine owner invariants do not justify unsafe test forcing. Prior accepted coverage was inspected before the change. No new production branch/module/extraction or exclusion was introduced, and the sole changed line has behavioral assertions and execution evidence. These inherited whole-owner gaps remain open limitations; they do not invalidate the bounded header change. Mutation is excluded by user direction and no score is claimed.

The source-free guard comparison is independently reconstructed, including enumeration of the full copied-input pathset: 583 published inputs versus 586 current, 580 unchanged existing inputs, three changed and three new native-only files, zero removed. All 586 current digests match the supplied current manifest; old manifest SHA is `87c00e80f435b1c217438bf9ccaf9b2c4a446b2c0680f1dc38aa40579ba3af62`. Unchanged lib.rs cfg excludes the native owner and nested tests from default compilation; default features are empty and optional native dependencies remain gated. Python's native-ON leg configures and inventories CTest only; its actual default build/runtime leg does not execute the placeholder bundle as a native production proof. Thus reuse of the published 9f9252c guard execution (exit 0, 88.445691 seconds, 3 selected runtime CTests) is justified only for that unchanged default build/inventory boundary. It is not a fresh run, same pathset, all-input equality or reused native behavior proof.

Read-only real-host access independently confirms all fifteen staged bundle files match the recorded input hashes and lengths, including all fourteen declared assets and the extra 77-byte `fonts/.gitattributes`; the complete physical pathset also matches. The recorded 84,114,616-byte instrumented executable still hashes to `9831bd524a5eee53f8f9aba253b7b567fe6da9be63f166fe4d2b39cc2148e43a`. These reads used the host mount because sandbox `/dev/shm` is a separate empty mount; no missing-product-asset inference is made. Original logs/profiles/exports/source snapshots match their archive. All fifteen available frontend `dist-native` files also match the staged hashes. The resolver's fixed fourteen-entry table does not serve the metadata. This establishes exact byte correspondence, not precise historical copy chronology or preservation of every cache. No staged rebuild or native rerun was performed.

## Safety, architecture and simpler alternatives

The security decision stays at one private concrete response boundary. No caller-selectable policy, broader resource route, filesystem fallback, native privilege, dependency, CMake change, frontend change or format-neutral/audio dependency is added. Existing immutable resource preloading, exact URI resolution and navigation handling remain separate responsibilities. Test-only endpoint CORS does not affect production responses; the endpoint binds loopback with an ephemeral port, caps request input, uses finite I/O, and owns its stop/join cleanup. No uncontrolled download is used for these probes.

The fixed five-directive header is reasonable for the selected bounded allowlist. `default-src self` with style/connect overrides would use fewer directives but grant unused local resource roles. Four explicit class restrictions would leave other fetch roles without default denial. A configuration object or separate policy module would add machinery for one literal. Removing `unsafe-inline` is a possible future restriction only after deciding whether to change the established literal-style contract; React style props alone do not prove production necessity. No extraction or further refactor is justified by the current one-line production delta.

The actual image event is `img-src`, despite default-src supplying its source list. This review does not infer blanket denial of workers, frames, media, objects, manifests, hints, base changes, forms, inline/eval, popup or download facilities. Local workers can inherit script-self permission. Linux X11 / GTK 3.24.33 / WebKitGTK 2.50.4 is the measured platform, not a portability claim. Full native backend/session/audio/CLAP behavior remains unproved here.

## Full source and phase bindings

The complete current 19-path binding is:

| Path | SHA-256 |
| --- | --- |
| `CMakeLists.txt` | `e3b627179eb529887b33f6cc9c3a39542c1252628134f15b2b638dd9125ecd08` |
| `src/trigger-editor/Cargo.lock` | `861c5ce898d50232783078a83dddd697e8f37b8baa12d2a19e67be24d8b81814` |
| `src/trigger-editor/Cargo.toml` | `de516f06c2ab75b1c1c5050989ebda9beeee89ae960e48a94a0f8d733eda129c` |
| `src/trigger-editor/src/assets.rs` | `9b7e315dc6571118da9dfc01e88235e7f63354a9262619ff22ec9496e5403584` |
| `src/trigger-editor/src/lib.rs` | `79f49cb29ed9376f8119c06f687ad069d6a7f33114da686797d9962f034b2d2c` |
| `src/trigger-editor/src/native_x11.rs` | `354f93734772eaaf41df4c8d0f524c0de2424da1f31752d9c404ea90023236bf` |
| `src/trigger-editor/tests/common/mod.rs` | `46746923660a618e87bdb1f65f4ca63ef3a1f5defc250ef251f7fa2e096ef5aa` |
| `src/trigger-editor/tests/fixtures/local-policy.mjs` | `136578e291e562367d8e3f220218d5e99c149e905bb61c48b5e32c5fda9d2033` |
| `src/trigger-editor/tests/fixtures/navigation.mjs` | `3626e1d47e5d3be9d2c84d571c043005cb6ce797271e55826b46aefcfa0e8ff9` |
| `src/trigger-editor/tests/fixtures/production-observer.js` | `c64d08f416a0231e879884dad97fd45ff4c235ed7063206cb28470c5b4157c53` |
| `src/trigger-editor/tests/fixtures/resource-policy.mjs` | `83757912582dac57c5be48fced7cc91ee2b4e066ab20e0d27eb30712e011173c` |
| `src/trigger-editor/tests/fixtures/resource-policy.png` | `4ff6ab670a58c14270e034e2090d9a432caa263a14e0a25785386b0c12f880b5` |
| `src/trigger-editor/tests/internal/assets.rs` | `dace7c0861a8eebb2b4ca36891809bb7f9d63367122ecb44faed21aa41db4c4e` |
| `src/trigger-editor/tests/internal/local_policy.rs` | `cf75532edc8d9e37e637836ee717c1c3ff4cd484906a766bf45bfe60258b0158` |
| `src/trigger-editor/tests/internal/native_x11.rs` | `916ea71b2abac4d20ee76a8bf956aed285e08e633005c88688a96694ad9fb62d` |
| `src/trigger-editor/tests/internal/production.rs` | `8c3710ae967f1acdcbb76ab96416b67b0adbb5fe246b642a7d2cad1fe9a083e3` |
| `src/trigger-editor/tests/internal/resource_policy.rs` | `6402b8c32a038a8bc6c8b17b68450801d10594c36d684fca87f0f4dfba3af6ea` |
| `src/trigger-editor/tests/packaged_assets.rs` | `2520e026f6921fa697220bc40bcc4eda58711dde76529f1ac6957add63d5411c` |
| `tests/python/test_engine_only_build.py` | `44c2074f4b2c440372a419a028fcff37b78b8bb1800d592cb6ef4a7fe51186da` |

All phases below have independently verified own snapshots, run metadata and raw log hashes. Final equals means all 19 current paths, not just the production owner.

| Phase | Exit | Seconds | Differences from final |
| --- | ---: | ---: | --- |
| baseline-unchanged-four-resources | 0 | 5.085689 | native_x11.rs, resource_policy.rs |
| final-test-old-header-red | 101 | 4.049735 | native_x11.rs |
| final-test-restored-header-green | 0 | 4.943477 | none (all 19 equal) |
| green-calibrated-full-uri | 0 | 4.283982 | none (all 19 equal) |
| green-header-only | 101 | 4.118764 | resource_policy.rs |
| initial-native-coverage | 0 | 4.840210 | none (all 19 equal) |
| red-calibrated-full-uri | 101 | 4.111427 | native_x11.rs |
| red-four-resource-denials | 101 | 4.246109 | native_x11.rs, resource_policy.rs |
| release-build | 0 | 0.228466 | none (all 19 equal) |
| release-clippy | 0 | 0.459575 | none (all 19 equal) |
| release-ctest | 0 | 4.034962 | none (all 19 equal) |
| release-default-graph | 0 | 0.062970 | none (all 19 equal) |
| release-diff | 0 | 0.005654 | none (all 19 equal) |
| release-fast-coverage | 0 | 0.577608 | none (all 19 equal) |
| release-fmt | 0 | 0.057879 | none (all 19 equal) |

## Findings, lessons and publication envelope

Required findings: none. No action is required to source/tests/fixtures/raw evidence/requirements before this bounded candidate can proceed through its declared envelope. A future guidance proposal alone is not a failing finding.

One supported lesson candidate belongs to native browser test guidance: calibrate each resource class's actual CSP event serialization and successful browser load, retaining effects and endpoint accounting independently. Reusing the earlier fetch-origin shape caused a demonstrated false failure here. Proposed trigger/action: when adding a new browser resource class, retain its initial event record before fixing exact URL/directive assertions. Downside: an extra calibration artifact and platform-specific expectation. Disposition: already embodied in this slice's evidence; no standing-rule or memory edit authorized or made.

The approved finite envelope is exactly five metadata paths, resulting in 29 candidate paths:

1. Add the exact frozen external `review.md` and `verdict.json` under `docs/migrations/evidence/trigger-editor-resource-csp-2026-10-20/review/`, mode 100644.
2. Change only `receipt.json`'s review object to PASS plus the initial manifest, report and verdict digests.
3. Replace exactly one progress sentence, `Independent completion review and normal-hook publication are the next gates.`, with `Independent completion review passed for the frozen source/evidence candidate; normal-hook publication is the remaining gate.`
4. Regenerate portable SHA256SUMS over all portable files except itself.

The same reviewer must compare root's complete final 29-path manifest, exact old/new modes/digests and transformations before publication. No source/test/raw/design/spec/task semantic drift is allowed. Unexpected changes require a new whole-candidate review. Initial review files remain frozen after handoff; any final-envelope addendum may be delivered separately rather than changing their embedded identity.

Normal-hook commit message: `Restrict Trigger editor resource origins`. Compare the complete committed tree to the final manifest, fast-forward only the existing integration/original feature context, fetch without integration and require upstream to remain `c3f6ca8adf93089dd08630151d1a4e70b11408be`, then normal feature push and live/status/main verification. Installed `core.hooksPath=.githooks` and the pre-push entrypoint were read: it runs engine tests, and selects mutation only for changed engine sources; this candidate changes none. Hook execution is a remaining publication gate, not claimed review evidence. No bypass is authorized. Only the external publication receipt and existing deferred-work ledger status append follow. Preserve caches/worktrees/unknown processes; no main merge, archive, beta closure, relocation/deletion or extra outbound message is in this envelope.

The frozen report and verdict are released to root for exact copying and the declared metadata transformation. This reviewer retains no mutable native/build/endpoint resources.
