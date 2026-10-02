Numeric field for discrete or precise values (slice index, semitones, voice count read-outs); 72×24 default, 60×22 compact.

```jsx
<NumericField label="Slice" value={3} min={1} max={8} onChange={setSlice} />
<NumericField label="Voices" value={8} readOnly />
```

- Drag up/down scrubs (4px per step, Shift 12px), click types, Enter commits, Esc cancels, ↑/↓ step (Shift ×10).
