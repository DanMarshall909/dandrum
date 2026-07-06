## Context

Dandrum currently owns its own graph, patch compilation, and realtime render path. JUCE already provides a useful host-side abstraction for loading third-party audio plugins, but the engine does not yet have a first-class way to treat an external plugin as a graph module.

The goal is to let a patch host a user-selected plugin library while keeping the engine core plugin-agnostic and realtime-safe as much as practical.

## Goals / Non-Goals

**Goals:**

- Allow a patch to instantiate a hosted plugin as a module in the engine graph.
- Keep the plugin boundary explicit so built-in DSP remains separate from third-party plugin behavior.
- Support audio and, where practical, event/MIDI input and parameter/state metadata.
- Make plugin load failure, missing plugins, and unsupported formats visible at preparation time.

**Non-Goals:**

- Build a general plugin browser UI.
- Support every plugin format on every platform in the first step.
- Guarantee sandboxing or crash containment for arbitrary third-party plugins.
- Merge plugin hosting into the built-in module implementation model.

## Decisions

### Keep hosted plugins as a distinct module type

The plugin host should be a separate module type rather than a built-in module family. That keeps the runtime contract clear: built-in modules are deterministic engine-owned DSP, while hosted plugins are external code wrapped by the host boundary.

Alternative considered: model plugins as ordinary built-in modules. That would blur the engine/plugin boundary and make validation, lifecycle, and dependency handling harder.

### Use JUCE as the host abstraction at the edge

JUCE already abstracts plugin discovery, loading, and `AudioPluginInstance` lifecycle on the C++ side. Using JUCE at the wrapper edge avoids re-implementing plugin SDK support inside the Rust core while still letting the Rust engine own the graph and scheduling.

Alternative considered: implement plugin loading directly in Rust. That is possible, but it would add platform and format complexity without improving the engine’s core model.

### Prepare plugin instances before realtime rendering

Hosted plugins should be discovered and prepared during graph preparation so realtime rendering can call a ready instance with pre-resolved ports, parameters, and latency metadata.

Alternative considered: discover or load plugins during render. That would violate realtime constraints and make failures harder to reason about.

## Risks / Trade-offs

- Third-party plugins may misbehave or crash → Keep the boundary explicit and fail early where possible.
- Plugin formats differ by platform → Start with a limited, documented support matrix.
- Latency compensation can be subtle → Surface reported latency in the runtime model and test it explicitly.
- State serialization may be format-specific → Keep plugin state as opaque host-managed data.

## Migration Plan

1. Add capability/spec coverage for a hosted plugin module and its validation surface.
2. Extend the engine preparation model to carry hosted plugin metadata and state handles.
3. Add JUCE-backed host loading in the wrapper at the module boundary.
4. Add tests for load failure, state preparation, and audio pass-through before broadening format support.
