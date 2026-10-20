# Final frozen-source verification addendum

This closes the explicitly pending final-source verification in `/tmp/dandrum-lanes/evidence-host-snapshot/HANDOFF.md` (SHA-256 `757f264b1c13a8d663a68fd248203dea5b64d6670a023ada151fa06ddcf4f44d`). That original handoff, source files, evidence and manifest were not edited. This new phase measures the final error-mapping refactor and literal 23-alias assertion; it does not retroactively rebind older runs.

Root authorized ordinary warm checks in the original, non-symlink RAM cache. Cache deletion/relocation approval remains pending and neither was performed. Both LLVM target and build directories and ordinary Cargo target were explicitly `/dev/shm/dandrum-beta-20261020/host-snapshot-cov`; jobs2. Cache-runtime and root LLVM cache were not used. Resource monitor retained every sample and owned process group, would stop only that group at the declared floors, and never triggered. Minimum observed: disk1,208,512,512B, global RAM filesystem1,943,666,688B, MemAvailable14,408,081,408B. Floors were256MiB/1GiB/4GiB respectively. All four owned process groups have no remaining members. Cache ownership was released to root immediately after checks.

Final gates pass sequentially:

| Gate | Result | Wall seconds |
|---|---|---:|
| All normal host tests and LLVM export | 67 pass, including all five snapshot tests | 6.281933 |
| All-targets Clippy with warnings denied | PASS | 1.257592 |
| Cargo format check | PASS | 0.252000 |
| Fresh actual Rust reply → unchanged production JS parsers | PASS | 0.754313 |

`final-llvm-run.json/.log`, `final-clippy.json/.log`, `final-format.json/.log`, `final-parser.json/.log` retain exact commands/environment/timings/output. `parser/` is a new copied phase of the previous one-off calibration; no original calibration outputs were overwritten. The fresh instrumented host rlib is linked into the one-off executable; actual accepted and applied replies go through unchanged `patchFromSession` and `parseSessionConnection`. The literal signed stereo -1, precise u64 identity, 23 controls, real sample geometry, six regions, five displayed rules and both two-candidate selectors pass. `calibration-artifacts.json` hashes the actual linked rlib, executable and installed YAML package files. This remains a one-off real parser boundary check, not a maintained Wry/CLAP integration test.

Final authoritative LLVM file summaries: `parameters.rs`49/49functions,414/415lines,702/714regions; `protocol.rs`24/24functions,357/357lines,497/513regions. The sole zero-hit annotated physical parameter line is inherited state-restore handling181. No new snapshot/protocol physical executable line has an annotated zero hit in this run. Region gaps and error-path invariants remain disclosed in the original handoff; no exclusion, test-only setter or extracted production module was introduced. This is not a100% branch or whole-host coverage claim (LLVM branch count0).

`coverage-qualification.json` distinguishes emitted records from source behavior: parameters74/120emitted function records execute, but49/49distinct primary source spans execute; protocol31/54emitted records versus24/24distinct spans. Duplicate unexecuted monomorphized/compiled instances are retained in the raw report rather than described as distinct untested source functions. The file summaries above remain authoritative. A separate diagnostic union of unique kind0 source-region spans is703/714for parameters and497/513for protocol; the union counts a source span hit by any emitted instance and is not substituted for the LLVM file metric or branch evidence.

`source-verification.json` confirms every frozen supporting source/fixture/parser/spec input hash and mode still matches the original source manifest, protocol lane HEAD remains `0f02f575ff2419082370f4d35986d086c03b5d0b`, and Git diff checking passes. Exactly the original three candidate paths remain changed/untracked. Their final SHA-256 values are unchanged:

- parameters.rs `cfcfed9254885573cd4bf7253865df9d5ab9e5f88c3068492f39d0dc5aabb4b2`
- protocol.rs `1e12b7c163de522478ffe081cff47385209edc7427d8cf8366bc4271c1483a21`
- tests/snapshot.rs `d93cbf1f16216c1413d25027f56d731832b3aad62a20e13603ede69f3179f9be`

The bounded tracer's final-source verification gap is closed. Root still owns combined source-free/frontend/native integration, independent review, publication and the remaining task3.3 facade behavior. No OpenSpec task checkbox or overall-beta completion is claimed. No source, Git/index, specification, manifest/lock, standing guidance or cache cleanup change occurred in this follow-up.
