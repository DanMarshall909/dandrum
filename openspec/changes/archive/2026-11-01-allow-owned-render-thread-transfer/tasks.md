## 1. Public ownership-transfer proof

- [x] 1.1 Characterize signed gain/filter output and reset behavior through the current public API on the control thread; verify the literal baseline before production changes.
- [x] 1.2 Add external Send/ownership-transfer tests with a real render thread, returned ownership and independent instances; run and confirm the expected compiler E0277 failure against the unchanged internal traits.

## 2. Compiler-enforced owner constraints

- [x] 2.1 Require Send on erased prepared executors and filter algorithms only as proven necessary by the compiler; verify the focused external integration test passes without unsafe impl or locking wrappers and inspect the two-line production diff.
- [x] 2.2 Run complete engine-only configure/build/CTest with the existing task cache and verify allocation, filter/DSP, C ABI and service regressions; record actual counts, timing and C: free space.

## 3. Acceptance and publication

- [x] 3.1 Sync the two additive headless-engine scenarios and map genuine ownership-transfer tests; verify strict OpenSpec and spec coverage with no additional todo entries.
- [x] 3.2 Review the complete focused diff and publish the verified prerequisite with detailed commit body; record exact branch preservation and remaining full #24 requirements in the shared ledger and progress receipt.
