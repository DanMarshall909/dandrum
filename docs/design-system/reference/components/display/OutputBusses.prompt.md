Output busses for multi-output plugins. Each bus maps to a plugin output channel pair and shows what feeds it.

```jsx
<OutputBusses busses={[
  { id: 'main', name: 'Main', main: true, channels: '1/2', feeds: ['Snare', 'Hats', 'Reverb'], levels: [-14, -15] },
  { id: 'kick', name: 'Kick', channels: '3/4', feeds: ['Kick sub', 'Kick mid-high'], levels: [-8, -8], level: -1.5 },
]} onChange={setBusses} />
```

- Channel pair opens a menu of plugin outputs (Out 1/2 … 15/16); pairs already used by another bus are noted "shared".
- "Fed by" is computed by the editor from each layer's / FX bus's `output`. Empty bus reads "Nothing routed".
- Signal flow: layer → module chain → (sends → FX bus → chain) → output bus → plugin output pair.
