Waveform panel for the selected sample region — shows prepared bounds, fades, loop and slice markers plus the live start offset and playback cursor.

```jsx
<WaveformPanel kind="hat-open" regionStart={0.02} regionEnd={0.9} fadeOut={0.3} cursor={0.4} startOffset={0.05} />
<WaveformPanel kind="break" slices={[{ pos: 0, name: 'Kick' }, { pos: .125, name: 'Hat' }, { pos: .25, name: 'Snare' }, { pos: .3125 }]} selectedSlice={2} />
```

- Region inside = bright fill, outside dimmed. Flags: START / END (region), ◂L L▸ (loop), numbered (slices; selected = vermilion).
- Markers are read-only visualisations of prepared data.
- `display="spectral"` swaps the amplitude view for a log-frequency spectrogram (warm ramp Ink 0 → Ink 5 → Ember → Paper 1); all markers, fades, cursor and slices overlay identically. A Wave / Spectral toggle sits bottom-right in the well (`showDisplayToggle`).
- Slices may be named (`{ pos, name }`). Flags show "3 Snare"; unselected names truncate to the slice's width, the selected slice's name is always shown in full. Names are hidden in compact mode (number only) and are prepared patch data — not editable here.
