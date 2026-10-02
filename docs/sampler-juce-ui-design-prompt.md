# Dandrum Sampler and JUCE UI Design Prompt

Copy the prompt below into Claude Design. It asks for a reusable JUCE design system and a sampler mockup built from that system. The mockup is a design target; it is not a statement that the sampler VST3 interface or in-plugin modulation assignment already exists.

---

Design **Das Sampler** (formerly Dandrum Advanced Sampler), a desktop **JUCE C++ VST3 instrument**, and a reusable **Dandrum JUCE UI design system** that can serve later drum machine, synthesizer, and effects plugins. Produce an original visual language. You may take inspiration from Bitwig's clear, immediate approach to modulation, but do not copy its visual design, icons, layout, or branding.

## Current design reference

Use the reviewed [Dandrum design system](design-system/README.md) and its
[component references](design-system/reference/readme.md) as the visual baseline.
Call the first product view **Das Sampler**. Use warm brown flat surfaces, cream
text and ember selection/action accents; Barlow Semi Condensed labels, Barlow
titles and JetBrains Mono values; pointer-free knobs with a 270-degree value arc
and editable values on hover, focus or drag. Panels collapse to summaries, and
prepared-detail groups use rollouts. Preserve the 1200 x 800 and 820 x 560 layouts,
visible keyboard focus and restrained playback feedback.

The export is illustrative. Use actual prepared metadata, shared and per-pad host
parameters, and the maintained kit's snare velocities **1–63 / 64–127**. Apply the
maintained guide's adaptation rules when the exported mock data or original JUCE
handoff disagrees with these requirements.

## Product and sampler behavior

The sampler is intended for playable drum kits, chopped breaks, and modest chromatic instruments. Its engine prepares sample assets and patch structure before playback. The design should make these musical capabilities understandable:

- Play prepared sample regions as one-shots, gated sounds, simple loops, reversed regions, or pitched sounds, with region fades and optional loop crossfades.
- Map sample regions by MIDI key and velocity. Support velocity layers (soft snare 1–63, hard snare 64–127), deterministic round-robin and weighted alternates, bounded voices, voice stealing, and exclusive choke groups.
- Trigger explicit slices of a prepared break using a slice index or mapped MIDI notes.
- Expose **pitch ratio, start offset, level, pan, and variation** as prominent live controls. These public parameters should be easy for a DAW to automate or modulate externally. Show their actual shared or per-pad scope. Variation chooses among compatible hit alternates without changing the prepared sample map.
- Keep sample source files, sample maps, voice limits, region and loop definitions, and choke policy as prepared settings. Changing these requires an explicit patch reload; do not portray them as continuously automatable controls.

Use the reference drum kit as concrete content. MIDI note **36** plays kick, **38** plays snare with soft and hard velocity layers, **42** plays closed hat, and **46** plays open hat. The hard snare and open hat have alternates; the hats share a choke group. Show these four mapped sounds within a 4 × 4 pad grid, with the remaining pads clearly empty or available for future mappings. Do not invent additional loaded samples.

Use user-facing names such as **patch**, **preset**, **module**, **control**, **sample region**, and **sample map**. Do not expose YAML, internal module IDs, or engine implementation details in the musician-facing screen.

## Main plugin mockup

Create a high-fidelity main window around **1200 × 800 px**, plus one compact layout demonstrating how it remains usable at a smaller plugin size. Include:

1. A 4 × 4 playable pad grid with note labels, selected-pad state, velocity response, alternate activity, and a visible closed/open hat choke relationship.
2. A large waveform for the selected sample region. Show a playback cursor, prepared region bounds, fades, loop points where relevant, and explicit slice markers in a Slices view. Markers are visualizations of prepared data, not an in-plugin waveform editor.
3. A clear live-control area for pitch ratio, start offset, level, pan, and variation, with current values and subtle feedback when the host changes a parameter.
4. Selected-pad details showing key and velocity ranges, layers, alternates, region gain/pan/pitch, voice limit, stealing policy, and choke behavior. Distinguish read-only prepared settings from live controls.
5. A Slices tab or alternate view with a numeric slice-index control and a clear selected-slice state. Make it clear when this view belongs to a sliced-break patch rather than implying the reference drum kit contains a break sample.
6. Patch load status, missing-asset feedback, and an explicit **Reload Patch** action.

Provide a second mockup state with a modulation context menu open on a live control. Show how the same component language could be reused on a synthesizer or effects plugin without designing those complete products.

## Key maps, layers and outputs

Use the reusable **KeyMap**, **LayerStack** and **OutputBusses** components where
prepared metadata and host capabilities support them. Show key/velocity zones,
source identity, declared selection or layering semantics, module chains, sends
and actual named output buses with their channel counts. An overlapping zone or
round-robin alternate does not by itself imply simultaneous layering. Show
unavailable capabilities explicitly rather than inventing synth/patch layers or
fixed output pairs.

Keep zone bounds, module order and structural routing read-only in the plugin.
Selection, audition and supported public parameter changes remain available.
Do not expose the reference components' add/remove/reorder/reroute interactions
as plugin authoring; structural changes require external patch editing and reload.

## Implicit modulation interaction

Most eligible live controls should support modulation without permanent assignment buttons, exposed routing tables, or a modulation matrix dominating the screen. **Right-click a control to assign modulation.** Design the context menu with **Assign modulation…**, existing assignments, depth controls, and removal. While assigning, subtly highlight eligible destinations. Once assigned, indicate the modulation range directly on the control: for example, an arc around a knob or a range overlay on a slider. Reveal source and depth on hover, focus, or in the context menu. Include a keyboard-accessible way to open that same menu.

Specify these states for every modulatable control: unassigned, assignment in progress, assigned but idle, actively modulating, host-automated, focused, and disabled. Keep modulation feedback restrained so controls remain readable when many parameters are moving. Show how multiple assignments appear without turning the control into a dense graphic.

Distinguish the **design for in-plugin right-click assignment** from **DAW-owned external modulation and automation**. The plugin can expose public host parameters, but it cannot assume it can inspect or manage every modulation source owned by the DAW. Do not claim that the right-click assignment workflow is already implemented.

## Reusable JUCE component system

Create a component sheet and usage rules for:

- Rotary knobs, horizontal and vertical sliders, switches, toggles, buttons, icon buttons, tabs, segmented controls, menus, and numeric fields.
- Pad cells, lists and rows, waveform panels, meters, status messages, tooltips, and modulation indicators.
- Parameter labels and values, section headings, empty states, selected states, and focus indicators.

For each component, specify recommended dimensions, minimum size, internal padding, typography, alignment, hit area, value formatting, and states: default, hover, pressed, selected, focused, disabled, automated, and modulated where applicable. Include compact variants and explain when to use each component. Show consistent interaction for mouse drag, double-click reset, text entry, keyboard control, context menus, and tooltips where applicable.

Provide a design-token sheet with exact hex colors, typography using available or freely redistributable fonts, type sizes and weights, spacing scale, corner radii, stroke widths, shadows if any, and component sizing. Include contrast guidance and demonstrate how the tokens work at common display scaling factors. Keep the overall interface legible at smaller plugin-window sizes.

## JUCE implementation and asset handoff

Design for a JUCE plugin editor, where live values and visual feedback change during playback. Deliver:

1. The main sampler mockup, compact variant, modulation context-menu state, and reusable component sheet.
2. A token table and component specifications precise enough to implement in C++.
3. A suggested mapping between shared JUCE `LookAndFeel` drawing, reusable custom `Component` classes, and static assets. Parameter-bound controls must preserve host gesture semantics through the shared command service and timer-observed updates; audio-originated notifications must not post messages.
4. An asset manifest with descriptive filenames, intended pixel or vector dimensions, states, and where each asset is used.
5. Individually exportable **SVG icons** with simple paths JUCE can load reliably. Supply transparent PNG assets at **1× and 2×** only where raster imagery is genuinely needed. Use consistent view boxes and name assets systematically.
6. A short implementation guide describing what JUCE should draw dynamically: waveforms, playback cursor, markers, values, labels, knob and slider positions, pad activity, meters, focus, and modulation ranges. Do not bake changing content into image assets.

Obtain redistributable font binaries and license notices for local packaging; the reference export includes neither. Treat its CDN scripts, Google Fonts and runtime Babel as preview dependencies. Production WebView assets must be compiled ahead of time and packaged locally.

Prefer scalable vector icons and JUCE-drawn controls over large background images. Avoid embedded fonts, CSS dependencies, complex SVG filters, proprietary imagery, and image assets that contain parameter values or labels. Keep the design feasible for efficient real-time visual updates in a plugin editor. Provide assets in a form that can be handed directly to a JUCE developer, not only as a flattened mockup.

Do not add a step sequencer, granular engine, time stretching, host-synced warping, workstation-sampler articulations, or a full in-plugin sample editor. Keep the design focused on the sampler behavior described above and the reusable JUCE UI system.
