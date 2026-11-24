# Linux Trigger VST3 qualification in REAPER

OpenSpec task 5.2 passed with a single private bundle built from main at
`c62fd4924b981b6b17a1f2211c9f2530afa1646d`. Its 361-file manifest was
rechecked after both runs. The bundle digest is
`07d320697b398049de829e67630153aebe4242f5f4a055f5a9227f2696f0c7a5`.
Both native REAPER 7.84 processes mapped the exact binary with SHA-256
`16f00ecec644d9ece503886d42d360b33cf8dfac39cd326d8d853ab8c5841406` before
and after rendering. Each ran from a separate private directory with a fresh
profile, cache and data directory, the source checkout masked and no IP routes
in its network namespace.

## Live audio and native editing

REAPER reported JACK at 44.1 kHz and 1,024 frames while transport was playing.
A separate 44.1 kHz stereo PipeWire recorder linked to the REAPER outputs
captured positive and negative samples on both channels. At the sampled
playback position the automated A level read 0.25 and B stayed at 0.375.
Anticipative FX was disabled on both witness tracks. Per-sample timing is
asserted by the offline renders below.

Physical keyboard edits in the production editor set levels of 2.5 and 1.5
on A and B. REAPER reported 0.625 and 0.375, with the peer unchanged in each
case. A host edit to 0.4375 updated A's native control to 1.75, verified in
the captured editor image. B held 0.375.

While stopped, the real chooser admitted B's mapping. A thread-local hardware
breakpoint held the worker at the start of `MappingJob::prepare`. The host
closed B's floating editor and X11 confirmed that the native child was gone.
The breakpoint was cleared and the worker resumed with transport stopped.
A stopped project save taken before reopening contained B's new document and
its exact selected WAV payload. The adapter's state-save entry point reads
the accepted snapshot. Native Undo in the recreated editor removed the B
mapping and Redo brought it back. A's serialized state was identical across
these saves.

## Save, restore and signed render

The first process saved the project and exited cleanly. Both original WAVs were
then removed. The fresh process restored byte-identical Trigger state for both
instances before new playback. All 46 named normalized and physical values
matched, and each restored document contained only its own selected instrument
along with the packaged kit. The harness hashes the actual embedded payloads
against the independent fixture inputs.

Each render was 960 stereo frames at 48 kHz, including 896 exact silent tail
frames. The comparison evaluates every signed sample from known source PCM and
the specified gain, pan and envelope. Controls with the wrong instrument,
reversed polarity, shifted automation or stepped automation failed the
corresponding assertions.

| Render | Maximum PCM24 error |
| --- | ---: |
| A before save | 0 LSB |
| B before save | 0 LSB |
| A after fresh reload | 0.693 LSB |
| B after fresh reload | 0 LSB |

REAPER serialized the 64-frame endpoint at 63.99984 frames. The restored
oracle used the endpoint queried from REAPER before rendering. After fresh
playback, only A's automated level differed in the next saved state. The
pre-playback restore comparison was exact. Both REAPER processes and their
owned display and audio workers exited cleanly.

[Verification record](verification.json) and [restored values](values.tsv)
record the portable results. The full manifest, source hashes, run receipts,
capture and replay fixtures are retained privately under
`.git/codex/reaper-qualification-20261124/`.

Whole-product qualification, independent review, publication and archival
remain OpenSpec tasks 6.1–6.3. Native Windows and macOS qualification remain
separate milestones. [Issue #45](https://github.com/DanMarshall909/dandrum/issues/45)
