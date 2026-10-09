## Why

The updated advanced sampler guide specifies a complete editing and performance UI with v3 materials and palettes. The existing Slint catalog and performance strip do not implement that editor. Import the source faithfully and build a functional native Slint version with screenshot evidence against the actual reference.

## What Changes

- Preserve the 74-file updated guide with archive/per-file provenance and retain the existing reference.
- Add the complete native Trigger sampler editor: all 13 workspace views, 30 design states, three acceptance patches, adaptive editor sizes, play drawer and performance view.
- Implement all editing commands, command-based undo/redo, asset/analysis workflows, navigation, patch browsing, performance input and diagnostic/event views through a mockable native adapter.
- Include every v3 surface, soft/flat finish, accent/secondary color, modulation palette and host color, including subtle gradients, bevels, shadows and retained fonts.
- Register a maintained silent native demo, native model/binding tests, optional headless runtime verification and screenshot comparisons with at most three refinement cycles after functional completion.

## Capabilities

### New Capabilities

- `advanced-sampler-slint`: Complete responsive native advanced sampler editor, theming, command adapter, performance input, operational states and visual verification.

### Modified Capabilities

None. The editor preview does not add Rust engine/DSP capabilities or alter existing production editors.

## Impact

New native Slint application and adapter under ui/advanced-sampler, guide import under docs/design-system/advanced-sampler, CMake/demo registration, focused C++ and Slint runtime tests, feature/visual audit artifacts. Reuse existing licensed fonts/icons and component contracts. Work remains on the primary main checkout per explicit instruction; preserve other worktrees and caches.
