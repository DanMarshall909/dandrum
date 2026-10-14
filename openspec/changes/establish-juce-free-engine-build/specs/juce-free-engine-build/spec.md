## Purpose

Provide a reproducible native build and test entry point for Dandrum's existing Rust engine without requiring the JUCE host or a browser editor.

## ADDED Requirements

### Requirement: Independent engine build configuration

The root build SHALL offer an explicit engine-only configuration that builds the existing Rust engine and its independent native consumers without configuring, compiling or linking JUCE, invoking JUCE resource tools, or requiring browser assets or JavaScript packages.

#### Scenario: Build with host dependencies unavailable

- **WHEN** a clean source fixture containing the engine and independent native tests but no JUCE checkout or browser assets is configured in engine-only mode
- **THEN** configuration and compilation SHALL succeed and the native ABI tests SHALL render the expected signed stereo samples and preserve sample-offset note behavior through the existing engine
- **AND** the configuration SHALL register the Rust engine tests and independent native tests without JUCE host targets or generated JUCE assets

### Requirement: Explicit configuration scope

The root build SHALL retain its existing default JUCE comparison configuration. Engine-only mode SHALL explicitly reject simultaneous selection of a UI-only or JUCE product configuration rather than silently select a different product.

#### Scenario: Retained comparison configuration

- **WHEN** the engine-only option is omitted or explicitly disabled and a supported existing JUCE editor configuration is selected
- **THEN** the existing JUCE host and plugin targets SHALL remain available with their existing engine test targets

#### Scenario: Reject conflicting product selections

- **WHEN** engine-only mode is requested together with a UI-only preview, JUCE editor/product option or optional JUCE UI experiment
- **THEN** configuration SHALL fail with a diagnostic identifying the conflicting selections before requiring their external dependencies
