## Context

`add-juce-plugin-integration` establishes the Dandrum plugin as a runtime surface while this companion editor provides an authoring experience.

## V1 Scope

The YAML editor is deliberately **LLM-first**. Instruments are expected to be primarily generated and modified through AI or direct text editing. The graph exists to visualise, validate and navigate the YAML rather than replace it.

The editor consists of:

- YAML text editor.
- Read-only visual graph synchronised with the YAML.
- Validation and diagnostics.
- Safe save/reload through the existing plugin file watcher.

### Graph interaction

- Clicking a module or connection selects the corresponding YAML.
- Selecting YAML highlights the corresponding graph element.
- Hover synchronises between graph and YAML.
- The last valid graph remains visible while invalid YAML is being edited.
- Each cable receives a unique colour-pair identity (similar to electrical cable striping) to make tracing connections easy in dense graphs.

## Deferred

The following are intentionally out of scope for v1:

- Visual graph editing.
- Toolbox.
- Cable drawing.
- Context menus.
- Compound editing.
- AI graph transforms.

These will be introduced once the text-first workflow is mature.