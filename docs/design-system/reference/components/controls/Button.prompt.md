Buttons trigger one-off actions (Reload Patch, Locate file…); IconButton is the square icon-only form for toolbars.

```jsx
<Button variant="primary" icon="reload">Reload Patch</Button>
<Button size="sm">Locate…</Button>
<IconButton icon="more" label="Patch menu" variant="ghost" />
```

- Heights 28 / 24. Pressed moves 1px down. Never use Button for a latching state — use Switch.
