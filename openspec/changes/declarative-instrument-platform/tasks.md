## 1. New Primitive Modules

- [ ] 1.1 Implement noise generator primitive (`noise`) with white noise output and configurable seed
- [ ] 1.2 Implement impulse/click generator primitive (`impulse`) with event trigger input
- [ ] 1.3 Implement math/multiply primitive (`multiply`) with sample-wise multiplication
- [ ] 1.4 Implement note-to-control mapper primitive (`note_to_control`) with frequency, pitch CV, and velocity outputs
- [ ] 1.5 Implement envelope follower primitive (`envelope_follower`) with attack/release parameters
- [ ] 1.6 Implement delay line primitive (`delay_line`) with configurable delay time and feedback
- [ ] 1.7 Register all new primitives in the built-in module registry

## 2. Composite Expansion System

- [ ] 2.1 Implement composite module type with `composite_id` reference in YAML patch format
- [ ] 2.2 Implement composite definition loading from composite directories
- [ ] 2.3 Implement deterministic composite expansion to flat primitive graph
- [ ] 2.4 Implement composite port mapping from external ports to internal module ports
- [ ] 2.5 Implement composite parameter exposure (parameter name mapping through to internal modules)
- [ ] 2.6 Implement recursive composite nesting with maximum depth enforcement
- [ ] 2.7 Implement module ID prefixing for expanded composite instances

## 3. Script Module Constraints

- [ ] 3.1 Implement script pre-validation (parse and validate off audio thread before render)
- [ ] 3.2 Enforce no filesystem access in script runtime during render
- [ ] 3.3 Enforce no network access in script runtime during render
- [ ] 3.4 Enforce no heap allocation during script execution
- [ ] 3.5 Enforce no blocking calls in script runtime
- [ ] 3.6 Enforce bounded execution cost (instruction count or time limit per block)
- [ ] 3.7 Prevent audio-rate output ports on script modules
- [ ] 3.8 Report script runtime errors through structured diagnostics with stable error codes

## 4. Structured Validation and Diagnostics

- [ ] 4.1 Define structured diagnostic record type with stable error code, severity, YAML path, module ID, port name, expected/actual types, message, and suggested fix
- [ ] 4.2 Implement error code namespace system (`validation.*`, `graph.*`, `script.*`, `loading.*`)
- [ ] 4.3 Implement YAML source location tracking (file path, line, column) in diagnostics
- [ ] 4.4 Implement port-level diagnostics (include module ID + port name)
- [ ] 4.5 Implement type/value reporting in type mismatch diagnostics
- [ ] 4.6 Implement suggested fix field where safe to compute
- [ ] 4.7 Implement diagnostics collection interface for retrieving all errors/warnings from load, validate, and graph construction

## 5. YAML Format Extensions

- [ ] 5.1 Implement `presets` section with preset file loading and parameter value application
- [ ] 5.2 Implement `parameters` section with patch-level parameter bindings to module parameters
- [ ] 5.3 Implement `assets` section with external resource declarations and missing-asset diagnostics
- [ ] 5.4 Implement `metadata` section for versioning and authoring info (optional parse)
- [ ] 5.5 Implement `pad_map` section for drum machine event-to-pad routing

## 6. Drum Machine Event Mapper

- [ ] 6.1 Refactor drum machine to be a stateless event transformer (remove samples, synthesis, mixing)
- [ ] 6.2 Implement named pad event output ports from `pad_map` configuration
- [ ] 6.3 Add acceptance test verifying drum machine triggers external voice composites via event ports

## 7. Acceptance Examples

- [ ] 7.1 Create YAML composite for 808-style kick (sine osc + pitch envelope + noise click)
- [ ] 7.2 Create YAML composite for 909-style kick (sine osc + pitch envelope + saturation)
- [ ] 7.3 Create YAML composite for synthetic snare (tone osc + noise + dual envelopes)
- [ ] 7.4 Create YAML composite for closed/open hi-hat pair (noise + HPF + short/long envelopes)
- [ ] 7.5 Create YAML composite for subtractive synth voice (osc + filter + dual ADSR)
- [ ] 7.6 Create YAML composite for sampler voice (sampler + ADSR + pitch mapping)
- [ ] 7.7 Create YAML composite for effects rack (delay + reverb + filter + mixer)
- [ ] 7.8 Create YAML patch with script module performing event-to-control mapping
- [ ] 7.9 Create YAML patch with drum machine driving external voice composites
- [ ] 7.10 Add acceptance tests verifying each example loads and renders deterministically

## 8. Capability Discovery API

- [ ] 8.1 Implement module type enumeration API
- [ ] 8.2 Implement port metadata query (name, direction, signal type per module type)
- [ ] 8.3 Implement parameter metadata query (name, type, range, default, unit, enum values)
- [ ] 8.4 Implement module category query (primitive, composite, script, built-in)
- [ ] 8.5 Ensure capability discovery API is separate from rendering paths (no realtime impact)

## 9. Primitive Decision Framework Documentation

- [ ] 9.1 Document the five-category classification system and primitive gate criteria in architecture docs
- [ ] 9.2 Document the primitive roadmap with outcomes for each candidate module type
- [ ] 9.3 Add primitive gate checklist to contribution guidelines
