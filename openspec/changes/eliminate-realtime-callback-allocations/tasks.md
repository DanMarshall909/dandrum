## 1. Characterization And Guardrails

- [x] 1.1 Add realtime render capacity-regression tests proving repeated prepared-size renders do not grow pending-event, scratch-output, audio-output, or per-voice scratch capacity.
- [ ] 1.2 Add allocation-count or test-allocator coverage around `RealtimeGraphProcessor::render` for at least one simple mono compiled patch.
- [ ] 1.3 Add parity coverage for a representative mono patch, event-driven patch, sampler patch, voice-to-global patch, and polyphonic patch before changing render storage semantics.
- [ ] 1.4 Add tests for disconnected inputs and inactive voices so stale arena data cannot leak into output.

## 2. Render Plan And Compiled Buffer Metadata

- [x] 2.1 Introduce typed `BufferId`, `EventQueueId`, `CompiledEdge`, `RenderStep`, and `RenderPlan` structures.
- [x] 2.2 Derive a render plan from `CompiledPatch` during realtime preparation, including voice steps, global steps, input edges, output buffers, event queues, MIDI input binding, and audio output binding.
- [ ] 2.3 Move default control values into compiled/render-plan metadata where module declarations define defaults.
- [ ] 2.4 Replace callback-time port-name/source-port lookup with pre-resolved buffer IDs and event queue IDs.

## 3. Audio And Control Arena

- [x] 3.1 Add a prepared audio/control arena sized by maximum block size, compiled buffer count, and maximum voice count.
- [x] 3.2 Add APIs for clearing accumulation buffers, summing compiled edges, filling default control buffers, and borrowing short-lived input/output slices.
- [ ] 3.3 Update realtime mono/global rendering to use arena-backed input and output buffers for a minimal oscillator/gain/output patch.
- [ ] 3.4 Expand arena-backed rendering across non-event modules while preserving existing audio behaviour.

## 4. Module Processing Context

- [ ] 4.1 Introduce `ProcessContext` with typed access to input slices, output slices, input events, event writers, frame count, block start frame, and sample rate.
- [ ] 4.2 Change module processors from returning owned `ModuleOutputs` to writing into the provided `ProcessContext`.
- [ ] 4.3 Keep reusable DSP algorithms independent from graph routing by containing graph/port translation in module adapter code.
- [ ] 4.4 Remove obsolete `ModuleOutputs` audio/control output maps from the realtime render path after all call sites migrate.

## 5. Bounded Realtime Events

- [ ] 5.1 Introduce prepared fixed-capacity event queues and event writers with explicit overflow reporting.
- [ ] 5.2 Replace callback-time `Vec<BlockEvent>` collection of pending events with bounded prepared queues.
- [ ] 5.3 Replace event-port `HashMap<String, Vec<BlockEvent>>` routing with compiled event queue IDs.
- [ ] 5.4 Define and test overflow behaviour for note-on, note-off, automation/control, script-generated, and diagnostic events.

## 6. Polyphonic Arena Path

- [ ] 6.1 Split polyphonic rendering into event-to-voice routing, active voice processing, voice output accumulation, global processing, output binding, and voice retirement.
- [ ] 6.2 Replace per-block `Vec<Vec<BlockEvent>>`, per-voice `HashMap<usize, ModuleOutputs>`, and accumulation `HashMap` storage with prepared per-voice queues and arena buffers.
- [ ] 6.3 Ensure inactive voices do not leak stale arena buffers into accumulated output.
- [ ] 6.4 Preserve existing voice allocation and voice retirement semantics.

## 7. Verification And Cleanup

- [ ] 7.1 Remove realtime callback dependence on `HashMap<String, Vec<f32>>`, callback-time input `Vec` allocation, and callback-time event `Vec` growth.
- [ ] 7.2 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml` and fix regressions.
- [ ] 7.3 Configure/build with `$HOME/.local/bin/cmake -S . -B build` and `$HOME/.local/bin/cmake --build build` if the local environment supports it.
- [ ] 7.4 Run `ctest --test-dir build` if the CMake build is available.
- [ ] 7.5 Run `openspec validate eliminate-realtime-callback-allocations --strict` and fix validation errors.
- [ ] 7.6 Document any remaining verification gaps before archiving the change.
