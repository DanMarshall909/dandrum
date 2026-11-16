# Shared native editor integration checkpoint

This task 4.1 checkpoint preserves CLAP behavior while moving browser framing,
native sample opening and chooser admission/completion into the existing shared
Trigger host. Native dialogs, format callbacks and live child lifetime stay at
the adapter boundary. VST3 native attachment and Linux/REAPER qualification remain
open; this checkpoint does not claim those outcomes or redistribution clearance.

`BrowserRequest` borrows the unchanged wire frame and binds its canonical request
ID to it. It grants no session or filesystem authority. Live attachment checks
and owner-negotiated limits still apply in CLAP; shared command admission makes
the semantic decision. Bootstrap IDs retain full `u64` precision in JavaScript,
and replies reach the broker as opaque strings. Queueing is not consumption.

Native selection retains the existing opaque owner permit. Cancellation,
chooser/open failures and opened-file handoff use the same persistent selection
catalog/worker. Tests prove terminal-permit replay refusal, real metadata after
source removal, unchanged history/automation and literal signed playback. Unix
opening preserves exact OS path bytes, rejects final symlinks/nonregular files
and uses nonblocking opening so a FIFO cannot wait for a writer. Only display
names use lossy Unicode conversion; the existing sixteen-file bound remains.

The new bridge has 6/6 functions, 39/39 lines and 59/59 regions covered. The
existing command module has 13/13 functions, 143/143 lines and 202/202 regions
covered in the fresh focused integration run; new acquisition/admission/submission
function regions all execute. The wider owner aggregate includes unused
unit-build copies; this is not a whole-owner 100% or branch/MC/DC claim.

Current-source checks passed: 238 shared-host fast cases, 73 VST3 cases, 9 CLAP
release unit cases, 20 dynamic CLAP cases and eight actual copied-release X11
checks. Required FIFO/allocator/VST child cases run through marker-checking
parents. Native framing, gestures/rendered feedback, selection metadata, Import,
Empty, lost-reply recovery and mapping/Undo after editor recreation all pass;
all eight owned runs report no remaining workers and removed scratch. Two
maintained CLAP install/notice fixture tests, strict owner lint/format, OpenSpec,
spec mapping and nine selected CTest checks also pass. Full Linux final-current-
source slow/validator/REAPER gates are still required by their own tasks.

The pre-extraction mapping/view-loss timeout and successful fixed-input repeat
remain private evidence; the fresh current-product case passes in 4.67 seconds.
An initial bridge-test construction compile error is retained as a diagnostic,
not presented as a semantic behavior failure. This is a characterized extraction,
not a change to the musical session or wire policy.

Dan approved the finite resource reassessment on 16 November: 56 GiB cumulative
outputs and 112 GiB managed retention, preserving caches, 2 GiB scratch, 8 GiB
available disk/RAM floors and serial builds with at most four compiler jobs.
Every heavy producer passed the full no-eviction preflight; mutation remains
excluded. The separately reviewed cache-preservation repair committed through
the normal engine coverage hook at `f46069c45ae6ca876b44e35618930922320d046f`.

[verification.json](verification.json) records source/artifact hashes, exact
native test names, coverage scope and outstanding acceptance. Raw logs, copied
private products, source-bound profiles/objects and review stay private. Native
Windows/macOS remain mandatory later milestones.
