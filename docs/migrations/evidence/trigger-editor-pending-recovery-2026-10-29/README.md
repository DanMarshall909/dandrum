# Recreated production editor follows native pending work

This task5.5 consumer slice builds on the separately committed pending-request
metadata prerequisite7b149a50. A fresh JUCE-free NativeEngine follows the explicit
activeRequestId through existing bounded status calls, then reads the canonical
accepted patch/history through its live attachment. It never resubmits preparation.
The flat2000-probe limit applies even if other work keeps becoming active. Status
identity, revision/cursor progression, attachment/instance identity and terminal
metadata are checked before accepting a snapshot; absent optional IDs preserve
legacy/comparison initialization and never trigger historical cursor guessing.

A retained failed job now leaves accepted editing usable with its bounded failure
in Store's initial status. A successful later operation clears that warning. No
browser history/job owner, new queue/module, dependency or state codec is added.
Closure checks stop each awaited initialization stage; late meter continuations
cannot issue a new native visibility request or attach parameter subscriptions.

Genuine REDs cover dropped optionalu64 metadata, initialization returning before
following an admitted lost-reply document, unusable failed startup, one extra
request after negotiation closure, already-closed initialization, and the early
independent read-only audit's late title/feedback continuation finding. Guard
characterizations for malformed outcomes/connections were added after the first
success implementation; they are not claimed as pre-implementation REDs. Original
precursor/expanded logs and exact later RED overlays are retained distinctly.

Final278Node tests and production types/components pass. NativeEngine100%lines/
88.22%branches/92.24%functions; NativeLive100/87.80/85.71; SessionConnection100/96.30/
100; Store100/87.50/90.48. These are measured component scopes, not fullGUI coverage.
The production consumer test closes an actual NativeEngine with a withheld
admission reply, follows running/completed through a fresh consumer, preserves
nondefault values and one native history entry, then uses native Undo without
repreparing. Failure/closure/state-validation cases protect refusal and lifetime.

Covered test refactor: the2000-step bound initially reparsed YAML2000times and
cost12.893s. Reusing the same immutable fixture document while varying only native
connection metadata preserves every status step and completes in41.812ms. The
initial source/run remains retained; no test was removed or limit weakened.
Production refactor review kept one focused recovery loop and existing owners;
a generic startup framework or persistent browser job registry is unnecessary.

[Binding](binding.json), [index](raw-index.json) and [readback-verified raw](raw.tar.gz)
retain source/RED overlays, final rawV8, commands, complete output/exit and owned
cleanup receipts. All owned scratch/workers cleaned; reusable disk-backed caches
and prior replay objects preserved. CurrentRust release/native/official and real
browser qualification are still pending; existing1c770ba4 evidence applies only
to published6ac70423. Next calibrate this consumer with actual copiedReact/Wry/GTK
selected import, lost reply, complete recreation and signed PCM under real runtime
backpressure, then independently review the combined candidate before publication.
StrictengineClippy129baselineFAIL remains unchanged. No beta task/scenario closes,
main68dd655 is preserved, mutation excluded, VST3/standalone deferred.
