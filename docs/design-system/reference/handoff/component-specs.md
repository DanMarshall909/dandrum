# Component specifications

Logical px at 100%. "Font" lists family · weight · size. Every interactive component has a 2px Paper 1 focus ring with a 2px gap.

## Rotary knob — `dd::Knob`
- Sizes: lg 64 (hero live controls), md 48 (standard), sm 36 (compact), xs 28 (minimum, dense secondary).
- Cell: label (top) → knob → 14px strip that only holds the CLIP tag. Values are not shown on the panel.
- Value popup (default): opens on hover (closes 250 ms after the pointer leaves knob + popup), keyboard focus, drag, or key/wheel nudge. Anchored under the knob with a 4px arrow; Ink 0, Line 3 border, radius 4, padding 6, min-w max(104, knob + 32), float shadow. Rows: editable value field (h 24, Ink 2 well, Mono 13 + unit; click or Enter to type, Enter commits, Esc cancels; accepts units) · ▴peak ▾trough (Mono 10, click resets) + CLIP tag · hairline · one row per modulation (glyph · source · depth %) · “Host automation” note when applicable. `valueDisplay="always"` keeps an inline read-out for dense read-only views.
- (Inline variant) label (top) → knob → value. Padding 6 (4 at sm). Gap 5 (3). Min cell width = knob + 12. Hit area = whole cell.
- Geometry (md): mod band 6, track r 16, cap r 12.5, inner combined-mod arc r = cap − 3 stroke 2. No pointer line: the value arc is the only position indicator.
- Stroke weight follows activity, and only for the element being changed: everything is thin at rest (track + value arc 1.5, 1.25 sm; mod rings 1). The value arc alone goes 3 (4 lg, 2.5 sm) while the base value changes (drag, 600 ms after key/wheel input, host write). A mod ring alone goes 2 while its source is live or its depth is edited in the menu. The background track never thickens. Radii are computed from the thick widths so nothing moves.
- Peak / trough: track the highest and lowest effective value since reset (editor-side, reset on click, patch load, or when all assignments are removed). Draw 1.25px Paper 1 ticks on the inner arc — outward from the arc for peak, inward for trough, length 3 (2.5 sm). Numbers appear in the value popup: Mono 10 Paper 3, "▴<peak> ▾<trough>" with the same formatter as the value; click resets.
- Clip warning: if value + Σ positive depths > max (or value + Σ negative depths < min), draw a circle (track width/2 + 1.5) at that end of the track — hollow 1.5px error red when the range merely exceeds; solid error red while the effective value is pinned at the limit. A 14px "CLIP" tag (UI 700 10, triangle glyph) follows the value: outlined vs solid on the same rule.
- Combined modulation inside the cap: faint (30%) arc spanning value + Σ negative depths → value + Σ positive depths; bright arc base → effective (value + Σ depth × source); dot at effective. One assignment = slot colour; several = Paper 1.
- Font: label UI 600 12 caps +0.06em Paper 2 (11 at sm); value Mono 500 13 Paper 1 (15 at lg, 11 at sm); unit Paper 3.
- Value formatting: ratios 3 decimals + ×; dB 1 decimal with U+2212; pan L50…C…R50; percent integer; ms integer.
- States: default · hover (cap #4A3D31) · pressed/dragging (cap Ink 6) · focused · disabled · automated (host blue) · modulated (rings) · assigning.
- Interaction: vertical drag 200px full range, Shift fine; double-click reset; wheel step; arrows ±5% / ±1%; Home/End; Delete reset; Enter type; right-click / Shift+F10 / Menu / M → modulation menu.
- Use for every continuous live parameter. Bipolar origin at 12 o'clock for pan, pitch, ±amounts.

## Linear slider — `dd::LinearSlider`
- Horizontal 160 × 24 (compact 112 × 20, min 80); vertical 24 × 120 (20 × 88). Track 4, thumb 12 × 16, radius 2.
- Label row above (horizontal) with value right-aligned; vertical shows value below.
- Mod bars 2px at ±6.5px from track centre; live position = 3px tick.
- Use for levels, sends, crossfades, and wherever a position reads better than rotation.

## Button / IconButton
- Button h 28, padding 0 12, gap 6, icon 16; sm h 24, padding 0 8, icon 14. Radius 4. Font UI 600 13 (12 sm), +0.02em.
- Primary: vermilion fill, ink-on-accent text — only one per view (Reload Patch when stale). Secondary: Ink 4 + Line 2 border + 1px top highlight. Ghost: transparent.
- Hover lighter face; pressed Ink 6 / vermilion-lo + 1px down; disabled Ink 3 + Paper 4.
- IconButton 28 (24). Always has a tooltip label.

## Toggle & Switch
- Toggle 32 × 18 pill, thumb 12; compact 28 × 16. Label right, UI 500 13. Settings and preferences.
- Switch (instrument key) h 28, min-w 64, LED bar 3 × 12; compact h 24. Label caps 12. Latching performance modes. Selected = vermilion LED + Paper 1 label.

## Numeric field
- 72 × 24 (60 × 22 compact, 48 min). Padding 0 6. Radius 2. Well Ink 0 + inset. Mono 13 value, unit UI 11 Paper 3.
- Drag up/down 4px per step (Shift 12px); click / Enter types; ↑/↓ step (Shift ×10); Esc cancel.
- Prepared variant: dashed Line 2 outline, transparent, Paper 2 text, no interaction.

## Tabs
- h 32 (26 compact); item padding 0 12 (0 8). UI 600 13 caps +0.10em. Selected Paper 1 + 2px vermilion underbar inset 12; idle Paper 3; hover Paper 2.
- Optional badge (10px caps, 1px Line 3 border) scopes a tab — e.g. “Slices · Break patch”.

## Segmented control
- Well Ink 0, padding 2, gap 2, radius 4; segment h 24 (22), min-w 40 (32), radius 3. UI 600 12 caps.
- Selected: Ink 5 face + cap shadow + Paper 1. ≤5 options, ≤8 chars each.

## Menus — `PopupMenu` / ModulationMenu
- Width 248, padding 4 0, radius 4, Ink 3, Line 3 border, float shadow. Title row 11 caps Paper 3 + hairline.
- Item h 26, inset 4, padding 0 10, icon column 16, gap 8. UI 500 13. Shortcut Mono 11 Paper 3. Active row Ink 5.
- Assignment row: glyph · source · depth % (slot colour) · remove ✕ (20 hit) over a 2px bipolar depth bar (±100%, 4 × 10 thumb).
- Host-automation note: UI 11 Paper 3, never interactive.

## Pad cell — `dd::PadComponent`
- 76 (52 compact, 44 min); gap 6 (5); padding 7 (5); radius 4; Ink 4 face; Line 2 border; cap shadow.
- Top: note number Mono 11 Paper 2 + name Mono 11 Paper 3 (C4 = 60); choke badge right. Middle: sound name UI 600 13 (12). Bottom: 3px velocity bar + layer ticks (6 × 2) + alternate dots (4).
- Activity: Paper 1 wash at 16% × level + 3px top strip at level; decays over the region length.
- Selected: vermilion 1px border + 1px inset. Empty: dashed Line 2, “Empty” 11 Paper 4, selectable but silent. Choked: name struck through for 500 ms; choke bracket flashes Paper 1 for 180 ms.
- Click height sets velocity (top loud). Space/Enter plays at 0.8.

## Waveform panel
- Height 250 (150 compact). Well Ink 0, radius 2. Flag strip 18 at top. Region fill #E6D6BE, outside 28% #9E8F7D + 45% Ink 0 dim.
- Flags h 16 Mono 10: START / END (region), ◂L / L▸ (loop, dashed line, 6px bar at bottom, hatched crossfade), numbered slices (selected = ember flag, 2px line, 14% wash). Slices may carry a patch-defined name: flag reads number (Mono 10) + name (UI 600 11, Paper 1); unselected flags clip to the slice width with ellipsis, the selected flag shows the full name above neighbours; compact shows numbers only. The same name appears on the slice's pad and in Pad details.
- Display: Wave (default) or Spectral, toggled by a 14px two-segment control at the bottom-right of the well (6px inset); choice persists per editor (UI state, not a host parameter). Spectral = log-frequency spectrogram 20 Hz–20 kHz, FFT 1024 / hop 256, colour ramp Ink 0 → Ink 5 → Ember-lo → Ember → Paper 1 over −44…0 dB; outside-region columns at 45% intensity; frequency labels Mono 10 Paper 3 at left. All overlays identical in both modes.
- Fades: 1.5px Paper 1 ramp line + 55% Ink 0 wedge. Start offset: 2px line + 6px triangle (host blue when automated). Cursor: 1px Paper 1 with 1px dark halo.

## Meter
- 6 × 96 per channel, gap 2, scale −60…+3 dBFS. Green < −12, amber −12…0, red > 0, 1px 0 dB tick, 2px peak hold, 4px clip LED (click to reset).

## Rows
- ListRow h 26 (22), padding 0 8 0 10, gap 8; active dot 6; selected = Ink 4 + 2px vermilion edge bar. UI 500 13; value Mono 11.
- PropertyRow min-h 22 (20); label min-w 92 (64) Paper 3; value Mono 13 (11). Prepared: Paper 2, dotted underline, 12px lock.

## Status message
- Banner: padding 8 10, radius 4, status wash + 1px status border; icon 16; title UI 600 13 Paper 1; body Paper 2; optional action right.
- Inline (header/status bar): icon 14 + title + text, no box.
- ok ✓-circle · warn triangle · error ✕-circle · info i-circle · busy reload icon.

## Tooltip
- Ink 0, Line 3 border, radius 4, padding 6 8, max-w 240, float shadow. Title UI 600 13 + value Mono 13; body UI 12 Paper 2; mod chips (h 18).
- 500 ms hover delay; immediate on keyboard focus; hidden while dragging (value moves into the tooltip slot instead).

## Panel & section heading
- Panel radius 6, Ink 2, 1px Line 1; header h 28 (24) Ink 3, padding 0 12 0 8 (0 8 0 6); chevron 12 (right = collapsed, down = open, 90 ms rotate); heading UI 700 13 caps +0.10em Paper 2; body padding 12 (8); 4px seams between panels.
- **All panels collapse.** Click header / Enter / Space. Collapsed = header only, hairline removed, mono 11 Paper 3 summary after the title, siblings reflow into the freed space. Header actions don't toggle. Persist collapsed state per panel in the editor's ValueTree (not a host parameter).
- Prepared panels add a lock glyph and a “Prepared” tag.

## Rollout (collapsible region)
- Header h 22 (20), bleeds 4px either side, radius 2; chevron 12 · title UI 700 11 caps +0.10em Paper 2 · 1px Line 1 rule filling the row. Hover Ink 3.
- Collapsed: rule replaced by a one-line mono 11 Paper 3 summary of the key values (e.g. “8 voices · Oldest · no choke”).
- Body indented 18 (aligns with title text), padding 4 0 8 (2 0 6).
- Keyboard: Enter/Space toggle, ← collapse, → expand. Use for every multi-group editor (pad details, future synth/effect editors); compact layout starts secondary rollouts collapsed.

## Scrollbar
- Thickness 6 hit area; visible track = 1px Line 2 hairline centred; thumb 4px wide, radius 3, Ink 6, min length 24; hover/drag Paper 3. No arrows, no corner box. Only appears when content overflows.
- JUCE: `LookAndFeel::drawScrollbar` + `getDefaultScrollbarWidth() → 6`; `ScrollBar::setAutoHide(true)`.

## Empty state
- Dashed Line 2, radius 4, padding 24 (12), icon 20, title UI 600 13 Paper 2, body 12 Paper 3, max-w 280, one optional action.
