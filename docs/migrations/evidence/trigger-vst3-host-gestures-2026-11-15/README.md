# VST3 host gesture checkpoint — 15 November 2026

This candidate closes the local criteria of task3.2, together with the earlier
[stopped-feedback checkpoint](../trigger-vst3-stopped-feedback-2026-11-15/README.md).
Independent GPT-6 Astra checkpoint review passed for staged candidate
`7dc84cdfc341806c1632c00ec6991b83ee967702` and its finite local-checkpoint envelope. The accepted Linux goal remains
active: native VST3 editor integration, installed/offline REAPER proof, the full
pinned validator, combined review/publication and archive remain outstanding.
Mutation is excluded; all products, samples and LLVM profiles stay private.

The processor stages the shared owner's ordered automation ledger and sends
normalized public parameter IDs through standard VST3 connection interfaces.
Only a correlated application acknowledgement consumes its current ledger head.
The controller calls the host's retained IComponentHandler on the main thread,
records each accepted phase before acknowledging it, and retries refused phases
without repeating successes. A missing handler retains pending work. Begin/End
use the original handler; replacement and reentrant lifecycle calls are fenced.
Host callbacks preserve zero native Undo/Redo entries.

The shared ledger remains bounded at1024 entries and reserves each explicit
End in advance. The real ABI capacity test withholds Begin acknowledgement,
admits1022 ordered values, rejects another value and Begin, admits the reserved
End, then drains exactly1024 accepted callbacks after acknowledgement resumes.
A later edit proves capacity returned. Invalid delivery counters, identities,
normalized values, phases, conflicting receipts and out-of-order events preserve
host callbacks and the valid pending delivery. Exact retries are acknowledged.

Closing bypasses the full/refused ordinary eight-packet queue through one
terminal slot. Accepted host Begins are balanced before processor ownership is
cancelled. Refused End retains the owner, endpoint and service for retry; the
backpressure test resumes actual stereo audio with literal-1.25 and-895/32768
at offsets3/4, then proves one accepted End and successful cleanup. Cancellation
removes only the departed editor's tagged automation, once, while retaining
admitted scalar values, other editors' ordering and reserved capacity. It is
explicit cancellation, not an acknowledgement of an undelivered host callback.

Retired scopes cannot cancel a replacement attachment. Closing/rebind, refused
terminal sends and lost intermediate receipts have real proxy tests. The final
Ack has a finite retention boundary: if a proxy accepts but drops RetiredAck
after the controller releases its return channel, the processor retains a
one-way tombstone until explicit matching local disconnect or final COM lifetime
release. The test exercises that finalization and stale receipt rejection;
unlimited delivery recovery or simultaneous teardown knowledge is not claimed.

Independent review found lifecycle gaps in both connection publication and
local disconnection. Public AddRef/Release callbacks first produced assertion
REDs, then passed with a scoped processor guard. It ends before normal handshake
pumping, retaining existing inline disconnect/reconnect behavior. Thread
rejection precedes affine Cell/RefCell access. The manual test peer now owns its
allocation by COM reference count; initial unsafe assertion-unwind fixture trials
are retained as failed fixture diagnostics, separate from the confirmed REDs.

Current-source gates pass73 VST3 tests in both ordinary and instrumented lanes
(the three ignored allocator/reentry children are parent-executed),230 shared
host fast tests, two doctests, and the actual256MiB portable-state test:57.07s
execution/58.41s complete producer. The fresh CLAP release passes20 dynamic
tests, including its parent-executed allocator child, and actual copied native
host gestures and rendered feedback under inherited X11. Both native owned
runs report exit0, scratch removed and no workers. Strict all-target host/VST3/
editor-enabled CLAP Clippy, formatting, strict selected OpenSpec, the canonical
scenario map and nine relevant configured CTest checks pass.

Fresh LLVM exports select only profiles produced by these exact owner runs and
record their object identities. All regions of the used shared cancellation
API and its retain predicate execute; unused unit copies remain disclosed.
The full owner and changed-path gap receipts also retain defensive liveness,
owner/descriptor invariants, counter exhaustion and terminal-pump/supersession
edges. Whole-owner100%, branch/MC-DC coverage and arbitrary hostile-memory
safety are not claimed. A covered scoped guard was reused; no module extraction
or new session/history authority was introduced.

`source.json` binds six source/test files and their base patch.
`verification.json` binds final commands/timings, fresh profiles/objects, source
coverage/gaps and copied native product/test/resources/cleanup. Compressed
members are UTF8 diagnostics only, including retained REDs, fixture errors and
lint trials. Two intermediate full VST3 logs were superseded by the final run;
their final producer receipts are not claimed as separately retained raw runs.
Private raw artifacts are under `/tmp/dandrum-vst3-remaining-20261115/gestures/`;
retention and independent review are indexed in `.git/codex/`.
