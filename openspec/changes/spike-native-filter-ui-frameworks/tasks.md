## 1. Shared experiment

- [x] 1.1 Pin optional framework dependencies and build two standalone/VST3 effect targets without changing normal builds.
- [x] 1.2 Prove stereo filter input/output with a signed impulse oracle; implement shared processor parameters, bypass and state restoration.
- [x] 1.3 Implement bounded capture and shared response/spectrum/history/control models; prove known-signal analysis and host gesture behaviour.

## 2. Renderers

- [x] 2.1 Implement JIVE-owned layout/styles and graph/history/meters/custom knobs, with actual input and host-state checks.
- [x] 2.2 Implement Slint-owned drawing/controls through a custom JUCE platform adapter, with actual input and host-state checks.
- [x] 2.3 Verify simultaneous instances and editor close/reopen, plus rendered screenshots at multiple sizes.

## 3. Launch and evaluation

- [x] 3.1 Specify launcher behaviour with tests, then register both demos with isolated native-only spike builds and README commands.
- [x] 3.2 Measure comparable idle/active rendering resource and frame costs; retain commands, identities and limitations.
- [x] 3.3 Document implementation findings and explicit adopt/revise/defer results; run launcher/owning checks and strict OpenSpec validation before handoff.

## 4. Design-guide follow-up

- [x] 4.1 Restyle the Slint filter view using maintained design tokens and local fonts; verify rotary/node gestures, attribution and rendering at supported sizes.
- [x] 4.2 Fix the GNU Make Slint compiler failure; verify a real nested Make/Cargo regression, maintained launcher gates and the original checkout's complete demo build/launch.
