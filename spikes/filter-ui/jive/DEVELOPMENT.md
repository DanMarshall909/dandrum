# JIVE filter editor spike notes

The editor uses the actual JIVE `Editor` interpretation and `View` extension
hooks. JIVE owns the ValueTree hierarchy, flex layout, text and style sheets;
JUCE paints specialised response/spectrum, history, meters and rotary graphics.
Each interpreted editor owns a View, 30 Hz timer and model. Its custom widgets
share that model so destruction order cannot leave a widget holding a dead
model. There are no audio-thread UI attachments or static editor globals.

Source-integration frictions identified before compiler validation:

- `ComponentFactory::set()` inserts and cannot override the existing `Knob`
  factory. `View::createComponent()` supplies custom JUCE sliders while keeping
  JIVE's `Knob` decorator and reactive ValueTree machinery.
- `makeView()` stores constructor arguments in a tuple. Passing the processor
  by reference would copy a noncopyable AudioProcessor; passing a pointer keeps
  the host-owned lifetime explicit.
- JIVE flex has no CSS `gap` property. A small ValueTree authoring helper
  converts gaps to child margins before interpretation.
- Text alignment uses `justification`.
- A graph interaction check must dispatch the host message loop after a knob
  edit. Sleeping and calling pending timers can leave JUCE's timer-start message
  undispatched. The check now waits up to 240 ms for the actual painted node
  geometry to match host state, fails with visible/expected coordinates if it
  does not, and drags that verified visible node.
- The real VST3 host initially found a missing custom editor. Individual JIVE
  implementation files include `Interpreter.h` directly, bypassing the umbrella
  header's `JucePlugin_Name` detection. Define `JIVE_IS_PLUGIN_PROJECT=1` for
  the complete plugin and UI-check targets, as upstream's gain-plugin example
  does. Defining only `JucePlugin_Name` was insufficient. The factory now also
  applies the declared 1080×760 extent after interpretation; retain the host's
  initial-size assertion instead of resizing before checking it.
- The interpreter can be local because the static tree is interpreted once.
  Widget properties have their own listeners; no runtime tree insertion occurs.

`checkJiveInteractions()` is an optional message-thread smoke check. It drives
the custom rotary and actual graph mouse methods and checks host parameter
updates. Shared runtime checks should additionally count host gestures, test
two simultaneous editors, close/reopen and host-state polling, render captures
at multiple sizes, and verify observed audio analysis. These notes claim no
production readiness or platform compatibility beyond recorded build/runtime
evidence in the shared experiment report.

Integrated verification passed all nine owning CTest checks, including actual VST3
host captures, input/host state, two editors and close/reopen. Recorded outcomes
and limits are in the [experiment report](../README.md).

Dependency pin: JIVE 1.2.0, `89d5787a762e674ee8b7141031a99e6743948f05`.
Its [upstream MIT licence notice](https://github.com/ImJimmi/JIVE/blob/89d5787a762e674ee8b7141031a99e6743948f05/LICENSE.md)
remains in the fetched source tree; preserve it when redistributing this dependency.
