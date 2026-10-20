# Task 3.2 asset component handoff

Author source is frozen at base dbabae61f19543ee97d91cadcebe56b4c0bf043b plus the ten files in frozen-source-manifest.json. No commit/push or task checkbox change. Task 3.2 remains incomplete; this is a material headless packaged-resource component for independent review before native Wry ownership work.

## Behavior and owner

New dependency-free dandrum-trigger-editor crate owns EditorAssets::open(resource_root) and serve(exact_uri). It preloads all 14 browser assets (index, JS, CSS, eight TTFs, three OFL notices) from explicit resources/editor before exposing an owner. Serving borrows immutable owned bytes, performs no request-time filesystem lookup, and accepts only declared dandrum://localhost URLs. Missing/empty/nonregular/unreadable files and links are rejected; alternate origins, queries, fragments, encoded names, traversal and unknown names have no fallback. The fifteenth production file, fonts/.gitattributes, is not served. No host/core/session dependency and no unsafe code. No Wry/native dependency is adopted in this boundary.

A Rust response borrows its byte slice from the asset owner; the compiler prevents retaining that response after its owner is dropped. This keeps the resource lifetime explicit without copied response buffers or reference-counted global caches.

## TDD chronology and refactor

1. Rebuilt actual native production bundle using existing frontend node_modules after checking every tracked web/trigger file matches runtime source; actual build receipt binds all sources and output bytes. Output is 15 files, 1,566,980 bytes. Vite monolithic chunk warning retained. No install, asset pipeline change, or source edit in frontend.
2. Actual production calibration test was written first and observed RED against todo preload stub (exit101, 01-assets-red.log). Four policy tests were added and likewise failed against that stub (exit101, 01-assets-policy-red.log). Historical exact command shapes are below; historical wall timing and frozen RED file capture are unavailable. The reconstructed stub is explicitly labelled retrospective in 01-red-source-provenance.json; it is not represented as a historical captured snapshot.
3. Minimal fixed-table preload/lookup implementation made four fast tests and the explicitly selected actual production calibration pass. Owning module coverage then passed. Existing immutable preloads are independently asserted after deleting a copied input while the complete original input still exists; reopen fails rather than searching the original.
4. Before CMake edits, the configured test inventory lacked both proposed editor tests; assertion wrapper exit1 is retained. Baseline CMake/Python source is recoverable from immutable base commit; receipt original plus explicitly corrected configure/inventory argv are retained. This is genuine missing-registration RED, not an unrelated build failure.
5. Added std editor release library to aggregate scaffold, fast registered CTest, and separately registered actual production calibration CTest. Both executed successfully. The affected Python source-free guard asserts the new library/test inventory and runs only the fast editor test against its intentionally tiny build-only editor fixture; this distinction is explicit in CMake. Its existing staging assertions are characterization, not a newly invented RED.
6. Registered production CTest was calibrated by temporarily hiding app.js only in task-owned staged output: it failed exit8 with named missing-file diagnostic, then staged file was restored in finally. Original production input stayed intact. This is a resource behavior fixture calibration, not a code mutation run.
7. Refactor review retained one narrow fixed asset table/cache and shared test-only Package fixture. Independent test file/MIME expectations deliberately remain separate from production constants. No generic URL/path/security abstraction was added. Added context to production-test file errors. Final Clippy initially rejected test permission literal spelling 0; changed only to octal 0o000 and reran cleanly. That lint failure is not a behavior RED.

Historical RED commands (cwd /tmp/dandrum-lanes/runtime; elapsed unavailable):
- env CARGO_TARGET_DIR=/dev/shm/dandrum-beta-20261020/editor-target CARGO_BUILD_JOBS=2 DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR=/tmp/dandrum-lanes/frontend/web/trigger/dist-native /home/dan/.cargo/bin/cargo test --manifest-path src/trigger-editor/Cargo.toml --test packaged_assets -- --ignored ->101
- same target/jobs, /home/dan/.cargo/bin/cargo test --manifest-path src/trigger-editor/Cargo.toml --lib ->101
Registration RED exact commands, cwd, timing correction and immutable baseline are in 02-registration-red-receipt*.json and 01-red-source-provenance.json.

## Verification and source binding

Final focused commands, exact argv/cwd/environment/exit/elapsed are in final-run-receipts.json. Latest results: fmt PASS; all-target Clippy -D warnings PASS; ordinary Cargo test PASS (4 unit tests, 1 explicitly ignored production calibration, 0 doc tests); explicit registered CTest PASS 2/2, actually executing 4 fast tests plus 1 production calibration against staged full production input. The ignored default test is not used as production evidence. Build/configuration exact receipts and verbose registered test output are retained.

Final LLVM owning-module scope is --lib, cargo-llvm-cov0.8.7. Raw LCOV reports LF/LH52/52 and FNF/FNH5/5; emitted distinct DA records49/49 are all hit. The earlier 49 figure came from counting DA records, not a different intermediate run: initial and final LCOV hashes are identical. Five distinct source function/closure starts are covered. Raw mangled instantiations7/11 expose four uncovered duplicate generic closure copies. No uncovered source lines or source function starts, no branch coverage claim. Reproduce accounting with python3 summarize-coverage.py; coverage-summary.json binds source and raw LCOV SHA256. Wrapper lib.rs has no executable behavior beyond exports.

Source-free guard outcome and final receipt are appended below after completion. First attempt was deliberately interrupted by its own resource monitor at physical-space threshold (exit -2,74.470s,minfree1,069,330,432bytes), after successful aggregate release build while preparing runtime tests. This is a resource/setup interruption, not a failing behavior test and not RED. Shared grown cache was preserved. Separate hash-matched RAM candidate uses the unchanged Python guard and a private target seeded by COPY of warm release subtree; original caches remain untouched. Input hashes, copy provenance, exact argv/cwd/environment/exit/elapsed and resource samples are retained as 10/11 records.

Final candidate copies are under final-source/, tracked existing-file patch under tracked-candidate.patch, ten-file hashes under frozen-source-manifest.json, and git status under git-status.txt. git diff --check passed. All asset source hashes matched pre-guard manifest before freezing; no production changes after final focused checks. New files are untracked and must be added explicitly during root-owned focused commit.

## Claims and remaining work

This proves actual production asset bytes, MIME declarations, local CSS font references, TTF headers, OFL notices, immutable serving, failure policy, packaging and registered headless pipeline. It does not prove browser font rendering, network enforcement, a Wry/X11 parented child, main-thread lifecycle/failure cleanup, connected real-session UI, native callback servicing, CLAP GUI automation, or continuous live audio. Those remain task3.2/native or later3.3/4.x boundaries. No fake native backend, undocumented timer fallback, or full task completion claim.

Root owns independent review, portable evidence curation, normal-hook commit/publication, and eventual task completion. Author must not start native/Wry implementation until the component is reviewed and committed. No mutation tests were run, per explicit exclusion. Original build/Cargo/node caches were preserved; only private RAM copies/targets were created.

## Final source-free result

Unchanged affected Python guard PASS exit0 in138.699s from exact hash-matched RAM candidate. It built dandrum-trigger-clap-editor aggregate and passed selected runtime CTests3/3 (clap-dynamic-host, trigger-host-tests, trigger-editor-tests). Raw stage logs and full CTest LastTest.log are in11-source-free-stage-logs/. Resource monitor did not stop it; minima in11-resource-minima.json preserve exact accounting. Final ten source hashes match the frozen manifest after guard completion. No author build/test process remains active. All requested author checks are complete; frozen candidate ready for root independent review.
