repo: DanMarshall909/dandrum
branch: main

## Last sync
date: 2026-10-08T06:09:37Z

### Updated in this project
- Design handoff for Codex in design-reference/advanced-sampler/ (DESIGN, COMPONENTS, INTERACTIONS, INTEGRATION, IMPLEMENTATION, prototype, screenshots)
- Mapped design controls to drums.* public parameters, sample map fields, bridge noteOn/noteOff and existing primitives

## Screen map
| Screen | Repo files |
| --- | --- |
| Pads, Play drawer, Perform view | src/juce-plugin/SharedInstrumentUi.h, src/juce-plugin/InstrumentHostWebBridge.h, examples/patches/advanced-drum-kit.yaml |
| Mapping, Layers | examples/patches/advanced-drum-kit.yaml, docs/advanced-sampler.md |
| Modulation, Voice | src/rust-engine/src/builtins/module_types.rs, openspec/specs/control-primitives/spec.md |
| Patch browser, Reload | openspec/changes/add-yaml-editor/proposal.md, src/juce-plugin/InstrumentFileWatcher.h |
| Whole editor | openspec/changes/add-renderer-independent-plugin-ui/proposal.md, docs/nomenclature.md |

## Sync history
- 2026-10-07T01:08:30Z: reviewed sampler VST3 docs, nomenclature, UI plan, design-system adaptation rules
