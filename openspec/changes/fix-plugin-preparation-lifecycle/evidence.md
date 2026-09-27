# Lifecycle verification

Base: `b122cae` on local branch `agent/fix-plugin-preparation-lifecycle`.
Complete raw logs and rendered artifacts are retained in `/tmp/dandrum-lifecycle-evidence/`.

## Preparation order

- Baseline: `cargo llvm-cov --manifest-path src/rust-engine/Cargo.toml --lib --tests --show-missing-lines --summary-only`, exit 0, 34 seconds. Existing preparation/construction lines were exercised. `synth.rs` had 81.71% line coverage; unrelated error formatting/fallback branches were uncovered. No existing test asserted load order.
- RED: `cargo test --manifest-path src/rust-engine/Cargo.toml --lib synth::tests::loaded_instrument_uses_host_rate_independently_of_preparation_order`, exit 101. After correcting test setup (ownership, valid mixer boundary, triggering a voice), the test passed at 44.1 kHz and failed the load-order comparison at 48 kHz (`order-red.log`).
- GREEN: `cargo test --manifest-path src/rust-engine/Cargo.toml --lib synth::tests`, exit 0, five tests. Both loading paths agree at all three rates; an independently calculated 220 Hz sample proves actual rate correctness.
- Coverage: focused library coverage (`-- synth::tests`) checks every added preparation line. Baseline and focused reports retain unrelated uncovered code explicitly.
- Production refactor review: keep preparation inside the existing runtime owner; no new module or duplicate parameter-value store. Runtime reconstruction reuses existing constructor semantics. Retained graph/assets cost memory but avoid filesystem dependence and per-Primitive reset logic.
- Test refactor review: one table covers rates and loading APIs; shared stereo renderer checks the public return count. The independent sine oracle prevents two equally incorrect engines passing by agreement.

## Repeated preparation

- `cargo test --manifest-path src/rust-engine/Cargo.toml --lib synth::tests`: six passing tests after adding repeated preparation checks. Retained 1.5 and 2.0 pitch ratios must audibly produce 330 and 440 Hz using an existing slot handle at 48, 96 and 44.1 kHz.
- The sampler test now asserts prepared assets remain playable, transient sample position and queued notes clear, event capacity grows to 1024 and shrinks to 2, and the existing drop-newest policy applies at that capacity. Repeated identical settings also reset the session.
- The initial fault injection was rejected by `deny(dead_code)` before tests ran; it is not counted as a kill. Revised fault injection leaves the call compiled but skips it for valid block sizes. This produced three assertion failures (order, retained audible values, sampler reset/capacity), with three other tests passing. The verified diff and output are `reprepare-fault.diff` and `reprepare-red.log`; the production source was restored in `finally`.
- No additional production change was needed for this criterion: rebuilding runtime state for the first criterion already implements it. Tests extend the owning sampler fixture and reuse the sine fixture; no new test-only production API was added.

## Verification setup findings

- Native test supplies six distinct non-default values through host parameters, then tests both that setup and saved-state restoration before preparation. The saved instrument has a modified 50 ms attack, so restoring the default graph by mistake is audibly detectable. Expected physical values are independently derived from authored ranges: 70 Hz, 537.5 ms, punch 0.375, click 0.75, sub decay 293.75 ms and sub level 0.625. At each host rate, a missing-file replacement is rejected; both stereo channels match a prepare-before-load FFI reference across eight blocks, remain audible, and retain all 64 host objects/IDs/order. The initial focused native run passed in 1.43 seconds; the final version is included in the full CTest gate below.
- Follow-up discovered while building the native oracle: untouched constructor defaults do not match an FFI render configured with the authored public defaults (click at note onset was 0 versus 0.996656). Explicit host writes make the paths agree. Investigate `setSlotNormalisedValue` calling `setValue` without notifying the APVTS raw-value listener; keep this separate from lifecycle correctness. Also review the existing host MIDI velocity conversion (`getVelocity() * 127.0f`) against JUCE's integer-velocity API. Neither path is modified here.
- `cargo-mutants` 27.1.0 default copying omits repository-level examples outside the Cargo crate, causing 39 baseline failures before any mutant ran. Use its supported in-place mode with exclusive source ownership and retain the failed-copy evidence separately.
- The strict coverage driver found three stale pre-existing exclusions in `coverage-allowlist.txt` (also absent from baseline uncovered lines). Remove those obsolete exclusions; no new exemptions are introduced. LLVM reports mismatched function data when combining library and integration-test binaries; the same warning persists after clean collection and is recorded as a measurement limitation.

## Fixture preservation

The README workbench render command was run before and after the production change against `examples/sound-design/tb303-acid-poc.yaml`. `cmp` passed for both outputs:

- WAV SHA-256: `1ac813cea4ba929141af90b713a37378ad253ffbe2c1e08e32d130abd567b077`
- Metrics CSV SHA-256: `338a1cbfe4ee8faf7a08fa6f863e82fe9777320b75f870a494bae68dd249f824`

## Refactoring and limits

The Rust change replaces the owned runtime with a freshly constructed runtime using a clone of its current compiled parameter table. That clone preserves values and numeric handles; assignment drops the old DSP resources outside rendering. No Primitive algorithm, graph routing, fixture timeline, FFI signature or C++ production code changes.

The native test owns its FFI reference through `std::unique_ptr` with the Rust destroy function as its deleter, so early test failures release it. Shared helpers remain in their existing files; no extracted module requires a new coverage boundary. Retaining graph and legacy sampler inputs trades extra memory for simple, deterministic preparation without filesystem access.

## Mutation analysis

Final command: `cargo mutants --manifest-path src/rust-engine/Cargo.toml --in-place --cap-lints true --in-diff /tmp/dandrum-lifecycle-evidence/production.diff --output /tmp/dandrum-lifecycle-evidence/mutation-calibrated -- --lib`.

Cargo-mutants 27.1.0 completed in 69 seconds, exit 0: **3 generated, 2 caught, 1 unviable, 0 survivors/timeouts**. Both preparation-body removal mutants were caught by the three named lifecycle tests with assertion failures. The constructor-to-`Default::default()` mutant is unviable because `RealtimeGraphProcessor` has no `Default` implementation; it is not counted as behavioral evidence. Full logs, diffs and structured outcomes are retained in `mutation-calibrated/mutants.out`.

The first in-place run omitted `--cap-lints true`, so `deny(dead_code)` rejected otherwise viable removals before tests ran. Its three unviable results are not counted as kills. The final run explicitly caps lint severity and uses a passing 749-test library baseline. Independent hand injection also reproduced the same three lifecycle assertion failures. No incremental result reuse is claimed; analysis is scoped to changed Rust production lines.

## Reflection candidate

For a Rust crate nested inside a larger fixture repository, verify that cargo-mutants' baseline includes those fixtures before interpreting results. This run required exclusive in-place execution and explicit lint capping. A project-specific mutation workflow note could prevent future empty or unviable runs being mistaken for passing evidence; no standing instructions or skills were changed under this task.

## Completion gates

Commands ran from `/home/dan/code/dandrum`. Native commands used `PATH=/usr/bin:/bin:/home/dan/.local/bin:/home/dan/.cargo/bin` to avoid the unrelated Linuxbrew linker conflict previously recorded in the integration review.

| Gate | Result | Retained evidence |
| --- | --- | --- |
| `cmake --build build -j2` | Exit 0, 108.45 seconds; VST3 and all native test targets built | `full-build.log`, `full-build.time` |
| `ctest --test-dir build --output-on-failure` | Exit 0, 10/10 passed, 22.25 seconds; includes full Rust tests, FFI, sample-accurate MIDI, realtime callback checks, plugin construction/state, fixed host surface, reload and Sound Lab | `ctest.log`, `ctest.time`, `build/Testing/Temporary/LastTest.log` |
| `scripts/check-rust-coverage` | Exit 0, 28.68 seconds; strict files fully covered without stale exemptions | `strict-coverage.log`, `strict-coverage.time` |
| Full Rust coverage (`cargo llvm-cov --lib --tests --show-missing-lines --summary-only`) | Exit 0; 749 library tests and all integration tests passed; all added production preparation lines covered | `final-coverage.log` |
| Focused mutation analysis | Exit 0; 2 caught, 1 unviable, no survivors | `mutation-calibrated.log`, structured report under `mutation-calibrated/` |
| Existing TB-303 fixture render | WAV and CSV byte-identical before/after | `fixture-baseline.log`, `fixture-final.log`, `baseline.*`, `final.*` |
| `openspec validate fix-plugin-preparation-lifecycle --strict` | Passed before completion review | CLI output; rerun in finalization |

The build emits the existing Cargo/Make jobserver descriptor warning but completes successfully. Coverage retains the LLVM combined-binary warning documented above; focused coverage and assertion-fault evidence corroborate the changed boundary.

The local coverage-policy cleanup is commit `60e4948`. Main remains at `b122cae`; fetched `origin/main` remains `8cc0823`, already an ancestor. Independent review and the exact finalization manifest are retained under `/tmp/dandrum-lifecycle-evidence/`. Finalization is limited to the approved spec/map/task files, validation and local commit; no merge or push.
