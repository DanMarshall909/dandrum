# R1 frozen test repair handoff

Candidate `/tmp/dandrum-lanes/runtime`, HEAD d1662501a375710d9892371453c2614ad96518ed. Independent finding R1 repaired within the two authorized test paths. Production runtime remains byte-identical at3db6c16c237c0bbb024e57b34b72507dcf70111ead88c3c6b88f66df08d81c0b. No Git/index/docs/spec/dependency/engine edits. Source ownership and new target/scratch execution ownership returned to root; no owned workers remain. This does not approve publication or complete task3.3: root must refresh source-free optimized evidence and the whole candidate packet for the same reviewer.

## Observable repair

Shared fixture `with_voice_velocity(mode, voices, pan, velocity_level)` optionally connects actual `voice.velocity` to `sample_player.level`. Existing constructors select false and preserve every constant-amplitude lifecycle oracle. The real poly kernel publishes velocity/127; trigger metadata alone does not affect this sample player's amplitude, so the explicit control cable is necessary.

`velocity_sensitive_child_calibrates_signed_pcm_and_release_from_host_events` independently calibrates host velocities32 and96, positive/negative nonconstant prefix, sustained tail, and sample-offset7 gate release. Expected fractions are algebraically reduced directly from the decoded WAV, velocity/127 and pan-0.5; no expected sample comes from a render. Tolerance1e-7 covers f32 rounding and is far smaller than one velocity step.

`editor_onset_keeps_latest_coalesced_velocity_even_after_an_unheard_release` exercises32→96 and96→32 before audio observes intent, both held and already-Off. It pins the latest requested velocity's signed prefix/tail, observes the held tail before later Off, and pins three subsequent silent callbacks for a released unheard tap. Thus fixed, first, desired-only, or zeroed retained velocity cannot satisfy these audible oracles. It reuses allocation/deallocation-sentinel render helpers. No fault injection/mutant, mock, test-only production seam or production change was introduced.

This is honest passing characterization of existing correct behavior. Initial calibrated fixture passed before the editor assertion draft; the resource hold prevented running that draft until the new target assignment. No fabricated assertion RED is claimed. Earlier calibration receipt remains separately at `/tmp/dandrum-lanes/evidence-runtime-notes/r1-01-velocity-calibration`; paused draft provenance remains at r1-02-editor-draft. The current full editor and full host runs also execute the calibrated fixture at the exact final source.

## Actual verification profile and drivers

All six new gates use actual Cargo test/fmt/clippy/llvm-cov drivers, jobs1, with CARGO_TARGET_DIR, CARGO_LLVM_COV_TARGET_DIR and CARGO_LLVM_COV_BUILD_DIR all equal `/run/user/1000/dandrum-beta-20261020/runtime-notes-r1-target`; TMPDIR is the new sibling runtime-notes-r1-scratch. CARGO_PROFILE_DEV_DEBUG=0 and CARGO_PROFILE_TEST_DEBUG=0 are explicit. This is an unoptimized Cargo dev/test profile WITHOUT debuginfo, not the default-symbol debug profile. Optimization, debug assertions, overflow and panic configuration were otherwise unchanged. Verbose normal compiler commands show `-C strip=debuginfo`, normal test harness and incremental compilation, with no optimization override. Actual commands are retained in actual-profile-commands.json and complete -vv logs; tool versions are retained.

The focused LLVM driver's own verbose command shows its real RUSTC_WRAPPER and forwarded `-C instrument-coverage --cfg=coverage` for the host crate/test owners, plus the raw LLVM_PROFILE_FILE and actual llvm-profdata/llvm-cov export commands. It reuses ordinary dependency compilation; its4.52s is not a claim of cold instrumented engine compilation. Standard cargo-llvm-cov invalidated its own package artifacts inside the NEW target as logged; no manual clean/removal/relocation or writes to any original cache occurred.

| Gate | Result | Elapsed |
| --- | --- | --- |
| Focused R1 editor characterization, cold Cargo build |1 passed|69.85s|
| Full editor-note integration binary |17 passed|0.50s|
| Full host Cargo test selection |92 passed|6.53s|
| Cargo fmt --check |PASS|0.50s|
| Cargo Clippy --all-targets -- -D warnings |PASS|29.15s|
| Focused Cargo LLVM --lib --test runtime --test editor_notes |49 passed|4.52s|

Each phase has exact crate-relative source copies, hashes, command/cwd/env, exit/timing/resource receipt and raw output. Every gate binds the same final source hashes. Mutation remains excluded. Existing dependency crossterm warning is visible in logs; strict host Clippy passes. Tests do not invoke Node/Wry/native UI. No profile or selection difference is hidden or described as a default-symbol/optimized/source-free gate.

## Coverage and refactor assessment

R1 closes an executed-but-unasserted behavior, so unchanged production coverage is expected. Focused runtime whole-file figures include unit tests: functions90/93, emitted instantiations145/162, lines1121/1154, regions1817/1855; no branch/MC/DC instrumentation. Production-only aggregation by exact coordinates: emitted functions121/138, distinct functions66/69, distinct regions942/963. Same21 disclosed production region gaps remain; exact coordinates and derivation are in coverage-analysis.json/finalize.py. A passing full uninstrumented host suite does not close focused LLVM gaps. Neither private defensive queue-prefill laws nor these new velocity cases claim100%public failure coverage.

The opt-in fixture reuses the existing WAV, pan, gate routing and real preparation. Both shared fixture owners compile under all-target checks; no dead-code suppression or production extraction was needed. Small expected-PCM helpers keep the calibration and editor assertions tied to independently fixed fractions without duplicating long setup. Further refactor would obscure this narrow repair; no additional source change is warranted.

## Resources and freeze

No guarded phase crossed its floor. Minimum over recorded runs: target1794453504 bytes, /tmp1187536896 bytes, /dev/shm1057980416 bytes, MemAvailable12947898368 bytes. Final audit recorded /run/user free2117758976, /tmp1113546752, /dev/shm1057980416. No owned process group remains. Return `/run/user/1000/dandrum-beta-20261020/runtime-notes-r1-target` and sibling scratch execution ownership; preserve all new and existing cache/evidence content. Original cache-runtime and disk cache-host-snapshot-cov were not used during this resumed verification. Root was warned final /tmp headroom was about38MiB above its1GiB floor.

Original early phase provenance defects remain disclosed in the earlier handoff/review; R1 evidence uses unique crate-relative source capture throughout. Independent reviewer acceptance and refreshed affected source-free evidence are outstanding root gates, not author claims.

## Final source SHA256

- `src/trigger-host/src/runtime.rs`: `3db6c16c237c0bbb024e57b34b72507dcf70111ead88c3c6b88f66df08d81c0b`
- `src/trigger-host/tests/editor_notes.rs`: `669c8d141661eb649eb2a5d83b1d71783cca6e09d15ba8e85093cd0b938f270a`
- `src/trigger-host/tests/support/editor_notes.rs`: `8eff39ddbb42447a45f58700da84e65f87fc30d0c58ad7dfa8c13939e098ceae`
- `src/trigger-host/tests/runtime.rs`: `49e3abd589ed590d5c223b41296c9951722ec20aa368858c3701ca40e3ccf461`
