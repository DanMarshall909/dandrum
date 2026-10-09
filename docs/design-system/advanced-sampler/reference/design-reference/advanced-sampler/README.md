# Advanced Sampler (Trigger) — design reference

Design handoff for the Dandrum Advanced Sampler editor ("Trigger"). Reference material only. Nothing in this folder is part of the build.

| File | Contents |
|---|---|
| `DESIGN.md` | Window sizes, layout grid, typography, colour, themes, responsive rules, visual states |
| `COMPONENTS.md` | Component hierarchy, design-system components used, prototype-only components |
| `INTERACTIONS.md` | Every interactive control: action, result, keyboard, open questions |
| `INTEGRATION.md` | Mapping from design controls to existing Dandrum parameters, primitives, bridge functions; mismatches |
| `IMPLEMENTATION.md` | Proposed vertical slices, dependencies, acceptance criteria, unresolved questions |
| `prototype/` | Original Claude Design source (Design Components) and the design-system bundle it loads |
| `screenshots/` | Reference captures of 20 design states (top 540 px of the preview; see note below) |

## Source of truth

- **Primary prototype:** `prototype/Advanced Sampler v2.dc.html` (page rail, Play drawer, patch browser, Perform view). Shared children: `prototype/Play Pads.dc.html`, `prototype/Play Keys.dc.html`.
- **Comparison prototype:** `prototype/Advanced Sampler v1.dc.html` (top tab bar, earlier layout). Keep only for comparison; v2 supersedes it where they differ.
- **Design system:** `docs/design-system/` in the repo (adaptation rules in its README override the export). The copy under `prototype/_ds/` is the exact bundle the prototype loads, including additions made during this design pass (see `COMPONENTS.md` § 4).
- **Repo baseline inspected:** `DanMarshall909/dandrum@main`, commit `d02ee1b`, 2026-10-08.

## Opening the prototype

Open `prototype/Advanced Sampler v2.dc.html` in a browser. It needs `support.js` and `_ds/…` beside it (already in place). The bar above the window ("Design states") jumps to each of the 30 states; the window itself is fully clickable. Tweaks (theme, accent, secondary, modulation palette, host colour) are props on the root component.

## Notes on the earlier planning documents

`prototype/notes/Codex Implementation Plan (superseded).md` was written for v1 against an HTML-app architecture with its own store. It is retained for its detailed layout measurements only. Where it conflicts with this handoff or with the repo's `add-renderer-independent-plugin-ui` change, this handoff and the repo win. `prototype/notes/Trigger Repo Reconciliation.md` lists repo conflicts found on 2026-10-07; they are restated and updated in `INTEGRATION.md`.

## Screenshot limitation

The captures cover the visible part of the preview (state picker plus the top of the window). Use the prototype for full-window comparison; the screenshots are an index, not a pixel reference.
