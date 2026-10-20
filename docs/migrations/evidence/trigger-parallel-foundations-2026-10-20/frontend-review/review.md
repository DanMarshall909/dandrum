# Frontend task 3.1 independent completion review

Verdict: PASS, limited to the five frozen frontend paths and task 3.1. No blocking findings. This is not completion approval for the overall OpenSpec change or any native adapter/product scenario.

## Identity and scope

Reviewed repository `/tmp/dandrum-lanes/frontend`, HEAD/base `a116c2cc3ff0d14f3fc128010db3a4e9365964e2`; recorded feature upstream `refs/remotes/origin/feat/juce-free-host-adapters` = `16184e21e957e2adcca2f50845cf33f3b49ec510`. The upstream is the merge base/ancestor of HEAD. The five scoped paths have no committed delta between those two objects. Parent reports a fresh fetch retained this upstream object and its network-enabled shared-rules preflight passed. My restricted preflight could not query the remote; it made no changes.

I independently enumerated tracked changes plus untracked files, read the diff from the frozen base and all three added files, and computed SHA256/candidate Git blobs. Every path and SHA256 matches `manifest.json`; no extra changed/untracked source paths exist. `independent-manifest.json` records candidate blobs and normalized Git modes (all 100644). The packet's `staged_object` values accurately describe the original index, not candidate content. Filesystem group-writable permissions normalize to the same Git modes.

Read authoritative AGENTS.md, shared AI Rules, coverage-and-mutation and reflect guidance, and all OpenSpec apply context files (proposal, design, tasks, both delta specs). `openspec status` and `instructions apply` identify task 3.1 exactly as the frontend selector refactor; task 3.3 owns native projection/wiring. Strict OpenSpec validation passes. Mutation is expressly excluded in the accepted task list; none was run or claimed.

## Task to implementation matrix

| Accepted task/promise | Implementation | Assessment |
|---|---|---|
| 3.1: versioned transport selection | `engine/native-transport.mjs` validates protocolVersion 1, identity and the three callable backend methods; declared DANDRUM wins, invalid declarations throw | Complete for callable JS boundary |
| 3.1: native adapter uses selector | `NativeEngine` resolves default backend through the helper and exposes backendId; explicit backend injection remains the existing test/caller seam | Complete |
| 3.1: application launch uses same decision | `main.jsx` selects inside try/catch, initializes NativeEngine for either native host, shows visible startup failure on invalid contract | Complete |
| 3.1: silent browser retained | No recognized native host selects existing MockEngine and explicitly silent UI | Complete |
| 3.1: retained JUCE comparison behavior | Legacy JUCE backend is normalized into versioned descriptor; existing invocation/feedback/cleanup implementation remains shared | Complete within existing JS/browser evidence |
| 3.1: behavior-first JS tests | Seven added unit tests, four browser tests, retained RED logs and characterization; final maintained suite and builds pass | Complete |
| 5.1 / 5.2 frontend contributions | Focused coverage, ordinary fast/slow evidence, refactor analysis, independent review | Partial contributions only; do not mark whole tasks complete |
| 2.2–2.4, 3.2–3.3, 4.1–4.3 | No Rust, Wry, CLAP, protocol serializer, resources or host integration changes in envelope | Excluded; no completion claim |

## Criterion to test matrix

Names refer to `tests/native-transport.test.mjs` (unit) and `tests/browser/native-transport.spec.mjs` (browser) unless otherwise stated.

| Observable criterion | Specific evidence/oracle | Verdict/limit |
|---|---|---|
| Retain JUCE native calls, feedback and listener disposal | Unit `the comparison host default backend preserves native calls, feedback and cleanup`: exact setParameter argument tuple/value/generation, named feedback changes value to .61, listeners empty after close | Proven at JS boundary |
| Only absence selects silent preview | Unit `only absent native hosts select the silent browser preview`; browser `an ordinary browser continues to show an explicitly silent preview` checks visible mock/silent header, editor and no alert | Proven |
| Refuse unsupported versions without falling back | Unit `incompatible native protocol versions fail without falling back to JUCE or preview` covers missing/0/2/string/null; browser incompatible-host test checks exact alert and absence of editor | Proven |
| Declared v1 host must have identity and callable contract | Unit `a declared native host must expose its identity and callable transport contract` covers wrong/missing identities and each missing required method | Proven |
| Preserve invocation semantics | Unit `native invocation preserves comparison host rejection and absolute preset paths`: string/status rejection and absolute preset-path result | Proven |
| Prefer DANDRUM to JUCE and use same calls | Unit `the versioned backend takes precedence over the comparison host and carries native capability errors`: juce-free identity, zero comparison calls, exact new parameter call | Proven |
| Preserve structured capability failure and accepted frontend state | Same unit checks identical Error object/code/method plus unchanged patch; initialization capability unit does likewise for rejected session snapshot | Proven at callable bridge boundary |
| Production JSX launch selects v1 host | Browser `the production entrypoint uses the versioned host instead of the comparison bridge`: injected JUCE throws if used, real editor renders, native header and exact title call, no alert | Proven in browser fixture |
| Startup capability failure remains visible, never simulated | Browser `a rejected native snapshot displays its capability error without simulating an instrument`: exact visible alert, no editor | Proven |
| Existing native control and adapter behavior retained | Existing `native-controls.test.mjs` tests exact parameter values/ranges and gesture bracketing; existing native-engine suite covers native publication, generation feedback, notes, meter, recovery and cleanup; all pass | Regression evidence, not native implementation calibration |
| Existing comparison layout remains usable | Existing `native-frame.spec.mjs` tests JUCE-shaped injected backend across viewports, title/resize calls, rejected preparation and no page errors | Browser geometry fixture only |

## Spec scenario coverage limits

These specs deliberately demand integrated native evidence. This five-path slice proves useful prerequisites, not any complete scenario below. Do not add full-scenario proof mappings or mark them satisfied solely by these tests.

| Delta-spec scenario | Evidence in this slice / outstanding boundary |
|---|---|
| Copied production bundle connects to the JUCE-free backend | Only production JSX using callable injected v1 backend; copied assets, real session, native identity display and actual host parent remain open |
| Invalid transport input cannot mutate the instrument | Only malformed JS descriptor and callable Error propagation; method/payload/message bounds, identities, conflict rejection, native state/public controls/audio remain open |
| Actual X11 child follows complete host lifecycle | None; Wry/X11 lifecycle excluded |
| Failed or premature parenting preserves audio | None; native failure and signed audio excluded |
| Destruction releases view-owned activity | JS listener cleanup regression only; complete WebView/held gesture/note cleanup excluded |
| Real control gesture and host feedback round trip | Retained JS knob/control regression only; CLAP events/nonzero sample offset/signed output/journal proof excluded |
| Lost editor reconnects and undoes one accepted mapping | Retained callable session tests only; destructive native reconnect/two instances/signed output excluded |
| Discover and render copied Trigger product | None; binary/resources/audio excluded |
| Fresh instance restores mapping sample and public values | None; native state excluded |
| Invalid state preserves running Trigger instance | None; native state excluded |
| Two Trigger instances retain independent session/runtime state | None; native instances excluded |

## Verification and effectiveness

Independent bounded rerun: exact `npm test` in `web/trigger`, approved unrestricted subprocess execution, exit 0, **102/102**, 1124.793104 ms; raw `independent-unit-unrestricted.log`. Initial sandbox `npm test` failed only existing component-guard (17 other test files passed); direct guard reproduced an empty subprocess diagnostic assertion, consistent with the previously recorded restriction. It was not repaired or bypassed. Independent focused `node --test tests/native-transport.test.mjs` also passed. `git diff --check a116c2cc` and strict scoped OpenSpec validation passed; source status remained unchanged.

Retained final source manifest hashes match every reviewed byte. Inspected raw V8 report URL/range entries for both changed modules and final coverage output: selector 100% lines/branches/functions; NativeEngine 100% lines, 84.53% branches, 90.43% functions. V8 reports contain source URLs/ranges, with exact bytes bound by companion source.sha256, rather than embedded source text. JSX is not measured by the Node run; its changed launch decision/error path is checked by browser assertions. Existing NativeEngine branch/function gaps are not claimed closed. New extracted selector has full coverage, backed by assertions that distinguish each meaningful selection outcome.

Inspected retained successful `npm run check`, browser/native Vite builds, targeted browser run (5/5) and full ordinary slow browser run (31/31, 1.4m). Did not repeat builds or browser suite: this bounded independent review reused matching retained evidence and reran the fast suite. Browser tests run the production entrypoint under Vite with injected callable fixtures; they do not run a copied production build in Wry or calibrate Rust serialization. Existing >500 kB native Vite chunk warning is disclosed and unrelated to selector correctness.

Assertions name concrete outcomes rather than merely checking calls happened: exact values/argument order, native feedback update, same structured Error identity, no comparison usage, no state change on rejection, visible exact errors and no editor fallback. RED logs show intended failures (missing backend identity and missing validation exceptions), not missing imports/setup failures. The pre-edit baseline reported native-engine/native-live line coverage and characterization before changing the selector; the original sandbox component-guard failure is disclosed. Fast/slow lanes remain the existing ~seconds unit/~1.4m browser division; no lane moved on one timing sample. Mutation has no score because it was excluded.

## Architecture and simpler alternatives

A single 11-line selector is shared by adapter construction and React launch; it introduces no duplicate session model, protocol serializer or native transport facade. The native engine/DSP ownership boundary is unchanged. Legacy JUCE fallback is justified by a real retained comparison consumer and its regression suite. Keeping the existing explicit backend injection avoids unnecessary caller churn. Duplicating detection inline would increase drift risk; introducing transport classes/adapters or extra fixture infrastructure would add surface without changing this bounded contract. The chosen helper is proportionate. No further extraction is warranted for this slice.

The future facade must implement the existing callable methods/positional arguments and reject unsupported capabilities; actual snapshot projection, meter capability policy, title/resize policy, wire envelopes/replies/events and destructive cleanup still require task 3.3 integration and tests. This is a stated readiness limit, not a blocker for the accepted unwired frontend prerequisite.

## Exact post-review publication envelope

This PASS covers **only** the semantic bytes, SHA256s and Git modes in `independent-manifest.json` / original manifest for:

1. `web/trigger/engine/native-engine.mjs`
2. `web/trigger/engine/native-transport.mjs`
3. `web/trigger/main.jsx`
4. `web/trigger/tests/browser/native-transport.spec.mjs`
5. `web/trigger/tests/native-transport.test.mjs`

Parent may copy these exact bytes into the already prepared integration worktree, verify candidate SHA256s/blobs/modes, stage only those paths, inspect the staged diff, and create their focused commit with normal hooks. Other integration-lane changes retain their own review obligations. Parent may separately propose source-bound evidence and task **3.1 only** status changes for review in a combined packet; those extra paths/bytes are not approved by this five-file verdict. No semantic edits are permitted after this PASS without revalidation/review.

Before normal push, fetch without pull/rebase/merge and require intended feature upstream to remain `16184e21e957e2adcca2f50845cf33f3b49ec510`; any upstream or candidate-byte change invalidates this PASS. Review final integration/source evidence and all applicable hooks before publishing. Existing core.hooksPath is `.githooks`; active pre-push runs Rust tests and conditional changed-Rust mutation, so do not claim this hook itself enforces the frontend suite. No hook bypass, force push, PR/main merge, source/build/cache cleanup, native product readiness, spec sync/archive or other-path publication is authorized here.

No transferable standing-guidance change was required by this review; no memory/rules edits were made.
