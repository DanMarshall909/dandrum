# Remaining Trigger beta baseline

Task 1.1 at published planning baseline `eeb91149`; the completed CLAP foundation is `2c7bcaa5`. The new change has 25 implementation tasks. This records current facts and qualification gaps, not new product readiness.

| Area | Observed baseline |
| --- | --- |
| Native platform | Linux x86_64, Pop!_OS 22.04; explicit X11 `:10.0` query succeeds; WebKitGTK 2.50.4, GTK 3.24.33, ALSA 1.2.8 |
| Rust tools | rustc/Cargo 1.96.0; cargo-llvm-cov 0.8.7 |
| C++/CMake | G++ 11.4.0, CMake 3.30.9; no qualifying newer installed compiler identified by bounded readiness inventory |
| Native availability | Actual ALSA PCM/control/sequencer nodes and X11 tools exist; Qtractor is not on PATH. Availability is not opened-device, native-input or named-DAW acceptance. |
| Current protocol/state | Version 1 request limit 16 KiB, output limit 64 KiB, 16 native attachments, 128 replay entries per attachment; version 2 state limit 65,536 bytes, packaged-kit resources only |
| Runtime handoff | Capacity-one activation/reclamation; maximum 16,384 frames and 1,024 bounded events; public parameter state/independent endpoints remain as verified CLAP owners |
| Storage | All three active Cargo/LLVM targets resolve to physical disk; no build root is under `/dev/shm` |

The four measured reusable target sizes are 47,426,981,888 bytes (runtime), 13,990,686,720 bytes (host/LLVM), 11,941,531,648 bytes (editor guard) and 48,052,994,048 bytes (feature engine-only target). They are retained, not copied or deleted. Exact commands/output and byte/source bindings are in the raw packet. Current disk/RAM samples and a three-sample vmstat capture are retained; occupied swap alone is not an active-pressure claim.

The latest normal planning commit hook passed in 72.364s with minimum 20,489,183,232 bytes MemAvailable and 29,088,632,832 bytes disk free. Its normal push passed in 13.066s with minimum 21,573,693,440 bytes MemAvailable and 28,966,916,096 bytes disk free. These are observed global headroom samples, not a measured owned-process peak. The tracked older measured 4 GiB worktree reservation remains; the task uses the stricter 8 GiB MemAvailable floor and one heavy job. New large producer runs must also retain sufficient disk headroom for their measured outputs and clean exact owned scratch on every terminal path.

The wrapper candidate remains v0.16.0/commit 1cca996e and MIT SDK 3.8.0_build_66/commit 9fad9770. Their exact cached notices and pinned source/readiness hashes are retained. This is not a complete fetched SDK/submodule graph or transitive licence audit. The SDK lists GCC 13.3+/Ubuntu 24.04 while current native tools are older; actual task 1.4 qualification must establish its precise tested combination rather than infer failure or support. VST3 parameter/state paths exist, while Linux stopped/editor-absent servicing and ignored GUI failures still need real proof. Standalone stubs/argv0/resource behavior are separate findings.

Selected-file work is absent. The production frontend can retain the 23 kit aliases, but its normal stack overlaps kit keys, New Empty changes the supported layout, and nondefault values alter root defaults that current compatibility compares literally. These are source-derived seams, not executed imported-sample failures. Generic session replacement is reusable, while interactive native history, resource authority and portable state require new implementation. Proposed 16-file/64 MiB-per-file/256 MiB-total encoded limits and decoded-frame/transport expansion are unqualified budgets, to be calibrated before enabling import; they are not claimed measured here.

Existing hook characterization passes 5 fake-Cargo cases without running a mutation job. It confirms the current pre-push selector would still request mutation for changed engine files. User direction excludes mutation, so task 1.2 must fix the maintained normal-hook policy before such publication, preserving ordinary unit tests and strict coverage. No hook has been bypassed or changed by this audit.

The raw archive retains tool/resource queries, baseline source, notices, current ref/worktree inventory, native availability, source-only readiness reports and exact successful normal-hook receipts. Main remains clean at 68dd655; active product implementations, fractional compatibility, selected samples, fresh portable restoration, named DAW and offline-installed beta acceptance remain open.
