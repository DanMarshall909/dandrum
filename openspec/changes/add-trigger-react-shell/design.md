## Context

The user's archive `Advanced Sampler App Design.zip` (SHA-256 `e6cc080814bdfcd45a99d1186dd0e84ff36f1bffe59228b76e5d024b77f457b5`) contains the canonical Trigger mockup, platform specification, implementation plan, shell plan, tokens and compiled React primitives. The implementation plan selects a silent mock-engine shell. The user explicitly selects React and the Trigger name.

## Goals / Non-Goals

**Goals:** implement the entire supplied shell plan through phase 3, including all eight pages, all 24 action-reachable states, four exact window geometries, commands/undo, every menu action, component catalog, contract and feature audit. Keep the supplied source immutable and compare actual reference renders with the React implementation.

**Non-Goals:** real audio, DSP, decoding, disk presets, JUCE port or replacement of existing plugin adapters.

## Decisions

- Isolated `web/trigger` React/Vite package on current main. Existing host integration worktrees are independent.
- Reuse the supplied React primitives as local modules, using their authoritative CSS tokens and locally bundled fonts. Convert the canonical template into JSX, then split at the component boundaries defined in the implementation plan. Do not embed the Design Components runtime, an iframe, or runtime string evaluation in the final app.
- A JSON model is owned behind `EngineAdapter`. Store dispatches reversible operation commands and retains editor selection/layout separately. Per-field before-values are command operands; global patch snapshots are not undo entries. Gesture updates merge into one command, as do same-control rapid edits within 600ms.
- Prepared edits expose preparing/ready and asynchronous failure. Live parameters change immediately. Mock telemetry is sampled separately at 30Hz and decays voices/playheads/meters; it is never represented as real audio.
- Source render wins where supplied mockup and prose disagree, except the user's explicit 2026-10-06 decision: enforce 11px minimum text and adjust surrounding layout. Record intentional typography/reflow differences in final visual comparisons.
- Target fewer than 200 lines per authored React component. Keep patch/state/adapter and presentation projection logic in separate modules; preserve the received compiled library as vendor source. This supersedes the earlier 500-line preference.
- Loading a different patch or reloading prompts before discarding unsaved edits and clears undo history after successful replacement. Parameter and structural edits use command undo without repeated prompts.
- Multi-file imports offer root + velocity, one per key, stack and round robin. Recognize common explicit pitch/MIDI, velocity and round-robin tokens and show their interpretation; do not presume one universal filename standard.
- Tests own observable adapter behavior and actual rendered UI interactions. A source-to-implementation comparison at matching state and density owns visual claims.
- User decision on 2026-10-06: implement the shell without test-first TDD. Verify meaningful behavior and rendering after implementation; this overrides the repository's test-first ordering for this shell.

## Risks / Trade-offs

- Many source menu callbacks are inert and some display data is constant → convert each to typed adapter commands and record it in the feature audit.
- Advertised handoff/component sources and font files are missing from the ZIP → reuse embedded React/icon implementations and download redistributable fonts with provenance.
- Mock timings differ from Dandrum → expose configurable timers and failure switches; document the adapter boundary without claiming engine integration.
- Exact geometry is desktop/plugin oriented → preserve the specified minimum 820×560 and add a viewport wrapper that permits inspection below minimum without scaling the editor.

## Migration Plan

Import/reference capture, foundation and commands, frame/components, full pages and interactions, async telemetry/log, catalog/audit/contract, browser and independent review. Keep this change open until every required gate passes; no existing main-spec requirement is modified by the standalone shell.
