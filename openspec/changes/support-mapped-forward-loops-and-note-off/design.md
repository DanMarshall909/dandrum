## Context

Trigger's document compiler already emits forward-loop metadata and preparation validates it. The mapped renderer ignores that metadata, although the separate `sample_player` implements wrapping and crossfades. A signed prepared-map test reproduced the resulting silence after the region end. The React model exposes Once, Gated and Loop, while the native compiler currently rejects Gated.

## Goals / Non-Goals

**Goals:** Map Once, Gated and forward Loop to real prepared voices; keep sample-accurate note-off and safe bounded voice lifetime; retain accepted settings through native edit history and state restore.

**Non-Goals:** Ping-pong, sustain, CC64, automatic loop discovery, time stretching, overlapping layer policies or a new built-in primitive.

## Decisions

- Add an optional `playback_mode` to each sample region. Omission means `looped` when a forward loop is declared and `one_shot` otherwise, preserving authored documents. Explicit values are `one_shot`, `gated` and `looped`; preparation rejects inconsistent loop metadata or unsupported values. Region ownership matches Trigger's per-region control.
- A mapped `one_shot` ignores note-off and finishes at its region end. `gated` and `looped` stop at the note-off frame, matching the existing `sample_player` gate rule. This is a zero-frame bounded stop. A retrigger starts at the configured start offset. An ended or released voice is reported inactive to the poly allocator.
- Reuse the generic player's forward-loop cursor and equal-length crossfade rule for the mapped renderer. The end frame is exclusive; crossfade length is rounded from milliseconds at the source rate, capped at half the loop, and a crossfade skips its consumed head on wrap. The same cursor rule applies to reverse playback. Keep stereo planes independent and apply gain/pan after the blend.
- Carry the accepted playback mode through the native session snapshot and Trigger projection/compiler. Invalid values fail preparation before a candidate replaces the active patch; undo, redo and state restore use the same compiled document path.
- Characterize the source defect at the prepared map boundary, then prove a representative native host render. Keep detailed rate, short-window, reverse and crossfade cases in Rust engine tests; keep JS tests focused on compiler/projection and transactional edits.

## Risks / Trade-offs

- A looping voice can remain active indefinitely without a note-off. The existing bounded poly limit, steal policy, choke and reset still bound its ownership; explicit release and reset tests will check retirement.
- Crossfade behavior at very short windows can be off by one frame. Signed sample sequences at zero, one and half-window fade lengths and multiple host rates will pin the rule.
- Adding a region field crosses Rust, C++, and browser snapshots. Existing documents retain inferred defaults, and tests will compare the native snapshot with the accepted YAML after restore.

## Migration Plan

No authored-document rewrite. Existing regions without `playback_mode` retain their inferred behavior. New edits write explicit mode. Reverting the feature requires reverting the schema and renderer together; earlier documents remain readable.

## Open Questions

None for this slice.
