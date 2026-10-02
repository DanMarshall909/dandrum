Rotary knob for any continuous live parameter — the primary Dandrum control; use lg (64) for hero controls, md (48) for standard, sm (36) in compact layouts.

```jsx
<Knob size="lg" label="Pitch" bipolar value={0.5} valueText="1.000×" modulations={[{ slot: 'A', depth: 0.2, live: 0.6, source: 'Velocity' }]} onContextMenu={openMenu} />
```

- Drag vertically (200px = full range, Shift = 4× finer), double-click resets, wheel steps, arrows ±5% (Shift ±1%), Delete resets, Shift+F10 / Menu / M opens the modulation menu.
- States: unassigned · `assigning` (dashed teal ring + wash) · assigned idle (ring 60%) · active (`live` dot) · `hostAutomated` (blue arc + value) · `focused` · `disabled`.
- Everything is thin at rest (1.5 / rings 1). Only the thing being changed thickens: the value arc (3–4) while the base value moves (drag, key/wheel nudge for 600 ms, host write); a single mod ring (2) while its source is live or its depth is edited (`activeSlot`). The background track never thickens.
- Inside the cap, a combined-modulation arc shows all assignments together: faint = total range, bright = base → current effective value, dot = effective value. Single source uses its slot colour; several use Paper 1.
- No pointer line: the value arc is the only position indicator.
- Peak / trough: small Paper 1 ticks on the inner combined arc (outward = peak, inward = trough) plus a "▴ 82 ▾ 31" mono 10 read-out under the value; click it to reset. Pass `peakText`/`troughText` for formatted values.
- Clip warning: when value + modulation range passes the parameter limit, a hollow error-red cap sits on that end of the track and a CLIP tag follows the value. When the modulated value is pinned at the limit right now, both fill solid red.
- Values are hidden on the panel by default (`valueDisplay="hover"`). Hovering, focusing, dragging or nudging opens a small popup under the knob with an editable value field (click it or press Enter to type; Enter commits, Esc cancels), peak/trough (click to reset), the CLIP tag and the modulation list. The CLIP tag also stays on the panel because it is a warning. Use `valueDisplay="always"` for read-outs that must stay visible.
