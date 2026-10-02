# Dandrum Design System

Dandrum is an instrument platform: desktop JUCE (C++) plugins — VST3 and standalone — for musicians who work quickly and experimentally. An instrument is built from **sources** (synth engines, samples, other Dandrum patches) that are mapped across keys and velocities and **layered** — a kick can be a synthesised sub under a sampled mid-high. This design system defines one reusable JUCE UI language for all of it. **Das Sampler** (working title in the brief: *Dandrum Advanced Sampler*) is the first product view; drum machines, synthesizers and effects use the same components.

The sampler mockups are **design targets**. They do not claim that the sampler editor or in-plugin modulation assignment exists today. The current repo ships WebView demo editors (TB-303, 808 kick) over a Rust engine.

## Sources
- Product brief: `docs/sampler-juce-ui-design-prompt.md` in https://github.com/DanMarshall909/dandrum (branch `docs/sampler-juce-ui-design-prompt`).
- Reference kit content: `examples/patches/drum-kit.yaml` (notes 36/38/42, 8 voices, oldest-steal) and `docs/nomenclature.md` (patch, preset, module, control vocabulary).
- Aesthetic references named by the user (not copied): FAW Circle / Circle² (simple geometry, colourful accents), original NI Massive (structured panels, compact groupings), Bitwig (modulation clarity).
- No logo, fonts, icons or visual design existed in the repo. Everything here is new.

## Platform behaviour the UI must express
- Any compatible source can be mapped: a synth engine, a sample region, or a whole Dandrum patch. Key × velocity zones split and layer them (**KeyMap**).
- Sources that share a zone or pad layer together and all trigger on every hit. Each layer carries an inline chain of modules (filter, gain, saturation…) processed left to right, like a device chain (**LayerStack**). There is no built-in band split; a synth sub and a sampled top are shaped with their own filters.
- Routing: each layer has a send per **FX bus** and picks an **output bus**. FX busses carry their own module chain and output. Output busses map to plugin output pairs (Main 1/2, 3/4 … 15/16) for multi-out hosting (**OutputBusses**). Flow: layer → modules → sends → FX bus → output bus → plugin output.

## Sampler behaviour
- Prepared sample regions play as one-shot, gated, looped, reversed or pitched, with fades and optional loop crossfades.
- Regions map by MIDI key and velocity; velocity layers; round-robin and weighted alternates; bounded voices; voice stealing; exclusive choke groups.
- Sliced breaks trigger by slice index or mapped notes.
- **Live (host) parameters:** pitch ratio, start offset, level, pan, variation (+ slice index on break patches).
- **Prepared settings:** sample files, sample map, voice limits, region / loop definitions, choke policy. Change only via patch edit + **Reload Patch**.
- Reference kit: 36 Kick · 38 Snare (soft 1–95 / hard 96–127, hard has 2 alternates) · 42 Closed Hat · 46 Open Hat (2 alternates). Hats share choke group 1. Other pads empty.

## CONTENT FUNDAMENTALS
- **Voice:** an instrument panel, not an app. Short, factual, present tense. No exclamation marks, no emoji, no jokes in UI copy.
- **Person:** labels are nouns (“Pitch ratio”, “Voice limit”). Messages address the musician implicitly or as “your DAW” — “Host automation and DAW modulation are edited in your DAW.” Never “I” or “we”.
- **Casing:** parameter labels, section headings, tabs and tags in CAPS (via tracking, not typed caps in source). Menu items, buttons and messages in sentence case: “Assign modulation…”, “Reload Patch” (product action names keep title case: Reload Patch).
- **Vocabulary:** patch, preset, module, control, source (synth, sample, patch), zone, key map, layer, module chain, bypass, send, FX bus, output bus, sample region, sample map, alternate, slice, choke group, voice limit. Never YAML, module IDs, “node”, “graph”, file paths beyond the sample filename.
- **Numbers:** mono, real minus sign (−3.0 dB), × for ratios (1.000×), en dash for ranges (vel 96–127), L/C/R for pan, note number + name with C4 = 60 (36 C2).
- **Errors:** say what failed, what still works, what to do. “hat_open_b.wav was not found. Open Hat plays alternate A only until the file is restored and the patch is reloaded.”
- **Ellipsis** only when an item opens further choice (Assign modulation…, Locate file…).

## VISUAL FOUNDATIONS
- **Palette:** warm, faded brown ramp (Ink 0–6, #130F0C → #524437) with subtly stepped panel surfaces; cream text (Paper 1 #F2E6D3). Colour is selective and role-bound: ember (faded orange #E08A4E, tokens `--dd-vermilion-*`) = selection + one primary action; teal/lilac/lime/pink = modulation slots A–D; blue = host/DAW; green/amber/red = status. Every colour role is paired with a shape, glyph, text or position.
- **Type:** Barlow Semi Condensed for UI (compact but open), Barlow Bold lowercase for the wordmark/titles, JetBrains Mono for values so live numbers never jitter. 11px absolute minimum; labels 12 caps +0.06em.
- **Spacing:** 2/4/6/8/12/16/24/32. Panels seam at 4px on the window, pad 12 inside, controls 8 apart, groups 16.
- **Backgrounds:** flat fills only. No images, textures, gradients, noise or glow. Depth comes from surface steps, recessed wells (Ink 0 + 1px inner shade) and a single small drop under caps/pads.
- **Corners:** small — 2 (fields, waveform), 4 (buttons, pads, menus), 6 (panels). No pills except Toggle tracks; no large rounded cards.
- **Borders:** 1px hairlines between panels; 1px Line 2 around controls; dashed outlines mean “prepared/read-only” or “empty”.
- **Shadows:** only `shadow-cap` (0 2 3 55%) on knob caps/pads/selected segments and `shadow-float` (0 8 20 50%) on menus/tooltips. Plus a 1px 8% white top highlight on caps.
- **Values on hover:** knobs show no numbers on the panel; hovering, focusing or dragging opens a small editable value popup (type to set), which also lists peak/trough and modulation sources.
- **Rotary knobs (signature):** plain cap with no pointer line — position is read from the value arc alone. 270° track with a bottom gap and an origin tick; arcs thin at rest, thick while changing; ≤2 thin modulation rings outside, combined modulation arc inside the cap.
- **Pads:** graphite faces with a top-lit activity strip and wash proportional to level; velocity bar, layer ticks and alternate dots along the bottom; ember ring when selected; dashed when empty; choke groups get a badge and a bracket between grouped pads.
- **Hover:** one step lighter face (Ink 4 → 5) or text Paper 3 → 2. **Press:** one step darker + 1px down shift for buttons; knob cap darkens while dragging. **Focus:** 2px Paper 1 ring with 2px gap, everywhere.
- **Motion:** functional only — pad activity decays with the sound, playback cursor moves, host tint fades over 400 ms, toggle thumb 90 ms ease-out, choke bracket flashes 180 ms. No bounces, no decorative animation.
- **Transparency/blur:** none, except low-alpha washes (assign wash, slice selection 14%, outside-region dim). No blur.
- **Imagery:** none; the waveform is the image.
- **Collapsing:** every panel collapses to its header with a summary; multi-group editors (pad details) use Rollouts like 3ds Max. Scrollbars are a 1px hairline with a small graphite knob.
- **Layout:** fixed header (patch, status, Reload Patch, size) and status bar (hints, assigning state, voices). Pads + prepared details on the left, waveform + live controls on the right. Compact layout keeps every function at 820 × 560.

## ICONOGRAPHY
- Own set: 33 stroke icons in `assets/icons/dd-icon-<name>.svg` — 24 grid, 1.5 stroke, round caps/joins, single colour #EEEBE3 recoloured at draw time. Drawn for this system because none existed; simple paths only so `Drawable::createFromSVG` loads them reliably.
- Same path data lives in `components/icons/Icon.jsx` for HTML mockups.
- Icons support text, rarely replace it: icon-only buttons always have a tooltip.
- Modulation slot glyphs (● ▲ ■ ◆) are drawn shapes, not icons or Unicode; Unicode is used only for ♯ in note names and ◂ ▸ on loop flags.
- No emoji. No icon font. No PNG icons.

## Index
- `styles.css` → `tokens/` (fonts, colors, typography, spacing, shape, sizing, base)
- `components/` — React recreations of the JUCE components (for mockups)
- `guidelines/` — foundation and modulation cards
- `ui_kits/das-sampler/` — main window, compact, modulation menu, slices, missing sample, reuse panels
- `handoff/` — `DandrumTokens.h`, `juce-implementation.md`, `component-specs.md`, `asset-manifest.md`
- `assets/icons/` — SVG icons + `manifest.json`
- `tools/` — dev bundle fallback, tweaks panel
- `SKILL.md` — agent skill entry

## Components
- Controls: **Knob**, **Slider**, **Button**, **IconButton**, **Toggle**, **Switch**, **NumericField**
- Navigation: **Tabs**, **SegmentedControl**, **ContextMenu**, **MenuButton**
- Display: **PadCell**, **KeyMap** (key × velocity zones for any source + zoomable piano), **LayerStack** (layered sources with inline module chains, sends, output; also FX busses), **OutputBusses** (multi-output routing), **WaveformPanel**, **Meter**, **ListRow**, **PropertyRow**, **ParamLabel**, **ModIndicator**, **ModGlyph**
- Feedback: **StatusMessage**, **Tooltip**, **EmptyState**
- Layout: **Panel** (collapsible), **Rollout**, **SectionHeading**
- Icons: **Icon**

### Intentional additions
- **Icon** — wrapper for the new SVG set.
- **ModGlyph / ModIndicator** — slot shape + chip needed so modulation never relies on colour.
- **PropertyRow** — the prepared-setting row that distinguishes read-only patch data from live controls.

## Open assumptions
- The five live controls are treated as patch-wide host parameters (not per pad).
- In-plugin modulation sources (Velocity, Random per hit, Mod wheel, Aftertouch) are proposals; the brief does not define them.
- Note names use C4 = 60 (36 = C2).
