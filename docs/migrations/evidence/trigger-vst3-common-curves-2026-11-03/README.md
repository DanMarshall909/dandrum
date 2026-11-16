# Common normalized curves — 3 November 2026

Partial task3.1 checkpoint. The common Trigger runtime now accepts borrowed
normalized point slices, validates the entire bounded batch before ownership or
queued work changes, interpolates from the previous endpoint at frame -1, fills
preallocated physical control planes and holds final endpoints. Notes retain
list order; controls are present before DSP samples a note's onset. No point per
sample is appended to a queue. CLAP retains the empty-curve discrete-event path.

Independent literal PCM oracles check every ramp/jump sample, equivalent16 vs8+8
partitions, and a start-offset jump at note frame5 selecting source frame11999
(value105/32768), followed by silence. A precision witness distinguishes the
original normalized endpoint from a physical round trip and checks reseeding
after a physical write. Invalid/empty/duplicate/excess curves, nonfinite/out-of-
range values, bad offsets and mixed discrete parameters preserve queued work,
feedback, scalars and an admitted replacement; the later valid call produces
literal signed -1 stereo. Measured processing allocates/frees nothing.

The owning fast suite, exact-source LLVM coverage and strict Clippy pass. Shared
conversion module:6/6 lines,12/12 regions,2/2 functions. Owner coverage remains
partial and is recorded in verification.json, with duplicate harness variants
visible. CLAP editor unit boundary passes11 tests; its calibrated FIFO child is
executed by the parent. The initial default-feature CLAP run had zero tests and
is retained as a non-qualifying trial. Compiler/accessor setup failures are kept
separate from the semantic RED: the no-interpolation scaffold returned -.5 where
the first signed curve sample requires -.625. Full logs are text-only in the
archive; private LLVM JSON/binaries remain outside Git.

A bounded independent read-only reader found no concrete curve defect and
identified the now-tested endpoint-precision gap and redundant range checks.
This is not the required independent GPT-6 Astra completion review. The VST3
host-queue translation, callback/native tests, state, production editor, installed
REAPER, official validator and final review/publication are still outstanding.
