# Trigger: mockup vs repo and tickets #5–#17

Repo reviewed: `DanMarshall909/dandrum@main` (d02ee1b). Tickets are taken from the summary supplied in chat; I did not read the ticket bodies, so each row below needs checking against the ticket's acceptance criteria.

## 1. What changed in the repo

- **Sampler exists as a VST3.** `docs/advanced-sampler.md`: "Dandrum Sampler" with an embedded drum kit (notes 36 kick, 38 snare, 42 closed hat, 46 open hat), a WebView editor with four pads, five shared host controls and per-pad controls.
- **Public parameters are fixed names.** `drums.pitch_ratio` 0.125–8, `drums.start_offset` 0–1, `drums.level` 0–4, `drums.pan` −1–1, `drums.variation` 0–1, plus `drums.<pad>.…`. Shared and pad pitch/level multiply. Start offset, pan and variation add. 64 stable host slots.
- **Snare split is 1–63 / 64–127** (the mockup uses 1–95 / 96–127). Hard snare and open hat have alternates; hats share choke group 1.
- **Structure is prepared, not live.** Source files, regions, sample map, velocity layers, round-robin, voice limit and choke are changed by editing a patch and Reload Patch. The design system README and the UI plan say structural edit callbacks stay disabled and in-plugin modulation assignment is only proposed. DAW modulation sources are not inspectable.
- **Kernel migration.** Named buses, root ports, `dandrum_kernel_*` FFI, poly regions, `feedback_delay`, public preset surface (`instrument`, `preset_surface`).
- **Lifecycle fix** (`fix-plugin-preparation-lifecycle`): host sample rate applied at preparation regardless of load order.
- **Reusable host UI** (`extract-reusable-instrument-host-ui`): `InstrumentDemoConfiguration`, shared WebView bridge, Sound Lab, second instrument (kick).
- **Renderer-independent UI plan** (`add-renderer-independent-plugin-ui`): typed snapshots, commands with generation and gestures, bounded telemetry at 30 Hz, native and WebView renderers, capability-aware KeyMap / LayerStack / OutputBusses. Every task unchecked.
- **Dev experience review** (2026-09-27): load order, resource root, error surfacing, gesture semantics (one drag, one begin/update/end), automation slot stability.
- **Design system imported** into `docs/design-system/` with adaptation rules that override the export: pointer-free knob, real snare split, no fabricated output pairs or module chains.
- **Future proposals, not v1:** `add-workstation-sampling` (articulations, key switches, release triggers), `add-creative-sampling`, `add-sample-streaming`.

## 2. Mockup conflicts with the repo contract (decide first)

| Mockup | Repo | Proposed resolution |
|---|---|---|
| Zones, tree, layers, routes, slices all editable live; delete/duplicate/paste | Structural edits are patch-time; plugin shows them read-only | Tickets #7 ("map, play, edit and reopen") and #6 imply editing is in scope. Treat structural edits as **prepared edits**: apply to a draft, show a "Changes pending · Reload Patch" bar, commit through one reload transaction. Live parameters stay immediate. Needs your sign-off. |
| Fixed modules (Filter, Amplifier), fixed Reverb/Delay send columns, fixed 8 macros | #6: remove fixed module and send assumptions, capability-driven controls | Render sections and send columns from engine capabilities; show unavailable ones disabled with a reason. |
| Amp envelope → Amp gain route is locked (lock icon, no remove) | #9: replace, detach or restore the default amplitude modulator | Replace lock with a "Default" tag. Actions: Replace…, Detach, Restore default. Warn that detaching removes release and voice reclamation unless another modulator provides it. |
| Route has Amount % only; inspector "Scope" is decoration | #8: per-voice vs shared, additive vs multiplicative, units, update timing | Add to Route: Scope (Per voice / Shared), Combine (Add / Multiply), Units (from destination), Rate (Audio / Control / Block). Show the combined result in the preview. |
| Controls in semitones, dB, % | Ratio 0.125–8, level 0–4, offset 0–1 | Display formatted from the public parameter (pitch ratio shown as ×, with semitone readout); keep the host range as truth. |
| Snare 1–95 / 96–127; "Felt" kit invented | Snare 1–63 / 64–127; kit is the maintained drum kit | Seed Drums with the repo kit and 1–63 / 64–127. Felt Keys remains an invented acceptance instrument (#17). |
| Structural simulation (voices, playhead) | "No simulated audio state" in acceptance fixtures | Mock engine stays a dev tool behind the adapter; production binds to telemetry capabilities only. |
| Terms: Voice Template, Selector, Trigger Rule, Connection, Bus | `nomenclature.md`: module, primitive, defined module, patch vs preset, cable, port, named bus, poly region, control signal | Use repo terms in UI copy. Keep my terms only in the engine contract mapping table. "Patch" menu must distinguish Patch from Preset. |
| No Reload Patch, no load diagnostics | Explicit Reload Patch, detailed Rust load errors, failed reload keeps active patch | Add Reload Patch button and status states (preparing, failed with Rust message, kept previous). |

## 3. Ticket coverage

| Ticket | In mockup | Gap to design |
|---|---|---|
| #6 Graph model, capability-driven controls | Voice sections, tree | Capability states (unavailable, read-only, scoped), parameter ownership badge (shared / per-pad / module) |
| #7 First audible integration | Sources, preview (visual) | Load → map → play → edit → reopen flow with real status; Reload Patch; error states |
| #8 Modulation semantics | Route table, editor | Scope, Combine, Units, Rate fields; additive/multiplicative preview |
| #9 Replaceable default envelopes | Locked Amp env route | Replace / Detach / Restore default (see above) |
| #10 Editing scope, performance view | Inherit output, Mixed, compact mode | Scope chip on every control (Instrument / Group / Sound / Selection), override markers, "N overrides" reset; performance view with PadCell grid and KeyMap |
| #11 Articulations, release triggers, expression | None | Keyswitch lane on KeyMap, articulation list per group, release-trigger layer with "from note" link, pedal behaviour |
| #12 Coherent variations | Random "never same twice" | Take-lock across mics, layers and release; seed field; "no immediate repeat" toggle |
| #13 Selection and blend axes | Velocity crossfade only | Axis picker (Velocity, Macro, Modulator, Key) with Switch vs Blend and a cost readout (extra voices) |
| #14 Slice-to-MIDI export | Slice list | "Export MIDI…" with timing preview and drag-out target |
| #15 Offline auto-sampling | Lush as source | "Sample this patch…" dialog: note range, velocity layers, round-robin count, tail, progress, provenance stored on the Asset |
| #16 "Why did this play?" | Log drawer concept in plan | Diagnostics panel: last hit → rule matched → selector choice → voice decision → or reason for silence; bounded history |
| #17 Three-instrument suite | Keys, Drums, Break in one patch | Make each a separate acceptance patch (keyboard, expressive kit, sample+synth hybrid) plus a patch picker |

## 4. Recommended mockup changes (not applied yet)

1. **Draft and Reload bar** under the tab bar for prepared edits.
2. **Capability states** in Voice sections and routing columns.
3. **Modulation Route** gets Scope / Combine / Units / Rate, and default envelope actions.
4. **Scope chips** and override markers; performance view built on PadCell + KeyMap.
5. **Diagnostics panel** ("Why did this play?") in the bottom drawer.
6. **Seed data**: maintained drum kit with 1–63 / 64–127.
7. **New tool entries**: Export MIDI (Slices), Sample this patch (Lush), Articulations and Keyswitches (Mapping), Blend axis (Layers).
8. Terminology pass using `nomenclature.md`.

## 5. Updates for `Codex Implementation Plan.md`

- Section 0 stack: the repo ships JUCE plus a WebView editor; the target is both renderers behind typed snapshots, not an HTML app with its own store.
- Section 7 store: commands map to the repo's command service with generation, begin/update/end gestures and finite-value validation.
- Section 8 engine contract: replace my method names with the repo's public parameter IDs, root ports, named buses and job IDs. Add `reload` as a transaction and `diagnostics` queries.
- Phases: re-order to match the tickets: #6 contract, #7 first audio, #8, #9, #10, #17, then #11–#16.

## 6. Open questions

1. Are structural edits (zones, layers, routes, tree) in scope for Trigger v1, or read-only with Reload Patch? Tickets #7 and #10 suggest edits.
2. Keep the Felt Kit as an acceptance patch, or replace it with the maintained drum kit and the three #17 instruments?
3. Should the tickets' preference for the existing PadCell grid make a 4×4 pad view the default Drums page?
4. Apply section 4 now, or update the Codex plan first?
