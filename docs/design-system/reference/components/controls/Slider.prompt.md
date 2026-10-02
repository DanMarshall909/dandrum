Linear slider for parameters that read better as a position than a rotation (levels, crossfades, mixer-style strips); prefer Knob in dense instrument panels.

```jsx
<Slider label="Level" value={0.8} valueText="−1.9 dB" modulations={[{ slot: 'B', depth: -0.15 }]} />
<Slider orientation="vertical" label="Send" value={0.4} valueText="−8.0" />
```

- Modulation ranges draw as 2px bars beside the track (slot 1 above/left, slot 2 below/right).
- Same mouse/keyboard model as Knob; click on the track jumps.
