## Context

The user requested the whole Slint library from the design guide, explicitly asking for subcomponents. The maintained guide overrides illustrative export behavior; `ui/design-system/tokens.json` already governs CSS/C++. The existing Slint 1.18.1 macro experiment proves software rendering but does not provide the complete component vocabulary.

## Goals / Non-Goals

**Goals:** Implement every public guide component, keep authored component files small, expose reusable visual/input atoms, use shared assets/tokens and demonstrate real UI interaction in a catalog. Make display models and structural capabilities explicit.

**Non-Goals:** Implementing a new sampler engine, replacing the production JUCE/Web renderers, building the full Trigger application's model, or claiming real audio from catalog fixtures.

## Decisions

1. **Pure Slint library with typed contracts.** Public components import through `ui/slint/dandrum.slint`; individual component imports remain supported. Data models use stable opaque IDs and copied values. UI selection/focus/zoom/collapse is local; structural operations emit gated callbacks. This keeps the library usable from C++, Rust, or the viewer without duplicating engine state.
2. **Composition at behavioral seams.** Separate rotary geometry from range gestures and value editing; separate waveform data from coordinate overlays; separate panel headers from body content. Target at most 200 authored lines per `.slint` file, with generated tokens/icon registry exempt.
3. **One token source.** Extend the existing generator rather than manually transcribing a Slint palette. Slint lengths carry px units, em tracking remains a font-relative ratio, and native fonts select the registered first family while CSS retains fallbacks. `theme.slint` imports the retained licensed font binaries.
4. **Maintained guide takes precedence.** Pointer-free knobs, minimum 11px text, 240px menus and capability-driven prepared displays supersede conflicting illustrative details. Native built-in context-menu mechanics provide accessible right-click and keyboard entry, while custom menu content is composable where needed.
5. **Catalog and harnesses.** Controlled fixtures belong only in catalog/test code. Slint compiler checks, screenshots and actual runtime property/event assertions complement token generation and source/asset inventory tests. Test one interaction boundary per component family before implementation, then verify compositions at compact/default sizes.

## Risks / Trade-offs

- Font rasterization differs from Chromium → inspect native screenshots against the guide's geometry and roles, retaining exact fonts and vectors.
- A UI control could imply unsupported engine edits → require explicit capabilities/read-only state and test that rejected actions emit no command.
- Preview validation alone does not prove JUCE host input or audio → record viewer/native/host evidence separately; no audio claim.
- Long lists and dense controls could overflow → use bounded viewports, scrolling and compact composition without reducing text below 11px.

## Migration Plan

Additive library and catalog first. Existing consumers can adopt individual components through the public import surface. Keep the old macro spike intact so its verified native input lane remains available. Reverting the library/token-output change restores the earlier source layout without state migration.
