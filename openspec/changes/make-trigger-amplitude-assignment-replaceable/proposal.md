## Why

Trigger treats its default amplitude envelope and route as fixed mock objects. A musician cannot substitute a reusable modulator or detach the assignment, and the native adapter does not render modulation. Mapped note-off and loop playback are now proven, so the remaining amplitude assignment can be specified against real voice lifetime instead of a fixed display name.

## What Changes

- New sampler patches install a default envelope-to-amplitude assignment and retain the familiar amplitude-envelope controls. An advanced workflow can replace the source, detach it, or restore the default without requiring graph editing in the normal view.
- Make assignment policy and stable identities independent of the envelope's name. Permit a supported source to drive amplitude and another compatible destination under the scope and combination rules of #8.
- Keep poly-region voice completion independent of the amplitude route. Specify note-off, one-shot completion, sustained/looped playback, choking, stealing, and held-note structural edits with bounded voice reclamation.
- Persist the choice and migrate existing silent-mock default patches with their stored values intact; prove the new native default against an independent signed reference. Make replacement, detach, restore, and rejected edits reversible through the accepted native session.
- Prove default equivalence, alternative and shared routing, lifecycle, native rendering, and UI interactions with criterion-specific tests.

## Capabilities

### New Capabilities

- `trigger-amplitude-assignment`: Default, replaceable, detachable and restorable amplitude assignment; separate voice completion; safe commit, persistence, and rendered acceptance.

### Modified Capabilities

- `trigger-react-shell`: Replace the fixed Amp-envelope route interaction contract with advanced assignment actions and capability-aware presentation.

## Impact

Trigger's patch projection, default template, modulation model, editor commands and undo; native document compiler, session persistence and Rust prepared graph; focused JavaScript, Rust and native host tests. The minimum stable identity and poly ownership contract from #6 and the route scope, domain and combination contract from #8 are prerequisites for native implementation; #7 supplies the native UI acceptance path. No second voice allocator or new sampler-specific graph schema is introduced.
