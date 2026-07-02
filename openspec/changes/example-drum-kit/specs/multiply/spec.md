## ADDED Requirements

### Requirement: multiply module computes a × b
The `multiply` module SHALL accept two control inputs (`a` and `b`) and produce a single control output (`product`) where `product[i] = a[i] × b[i]` for each sample `i`.

#### Scenario: Multiply two constant signals
- **WHEN** input `a` is all 0.5 and input `b` is all 0.8 for 64 frames
- **THEN** every output sample SHALL be 0.4

#### Scenario: Output length matches inputs
- **WHEN** the module is processed for 128 frames
- **THEN** the output control vector SHALL have exactly 128 samples

#### Scenario: Zero times any value is zero
- **WHEN** input `a` is 0.0 and input `b` is 0.9
- **THEN** all output samples SHALL be 0.0

#### Scenario: Negative values multiply correctly
- **WHEN** input `a` is -0.5 and input `b` is 0.5
- **THEN** every output sample SHALL be -0.25

### Requirement: multiply is registered and dispatchable
The multiply module SHALL be registered in the built-in module registry, have a corresponding `ModuleKind::Multiply` variant, and be dispatchable at render time.

#### Scenario: multiply is in built-in registry
- **WHEN** the registry is queried for the `multiply` module type
- **THEN** a definition SHALL be returned

#### Scenario: multiply renders without error
- **WHEN** a patch containing a multiply module is loaded, prepared, and rendered
- **THEN** rendering SHALL complete without panic or error
