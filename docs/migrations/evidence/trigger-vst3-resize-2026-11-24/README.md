# Trigger VST3 resize rollback and REAPER requalification

This requalifies Linux Trigger VST3 after the editor resize rollback repair in
`c47bfd4e932bcdbe359f2a78c8ddb22a5a3361a8`. The repaired private bundle contains
361 files and has SHA-256 manifest digest
`5a0a145524d4768d7cfbaa9a5afbcabefb906354e08009b40a30cde023d84c33`. Both REAPER
processes and the source-hidden native fixture loaded binary SHA-256
`813a527bf44f007efed08a1be50d11e2b8d40f561a78278f8981523c001fb2c5`. Package
inspection matched every unchanged resource and notice to the earlier qualified
manifest. The repaired release resolved every ELF dependency without JUCE.

## Editor geometry

The ordinary view test reproduced a rejected 2400 × 1600 SDK rectangle
replacing the last accepted 1200 × 800 size. The native fixture independently
reproduced a 1600 × 1066 X11 child after a rejected fractional resize. The repair
preserves both sizes. A targeted control that removed the timer guard adopted a
provisional 2400 × 1600 parent during the host callback. With the guard in place,
the child stayed at 1200 × 800. A second control made native synchronization
failure return success after the host's acknowledgment. The test rejected that
result when the X11 parent disappeared and confirmed the repaired view retained
its accepted size, then released its native child when the processor terminated.

Independent review then identified a later failure. The outer child could
already be at 1600 × 1066 when the checked GTK plug request failed and the
SDK rectangle returned to 1200 × 800. The native regression deliberately
destroyed the plug to reproduce that sequence. The repaired path restored the
surviving child to 1200 × 800 before returning the error. The shared-editor
fixture also restored the last accepted 1500 × 1000 presentation after its
host parent changed to 2400 × 1600.

The final run passed 89 ordinary VST3 tests, all three native journeys, strict
Clippy for both affected Rust crates, and the normal pre-commit hook with
1,096 engine tests and strict coverage. The ordinary and native profiles
together exercised all 32 changed executable lines in the adapter. The
shared-editor coverage exercised all 24 changed executable lines under both
GTK DPI configurations. The native VST3 run measured 702 complete callbacks
with zero allocations or frees and verified 350 signed renders. The copied
release ran with the checkout hidden and no network routes. It measured
732 complete callbacks with zero allocations or frees and verified
365 signed renders.

## Installed REAPER

The two private REAPER 7.84 processes mapped the same repaired library from a
source-hidden, route-free environment. The first process recorded signed stereo
audio at 44.1 kHz and 1,024 frames over PipeWire/JACK. Native gestures changed
each instrument independently, and host feedback displayed 1.75 in the editor.
A stopped mapping was admitted before the native child was destroyed. The
accepted selection appeared in the saved project while the editor was still
closed. Recreating the editor preserved native Undo/Redo and left the other
instrument's state intact.

After removal of the source WAVs, a fresh process restored both complete
Trigger states byte for byte and all 46 named values. All four 48 kHz signed
renders matched the independent oracle. Three were exact and the restored
automation render differed by at most 0.693 of one PCM24 least-significant bit.
The oracle used the automation endpoint returned by REAPER after reload and
rejected reversed polarity, a one-frame shift and stepped automation.

The [verification record](verification.json) binds the run to the bundle, host,
project, state and signed renders. The [restored values](values.tsv) record the
46 named controls. All four Python capture replay tests passed against the
retained private inputs.

## Validator and existing qualification

The pinned Steinberg SDK 3.8.0 build 66 validator ran with `-e`. It reported 535
passes and the same two capacity-limit failures in the float32 block and sample
accuracy tests. Both submit 1,472 points in one block. The supported aggregate
capacity remains 1,024 automation points and notes.

The CLAP product was rebuilt and installed from the same source, with binary
SHA-256 `62c9a53db8cdbf5b9d15f00f8f9c1fc3908e2c1facea56918dd80184168481d4`.
All 35 configured CLAP cases passed, including discovery, editor lifecycle,
native state and automation and the installed PipeWire/JACK case. The
installed host completed 376 audited callbacks, received 18 MIDI events
and observed both gestures balance.

The [whole-product qualification](../trigger-vst3-full-product-2026-11-24/README.md)
continues to document the unchanged host, engine, shared-state and React inputs.
This record covers the repaired Linux candidate used for [issue #45](https://github.com/DanMarshall909/dandrum/issues/45).
