# Dispose discarded native import choices through every entrypoint

Independent completion review of8d161936 found that Escape and a new file offer
bypassed model.cancelImport and abandoned the previous ready selection. The first
packet's method-only cancellation test was insufficient;8d161936 was not pushed.
This focused repair remains part of the same CLAP-only beta tasks5.3/5.4/5.5.

One current-dialog cancellation method now serves Cancel, keyboard Escape and
replacement. It clears the current identity synchronously before awaiting native
release, preventing repeated or obsolete callbacks from releasing twice or
closing a replacement. Replacement awaits disposal; it snapshots offered file
metadata first and treats empty offers as no-ops. Closed presentation makes no
late request/update. Cleanup failure stays visible while another mapping choice
can proceed. Accepted patch/history and independent content authority are intact.
No new owner/module/dependency or Rust production code is introduced.

Four genuine fastRED cases follow installed keyboard Escape/Ctrl+O, repeated/
retired cancellation and cleanup failure; two furtherREDs catch empty-offer
cancellation and a transient offered-file list disappearing across the await.
Final249Node tests pass. Import presentation is100%measuredlines/74.63%branches/
64%functions; the inherited broad input-actions file remains62.77/46.51/31.25.
New cancellation behavior has direct outcome assertions; these measurements are
not full-GUI coverage. Production/test refactor review kept one disposal method
and one file snapshot, with no extra queue, growing ownership set or framework.
The keyboard fixture uses production installation and disposes its own DOM/listeners.

Two production browser cases cover DOM-dispatched and browser keyboard Escape,
within14passing entrypoint tests. Actual copied React/Wry/GTK acquires two real
files, cancels the new mapping choice, observes native phase released and checks
unchanged patch/history/current controls and signed selected playback. All31native
CLAP checks pass in120.42s; the expanded import case runs2.12s. Formatting and
strict all-target adapterClippy pass on that exact native test source. The
unchanged CMake boundary retains9passing source-free cases from8d161936.

Calibration is explicit. Initial native setup selected before GTK's folder model
was ready; initial browser setup assumed a collapsed tree. Neither is behaviorRED.
The first integrated native rerun failed with an ambiguous first-ready selector;
retained outcomes may belong to older views. The fixture now identifies exactly
one new ready selection absent before this chooser. That diagnostic run passes.
A fresh disposable replay combines the final correlated test ELF with the exact
pre-repair8d161936bundle: Escape hides presentation but the same new choice stays
ready, genuinely RED. The final repaired artifact releases it. Prior failed runs
are retained, not relabeled as PASS. Bounded passing native UI observations and
all exact replay identities remain in the raw evidence.

[Binding](binding.json), [index](raw-index.json) and [readback-verified raw](raw.tar.gz)
contain source overlays, RED preimages, V8, browser/native/static/spec command and
cleanup receipts. External replay archives are bound by exact hash; current
product remains33af9b90, and final browser/resource identities are separately bound.
Only owned disposable replay scratch/workers were removed, including23003136bytes
for the corrected historical native replay. Reusable disk-backed caches remain;
physical free is about97GiB, with the estimated retention excess still explicit.

The systemic finding is narrow: a method-only cancellation assertion did not
cover advertised keyboard/replacement routes. Tests now exercise those routes;
no new standing rule or memory was written. Main remains clean68dd655, mutation
excluded, VST3/standalone deferred. No task/scenario closes; Empty/lost-reply/
portable/installed/native-input/namedDAW requirements remain. StrictengineClippy
129baselineFAIL and unchangedproductofficial34PASS/10explicitSKIP/0FAIL remain
explicit. Normal hooks and renewed independent approval/publication are separate
gates in the external native-import-review record.
