Pad cell for 4×4 (or larger) playable grids in samplers and drum machines.

```jsx
<PadCell note={38} name="Snare" layers={2} activeLayer={1} alternates={2} activeAlternate={0} velocity={0.9} level={0.7} selected />
<PadCell note={39} mapped={false} />
```

- Click height sets velocity (top = loud). Space/Enter plays at 0.8. Empty pads are dashed and only selectable.
- Layer ticks (stacked bars) and alternate dots are prepared-data indicators; the filled one is what just played.
