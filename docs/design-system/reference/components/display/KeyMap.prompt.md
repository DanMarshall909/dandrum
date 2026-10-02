Key × velocity map with an aligned, zoomable piano keyboard. Splits and layers anything Dandrum can play: sample regions inside a patch, or whole compatible patches (synths, drum kits, other samplers).

```jsx
<KeyMap lowNote={24} highNote={96} zones={[
  { id: 'kick', name: '808 Kick', source: 'Patch', lo: 24, hi: 35, velLo: 1, velHi: 127, root: 24 },
  { id: 'bass', name: 'TB-303 Bass', source: 'Patch', lo: 36, hi: 59, velLo: 1, velHi: 127, root: 48 },
  { id: 'strs', name: 'Strings soft', source: 'Sample', lo: 60, hi: 84, velLo: 1, velHi: 95, root: 60 },
]} onChange={(zones) => setZones(zones)} />
<KeyMap editable={false} lowNote={33} highNote={50} zones={kitZones} />
```

- X = MIDI key (one column per semitone, black-key columns darker, octave lines at C). Y = velocity, 127 at top.
- Overlapping zones play together (layers). Identical zones draw as an offset stack; a LAYERED row lists every zone under the selection — click to select.
- `source` names the target type and shows as a tag; the component does not care what the target is.
- Zoom: − / + / Fit in the header, Ctrl/Cmd + wheel at the pointer. Velocity axis stays pinned while scrolling.
- Selected zone: ember outline + ember rail over its keys + tinted keys; root key gets a small square.
- Drag body = move (root moves with it), edges/corners = resize; snaps to whole keys and velocity steps.
- Keyboard: click plays at velocity from click depth; matching zones flash and decay over 400 ms.
- Use `editable={false}` (dashed zones, PREPARED tag) when the map is a prepared setting that changes only via patch edit + Reload Patch.
