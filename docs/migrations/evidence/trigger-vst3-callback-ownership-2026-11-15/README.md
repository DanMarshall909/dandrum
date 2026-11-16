# Complete VST3 callback ownership — 15 November 2026

Task 2.4's existing real-ABI signed audio/MIDI, exact offsets, fractional clocks,
independent instances, rejection/capacity and reset proofs now include the three
remaining ownership/lifecycle cases identified by an independent read-only audit.
Production is unchanged; these are characterizations of existing behavior.

The existing calibrated allocator child coordinates main-thread state preparation
with its actual audio worker outside measured callbacks. A rejected batch retains
a pending legacy replacement. A valid callback then adopts it, retires the old
runtime and renders literal stereo -0.14536858/-0.1444931 after offset5, with zero
allocations/frees across the complete callback. A second replacement maps kick72
and restores physical level2. Queued native level3.5 is cancelled by measured
reset while the replacement remains pending. Adoption starts silent; a later
offset3 note renders -1 and -716/32768. Shared state confirms reset cancellation,
normalized level0.5 and empty structural history. Existing positive allocation,
preexisting-free and empty-bracket calibrations remain mandatory.

The existing bounded reentry child now invokes nested process and reset from a
foreign event getter, both while stopped and during sounding playback. Both
reject promptly; independent nested buffers clear. The outer next sample remains
11869/32768 on both channels. An ordinary reset after ownership returns succeeds
and renders silence. No private interface or new child launcher was introduced.

A pending restored runtime blocks stopped/deactivated reconfiguration and retains
the original16-frame configuration and signed restored sound. After adoption,
main-thread host service reclaims retirement; process modes1/2 then accept32-frame
44,100.5Hz setup and render the literal restored onset at offset31. The first
positive fixture omitted that required service and correctly remained Busy; its
failure log is retained as fixture calibration, not a product RED or repair.

All55 VST3 tests pass in the instrumented full ordinary selection; three ignored
allocator/reentry children are executed by their parents. Focused ordinary audio,
reentry and allocator runs also pass. Strict all-target Clippy, formatting,
scoped OpenSpec and unchanged canonical spec coverage pass. Relevant CLAP/shared
behavior is source-identical to the preceding controller-state checkpoint; its
actual20-case CLAP and226+1 shared-host results are reused, not rerun.

Fresh export uses only23 profiles written by this full run and the exact current
cdylib. Audio coverage is335/339 lines,466/470 regions and20/20 functions.
Unexecuted regions are the teardown wait and two runtime invariant-error returns;
generic missing-endpoint copies are also disclosed. No branch/MC-DC, whole-owner
100%, arbitrary foreign-memory validation or native DAW acceptance is claimed.
The source-identical production has no new extraction/refactoring need. Tests
reuse existing fixtures and bound each cross-thread wait outside callbacks.

Private raw logs, fresh LLVM JSON/profiles, object identities and allocator
artifacts remain at `/tmp/dandrum-vst3-remaining-20261115/`. The text packet here
retains diagnostics and compact source-bound receipts. The full Linux proof,
host gestures/feedback, native editor, installed REAPER, official validator and
combined review/publication remain unfinished. Mutation remains excluded.
