Renders one Dandrum stroke icon (24-unit grid, 1.5 stroke, round caps) — use inside buttons, rows and status messages.

```jsx
<Icon name="reload" size={16} />
<Icon name="warning" color="var(--dd-warn)" />
```

- Same path data as `assets/icons/dd-icon-<name>.svg`. JUCE loads those files with `Drawable::createFromSVG` and recolours with `replaceColour(Colour(0xFFEEEBE3), newColour)`.
- Sizes: 16 (default), 14 (compact). Never below 12.
