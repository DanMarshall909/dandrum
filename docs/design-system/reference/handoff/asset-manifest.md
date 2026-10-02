# Asset manifest

All static assets are single-colour stroke SVGs. Everything else (controls, waveform, markers, meters, values, labels, focus, modulation) is drawn by JUCE at runtime — see `juce-implementation.md`.

## Conventions
- Folder: `assets/icons/`. Filename: `dd-icon-<name>.svg` (kebab-case).
- `viewBox="0 0 24 24"`, `fill="none"`, `stroke="#F2E6D3"`, `stroke-width="1.5"`, round caps and joins. Only `<path>`, `<circle>`, `<rect>` — no filters, masks, gradients, text, or CSS.
- Render sizes: 16 (default), 14 (compact), 12 (inline badges), 20 (empty states). Never below 12.
- Load: `auto d = juce::Drawable::createFromSVG(*juce::parseXML(BinaryData::dd_icon_reload_svg));` then `d->replaceColour(juce::Colour(0xFFF2E6D3), targetColour);` and `drawWithin(g, bounds, RectanglePlacement::centred, 1.0f)`.
- States are colours applied at draw time (Paper 1 default, Paper 2 secondary, Paper 4 disabled, status colours) — no per-state files.

## Raster assets
None required. No PNGs are shipped: there are no textures, photos or background images. If a future skin needs raster art, supply `<name>@1x.png` and `<name>@2x.png` with transparent backgrounds and no baked text or values.

## Fonts (BinaryData)
| File | Use |
|---|---|
| BarlowSemiCondensed-Medium.ttf / -SemiBold.ttf / -Bold.ttf | UI labels, body, headings |
| Barlow-Bold.ttf | Wordmark, patch title |
| JetBrainsMono-Medium.ttf / -SemiBold.ttf | Values, note numbers |

## Icons
| File | Size | States | Used in |
|---|---|---|---|
| `dd-icon-reload.svg` | 24 × 24 vector | single colour (recolour) | Reload Patch button, busy status |
| `dd-icon-warning.svg` | 24 × 24 vector | single colour (recolour) | Warn status |
| `dd-icon-error.svg` | 24 × 24 vector | single colour (recolour) | Error status |
| `dd-icon-ok.svg` | 24 × 24 vector | single colour (recolour) | OK status, checked menu items |
| `dd-icon-info.svg` | 24 × 24 vector | single colour (recolour) | Info status, tooltips |
| `dd-icon-lock.svg` | 24 × 24 vector | single colour (recolour) | Prepared settings, prepared panel header |
| `dd-icon-chevron-down.svg` | 24 × 24 vector | single colour (recolour) | Menu buttons |
| `dd-icon-chevron-right.svg` | 24 × 24 vector | single colour (recolour) | Submenu items |
| `dd-icon-chevron-left.svg` | 24 × 24 vector | single colour (recolour) | Menu Back item |
| `dd-icon-close.svg` | 24 × 24 vector | single colour (recolour) | Remove assignment, dismiss |
| `dd-icon-plus.svg` | 24 × 24 vector | single colour (recolour) | Empty pad / add |
| `dd-icon-minus.svg` | 24 × 24 vector | single colour (recolour) | Decrement |
| `dd-icon-more.svg` | 24 × 24 vector | single colour (recolour) | Overflow menus |
| `dd-icon-modulate.svg` | 24 × 24 vector | single colour (recolour) | Assign modulation… |
| `dd-icon-host.svg` | 24 × 24 vector | single colour (recolour) | Host/DAW parameter notes |
| `dd-icon-choke.svg` | 24 × 24 vector | single colour (recolour) | Choke group badge |
| `dd-icon-alternate.svg` | 24 × 24 vector | single colour (recolour) | Alternates (round-robin / weighted) |
| `dd-icon-layers.svg` | 24 × 24 vector | single colour (recolour) | Velocity layers |
| `dd-icon-reverse.svg` | 24 × 24 vector | single colour (recolour) | Reversed region tag |
| `dd-icon-loop.svg` | 24 × 24 vector | single colour (recolour) | Loop region tag |
| `dd-icon-one-shot.svg` | 24 × 24 vector | single colour (recolour) | One-shot play mode tag |
| `dd-icon-gate.svg` | 24 × 24 vector | single colour (recolour) | Gated play mode tag |
| `dd-icon-slice.svg` | 24 × 24 vector | single colour (recolour) | Slices view, slice empty state |
| `dd-icon-keyboard.svg` | 24 × 24 vector | single colour (recolour) | Keyboard/MIDI focus |
| `dd-icon-folder.svg` | 24 × 24 vector | single colour (recolour) | Locate file…, load patch |
| `dd-icon-file-missing.svg` | 24 × 24 vector | single colour (recolour) | Missing sample on pad/row |
| `dd-icon-reset.svg` | 24 × 24 vector | single colour (recolour) | Reset to default |
| `dd-icon-midi.svg` | 24 × 24 vector | single colour (recolour) | MIDI activity |
| `dd-icon-pan.svg` | 24 × 24 vector | single colour (recolour) | Pan (compact headers) |
| `dd-icon-pitch.svg` | 24 × 24 vector | single colour (recolour) | Pitch (compact headers) |
| `dd-icon-level.svg` | 24 × 24 vector | single colour (recolour) | Level (compact headers) |
| `dd-icon-variation.svg` | 24 × 24 vector | single colour (recolour) | Variation (compact headers) |
| `dd-icon-settings.svg` | 24 × 24 vector | single colour (recolour) | Settings |
