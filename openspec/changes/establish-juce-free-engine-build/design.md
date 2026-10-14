## Context

See proposal.md for motivation. The existing root build registers genuinely independent Rust/C++ tests before entering JUCE, but performs React discovery first. The C ABI declaration header contains standard-library types only despite living under src/juce-wrapper. Existing RustEngineFfiSmoke kernelBusSmoke exercises a real gain graph with literal signed stereo inputs -0.5 and +0.25 and a reset; reuse it as behavioral evidence.

## Goals / Non-Goals

**Goals:** expose the independent engine/native test build, prove it with missing host dependencies, retain the comparison configuration and reject ambiguous product selections.

**Non-Goals:** full J1 completion, #23's session/SDK implementation, format adapters, platform-support certification, browser resources or production cutover. These remain required under #24 and must not be relabeled complete by this slice.

## Decisions

- Add DANDRUM_ENGINE_ONLY, default OFF. Check conflicting UI/product flags before their discovery; skip browser setup and return after registering independently buildable tests and before entering JUCE. Keeping registration shared avoids a duplicate engine build recipe and preserves default behavior.
- Reuse the existing engine and C ABI rather than add an unused host facade or parallel session. A canonical declaration-path move can follow when adapters consume it; folder naming does not imply a linked dependency.
- Test the actual configure/build/runtime boundary using a minimal first-party source fixture containing no third_party/JUCE and no web assets. Use task-local build and Cargo targets. Verify CTest inventory and run signed FFI and sample-offset smoke tests. Avoid moving/deleting the original vendored checkout.
- Characterize retained JUCE configuration and explicitly rejected incompatible options through build regression tests. Keep these checks separate from native DAW, UI and physical-device acceptance.

## Risks / Trade-offs

- [A core build could be mistaken for a replacement product] → Document its limited target set and keep #24 J1-J6 and Gates A-C open until their stronger evidence exists.
- [Independent registration acquires a JUCE include/link later] → The missing-dependency fixture builds the registered native targets and inspects the generated test/target inventory.
- [Fixture build cost] → Reuse a task-local Cargo target for regression runs where safe; do not share active build trees across worktrees or delete reusable caches.
- [Cross-platform Rust native library names/system libraries] → Initially verify the existing supported WSL/Linux toolchain; record untested platforms without changing #24's required production format/device scope.

## Migration Plan

Test first, add the explicit configuration, verify both modes and conflicting flags, document commands, sync the proving scenarios and map tests, then commit and push the focused slice. Continue replacement proofs in follow-up OpenSpec slices on this task branch. JUCE comparison remains available by leaving DANDRUM_ENGINE_ONLY disabled.
