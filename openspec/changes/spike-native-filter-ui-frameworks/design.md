## Context

The user authorizes two comparable framework spikes, not a production UI migration. Use the optimized React branch as the integration base, preserving its existing engine and editors. The production renderer-independent change remains independently pending.

## Goals / Non-Goals

**Goals:** Two opt-in real stereo effect plugins and standalone demos; equal input, parameter and analysis workloads; framework-owned drawing and controls; reversible, inspectable evidence; components under 200 lines where practical.

**Non-Goals:** New DSP primitives, changes to existing plugins, a production framework adoption, a new plugin framework, or a broad UI migration.

## Decisions

- Pin Slint 1.18.1 (`372cf0ee5577c3dfec309a45e7b778ba4e81b734`) and JIVE 1.2.0 (`89d5787a762e674ee8b7141031a99e6743948f05`). Fetch only with the optional spike build. Preserve upstream licence notices and include the standard AboutSlint attribution widget in the Slint UI.
- Build a stereo high-pass → bell → low-pass Rust graph with stable host parameters, bypass, optional generated audition input and state round-trip. The same processor serves both renderers. UI commands enter host begin/update/end gestures on the message thread; audio observes atomic parameters. Prepare/destroy the Rust graph off audio; copy host input to preallocated buffers before rendering output.
- Share numeric response/spectrum/history snapshots and control mapping, not the drawn widgets. Calculate the response from an off-audio impulse render through a separate instance of the actual Rust graph. A preallocated single-producer/single-consumer capture FIFO drops visual samples instead of making audio wait. FFT/history work and geometry conversion occur on the message thread for this isolated spike, with costs included in measurement.
- JIVE owns a declarative ValueTree layout and styles with custom JUCE graph/history/meter widgets and rotary LookAndFeel. Poll host values through the UI timer instead of attaching audio-thread notifications to AsyncUpdater.
- Slint uses its own Path/Rectangle/Image/TouchArea controls and a custom platform adapter inside the JUCE editor. Register the platform once per linked runtime, create per-editor windows/renderers/buffers, and keep the JUCE event loop. Use full-buffer software rendering, which supports Paths; test the limitation that gradient Path brushes flatten to a solid colour. Native window embedding and a GPU renderer are a fallback investigation, not a silent change of comparison scope.
- Keep launcher changes maintained and tested. Both entries use an isolated Release/native-only spike configuration; existing demo flags and package preparation remain compatible.

## Bounded Experiment

Question: can each framework implement and embed a demanding audio-plugin UI with productive declarative development and materially less editor resource cost than the measured React setup?

Spend at most 45 minutes on an initial renderer integration approach before revising the approach based on a concrete compiler/runtime failure; retain that evidence. Success means both plugins build, filter actual audio, render their graph/spectrum/history/controls, respond to drag and host state, survive two instances and close/reopen, and have resource evidence with explicit limits. Failure of an approach triggers a documented revise/defer decision rather than a production migration. Stop the spike once these bounded criteria and a comparison report are verified; do not expand into unrelated production tasks.

## Risks / Trade-offs

- Full-buffer Slint software rendering has limited shadows, transforms and gradient Paths → use supported geometry and report the backend limitations; do not imply all Slint renderers were evaluated.
- The remote display may limit visual/input verification → use task-owned captures and deterministic runtime checks, and report any physical interaction gap explicitly.
- Shared processing can hide equal DSP defects → retain an independent signed impulse oracle and actual VST3 host checks for each renderer.
- Runtime/global lifetime differs between frameworks → verify simultaneous instances and repeated editor destruction/recreation.
