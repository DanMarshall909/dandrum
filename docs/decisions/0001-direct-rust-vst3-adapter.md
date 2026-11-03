# ADR 0001: Use a direct Rust VST3 adapter for Trigger

- Date: 2026-11-03
- Status: Accepted architectural direction; implementation and qualification pending
- Decision owner: Dan

## Context

The private Linux Trigger CLAP beta is complete. Trigger already has a
format-neutral host, persistent instrument session, bounded runtime handoffs,
portable patch/sample/parameter state and a React/Wry editor. VST3 should reuse
these owners while retaining the existing CLAP product.

CLAP-first projects demonstrate that wrapping a Rust CLAP plugin as VST3 is
feasible. Dan prefers to own the format adapter rather than depend on an
external wrapper whose behavior and fixes are outside the project's control.
Low-level interface bindings are acceptable because implementing the same
binary interfaces locally would duplicate their work.

## Decision

1. Implement a direct Rust VST3 adapter over the existing Trigger host. Do not
   introduce `clap-wrapper` or replace the current architecture with a broader
   plugin framework. A wrapper is not an automatic fallback; adopting one would
   require an explicit revision of this decision.
2. Use `coupler-rs/vst3-rs` for VST3 interface bindings and COM helpers. Pin and
   inspect the exact version or revision during implementation planning. Keep
   the dependency and unsafe host-interface handling at the adapter boundary;
   Dandrum owns translation, lifecycle, threading and failure behavior.
3. Reuse the engine, session/history, sample ownership, public parameter
   definitions, portable state and runtime activation/reclamation machinery.
   Each plugin instance has one authoritative instrument session and history.
   Controller/view projections must not create another musical authority.
4. Reuse the React/Wry editor through shared integration where appropriate.
   Separate platform window parenting, event-loop servicing, resource location
   and packaging from musical behavior. GUI work, preparation and reclamation
   remain outside the realtime audio callback.
5. Deliver Linux first. Native Windows and macOS support are mandatory outcomes
   of the VST3 work; Linux completion is an intermediate milestone. Plan the
   platform boundaries from the beginning and qualify each native platform.
6. Use REAPER as the initial acceptance host on all three platforms. Add Ableton
   and Bitwig compatibility qualification closer to release on their supported
   target platforms. Initial REAPER acceptance does not establish acceptance in
   those later hosts.

This decision supersedes the wrapper-candidate and wrapper-fallback direction
for the later VST3 increment in the historical investigation and archived beta
planning. It does not change the completed CLAP beta's scope or evidence, select
an AU/standalone implementation, or authorize implementation during refinement.

## Architecture

```text
CLAP host -> existing CLAP adapter --+
                                    +-> shared Trigger host/session -> Rust DSP
VST3 host -> direct Rust adapter ---+                  ^
                                                      |
                                             React/Wry editor
```

VST3-specific types and host-loop contracts stay outside the headless engine.
Shared editor orchestration currently located in the CLAP adapter may need
covered extraction; this decision does not prescribe that extraction's shape.

## Alternatives considered

| Alternative | Assessment |
| --- | --- |
| Wrap the existing CLAP with `clap-wrapper` | Credible Rust/Clack and WebView precedents, with less initial format plumbing. Rejected because it adds an externally owned translation and lifecycle layer. |
| Adopt a broader multi-format plugin framework | Supplies more infrastructure, but would require adapting existing parameter, state and editor integration. Not selected for this format addition. |
| Implement and maintain the VST3 bindings locally | Offers direct ownership of the ABI definitions, but duplicates the accepted low-level dependency. Not selected. |

## Consequences and qualification

Dandrum gains direct control over VST3 behavior and takes responsibility for
COM ownership, host call sequences, event translation, automation, state streams
and native editor lifetimes. The bindings do not establish safe use or host
compatibility by themselves.

Begin with a bounded Linux/REAPER feasibility proof using the real Trigger
owners. Before dependent product integration, prove discovery and resource
resolution, signed rendered audio and timed notes, public parameter conversion
and sample-offset automation, fresh-instance state restoration and invalid-state
preservation, and actual editor destruction/recreation with retained session and
Undo history. Include stopped/editor-absent servicing, independent instances and
callback safety. Compilation or the upstream gain example alone is insufficient.

Preserve the existing CLAP behavior through relevant regression checks. Native
Windows and macOS must earn their own installed-product and host evidence;
cross-compilation and Linux results alone do not complete those obligations.

## Open refinement items

- Processor/controller component structure and ownership, including operation
  without an instantiated controller or editor.
- VST3 automation curve handling, event ordering/capacity and stopped-processing
  behavior through the existing bounded runtime.
- Stable VST3 class/parameter identities and the mapping to shared public values.
- Platform-specific editor servicing and teardown, artifact/resource layouts,
  minimum OS versions, CPU architectures and exact REAPER configurations.
- The first proof's finite execution/storage budget, acceptance tests and stop
  conditions, followed by an implementation OpenSpec change.

These items remain open; recording the ADR does not resolve them or mark any
implementation task complete.

## References

- [Completed CLAP beta and migration evidence](../migrations/juce-replacement-progress.md).
- [Historical deferred-product plan](../../openspec/changes/archive/2026-11-03-add-juce-free-trigger-beta-products/future-products.md).
- [Historical JUCE replacement investigation](../investigations/juce-replacement-investigation.md).
- [vst3-rs bindings and scope](https://github.com/coupler-rs/vst3-rs).
- [Steinberg VST3 architecture and ownership contracts](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/API%2BDocumentation/Index.html).
- [Rust/Clack CLAP-first example](https://github.com/CrushedPixel/rust-clap-first-example).
- [NovoNotes Rust/WebView/CLAP example](https://github.com/novonotes/wrac-plugin-template).
