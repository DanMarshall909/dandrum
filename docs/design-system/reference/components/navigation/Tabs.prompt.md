Tabs switch between views of the same panel (Pads / Slices); SegmentedControl picks one of a few short options inside a panel (Mode: Pads · Keys, 1× / 2×).

```jsx
<Tabs items={[{ id: 'pads', label: 'Pads' }, { id: 'slices', label: 'Slices', badge: 'Break' }]} value="pads" onChange={setView} />
<SegmentedControl options={['Wave', 'Map']} value="Wave" />
```

- Tabs: selected = primary text + 2px vermilion underbar; ←/→ move between tabs.
- Use SegmentedControl for ≤5 options of ≤8 characters; otherwise a MenuButton.
