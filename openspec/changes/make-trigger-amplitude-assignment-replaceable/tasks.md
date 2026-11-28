## 1. Confirm prerequisite contracts and freeze the default

- [ ] 1.1 Record the minimum accepted #6 stable identity/poly ownership contract for voice amplitude and layer gain, and #8 gain domain, velocity response, combination, source-scope and update-timing contract in this change before native implementation; hold dependent work if either remains unresolved.
- [ ] 1.2 Inventory current mock and native amplitude controls, Velocity routes including Felt Kit's route, ID-based special cases, adapter refusals and test coverage; capture historic mock defaults with and without an explicit Velocity route and independently calculated signed native references as migration/equivalence fixtures.

## 2. Specify and implement the structural assignment

- [ ] 2.1 Write failing model/migration tests for default envelope and per-layer Velocity routes, renamed, replaced, detached and restored envelope assignments, invalid source/destination rejection, preserved unrelated routes and undo/redo.
- [ ] 2.2 Add stable assignment identity and template provenance to the patch projection; create the Velocity-to-layer-gain route with each new layer, migrate historic locked defaults and explicit Velocity routes without redirecting or duplicating them on pre-existing layers, preserve stored/visible values and compare the new native default against signed references.
- [ ] 2.3 Implement one atomic Replace, Detach or Restore command each for the envelope assignment, with typed/scope/multiplicity validation and retained base gain and layer Velocity routes when detached; pass the focused model tests.

## 3. Prepare native modulation and independent lifecycle

- [ ] 3.1 Qualify the one- and multi-voice sample-map expansion and a source-completion event routed to `poly.done`, including no-zone selection; write failing Rust prepared-graph tests for known signed output at two velocities on the same layer, disabled layer Velocity route with no hidden gain scaling, alternative envelope feeding voice amplitude and a second destination, independent per-voice state, shared-source scope and invalid-cable rejection.
- [ ] 3.2 Compose the supported per-layer Velocity-to-gain and voice-amplitude envelope paths from existing primitives and typed cables under #8, with no sample-player velocity gain special case; expose only capabilities that compile and render in the native adapter.
- [ ] 3.3 Write failing small-voice-limit tests for one-shot completion, gated/looped note-off and post-note-off silence, temporary zero amplitude, unmatched selection, choke, steal, reset and slot reuse under default/replaced/detached assignments.
- [ ] 3.4 Supply an explicit Trigger voice completion signal independent of the amplitude route and pass signed output and bounded-lifecycle tests; reject unsupported completion cases during preparation.

## 4. Connect the editor and accepted session

- [ ] 4.1 Write failing adapter and UI interaction tests for advanced Replace, Detach and Restore, visible independent layer Velocity routes, truthful unsupported states and release-tail control timing, assignment labels, familiar default controls, rename independence and keyboard-accessible disclosure/confirmation before structural commits.
- [ ] 4.2 Implement the native projection/compiler and advanced UI commands, preserving the current supported editor flow and rejecting unsupported operations without simulated publication.
- [ ] 4.3 Write failing native session tests for failed preparation with held notes, successful held-note commit, late note-off, undo/redo, saved-state migration and fresh-session restore; then implement the accepted transaction and persistence path.

## 5. Verify and deliver the accepted change

- [ ] 5.1 Map every added or modified scenario to a proving test in `spec-tests.map`, remove its todo entry, run focused and affected suites, signed native host rendering, coverage, strict OpenSpec/spec checks and a covered refactoring review; record any bounded gap.
- [ ] 5.2 Run focused owner mutation after the verified snapshot when allowed by current policy and storage admission, retain compact results, obtain independent completion review of the exact candidate, then publish and update issue #9 and the canonical handoff under repository gates.
