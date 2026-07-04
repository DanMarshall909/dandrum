## Overview

Add a Rhai-backed scripting runtime for deterministic event/control behaviour. Rhai is an implementation detail behind Dandrum's script runtime boundary; patch semantics should describe scripts in terms of input events, scalar controls, output events, scalar controls, and persistent numeric state.

Scripts execute at block rate. The graph processor gathers event and control inputs for the script module, invokes the prepared runtime once for the current block, then publishes bounded event/control outputs to downstream modules. Audio buffers never cross the script boundary.

## Runtime Boundary

The existing script model already contains the core abstraction:

- `ScriptProcessInput`: events, controls, execution context, and state.
- `ScriptProcessOutput`: events, controls, and updated state.
- `ScriptModuleState`: persistent script state.
- `ScriptExecutionContext`: operation-budget tracking.
- `ScriptRuntime`: runtime trait.

This change adds a concrete `RhaiScriptRuntime` that implements `ScriptRuntime`.

```rust
pub struct RhaiScriptRuntime {
    engine: rhai::Engine,
    ast: rhai::AST,
    state: ScriptModuleState,
    limits: ScriptRuntimeLimits,
}
```

The exact fields may change during implementation, but the runtime must preserve the separation between prepared script code and per-block input/output data.

## Patch Shape

A script module remains a normal YAML module with explicit typed ports. Script modules may declare event and control inputs/outputs. Audio ports are rejected for the first implementation.

Example:

```yaml
modules:
  - id: drum_router
    type: script
    parameters:
      language: rhai
      source: |
        fn process(ctx) {
            for e in ctx.events {
                if e.type == "note_on" {
                    if e.note == 36 {
                        ctx.emit("kick", e);
                    } else if e.note == 38 {
                        ctx.emit("snare", e);
                    }
                }
            }
        }
    inputs:
      - name: events
        signal_type: event
    outputs:
      - name: kick
        signal_type: event
      - name: snare
        signal_type: event
```

A future change may move script source into assets or external files. This change only requires inline source support.

## Host API

Expose only a tiny context object:

- `ctx.events`: bounded read-only event list for the current block.
- `ctx.controls`: bounded read-only scalar control map for the current block.
- `ctx.emit(port, event)`: append an event to a declared event output port.
- `ctx.control(port, value)`: set a scalar value for a declared control output port.
- `ctx.state_get(name)`: read a numeric state value.
- `ctx.state_set(name, value)`: write a numeric state value.

The API intentionally avoids filesystem, network, environment, process, time, random, threads, sleeps, logging, reflection over the host, or arbitrary callbacks.

Randomness, if added later, must be deterministic and seedable from patch state.

## Limits

Script runtime limits are required and must be enforced deterministically:

- maximum operations per block
- maximum call depth
- maximum number of input events visible to the script
- maximum emitted events per output port
- maximum scalar control outputs per block
- maximum state entries
- maximum key length
- maximum string length if strings are exposed beyond event/control field names
- maximum array/map sizes if exposed through Rhai dynamic values

When a limit is exceeded, the script produces no further output for the block and a structured diagnostic is recorded. Rendering continues.

## Realtime Contract

Preparation path may allocate, parse, compile, and validate scripts. Audio callback path must not parse source, load files, dynamically compile, import code, access external resources, or allocate unbounded memory.

Rhai execution is permitted in the render path only if:

- source has already been compiled to AST during preparation
- all host API data structures are bounded
- output buffers are bounded
- runtime limits are active
- failures cannot panic across the render path

If these guarantees cannot be met during implementation, script execution must remain unavailable to realtime rendering and limited to offline rendering until the gap is resolved.

## DSP Boundary

Rhai is not a DSP language in Dandrum. Scripts may influence downstream DSP through event and scalar control outputs, but they must not process audio buffers or run per sample.

Bespoke DSP belongs in Rust primitives. Reusable instrument structures belong in YAML composites. Scripts decide event/control policy.

Audio-derived control primitives such as `envelope_follower` and control mapping primitives such as `curve_mapper` are explicitly outside this scripting change. They should be modelled as Rust primitives that emit bounded control signals for dynamics, ducking, modulation, or sidechain-style routing.

## Failure Behaviour

Script failures must be deterministic and non-fatal:

- parse/compile failure: patch preparation fails with structured diagnostics
- validation failure: patch preparation fails with structured diagnostics
- execution-budget failure: current block script output is discarded or truncated according to the documented limit policy
- unsupported API use: validation failure where statically detectable, otherwise execution failure diagnostic
- undeclared output port: validation failure where statically detectable, otherwise execution failure diagnostic

No script failure may panic the graph processor.

## Validation Strategy

Validation should cover:

- language must be `rhai`
- source must be present and compile successfully
- script must define a callable `process(ctx)` entry point
- script module ports must be event/control only
- emitted output ports must exist when statically detectable
- configured limits must be non-zero and within safe engine maximums
- unsupported host capabilities remain unavailable
