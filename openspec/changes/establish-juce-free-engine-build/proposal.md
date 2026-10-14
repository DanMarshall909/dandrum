## Why

Issue #24 cannot prove host independence while the root build always configures JUCE, even for existing Rust/C++ engine tests. Establish a reproducible JUCE-free engine build before adding replacement format and editor adapters.

## What Changes

- Add an explicit root CMake engine-only configuration that builds the existing Rust engine and genuinely independent C++ service/ABI tests without configuring JUCE, browser assets or UI preview toolkits.
- Exercise configuration with JUCE unavailable in an isolated fixture and retain signed-output and sample-offset engine tests.
- Keep the default JUCE comparison build and document the new configuration's scope.

## Capabilities

### New Capabilities

- `juce-free-engine-build`: independent configure/build/test entry point for the existing engine and native ABI consumers.

### Modified Capabilities

None.

## Impact

Root CMake configuration, native ABI/build tests, README build documentation and proving spec mappings. This implements the build prerequisite of #24 J1. Replacement CLAP/VST3/AU, devices, #23 session contracts, parented React, packaging and native-host migration gates remain required later slices; this change does not claim those are complete. No engine rewrite, new session/journal, release-scope reduction or retirement of the comparison host.
