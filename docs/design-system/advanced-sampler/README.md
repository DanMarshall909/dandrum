# Advanced sampler reference

This is the unchanged 9 October 2026 export of Advanced Sampler App Design.zip.
[Provenance](provenance.json) records the archive and all 74 imported file hashes.
The earlier design reference remains preserved independently.

[Advanced Sampler v3](reference/Advanced%20Sampler%20v3.dc.html) is the visual authority.
Its palettes, soft/flat finishes, subtle gradients and theme aliases supersede
the nested v2 default-theme-only recommendation for this implementation.
The nested [design guide](reference/design-reference/advanced-sampler/DESIGN.md),
[interactions](reference/design-reference/advanced-sampler/INTERACTIONS.md) and
[component guide](reference/design-reference/advanced-sampler/COMPONENTS.md)
cover the complete editor, 13 workspace views, 30 states, three acceptance patches,
play drawer, patch browser and performance view.

Include every v3 theming option: eight surfaces (aluminium, brushed, ember,
graphite, midnight, forest, camo, paper), soft/flat finish, accent and secondary
colors, four modulation palettes and host automation color. Retain Barlow,
Barlow Semi Condensed and JetBrains Mono typography and their notices.

The [shell plan](reference/Sampler%20Shell%20Plan.md) specifies functional editor
commands, undo/redo, async mock loading/analysis and observable engine-adapter
behavior. The Slint implementation must identify its preview backend and avoid
claiming a real engine capability based on simulated state.
Bundled tooling was preserved, not installed or executed during import.
