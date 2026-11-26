## Why

ADR 0001 accepts a direct Rust VST3 adapter over the working Trigger owners.
A real Linux/REAPER feasibility proof must now establish the format's ownership,
automation, state and native-editor contracts before wider product integration.

Dan approved this plan and its resource envelope on 3 November 2026 by
requesting delivery of this change and explicitly approving the proposed plan
and resource budget. Task 1.1 records that authority.

## What Changes

- Add a direct Rust VST3 adapter using exactly pinned, inspected `vst3-rs`
  bindings, retaining one authoritative Trigger session/history per instance.
- Prove installed discovery/resource resolution, literal signed audio and timed
  notes, all public parameter conversions, sample-accurate automation curves,
  portable state restoration and failed-state preservation.
- Attach the existing production React/Wry editor to a real VST3 native parent;
  prove complete destruction/recreation with retained Undo, stopped and
  editor-absent service, independent instances and callback safety.
- Retain the CLAP product and prove affected shared behavior through regression
  checks. Require current-source tests/coverage, native and actual REAPER
  evidence, independent completion review and normal source/evidence publication.
- Maximize shared implementation across formats: reuse session/runtime, public
  parameter policy, state and editor orchestration; keep format-specific ABI,
  lifecycle and platform edges in narrow adapters.
- Keep native Windows and macOS as mandatory subsequent milestones, with named
  platform boundaries and native qualification criteria in this design.

## Capabilities

### New Capabilities

- `trigger-vst3-feasibility`: the bounded Linux/REAPER direct-adapter proof,
  including installation, lifecycle, automation, state and retained editing.

### Modified Capabilities

None proposed. Existing Trigger, CLAP, session and headless-engine contracts
remain obligations. If implementation needs a change to those requirements,
refine the affected delta before dependent implementation.

## Impact

New `src/vst3-adapter` and associated CMake, dynamic-host/native/DAW tests and
installation/notice evidence. Covered sharing of editor orchestration currently
inside `src/clap-adapter` may affect both adapters. Runtime curve support may
extend `src/trigger-host` without introducing VST3 types into shared owners.
Existing engine/session/state/resource/frontend owners are reused.

Linux proof products remain local/private; source, tests and reviewed portable
text evidence follow the repository's normal publication gates. This milestone
does not establish Windows/macOS, Ableton/Bitwig, Wayland, AU, standalone-device
or unrestricted redistribution acceptance.
