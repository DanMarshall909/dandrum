# Independent Trigger completion review

**FAIL** — the complete candidate plus exact envelope requires three in-scope corrections. No publication or envelope application is authorized by this review.

## Frozen identity

```json
{
  "manifestSHA256": "85ef576a81c38168cb35ec3713b80da28107570aea24174866144d2c57965e10",
  "envelopeSHA256": "0c36f534f7f358cee0d0cd47830c1efa39ac25938a053af0d0e8c64629bbc1e5",
  "candidatePatchSHA256": "ed582c11fb369231c74ef8bbbb2c0e04c9db3ede8dd316117ba0e4c0863a8ae2",
  "paths": 347,
  "branch": "work/trigger-react-shell-2026-10-06",
  "baseOID": "d02ee1b6126fa9fb579ac8380716c691a280abdb",
  "currentOID": "d02ee1b6126fa9fb579ac8380716c691a280abdb",
  "upstreamRef": "refs/remotes/origin/main",
  "upstreamOID": "d02ee1b6126fa9fb579ac8380716c691a280abdb",
  "integrationBaseOID": "d02ee1b6126fa9fb579ac8380716c691a280abdb",
  "independentReconstruction": "Matched all path/status/mode/content/index records, staged blob bytes, canonical manifest serialization and exact cached binary patch; 20 source files match ZIP."
}
```

Read authority: repository AGENTS.md, shared ai-rules.md, completion-review.md, coverage-and-mutation skill, received shell/implementation plans and owning OpenSpec proposal/design/spec/tasks. The explicit user waiver of test-first ordering is honored. Review was read-only; only external reviewer records and ignored test/build outputs were written. No approval delegation occurred.

## Findings

### R1 — P2: Queued parameter commands capture stale inverse values

Location: `web/trigger/engine/commands.mjs:2` (parameterCommand / Store.dispatch). Tasks: 1.2, 2.7.

reviewer-regressions.mjs test R1: while renameNode preparation is pending, queue Cutoff .5 then .8 in distinct gestures; undo returns .42 instead of .5. Both accepted edits and three undo entries are asserted first.

Consequence: Undoing one gesture also loses the preceding accepted gesture. The existing tests separately cover queued prepared commands and synchronous parameter gestures, but not their composition.

Required outcome: Capture the inverse parameter operand at first execution, preserve it through redo/merge, and assert queued distinct gestures undo and redo in order. Also retain current coalescing and cancellation behavior.

### R2 — P2: Undo of an imported replacement leaves dangling Source history

Location: `web/trigger/engine/operation-command.mjs:15` (scopes.importSamples). Tasks: 1.2, 2.1.

reviewer-regressions.mjs test R2: import replacement.wav with {replace:"k60f"}, then undo. Region.assetId is restored to k60f and asset-1 is removed, but history src.config.asset remains asset-1. replaceRegionAsset changes history, which importSamples filters from its delta.

Consequence: An ordinary Replace sample / Undo leaves the accepted patch and its Source operation inconsistent, violating complete reversible editing.

Required outcome: Include every operand changed by the composite replacement operation, including Source history, and test replacement undo/redo across assets, region and history while preserving unrelated fields.

### R3 — P2: The declared handoff adapter cannot satisfy the Store dependency

Location: `web/trigger/engine/contract.ts:29` (EngineAdapter / operationCommand.undo / Store.getLog). Tasks: 1.2, 3.3.

reviewer-regressions.mjs test R3 restricts MockEngine to all declared EngineAdapter methods. Rename succeeds, structural undo throws "engine.applyDelta is not a function". Store also directly consumes mutable log and setMockSwitch, neither declared. engine-contract.md labels applyDelta mock-only even though all structural undo/redo unconditionally require it.

Consequence: An implementation of the published EngineAdapter cannot replace MockEngine as promised; the concrete-mock-to-interface assignment check misses dependencies used by the consumer.

Required outcome: Make the actual Store/command dependency a complete typed and documented boundary (including reversible operands), and isolate or explicitly type diagnostics. Verify a contract-conforming adapter can execute edit/undo/redo without undeclared MockEngine members. No native implementation is required.

All three failures are reproducible with `node --test /tmp/dandrum-trigger-final-review/reviewer-regressions.mjs`; raw TAP output is in `reviewer-regressions.log`. The tests fail for the named observed behavior, not setup or discovery errors.

## Task-to-implementation matrix

| Task | Implementation | Evidence | Assessment |
|---|---|---|---|
| 1.1 | docs/trigger/reference; vendor/design-system; tokens/fonts; final/reference | All 20 preserved reference files match archive bytes; archive digest matches. Guard/build and captures provide evidence. | EVIDENCED |
| 1.2 | engine/{contract,store,commands,operation-command,entity-delta,mock-engine,presets} | engine, store-boundary, preparation, adapter-operations tests pass; R1/R2/R3 disprove complete reversible adapter boundary. | UNMET: R1,R2,R3 |
| 1.3 | TriggerController; TriggerFrame; components; shell/projections | presentation/model-bindings and frame/shared-controls; exact geometry JSON; source/render comparison. | EVIDENCED |
| 2.1 | sample-model; history-model; SamplePage; domains/history; import-actions | slices-history/editing/shared-controls provide substantial coverage; imported replacement undo loses Source operand. | UNMET: R2 |
| 2.2 | slices-model; analysis-model; domains/slices; assets-jobs | slices-history/preparation/adapter-operations and browser slices/states assert markers, progress, cancellation/failure preservation. | EVIDENCED |
| 2.3 | mapping-{geometry,clipboard,model}; filename-mapping; import-mapping | mapping/import/model-bindings and editing/states assert rectangle trimming, roots, policies and current model. | EVIDENCED except composite replacement R2 |
| 2.4 | layers-model; CandidateTable; VelocityCrossfade; selector-sim | adapter-operations/telemetry/model-bindings and instrument assert policy, weights, mute/solo, reorder and feedback. | EVIDENCED |
| 2.5 | voice-model; modulation-model/actions; macro-model; EnvelopeEditor | control-regressions/model-bindings/adapter-operations and instrument/modulation/shared-controls assert actual values, fixed route, MIDI, templates. | EVIDENCED |
| 2.6 | routing-model; domains/routing; ProcessorStack; RoutingGraph | instrument/shared-controls and adapter-operations assert outputs, sends, add/bypass and exposed controls. | EVIDENCED |
| 2.7 | ParameterKnob; control/entity menus; computer-keyboard; input-actions | editing/runtime/modulation/shared-controls and store-boundary cover normal input, but queued gesture inverses fail. | UNMET: R1 |
| 3.1 | telemetry-sim/reader; LogDrawer; computer-keyboard | telemetry/preparation/runtime assert independent subscriptions, release and replacement cleanup; silent simulation limits explicit. | EVIDENCED |
| 3.2 | PresetBrowser; requestReplace; import-actions; AssetsJobs | patch/states assert dirty confirmation, loading, recovery and 24 captures; folder OS interaction explicitly unautomated. | EVIDENCED |
| 3.3 | catalog; contract.ts; engine-contract.md; platform-spec; feature-audit | catalog browser sweep and story-props assertions pass, docs exist; contract omits required Store dependencies. | UNMET: R3 |
| 4.1 | tests/js/TriggerReactShellTest.mjs; CMakeLists; final evidence | Independent complete CTest passes 82.65s, spec checks pass; new P2 findings mean fixing review findings remains required. | UNMET: R1,R2,R3 |
| 4.2 | external review packet; envelope; tasks.md | Independent review now performed and retained; candidate has not earned final commit/spec sync/publication. | PENDING: FAIL |

## Criterion-to-test matrix

Test filenames without a prefix live under `web/trigger/tests`; browser files under its `browser` directory. The envelope maps every criterion to the actual complete CTest entrypoint; the following assertions, not that broad label alone, determine proof.

| Criterion | Executable evidence | Asserted behavior | Assessment |
|---|---|---|---|
| Four editor geometries | presentation.test.mjs; browser/frame.spec.mjs; browser/shared-controls.spec.mjs; geometry/geometry.json | Exact dimensions/rail/compact and expanded overview; narrow browser menu anchor and processor exposure. | EVIDENCED |
| Eight workspaces and all selections | model-bindings.test.mjs; browser/frame,instrument,modulation,slices,shared-controls | Eight tabs and controls; selected modulator/slice/module, macro and history own model data. | EVIDENCED |
| Empty, preset, reload and command undo | engine,store-boundary,preparation,adapter-operations; browser/patch,shell,editing; reviewer R1/R2/R3 | Ordinary load/undo/failure paths pass; queued parameter inverse, replacement history inverse and contract boundary fail. | UNMET: R1,R2,R3 |
| Sample, slices and analysis | slices-history,model-bindings,adapter-operations; browser/slices,editing,shared-controls; reviewer R2 | Markers/count/history and failure preservation asserted; composite source replacement undo incomplete. | UNMET: R2 |
| Zone and source editing | mapping.test.mjs; import.test.mjs; model-bindings; browser/editing,states; reviewer R2 | Neighbour/Shift/rectangular clipboard and all four import policies have assertions; imported source replacement inverse fails. | UNMET: R2 |
| Layers, voices and modulation | adapter-operations,model-bindings,control-regressions; browser/instrument,modulation,shared-controls | Selector values, voice mode, envelope units, nested source identity, macro bindings, fixed Amp route and undo. | EVIDENCED |
| Routing and effects | adapter-operations,model-bindings; browser/instrument,shared-controls | Known send value/output and undo; add/bypass actual processor; size exposure checks. | EVIDENCED |
| Gestures, menus and keyboard | store-boundary,control-regressions; browser/editing,modulation,runtime,shared-controls; reviewer R1 | Normal fine/typed/reset/gesture cancel and note release are covered; two queued distinct gestures restore wrong value. | UNMET: R1 |
| Action-reachable states and log drawer | telemetry,preparation; browser/runtime,states | 24 ordinary action captures plus assertions of loading, failures, dimensions, host and voice feedback; no design picker. | EVIDENCED within silent mock scope |
| Reviewable complete shell | component-guard,control-regressions; browser/catalog; documents/comparison artifacts; reviewer R3 | 97-entry/nine-state render sweep and prop callbacks documented; complete contract handoff not yet true. | UNMET: R3 |

## Seven review dimensions

### Task and specification completion: FAIL

Tasks are substantially implemented and scope remains standalone; R1–R3 prevent 1.2/2.1/2.7/3.3/4.1 completion. Keeping 4.2 and baseline sync pending was correct.

### Acceptance evidence: FAIL

The repository lane is real and green; the three retained probes fail asserted requirements. No RED-before-implementation claim is required because the explicit shell waiver overrides ordering.

### Test effectiveness: FAIL

Existing tests assert useful known values, identity/lifecycle invariants and undo, including five named fault calibrations. Fast Node/business tests and slower browser boundary tests are sensibly split. They miss composition across queue timing and operation scope, and contract consumption. Coverage percentages are correctly limited to Node scope and do not prove completeness.

### Architecture and operational safety: FAIL

Native DSP/JUCE and unrelated worktrees remain untouched. Props/callback components, separate telemetry, cloned patch preparation and operand undo are appropriate. R3 leaves an undeclared adapter dependency; R2 violates model consistency and R1 violates inverse command correctness. No secrets, remote writes, native audio or disk persistence are introduced by this shell.

### Better alternatives: ASSESSED

Preserve the current small-module presenter/adapter design. Capture inverse values on execution; ensure composite operation scopes include all owned changed collections; make consumer dependencies explicit in the handoff interface or a separate typed diagnostics boundary. These are smaller and safer than replacing command history with global snapshots.

### Reusable lessons: RECORDED

Two evidence-based lesson candidates below are proposed to the Trigger test/contract owner, with no standing guidance change. Existing precision/clipping/audition repairs were rechecked through their named tests.

### Publication soundness: FAIL

Manifest, index, binary patch, integration identity and four envelope content hashes independently match. Finite envelope is mechanically bounded and cleanup preserves state. Semantic defects prevent authorizing its status claims, spec synchronization and publication; refresh the entire packet after repairs.

## Reproduction and evidence limits

- Independently ran complete Trigger CTest: exit 0, 82.65 seconds. It discovers and executes type/component checks, 47 Node tests, production build and 23 browser tests. Raw result: `reviewer-ctest.log`.
- Independently ran strict owning OpenSpec validation and baseline spec coverage: exit 0; 559 scenarios / 320 mapped / 239 existing unrelated todo. Logs retained.
- Quiet shared-rules preflight returned exit 0 without advisories.
- Independently hashed all 347 candidate paths and staged blobs; patch, status, modes, refs and envelope payload bytes matched. Original ZIP digest and all 20 preserved reference files matched.
- Inspected Default Voice against source state 10 and Min Effects screenshots, geometry data, source comparison explanations, component/browser assertions. Typography-driven wrapping and live fixture differences are explicit. No unsupported pixel-exact claim is accepted.
- CUA Chrome was unavailable in this reviewer session. The independently executed Playwright browser lane passed; no fresh manual live-browser exploration is claimed.
- Reviewed Node coverage and bounded deliberate faults as diagnostics, not as proof of all JSX callbacks. Native audio, real decoding/persistence/JUCE, host automation bridge and OS folder chooser automation are not required to resolve these three findings.

## Envelope assessment

Envelope SHA-256: `0c36f534f7f358cee0d0cd47830c1efa39ac25938a053af0d0e8c64629bbc1e5`. Four exact transformations concern verification status, task 4.2, new baseline spec and ten spec-tests mappings. The new baseline body matches the accepted delta; existing mappings are preserved. Validation, ordinary hooks, unchanged-upstream fetch and normal work-branch push are bounded. Cleanup names no deletion. These operations are suitable only after a refreshed exact candidate earns PASS; current status transformations would be premature. No archive is proposed.

## Reusable lesson candidates

- Evidence: R1 passes separate queue and gesture suites but fails their composed state transition. R2 loses history through a composite operation scope filter. Owner: Trigger command/store test suite. Proposed change: Add focused cross-boundary inverse tests with pending operations and composite source replacement; assert exact owned operands plus unrelated-field preservation. Downside: Additional cases must stay at Node layer to avoid enlarging browser feedback time. Disposition: Candidate repair/test lesson; no standing-rule edit authorized.
- Evidence: R3: assigning concrete MockEngine to EngineAdapter compiles while the Store still needs undeclared applyDelta/log/setMockSwitch. Owner: Trigger adapter contract and type-check entrypoint. Proposed change: Check the consumer against its declared dependency and run edit/undo/redo through that contract surface. Downside: Typing the full boundary adds DTO maintenance; keep mock diagnostics isolated rather than promising native capabilities unnecessarily. Disposition: Candidate repair/test lesson; no global guidance change.

## Repair capsule

Preserve this candidate identity and review records. One writer repairs R1/R2/R3 and adds the owning regression/contract checks. Rerun relevant Node tests, complete Trigger gate and specification checks, update evidence honestly, then rebuild the full manifest and finite envelope and return the whole candidate to this reviewer. Recheck integration identity before refreezing. Do not apply current PASS-status envelope or mark baseline criteria proven yet.
