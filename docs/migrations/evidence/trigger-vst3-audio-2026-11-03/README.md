# Trigger VST3 audio checkpoint — 3 November 2026

This is partial task 2.4 evidence, not completion of the Linux feasibility proof.
The real dynamically loaded processor now implements stereo output, event input,
setup, Float32 processing and reset using the common Trigger runtime. Independent
PCM assertions prove signed samples, offset 5, fractional clocks, independent
instances, zero-sample time preservation, whole-batch rejection and cursor
preservation. The declared 1,024-note budget works even with a 16-frame block;
1,025 rejects atomically. Teardown during processing rejects without losing sound.

An explicit common-host preparation policy reserves event scratch off audio and
retains it through replacement/restoration/reconfiguration. CLAP continues using
its existing admission/shrink policy, protected by the existing rejection test.
The initial unconditional approach failed that test and was corrected; the test
was retained. Common rendering remains shared. No engine code changed.

Seven audio plus 27 preceding dynamic tests pass. The calibrated LD_PRELOAD
malloc/free parent runs its intentionally ignored child on a separate audio
thread, verifies positive controls, then observes zero allocations and frees
through measured complete valid/error/zero/note-off/reset callbacks. This is not
yet the full automation/native-callback sentinel. Clippy passes. Audio line
coverage is 242/253 (95.65%); remaining error/ownership gaps remain visible.
The exact dynamic-library LLVM object resolves duplicated export mappings in
Cargo's combined-object report. These are accumulated TDD profiles, not final
fresh-candidate coverage. See verification.json for source identities and counts.

The last coverage trial initially failed because a test reused the stereo value
written by getBusArrangement while expecting unsupported arrangement rejection.
The fixture now explicitly assigns the unsupported arrangement before that call;
production behavior did not change. Both trial logs are retained.

Debug-only disposable test copies strip DWARF to fit the approved 2 GiB scratch
budget; allocated code/data and LLVM maps remain identical. Product/cache inputs
are preserved. Release candidates will be copied byte-for-byte. Private binaries,
SDK, full LLVM JSON and raw outputs remain outside Git. No native editor, state,
curve, REAPER or complete official-validator claim is made by this checkpoint.
