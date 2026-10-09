## Context

The integrated reusable Slint library has tested interaction contracts and retained fonts/icons; the advanced v3 reference adds an entire editor and dynamic materials. See proposal.md and the imported guide. The primary checkout remains main by explicit instruction. Existing production audio editors and Rust DSP remain independent.

## Goals / Non-Goals

**Goals:** Preserve source provenance, implement every guide control with observable command behavior, and verify faithful native rendering across pages, states, sizes and theme choices. Components remain small and composable.

**Non-Goals:** Add unrequested DSP primitives or imply a real engine connection from simulated telemetry. This silent editor uses the guide's mockable adapter contract, making real-engine integration a later interchangeable adapter rather than a fake capability.

## Decisions

1. Root v3 HTML is the visual authority, including its aluminium default, brushed material and soft/flat finish. Nested v2 recommendations to defer non-default palettes are superseded by explicit user instructions. Import source unchanged and keep derived runtime artifacts separate.
2. Use Slint 1.18.1 with the native Skia software renderer for gradients/shadows/paths and embedded retained fonts. Reuse behavioral primitives and prepared display contracts; themed faces use app-owned semantic roles so palette changes reach the entire UI.
3. Native C++ domain store dispatches commands through a silent EngineAdapter. Command entries carry reversible operations; gesture updates coalesce into one committed edit. Native mock loading/analysis has generation-tagged jobs, bounded telemetry and explicit failures, independent of audio callbacks. GUI components consume typed models and emit callbacks only.
4. Shared Session global exposes typed model properties and command(action,target,text,value), parameter-gesture(id,phase,value), source-owned note-on/note-off/release-notes and guarded PC-key callbacks. Pure C++ model tests do not require Slint; generated binding/input tests prove GUI wiring separately. Viewer defaults are capture fixtures only.
5. ThemeSettings resolves all v3 semantic colors using independently extracted golden cases; SamplerTheme global receives the complete palette. Components use soft/light/brushed flags to compose gradients and highlights. Expose every theme setting and preserve it across layout/preset changes.
6. Preserve responsive geometry rather than scaling a bitmap. AppWindow composes header/tree/rail/workspace/inspector, shared performance views, patch/settings/value/context overlays and status/log/diagnostics surfaces. Native settings and clipboard/file operations complete editor workflows.
7. Test behavior first, then production changes. Retain a feature audit linking all controls to commands and direct tests. Compare actual compiled-app screenshots to the 39 preserved reference captures; track each comparison cycle, with at most three refinement cycles after the full functional baseline is implemented.

## Risks / Trade-offs

- Reference versions differ → retain all source bytes and identify v3 precedence explicitly.
- Dynamic themes could leave old hard-coded colors → resolve and test every semantic role and inspect all eight surfaces in native screenshots.
- Mock behaviors could be mistaken for real engine support → identify the silent preview backend and expose capabilities/events.
- Geometry changes could clip controls at minimum size → bind measured dimensions and exercise all sizes with actual input/render checks.
- Shared main editing could overlap → assign exclusive source paths to agents, inspect all worktrees before mutation and stage only finished logical changes.

## Migration Plan

Add the independent maintained sampler-slint demo; retain existing sampler/filter/library demos and all source branches/worktrees. Commit verified guide import, model/theme/application changes separately. Publish main normally after relevant checks. Existing editors continue to use their original runtimes.
