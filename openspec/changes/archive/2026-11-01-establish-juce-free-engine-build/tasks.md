## 1. Characterize and specify the build boundary

- [x] 1.1 Inspect existing independent engine/native tests and run the signed-output ABI baseline; retain raw results and identify any coverage gap before production edits.
- [x] 1.2 Add a missing-JUCE/missing-web integration test and explicit incompatible-option tests; run them against the unchanged root configuration and record expected failures.

## 2. Establish the independent configuration

- [x] 2.1 Add DANDRUM_ENGINE_ONLY with explicit incompatible-option diagnostics, bypass browser discovery and keep independent target registration before JUCE; verify the regression fixture configures, builds native targets and passes signed ABI and sample-offset smoke tests without JUCE assets.
- [x] 2.2 Verify existing default/disabled engine-only JUCE comparison behavior with a real native-only configuration and relevant retained test inventory; record environment/build limits explicitly.
- [x] 2.3 Document engine-only configure/build/CTest commands, target scope and remaining #24 gates; verify commands and review the focused diff.

## 3. Acceptance and publication

- [x] 3.1 Sync only the reviewed added scenarios into canonical specs and map genuine proving tests in spec-tests.map; verify scripts/check-spec-coverage and strict OpenSpec validation without new todo entries.
- [x] 3.2 Run relevant complete engine-only CTest/build checks, inspect the final diff and commit/push the focused verified slice with Why/What/Verification/Constraints; record remaining full-migration requirements and clean/preserved worktree state in the shared ledger.
