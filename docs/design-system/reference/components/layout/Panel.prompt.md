Panel groups closely related controls (Live Controls, Pad Details) and is always collapsible; Rollout is a collapsible region inside a panel (3ds Max-style); SectionHeading titles a non-collapsing group.

```jsx
<Panel title="Pad details" tag="Prepared" prepared summary="Snare · 38">
  <Rollout title="Voices & choke" prepared summary="8 voices · Oldest · no choke">…</Rollout>
  <Rollout title="Layers & alternates" defaultCollapsed summary="3 regions · 2 layers">…</Rollout>
</Panel>
```

- Panel header: chevron + caps title; click, Enter or Space toggles. Collapsed panels shrink to the 28px header (24 compact), show `summary`, and siblings take the space. Actions in the header don't toggle.
- Rollout header 22px: chevron, caps 11px title, hairline rule; collapsed shows a mono summary instead of the rule. ←/→ collapse/expand when focused.
- Scrolling bodies use the minimal scrollbar: 1px Line 2 hairline track, 4px graphite knob, Paper 3 on hover.
