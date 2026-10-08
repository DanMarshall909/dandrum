## Context

The user narrowed the requested full shell to performance mode: just macros. Reference is Trigger's React `MacroStrip` and `ParameterKnob` in worktree `/tmp/dandrum-trigger-shell` at 43c01c7. Its eight 64px cells have 6px gaps, 48px rotary drawings, 270-degree arcs, 11px labels and the bundled Barlow Semi Condensed / JetBrains Mono fonts. Defaults are 56, 40, 32, 18, 64, 50, 0, 0 percent; the final two controls are disabled.

## Goals / Non-Goals

**Goals:** Match the macro strip's geometry and styling; build reusable, accessible controls; exercise the official Slint skill, rendering and preview workflow; keep authored components under 200 lines; provide `./demo trigger-slint`.

**Non-Goals:** Full sampler editor, patch browser, MIDI learning, audio, automation, persistence, performance benchmarking, or replacing production plugins. Header controls that require the full shell are excluded.

## Decisions

- Use an isolated silent JUCE GUI app hosting compiled Slint 1.18.1 through the existing proven software renderer. Keep the warm build tree; no engine changes. Separate optional build switch while reusing the existing pinned Slint dependency setup.
- Use separate UI, assets and host source directories. A theme global supplies the exact React tokens; font assets retain their original OFL notices. Runtime knob arcs use Slint paths; fixed glyphs reuse supplied SVG assets if needed.
- Keep the demonstration's ephemeral macro values in a small Slint model, with no sampler-domain logic. This allows the same UI to run in the Slint viewer for rapid authoring; host code only supplies native embedding and event translation.
- Match the isolated strip at 900×90 and support widths down to 720px. The eight cells and seven gaps span 554px. A value editor fits the reserved area below each knob; no full-shell overlay dependency.
- Verify screenshots against the live React reference and verify native input dispatch, clamping, reset, typed editing and disabled slots. Launcher changes retain behavior-first tests, including GUI-app discovery and artifact resolution.

## Risks / Trade-offs

- Software rendering / native font rasterization can differ by a few antialiased pixels → reuse exact font files and dimensions; inspect native renders at two widths.
- The official skill follows current Slint development → trust pinned 1.18.1 source for signatures and feature support. Enable developer tooling only where supported; disclose native MCP limitations.
- This is a prototype → output and native interaction evidence are proportionate; maintained launcher behavior still follows normal tests and independent review.

## Bounded experiment

Question: Can the existing Slint embedding reproduce Trigger's macro controls and support a short AI authoring loop? Initial implementation budget: 90 minutes. Success requires eight correctly styled controls with verified native interactions and a maintained launch command. Failure requires documenting the failed boundary and revising the embedding or deferring it, without broadening to the full shell. Record adopt/revise/defer/stop at handoff.
