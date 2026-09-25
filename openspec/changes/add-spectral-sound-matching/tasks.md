## 1. Characterize Contracts

- [x] 1.1 Run the existing Rust and native Sound Lab tests and inspect coverage at the workbench, FFI, controller, and web-bridge seams before changing behavior.
- [x] 1.2 Add failing fixture/patch tests for the TB-303 public parameter surface and valid/invalid bounded matching declarations.

## 2. Spectral Objective

- [x] 2.1 Add failing tests for identical-audio loss, spectral/RMS/centroid discrimination, fixed whole-region gain alignment, reference validation, and finite score breakdowns.
- [x] 2.2 Implement multi-resolution log-spectral features and the reusable weighted objective with full changed-module coverage.
- [x] 2.3 Add failing tests for deterministic bounded search, self-reference improvement, monotonic progress, cancellation, and result provenance.
- [x] 2.4 Implement the seeded evolution search, patch-value application, coherent match artifact, content fingerprints, and CLI `match` command.

## 3. Provider-Neutral Graph Proposals

- [x] 3.1 Add failing tests for the provider-neutral request/response contract, sanitized context, local validation, and non-application of invalid proposals.
- [x] 3.2 Implement canonical graph-proposal types, module/topology summarization, strict response parsing, and local patch/preparation validation.
- [x] 3.3 Add failing fake-runner tests for Codex CLI arguments, stdin/schema/output handling, API-key removal, missing-login/process/timeout/cancellation failures, and bounded diagnostics.
- [x] 3.4 Implement the Codex CLI adapter using the existing ChatGPT login in an isolated temporary read-only execution directory.

## 4. Offline Native Boundary

- [x] 4.1 Add failing Rust FFI tests for a coherent match handle, progress cancellation, typed best values, manifest data, reference/candidate audio, and proposal results.
- [x] 4.2 Implement the match/proposal opaque handles and update the shared C++ bindings.
- [x] 4.3 Add failing C++ controller tests for matching progress, conflict rejection, cancellation, completed comparison artifacts, and retryable errors.
- [x] 4.4 Extend `SoundLabController` background ownership and state snapshots for match and proposal jobs.

## 5. Sound Lab Workflow

- [x] 5.1 Extend the web-editor contract test with failing assertions for reference selection, match/cancel controls, progress and score display, A/B audio, comparison plotting, acceptance, and proposal status.
- [x] 5.2 Add JUCE reference selection, match/proposal native functions and events, generation-addressed audio resources, and best-value acceptance through the public parameter surface.
- [x] 5.3 Implement the Sound Lab matching/comparison/proposal UI while preserving the existing render/analyse workflow.
- [x] 5.4 Extend the realtime source guard so matching, reference IO/hashing, optimization, and provider/process work cannot enter the audio path.

## 6. Verification And Delivery

- [ ] 6.1 Run focused RED/GREEN checks, rustfmt, the complete Rust suite, changed-code coverage, and focused mutation testing; strengthen any surviving behavior-relevant mutants.
- [ ] 6.2 Configure and build the native targets, run CTest, and manually exercise reference selection, matching/cancellation, A/B audition, acceptance, and provider-unavailable behavior in the standalone plugin host.
- [ ] 6.3 Strictly validate the OpenSpec change, sync accepted delta specs, map every scenario to a proving test, review fingerprints, and run `scripts/check-spec-coverage`.
- [ ] 6.4 Inspect status/diff/log, perform an independent completion review against the acceptance criteria, address findings, and commit the verified change in focused boundaries.
