# Controller component-state restoration — 15 November 2026

Task 3.3 now joins the existing processor-stream proof with controller-only
restoration. The controller derives all 23 normalized values from the shared
portable codec and canonical parameter destinations. It validates bounded
metadata, manifest/document agreement and encoded content identities before
publishing the complete projection. It creates no playable session, decodes no
samples and emits no host automation/gesture callbacks. Playability remains the
processor's admission decision: hash-valid non-WAV content projects its values
but fails processor restoration.

A successful restore supersedes an outstanding initial parameter snapshot.
Its later reply is still validated and acknowledged, including exact duplicate
delivery, without overwriting restored values. Failed restoration preserves the
pending reply's authority; a new connection obtains a fresh processor snapshot.
The real COM regression initially changed restored level 0.125 back to the old
default 0.25. The minimal repair keeps the request sequence and an apply-values
flag together, retaining existing correlation and bounded transport behavior.

Actual dynamically loaded controller tests cover legacy and portable state,
all independently derived named values, successful short reads, corrupt and
truncated input, trailing bytes, invalid metadata before payload I/O, zero
progress, failed/negative/excess progress in both read phases, and termination
during foreign metadata/payload reads. Existing processor tests retain fresh
selected content after source/file loss, signed PCM and invalid-load preservation
of nonempty Undo/Redo, values and sounding cursor. Native file-picker and REAPER
acceptance belong to the later combined proof.

Verification: all 54 ordinary VST3 tests pass, with all three intentionally
ignored allocator/reentry children executed by their parents. All 226 ordinary
shared-host tests and the separate 256 MiB portable-state slow test pass; the
slow case took 60.75 seconds. The rebuilt native-editor-enabled CLAP product
passes 20 dynamic tests and its parent-executed allocator child. Both changed
owners pass strict all-target Clippy and formatting. Strict scoped OpenSpec and
the unchanged canonical map pass (657 scenarios / 418 mapped / 239 backlog).

Fresh-profile coverage retains 17 host fast profiles, one separate slow profile
and four VST3 state/connection profiles. The executed shared BufferedState
projection has 26/26 regions, and the executed shared StateHeader validation has
43/43 regions. Separate uncalled unit-library copies remain visible. In the
actual cdylib, setComponentState runs 25 times with 48/49 regions executed; its
remaining defensive absent-projection return follows an already-checked live
controller. Reply handling executes 65/67 regions; the two remaining defensive
missing-bootstrap/projection returns predate this change. No branch, whole-owner
or universal callback-safety coverage claim is made. Exact object/profile hashes
and independent fresh exports remain private under the task evidence directory;
compact results and text logs are retained here.

Production refactoring reuses pure shared validation and the existing codec,
with canonical port names replacing runtime lookup for projection admission.
Validated private framing already proves payload slice bounds, so redundant
unreachable bounds-error branches were removed after characterization. Tests
reuse the existing stream/proxy fixtures; no new fixture framework or source
module was extracted. Initial coverage command/path setup failures are retained
separately from the semantic delayed-reply RED.

This is a local controller-state checkpoint. Full native editor, gestures,
installed REAPER, validator and combined review/publication remain open. No
canonical VST3 scenario is synchronized by this checkpoint. Mutation remains
excluded; reusable builds, dependencies and private binaries are preserved.
