## ADDED Requirements

### Requirement: Static parameter declarations

A graph definition SHALL be able to declare typed static parameters (integer, enumeration, string, or resource reference) with optional defaults. Static parameters are resolved at compile time and SHALL NOT be modulatable or connectable at runtime. String static parameters SHALL support construction-time inline text such as script source without making that text a runtime signal.

#### Scenario: Definition declares static parameter

- **WHEN** a graph definition declares a static parameter `channels` of type integer with default `2`
- **THEN** loading SHALL preserve the declaration for instantiation-time resolution and discovery

#### Scenario: Static parameter is not a port

- **WHEN** a connection targets a static parameter name
- **THEN** validation SHALL fail with a diagnostic explaining that static parameters are compile-time values, not ports

#### Scenario: Inline script source is a static string

- **WHEN** a script definition declares its inline source as a string static parameter
- **THEN** loading SHALL preserve the source for compile-time construction without exposing it as a connectable port

### Requirement: Static argument resolution

Nodes SHALL supply static arguments for the referenced definition's static parameters. Missing arguments use declared defaults; missing arguments without defaults, unknown argument names, and type mismatches SHALL fail validation before flattening.

#### Scenario: Missing required static argument rejected

- **WHEN** a node omits a static argument whose parameter declares no default
- **THEN** validation SHALL fail with a structured diagnostic naming the definition, node, and parameter

#### Scenario: Static argument type mismatch rejected

- **WHEN** a node supplies a non-integer value for an integer static parameter
- **THEN** validation SHALL fail with a structured diagnostic before flattening

### Requirement: Static parameters resolve port channel counts

A port's channel count SHALL be either a literal or a reference to one of its definition's static parameters, resolved before connection validation.

#### Scenario: One definition serves mono and stereo

- **WHEN** an `echo` definition declares audio ports with `channels: $channels` and is instantiated once with `channels: 1` and once with `channels: 2`
- **THEN** both instances SHALL validate, each exposing ports with its resolved channel count

### Requirement: Expansion caching is keyed by static arguments

The compiler SHALL cache expanded definitions keyed by definition identity plus resolved static arguments, and instances sharing a key SHALL reuse one expansion structure while receiving disjoint runtime state.

#### Scenario: Repeated instantiation reuses expansion

- **WHEN** a definition is instantiated many times with identical static arguments
- **THEN** the compiler SHALL expand it once per distinct static-argument set, and rendering SHALL give each instance independent state

#### Scenario: Distinct static arguments expand separately

- **WHEN** the same definition is instantiated with `channels: 1` and `channels: 2`
- **THEN** the compiler SHALL produce two distinct expansions with correctly resolved ports

### Requirement: No static expression language

Static arguments SHALL flow by literal value or by name from an enclosing definition's static parameters. Arithmetic, conditionals, functions, and any other expression forms SHALL be rejected.

#### Scenario: Name pass-through is accepted

- **WHEN** a composite forwards its own static parameter by name as a nested node's static argument
- **THEN** resolution SHALL substitute the enclosing definition's resolved value

#### Scenario: Arithmetic expression is rejected

- **WHEN** a static argument contains an expression such as `$channels + 1`
- **THEN** validation SHALL fail with a structured diagnostic before flattening
