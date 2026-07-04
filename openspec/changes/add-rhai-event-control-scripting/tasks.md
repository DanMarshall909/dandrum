## 1. Runtime Dependency And Boundary

- [ ] 1.1 Add `rhai` dependency to the Rust engine crate with the smallest feature set that supports AST compilation and bounded execution.
- [ ] 1.2 Add `RhaiScriptRuntime` behind the existing `ScriptRuntime` trait.
- [ ] 1.3 Add `ScriptRuntimeLimits` with engine-level maximums for operations, call depth, emitted events, controls, state entries, key length, and dynamic value sizes.
- [ ] 1.4 Add tests proving Rhai scripts are compiled during preparation rather than from source during render.

## 2. Patch Loading And Validation

- [ ] 2.1 Add failing tests for valid script module declarations using `language: rhai` and inline `source`.
- [ ] 2.2 Implement script parameter parsing for language and inline source.
- [ ] 2.3 Add validation tests rejecting missing source, unsupported language, malformed Rhai, and missing `process(ctx)` entry point.
- [ ] 2.4 Add validation tests rejecting audio input/output ports on script modules.
- [ ] 2.5 Add structured diagnostics for script parse, validation, unsupported language, unsupported port type, and missing entry point.

## 3. Host API

- [ ] 3.1 Add tests proving scripts can read input events and emit events to declared output ports.
- [ ] 3.2 Add tests proving scripts can read scalar controls and write scalar control outputs.
- [ ] 3.3 Add tests proving scripts can persist numeric state between block calls.
- [ ] 3.4 Implement bounded `ctx.events`, `ctx.controls`, `ctx.emit`, `ctx.control`, `ctx.state_get`, and `ctx.state_set`.
- [ ] 3.5 Add tests proving undeclared output ports produce structured diagnostics or deterministic dropped output.

## 4. Bounded Execution

- [ ] 4.1 Add tests proving an infinite loop or excessive work fails with an operation-budget diagnostic.
- [ ] 4.2 Add tests proving excessive recursion fails with a call-depth diagnostic.
- [ ] 4.3 Add tests proving emitted events are capped per output port.
- [ ] 4.4 Add tests proving state entries and key lengths are capped.
- [ ] 4.5 Add tests proving script failure never panics the graph processor.

## 5. Graph Processor Integration

- [ ] 5.1 Add `PerModuleState` support for prepared script runtime state.
- [ ] 5.2 Add `ModuleKind::Script` to render-supported kinds only after runtime preparation and validation are complete.
- [ ] 5.3 Add script dispatch to `process_module` for event/control ports only.
- [ ] 5.4 Add offline render tests for deterministic script event routing.
- [ ] 5.5 Add realtime graph processor tests proving prepared script modules render without parsing source during render.

## 6. Examples

- [ ] 6.1 Add an event-router example mapping kick/snare/hat note events to separate event output ports.
- [ ] 6.2 Add an accent/control example mapping note velocity to a scalar accent output.
- [ ] 6.3 Add an example showing numeric state persistence across blocks.
- [ ] 6.4 Document that scripts are not for audio DSP and that new DSP should be built as Rust primitives.

## 7. Verification

- [ ] 7.1 Run `$HOME/.cargo/bin/cargo test --manifest-path src/rust-engine/Cargo.toml`.
- [ ] 7.2 Run CMake/CTest verification if CMake configure/build is available.
- [ ] 7.3 Run `openspec validate add-rhai-event-control-scripting --strict` if the OpenSpec command is available.
- [ ] 7.4 Update task checkboxes only after the related tests and verification pass, or document the verification gap.
