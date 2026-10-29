# Account mixed-state exports in the existing catalog

Source bindings pin verified code/evidence at a6950931. A subsequent contract
clarification distinguishes capacity refusal after bounded encoded read/hash
from competing-acquisition Busy before payload allocation/read. Rust inputs
are unchanged; binding.json separately identifies the current clarified contract.

Independent review rejected the initial mixed-state candidate
`e854f0819b91e1198a8c2bc610f3943011c7823f`: packaged_state_sample decoded outside
catalog reservation/live-union accounting. That checkpoint was not published.
The [initial packet](../trigger-portable-mixed-state-2026-10-29/README.md) retains
its historical source bindings and genuine mixed-state RED/GREEN. Its source
hashes are checked against e854f081 rather than this repair.

The new regression retains256 distinct real file-selection results in the
catalog and publishes a mixed patch using one selected resource and the kit.
Before production repair, save incorrectly returns Ok instead of TooLarge;
exact fully qualified RED executes one test in0.09s. The earlier short exact
filter selected zero tests and is explicitly not RED.

Factory saving now uses the existing catalog restore/reservation/hash/decoder/
completion path. It reserves before payload allocation/read and retains the
temporary snapshot's quota charge until writing ends. Existing factory snapshot
reuse remains available without a second decode. Main save consequently needs
&mut controller for catalog bookkeeping; the audio endpoint stays independent.
The CLAP adapter releases this borrow before calling the host OutputStream.
No second decoder/cache/lock, interior-mutability scheme or capacity exception is
introduced. Call-site changes in tests are mechanical mutable borrows; their
existing assertions remain.

GREEN rejects exhausted resource capacity with TooLarge, releases unrelated
results, then rejects a live native acquisition reservation with Busy. Neither
writes a prefix nor changes the accepted patch, physical values or literal
kit[-.5,-.5]/selected[-.25,-.25]. Dropping the reservation permits saving, and the
factory's temporary quota charge disappears when save returns. The original
mixed fresh-instance test still restores both signed sources, all23values,
logical YAML and fresh history after source/root removal, with zero measured
callback allocation/deallocation and an unchanged independent instance.

Covered refactoring shares one complete bounded payload reader, including
trailing-byte rejection and error conversion, between portable restoration and
packaged export. Every annotated line of that extracted reader executes. Final
complete host/session/editor-adapter tests, formatting, strict all-target owner
Clippy, active strictOpenSpec and unchanged spec map631/392/239 pass.

An ordinary copied headless debug CLAP artifact passes the actual short-I/O,
host-value-read reentry and failure-atomicity stream test before the final reader
extraction. Final source-bound LLVM additionally runs that real dynamic host
against an instrumented headless DSO:8objects/9fresh profiles combine the6owner
executables with the DSO and dynamic-host executable. This qualifies the changed
save borrow at the real ABI boundary. Broad owner/branch/instantiation coverage
remains partial; annotations prove execution rather than assertions by themselves.

[Binding](binding.json), [raw index](raw-index.json) and [raw evidence](raw.tar.gz)
retain the review repair, preimplementation sources, actual RED, setup failures,
GREEN, final refactor checks, coverage/source/profile bindings and owned cleanup.
Exact final LLVM objects and the earlier ordinary dynamic artifact are archived
outside Git and hashed separately. Normal hooks and renewed independent review/
publication are separate gates in the existing external portable-mixed-review
record. Owned scratch/workers are cleaned; reusable disk-backed caches stay.

Task6.1/6.2 and the10/17CLAP-only beta remain open. Package-onlyv3/full payload
qualification/CLAP stream/currentGUI/official/installed/native-input/continuous
selected-audio/namedDAW gates remain. Strict engineClippy remains FAIL129 unchanged
baseline diagnostics, unwaived; prior GUI/official34success10explicitSKIP0FAIL is
prior-product evidence. Main stays clean68dd655, mutation excluded and VST3/
standalone deferred. No beta scenario closes here.
