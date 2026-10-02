StatusMessage reports patch load state and missing assets (banner or inline in the status bar); Tooltip reveals name, value and modulation on hover/focus; EmptyState fills panels with nothing to show.

```jsx
<StatusMessage kind="error" title="1 sample missing" action={<Button size="sm">Locate…</Button>}>hat_open_b.wav — alternate 2 of Open Hat disabled</StatusMessage>
<StatusMessage inline kind="ok" title="Patch loaded">4 sounds · 7 sample regions</StatusMessage>
<Tooltip title="Pitch" value="1.122×"><ModIndicator slot="A" source="Velocity" depth={0.25} /></Tooltip>
```

- Every status pairs an icon shape with colour: ok ✓ circle, warn triangle, error ✕ circle, info i.
