# Trigger Slint library recovery

This directory is the incremental, renderer-native component library for the Trigger instrument. The branch was based on `feat/filter-slint-design-2026-10-08` to preserve its token generator, Slint filter experiment, and CMake wiring.

## Source of truth

- `ui/design-system/tokens.json` generates `DesignTokens.slint`; never hand-edit generated tokens.
- `docs/design-system/reference/` holds the original visual design guide.
- `web/sampler/` and the Trigger React shell provide screen composition and behavioural references.
- Existing Rust engine and graph own audio truth. Slint only displays supplied state and emits user intent.

## Components recovered

| File | Public components | Contract |
| --- | --- | --- |
| `Foundations.slint` | `TriggerLabel`, `TriggerValue`, `TriggerPanel`, `TriggerSeparator` | Token-backed display/layout primitives |
| `Actions.slint` | `TriggerButton`, `TriggerToggle` | Input state is host-controlled; callbacks report intent |

## Delivery rules

1. Commit each independently reviewable component group.
2. Avoid duplicating the Rust sampler graph, effect identifiers, or modulation ownership in the UI.
3. Run Slint compilation and visual interaction tests before declaring a component verified.
4. Check generated tokens with `node scripts/generate-ui-tokens.mjs --check` and `node --test tests/js/DesignTokensTest.mjs`.
5. Keep unverified components clearly marked until native build and interaction tests pass.

## Remaining work

Controls (knob, numeric field, sliders, selector, menus), composition (tabs, rollouts, tooltips, scrollbars), Trigger performance (pads and mapping), prepared-data displays (waveform, spectral view, meters, layers, routing), component catalog, and native build/test integration.

**Verification:** These new files have been committed through GitHub but have not yet been compiled or visually tested in this session. The current environment cannot clone the repository directly.
