# Accept terminal native recovery on its final status probe

Independent completion review FAIL of4980b495 found an off-by-one: a valid completed
or failed outcome on probe2000 was followed by timeout after its idle snapshot had
already been validated. The one-line repair returns that snapshot immediately when
no active request remains. Connection/result validation and the2000-probe limit
are unchanged; a newer active job still consumes the shared remaining budget.

Two genuine REDs exercise1999running replies followed by completed/failed on the
last permitted probe and observe Native recovery timed out. A third case proves
still-running on that same probe remains a timeout. All three use the real
NativeEngine with the established Node22 fake clock,2000actual status calls and
no real60-second delay; no private loop limit or completion hook is substituted.
The existing continuously renewed-work timeout remains covered. Successful final
recovery retains nondefault values/history; failed recovery keeps accepted editing
usable with its error, and no preparation is resubmitted.

Final281Node tests/types/components pass. NativeEngine100%measuredlines/88.31%branches/
92.24%functions; unchanged NativeLive/Connection/Store partialbranch/function scopes
remain explicit in the earlier packet. No new owner, module, dependency or protocol
is introduced. Covered refactor review favors this direct return; a counter change,
extra polling request or general retry abstraction would change the existing bound.

Fresh productionbundle d157a01b passes all33actual native checks119.70s (selected
lost-reply case2.05s), all14Chromium cases9.3s, and the same pinned unfiltered official
validator on product2fe7909d:34success/10explicitSKIP/0FAIL. Current active strictspec/
map631/392/239 pass. Source-free9PASS203.235s and Rust fmt/strict host+adapterClippy/
focused hostLLVM remain valid: this repair changes only two web files. The retained
spec-run receipt executes an empty git diff across every copied source-free input
root; no extra cache-producing source-free rebuild was required. Canonical52strict
spec inputs likewise remain unchanged. StrictengineClippy129baselineFAIL persists;
this is not a waiver or passing engine lint gate.

[Binding](binding.json), [index](raw-index.json) and [readback-verified raw](raw.tar.gz)
retain exactRED preimages, all current command/log/exit/cleanup, finalV8, current
native/official identities and the exact externally archived final native replay.
Earlier4980 archives remain untouched for recovery/review. All owned workers/scratch
cleaned; reusable disk-backed caches/profiles/evidence remain. No task/scenario
closes. Mainclean68dd655, mutation exclusion and CLAP-only scope persist; VST3 and
standalone remain deferred. Physical/native input/device, continuous selected audio,
installed/offline, portable state and namedDAW requirements stay open. Normal-hook
repair checkpoint and renewed independent verdict/publication are separate gates
in the external recovery-review record; the original4980 publication envelope is
obsolete and must not be applied.
