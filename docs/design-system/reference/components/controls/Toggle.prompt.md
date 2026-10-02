Toggle = settings-style on/off (preferences, "Show tooltips"); Switch = instrument-style latching key with an LED bar for playing modes on the panel (Reverse, Mono).

```jsx
<Toggle label="Follow selected pad" checked onChange={setFollow} />
<Switch label="Rev" on={reversed} onChange={setReversed} />
```

- Switch LED is vermilion when on, recessed when off; label also brightens so state is legible without colour.
