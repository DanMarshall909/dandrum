Context menu — the single entry point for modulation assignment on live controls (right-click, Shift+F10, Menu key or M), also used for patch and dropdown menus.

```jsx
<ContextMenu title="Pitch" items={[
  { header: 'Modulation' },
  { type: 'assignment', slot: 'A', source: 'Velocity', depth: 0.25, onDepth, onRemove },
  { label: 'Assign modulation…', icon: 'modulate', onSelect: startAssign },
  { separator: true },
  { label: 'Reset to default', icon: 'reset', shortcut: 'Dbl-click' },
  { label: 'Type value…', shortcut: 'Enter' },
]} />
```

- Assignment rows: ←/→ change depth (Shift fine), double-click the depth bar to zero, Delete removes.
- Host (DAW) automation is never listed as an assignment — show it as a `note` only.
