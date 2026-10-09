# IMPLEMENTATION — proposed slices, dependencies, acceptance, open questions

This is a proposed sequence, not a ticket set. Each slice should go through the repo's OpenSpec workflow (`AGENTS.md`): proposal → tasks → TDD → spec coverage. Nothing here authorises structural authoring in the plugin; slices that need it are listed separately and blocked on Q-1.

## 1. Principles

- Build presentation components from props and callbacks only (`COMPONENTS.md` § 3). Engine access goes through the shared command/snapshot layer proposed in `add-renderer-independent-plugin-ui`.
- Read every value from the loaded instrument (zones, velocity splits, control groups, voice limit). Never hard-code the prototype's mock content.
- Capability-gate: a control whose engine capability is absent renders disabled with a reason, or is not shown.
- Live public parameters use begin/update/end gestures; structural data stays prepared.

## 2. Safe first slices (no new engine capability)

### S1 — Tokens and theme base
Generate CSS and C++ constants from the DS tokens (one source), including alias tokens re-declared per theme. Ship the default theme only.
- Depends on: renderer-independent change § tokens.
- Accept: every colour/radius/spacing in the new editor resolves to a token; no literals; default theme visually matches the prototype at 1200×800.

### S2 — Editor frame, read-only
Header (wordmark, Patch box showing the loaded patch name, load status, size switch), page rail with all pages present but only Pads enabled, status bar, three sizes.
- Depends on: S1; existing `getParameters` snapshot.
- Accept: side-by-side with the prototype at 820×560, 1200×800, 1600×1000 within 1 px for frame elements; disabled pages show a reason tooltip; no text below 11 px (see Q-10).

### S3 — Play drawer with PlayPads and PlayKeys
Drawer bar, open/close, edge resize, section toggles, pads/keys split, PlayPads (PadCell) and PlayKeys bound to `noteOn` / `noteOff`. Pad names and mapped keys from the prepared sample map.
- Depends on: S2; read-only zone metadata in the snapshot (planned; if absent, first add a covered read-only metadata accessor — Q-11).
- Accept: clicking a pad or key plays the mapped note with velocity from position; unmapped keys greyed; pad and keyboard Fit show exactly the mapped range of the reference kit (36–46); Fit → − → + returns the identical layout; same component instances render in drawer and Perform view; snare velocity split shown from the map (1–63 / 64–127).

### S4 — Live shared and per-pad controls
Knobs for the five shared `drums.*` controls and the per-pad controls of the selected pad, with value popups, middle-click reset, host tint.
- Depends on: S2; gesture semantics from the dev-experience review.
- Accept: one drag = one begin/update/end; host automation moves the knob and shows host tint; values displayed in design units (×, dB, L/C/R) while the host value stays the truth; parameter IDs and slots unchanged.

### S5 — PC keys and octave
Toggle, A–K / Z–X mapping, octave control, letters on keys.
- Depends on: S3; Q-5.
- Accept: off by default; when on, keys play only while the editor has focus and never in text fields; DAW shortcuts unaffected when off.

### S6 — Patch browser and Reload Patch
Browser popover (search, list, load), right-click menu, Reload Patch, pending/failed states. Export copies the loaded file; Import loads a chosen file through the existing reload transaction.
- Depends on: S2; reload transaction (exists); catalogue source (Q-3).
- Accept: loading a patch goes through the safe replacement; a failing patch keeps the previous one playing and shows the Rust error; dropped parameters warning surfaced; no in-plugin text editing.

### S7 — Perform view
860×610 layout: patch list, 4×4 PlayPads, 2×4 macro/controls grid, full-width PlayKeys, shared zoom state.
- Depends on: S3, S4, S6; Q-6, Q-8.

## 3. Slices that need new engine capability (blocked)

| Slice | Needs | Blocked on |
|---|---|---|
| S8 Telemetry: pad activity, voice count, meter, sounding keys, playheads | `plugin-visual-telemetry` | renderer-independent change |
| S9 Read-only Mapping / Layers / Sample pages (KeyMap, velocity lanes, candidates, waveform) | prepared metadata + `plugin-analysis-display` peaks | renderer-independent change |
| S10 Voice page controls (filter, envelopes, filter response drag) | A patch graph that includes `filter`/`adsr` and exposes their control ports as public parameters | Patch design; Q-12 |
| S11 Structural edits as draft + Reload (zones, layers, slices, regions, tree) | Draft model and a write path to the patch | Q-1 |
| S12 Modulation page and drag-to-assign | Plugin-side modulation routes | Q-2, ticket #8/#9 |
| S13 Macros | Macro concept in the patch/preset surface | Q-8 |
| S14 Routing and Effects editing | Per-layer sends, FX busses, editable chains | Q-1 |
| S15 Diagnostics, Freeze, Slices analysis, multi-file drop | Respective engine features | not found in repo |
| S16 Additional themes | Token pipeline from S1 | Q-7 |

## 4. Acceptance criteria common to all slices

- Visual: matches `prototype/Advanced Sampler v2.dc.html` in the corresponding design state at each size (screenshots in `screenshots/` index the states).
- Every control either works through the command layer or is disabled with a stated reason.
- No colours, fonts, radii or spacing outside tokens.
- Tests specify behaviour first (TDD per `AGENTS.md`); spec scenarios mapped in `spec-tests.map`.
- Parameter IDs, host slots, persisted state and MIDI behaviour unchanged.

## 5. Unresolved questions

1. **Q-1 Structural editing.** May the plugin author zones, layers, slices, regions, routing and modules (as a draft committed by Reload Patch), or are these read-only with external editing + file watch? The renderer-independent proposal currently says read-only.
2. **Q-2 Undo scope.** What is undoable: live parameter edits only, or also draft edits? Does undo interact with host undo?
3. **Q-3 Patch catalogue.** Where do patch categories (Drums/Keys/Hybrid/Synth), origin (Factory/User) and favourites come from and persist? `$LIB` / `$USER_LIB` folders? `instrument-presets`?
4. **Q-4 Wording.** "Import/Export YAML definition" (as requested) or "Import/Export patch definition" (DS content rule)?
5. **Q-5 PC keys.** Acceptable in a plugin window given host keyboard focus behaviour? Which hosts must be tested?
6. **Q-6 Window sizes and Perform view.** Is the Perform view a separate editor size the host resizes to (860×610), and are drawer state and size persisted per instance?
7. **Q-7 Themes.** Ship surface/accent/secondary themes, or default only? If shipped, are they a user setting or per patch?
8. **Q-8 Macros.** Do macros exist in v1? If not, does the macro grid show the five shared controls instead?
9. **Q-9 Inheritance UI.** Is the Overview / scope bar in scope for v1, given shared × per-pad combination rules?
10. **Q-10 10 px text.** Allow 10 px for rail labels, pad badges and key letters, or raise to 11 px and widen the rail?
11. **Q-11 Metadata accessor.** Is a read-only prepared-metadata FFI (zones, control groups, region names) acceptable ahead of the rest of the renderer-independent change, to unblock S3?
12. **Q-12 Filter/envelope patch.** Which acceptance patch exposes filter and envelope controls so the Voice page has real parameters?
13. **Q-13 Design-system additions.** Upstream the 8 new icons and `--dd-cap`/`--dd-pad` variables into `docs/design-system/`?
14. **Q-14 Velocity split display.** Confirm the prototype's 1–95 / 96–127 is mock only and the loaded 1–63 / 64–127 is what ships.

## 6. Verification of this handoff

- Reference assets present: prototype v2 + v1, shared Play Pads / Play Keys, `support.js`, DS bundle, `styles.css`, 7 token files, manifest, 20 screenshots, earlier planning notes.
- Every interactive control in `INTERACTIONS.md` has a defined behaviour (D) or an open question / missing capability reference.
- No production source was changed: all files are in this design project under `design-reference/advanced-sampler/`. The repo was read only. Copy this folder into the repo at `design-reference/advanced-sampler/` (outside the build) to hand it to Codex.
