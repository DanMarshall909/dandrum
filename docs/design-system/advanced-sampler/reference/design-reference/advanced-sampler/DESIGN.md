# DESIGN — layout, dimensions, type, colour, states

All values are logical pixels. All colours, fonts, radii and spacing come from design-system tokens (`--dd-*`, `--font-*`). Do not introduce literals.

## 1. Window sizes

The editor re-lays out at fixed sizes. It does not scale.

| Size | px | Differences |
|---|---|---|
| Minimum ("Min") | 820 × 560 | Page rail 40 px, icons only. Tree becomes a 36 px icon rail. Inspector 232. Play drawer closed by default. Header labels shortened ("Perform", icon-only "Why did this play?"). |
| Default ("Def") | 1200 × 800 | Rail 68 px with labels. Tree 208. Inspector 272. Play drawer open. |
| Expanded ("Exp") | 1600 × 1000 | Tree 248. Inspector 320. Taller waveform and editors; Voice page adds a read-only KeyMap overview. |
| Perform view | 860 × 610 | Separate mode (header "Perform"/"Open editor"). See § 6. |

Size is chosen with the Min / Def / Exp segmented control in the header.

## 2. Window frame (v2)

Vertical stack, background `--dd-ink-1`, 1 px outer border, overflow hidden:

1. **Header** 44 px (38 at Min), `--dd-ink-0`, bottom hairline `--dd-line-1`, padding 0 12, gap 12. Left → right: wordmark `dandrum` (Barlow Bold 20, lowercase) + 6 px ember dot + `TRIGGER` (13 bold caps +0.1em, hidden at Min); divider; Undo / Redo icon buttons; **Patch** box (MenuButton, 220 / 150 at Min); status message; spacer; "Voices 6 / 32" (12 mono); Min/Def/Exp; "Why did this play?" (icon-only at Min); "Performance view" button; 30 px meter.
2. **Banners** (only when relevant): Reload pending, missing file, unsupported file. `StatusMessage` compact.
3. **Body grid** `display:grid; gap:4; padding:4`, columns: `[rail] [tree] minmax(0,1fr) [inspector]`.
   - **Page rail** 68 (40 at Min). Groups with 10 px caps headings in the secondary colour: PLAY (Pads, Macros) · BUILD (Mapping, Layers, Voice) · SHAPE (Mod, Route, FX) · Overview. Item height 40 (30 at Min); icon 16; label 10 px caps. Current page: `--dd-ink-3` fill, 1 px `--dd-line-2` inset, icon in accent. Modulation shows a count badge.
   - **Tree column**: Instrument tree, Sources, Modulators (collapsible lists), footer hint. See v1 plan § 2.1 for row metrics (unchanged).
   - **Workspace** (`<main>`): the current page, a stack of panels.
   - **Inspector**: breadcrumb, title + type tag, optional History block (Sample/Slices pages), property rows.
   - Every column is a panel: `--dd-ink-2`, 1 px `--dd-line-1`, radius 6.
4. **Play drawer** (§ 5), margin 0 4 4, radius 6.
5. **Status bar** 26 px, `--dd-ink-0`, 12 px `--dd-paper-3`: page hint left, size right; shows drag/assign hints and transient notes ("Loaded Felt Upright", "PC keys on · …").
6. **Overlay layer** inside the window: context menus, patch browser popover, value popups. Menus flip to stay inside the window.

## 3. Pages (workspace)

Thirteen workspace views, reached from the rail and the design-state bar:

Pads · Macros · Sample · Slices · Mapping (KeyMap) · Layers (velocity / round robin / alternate / random / weighted) · Voice (chain strip, sections, Amp + Filter envelopes, Voices policy, Filter response) · Modulation (route table + modulator editor) · Routing (output bus + send table, OutputBusses) · Effects (LayerStack ×2) · Overview (inheritance) · Diagnostics ("Why did this play?") · Empty / multi-file drop.

Sample and Slices are reached from the tree (selecting a sample or a sliced asset), not from the rail.

Panel conventions: header 30–34 px `--dd-ink-3`, title 13 bold caps +0.1em; sub-panels 12 px. Section captions inside the inspector use the **secondary** colour.

### 3.1 Graph editors (Voice page)

- **Filter response**: 300 × 100 viewBox well, `--dd-ink-0`. Curve 1.75 px `--dd-paper-1`. Area under the curve filled `--dd-paper-1` at 7 % opacity. Resonance reference dashed `--dd-line-2`. Cutoff marker dashed accent. Draggable 10 px accent handle at the cutoff point.
- **Envelopes**: polyline + same 7 % fill. Corner handles 9 px. Sustain line dashed `--dd-line-3`. Live position line in modulation slot A colour.
- **Modulator shape**: dashed full-range shape `--dd-line-3`; solid "reaching destination" shape in slot colour with a 10 % fill down to zero (or centre line for bipolar).

## 4. Typography

- UI: `--font-ui` Barlow Semi Condensed. Values: `--font-value` JetBrains Mono (every number). Wordmark/titles: `--font-display` Barlow Bold lowercase.
- Minimum 11 px (10 px is used only for rail labels, rail group headings, pad note badges and keyboard letter labels; flag in review if the DS minimum must be held).
- Caps labels 12 px +0.06em; section captions 12 bold +0.1em; panel titles 13 bold +0.1em.

## 5. Play drawer

Bottom drawer present on every editor page.

- **Bar** 28 px `--dd-ink-3`: chevron + "PLAY" (secondary colour) toggles open/closed; one-line summary when closed ("Macros · pads C2–A♯6 · keys C2"); right side: section toggles (Macros, Pads [tap icon], Keyboard), divider, PC-keys toggle, octave − / range label / +.
- **Top edge** is a 7 px resize handle (ns-resize cursor, hover highlight `--dd-line-3`).
- **Heights**: standard 284 (Def), 352 (Exp), 114 (Min). Range 120 → window height − 300 (Min: 60 → 560 − 250). Dragging below half the minimum closes the drawer. Height transition 160 ms ease-out except while dragging.
- **Body** (left → right), each section optional:
  - **Macros**: grid, columns = floor((width − 6) / (knob + 6)); default 2 columns (Def/Exp), 4 (Min). Knob `sm` (Def), `md` (Exp), `xs` (Min). Width set by a draggable divider (max 50 % of window).
  - **Pads** (`Play Pads` component): grid of `PadCell`s, scrolls vertically, lowest row at the bottom.
  - **Pads | keys divider**, draggable.
  - **Keyboard** (`Play Keys` component): full range C0–C8, horizontal scroll, custom hairline scrollbar.
- **Pad sizing**: Fit (default) chooses columns and size so every mapped note is visible. Min 60 (Def), 72 (Exp), 36 (Min); max 96 / 110 / 64. If the mapped range cannot fit at the minimum, pads stay at the minimum and the rest scroll. Manual zoom 32–160 px in 8 px steps. Pads below 72 px use the `compact` PadCell style.
- **Keyboard sizing**: Fit sets white-key width so the mapped range (rounded out to white keys) fills the visible width, range 4–48 px; manual zoom ± 4 px. Unmapped keys: white `--dd-paper-4`, black `--dd-ink-4`; mapped keys: white `--dd-paper-2`, black `--dd-ink-1`; sounding key: accent. C labels on white keys; PC-key letters when PC keys are on.

## 6. Perform view (860 × 610)

Header (same as editor, with "Open editor"), then a Play panel:

- Bar 32 px: "PLAY", spacer, PC-keys toggle, octave controls.
- Top row (flex): **Patch browser** (fills remaining width, min 200: search field + list, one-click load, favourite star) · **Pads** 4 × 4 at 88 px (min 84), shared zoom/Fit · **Macros** 2 columns × 4 rows, `md` knobs, left hairline divider.
- Bottom row: **Keyboard** full width, 148 px tall, shared component.

## 7. Patch browser (editor popover)

Opens under the Patch box. 600 × 380 (520 × 320 at Min), `--dd-ink-2`, 1 px `--dd-line-3`, radius 4, `shadow-float`.
Header 34: "PATCHES", search, count, close. Body: categories column 132 (112 Min) — All, Drums, Keys, Hybrid, Synth, User, Favourites with counts — and a table (star · Name + Loaded tag · Type · Contents · Origin), rows 28. Footer 40: Import / Export / Copy definition icon buttons, Cancel, Load (primary; disabled and labelled "Loaded" for the current patch).

## 8. Colour and themes

Role colours (unchanged from the DS): accent = selection + the one primary action; mod slots A–D always paired with glyph ● ▲ ■ ◆; host colour = DAW automation; ok / warn / error = status.

Prototype props (Tweaks):

| Prop | Options | Default |
|---|---|---|
| `surface` | ember, graphite, midnight, forest, camo, paper | graphite |
| `accent` | #E08A4E, #E0574E, #D8B24A, #5BB38A, #5B9DFF, #C77DFF | #E08A4E |
| `secondary` | #A07A58, #7A5C45, #B5653F, #8E8A5A, #6F7F8C, #8A6A86, #5F8072, #9A8F7E | #A07A58 |
| `modPalette` | standard, warm, cool, mono | warm |
| `hostColor` | #5B9DFF, #4FD1C5, #B58CFF, #FFFFFF | #5B9DFF |

Theme rules implemented in the prototype (logic class, `themeVars`):
- Each surface defines ink 0–6, line 1–3 and paper 1–4 ramps. All alias tokens (`--surface-*`, `--text-*`, `--border-*`, `--color-*`) are re-declared per theme so DS components follow the theme.
- **Paper (light) surface**: accent, host, mod slots and status colours are darkened in 5 % steps until they reach 3.2:1 (roles) / 4.5:1 (status) against the panel. Text on accent picks white or ink by contrast.
- **Secondary colour**: used for section/group captions (lightened or darkened to 4.5:1 against `--dd-ink-3`) and as a mix into knob caps (`--dd-cap`, `--dd-cap-hover`, `--dd-cap-press`) and pad faces (`--dd-pad`, `--dd-pad-hover`).

Themes beyond the DS default are a design exploration; see IMPLEMENTATION Q-7.

## 9. Interaction states (visual)

Hover: one step lighter face or text. Press: one step darker, buttons 1 px down. Focus: 2 px `--dd-paper-1` ring, 2 px gap. Selected rows: `--dd-vermilion-wash` + 1 px accent inset or 2 px left edge. Disabled: `--dd-paper-4` text, 50 % row opacity. Prepared/read-only or empty: dashed outline. Host-automated control: host tint, 400 ms fade. Drop target: 1 px `--dd-line-3` inset; hovered drop target: accent inset + wash.

## 10. Design states (30)

01 Empty · 02 Single sample · 03 Sample editing · 04 Loop editing · 05 Slice editing · 06 Transient analysis · 07 Key map · 08 Velocity layers · 09 Round robin · 10 Voice shaping · 11 Modulation · 12 Macro editing · 13 Routing · 14 Effects · 15 Missing asset · 16 Unsupported asset · 17 Loading · 18 Analysis running · 19 Analysis failed · 20 Minimum 820×560 · 21 Default 1200×800 · 22 Expanded 1600×1000 · 23 Multi-file drop · 24 Compact / Perform view · 25 Pads · performance · 26 Reload pending · 27 Diagnostics · 28 Overview · inheritance · 29 Freeze · 30 Macros.

Three acceptance patches are selectable above the window: Expressive Kit, Multisampled Keys, Sample + Synth.
