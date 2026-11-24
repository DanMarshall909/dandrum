# Trigger VST3 resize rollback and REAPER requalification

This requalifies Linux Trigger VST3 after the editor resize rollback repair in
`ae15c3cef91d45fc77684f0d6a48662f5fbe5697`. The repaired private bundle contains
361 files and has SHA-256 manifest digest
`88a6ec5fa735375ba7dcea74e9be9c93f61bec378125f61a9b2422aa4dfb61f8`. Both REAPER
processes and the source-hidden native fixture loaded binary SHA-256
`cb74164a3b58fa800263ed9fe5a056326aa46776cb9f34fe34aeec2c66967617`. Package
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

The final run passed 89 ordinary VST3 tests, all three native journeys, strict
Clippy, and the normal pre-commit hook with 1,096 engine tests and strict
coverage. Separate ordinary and native profiles exercised all 34 changed
executable lines in the adapter together. The native run measured 696 complete
callbacks with zero allocations or frees and verified 347 signed renders. The
copied-release journey ran with the checkout hidden and no network routes. It
measured 682 complete callbacks with zero allocations or frees and verified
340 signed renders.

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

The production source change is confined to the VST3 view. The
[configured, shared-owner and installed CLAP checks](../trigger-vst3-full-product-2026-11-24/README.md)
remain applicable to their unchanged inputs. The installed and native evidence
here qualifies the repaired Linux candidate used for [issue #45](https://github.com/DanMarshall909/dandrum/issues/45).
