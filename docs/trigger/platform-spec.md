# Trigger standalone platform specification

This document updates the supplied `Sampler Platform Spec.dc.html` for the
implemented React handoff. The supplied file remains immutable in `reference/`.
The accepted OpenSpec change is `add-trigger-react-shell`.

## Boundary and ownership

Trigger is an independent React/Vite editor with a silent EngineAdapter. The
existing C++/JUCE host and Rust graph engine are unaffected. A subsequent adapter
will translate this contract into prepared graph edits and real asset analysis,
telemetry and live parameters. React components do not import engine code.

The adapter owns accepted Patch DTOs, stable IDs, assets, regions, trigger rules,
selectors, voice templates/modules, modulators/routes, macros, output buses,
processor chains and operation history. The Store owns command history and
editor state. Menus, focus, selection, viewport, browser query and temporary drag
values are editor state; they do not become patch data.

Prepared operations validate a cloned draft and publish preparing → ready only
on success. Failure preserves the accepted patch and undo operands. Live writes
apply immediately. Undo keeps affected properties/entities/order, never complete
patch snapshots. Separate pointer gestures remain separate; rapid same-control
edits coalesce through 600ms. Queued gesture cancellation restores only its edits.
Host telemetry values survive unrelated undo.

## Interface and presentation

The canonical declarations are [contract.ts](../../web/trigger/engine/contract.ts)
and [engine contract](../../web/trigger/engine/engine-contract.md). In addition to
the supplied API, the shell has transactional import/mapping, history, bulk slice,
processor parameter, template, macro rename and exact-source audition operations.
Native implementations must preserve their acceptance and failure semantics.
`applyDelta(UndoOperand[])` is a required prepared operation for command undo and
redo. Store and its telemetry reader are type-checked against EngineAdapter, and
an executable boundary test supplies only its declared methods. The adapter owns
its telemetry subscription lifecycle. Optional MockDiagnostics is injected
separately; a native adapter requires no simulator, mutable log or mock switches.

Eight workspaces use shared supplied controls, theme tokens and local OFL fonts.
The exact outer geometries are 820×560, 1200×800, 1600×1000 and compact 900×126.
The minimum uses sidebar drawers; expanded adds the Voice mapping overview.
All visible text is at least 11px. Below the plugin minimum, the browser scrolls
around the fixed native editor; it does not scale controls below readability.

Sample/history/slices use normalized asset coordinates. Scientific pitch labels
use C4=MIDI 60. Velocity is 1–127; routes are −1–1; live parameters/macros are 0–1.
Visible units have paired format/parse conversion. Histories apply bottom to top;
Source stays locked and source files remain unchanged. Slice history owns its
regions/rules and shared counts. A collapse produces a simulated rendered asset
record while retaining the original source and reversible operands.

Empty starts with no source mappings; Felt Kit seeds Keys, Drums and Break.
Imports expose sequential, root+velocity, stack and RR choices. Unsupported and
missing resources retain recoverable state. Pitch/loop/transient/loudness jobs
require explicit apply and preserve edits on cancellation or failure.

## Runtime and handoff

A separate 30Hz telemetry subscription projects silent voices/playheads/meters,
selector progress, modulation activity and simulated host automation. Computer
key release/blur, component teardown and patch replacement clean up playback,
listeners and jobs. An 180px docked drawer exposes bounded calls/events/results,
filtering and diagnostic switches. No audio context or DSP is created.

A component catalog uses the actual 97 component exports with nine states,
820/1200/1600 canvas choices, backgrounds, editable props and callback logs.
The [feature audit](feature-audit.md) records operations and proof for each area.
The [verification record](verification.md) separates Node coverage from browser
execution, native checks and manual source comparison. This shell is ready for
product phase-3 review; native mounting, decoding/audio and persistent presets
are the next separately scoped integration work.
