## ADDED Requirements

### Requirement: Built-in modules declare static parameters
Every built-in module type that accepts static configuration SHALL declare its supported parameters in the Rust module registry.

#### Scenario: Built-in parameter declarations are registered
- **WHEN** the built-in module registry is initialized
- **THEN** each configurable built-in module definition SHALL expose its static parameter declarations alongside its port declarations and delay-boundary metadata

#### Scenario: Built-in declaration supports authoring tools
- **WHEN** a future tool or LLM authoring workflow inspects a built-in module definition
- **THEN** the module definition SHALL expose enough parameter metadata to describe valid YAML parameter values without reading module DSP implementation code

### Requirement: Built-in parameter declarations are authoritative
Built-in module parameter declarations SHALL be the authoritative source for validating YAML module instance parameters and CLI override values targeting built-in modules.

#### Scenario: Unknown built-in parameter is rejected
- **WHEN** a YAML module instance or CLI override provides a parameter not declared by the target built-in module type
- **THEN** validation SHALL fail with a structured diagnostic before graph preparation

### Requirement: Built-in module state uses resolved parameters
Built-in module DSP state construction SHALL consume resolved parameter values prepared before rendering rather than parsing raw YAML values during processing.

#### Scenario: DSP state is prepared from resolved parameters
- **WHEN** a built-in module instance is prepared for offline or realtime rendering
- **THEN** its DSP state SHALL be constructed from validated resolved parameter values
