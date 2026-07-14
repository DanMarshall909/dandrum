## ADDED Requirements

### Requirement: Static parameter declarations

A graph definition SHALL be able to declare typed static parameters (integer, enumeration, string, or resource reference) with optional defaults. A resource declaration SHALL identify its required resource kind (`sample` or `impulse_response`), and a resource literal SHALL carry both its kind and path. Static parameters are resolved at compile time and SHALL NOT be modulatable or connectable at runtime. String static parameters SHALL support construction-time inline text such as script source without making that text a runtime signal.

#### Scenario: Definition declares static parameter

- **WHEN** a graph definition declares a static parameter `channels` of type integer with default `2`
- **THEN** loading SHALL preserve the declaration for instantiation-time resolution and discovery

#### Scenario: Static parameter is not a port

- **WHEN** a connection targets a static parameter name
- **THEN** validation SHALL fail with a diagnostic explaining that static parameters are compile-time values, not ports

#### Scenario: Inline script source is a static string

- **WHEN** a script definition declares its inline source as a string static parameter
- **THEN** loading SHALL preserve the source for compile-time construction without exposing it as a connectable port

#### Scenario: Resource declaration and literal retain kind

- **WHEN** a definition declares a sample resource static parameter and a node supplies `{ kind: sample, path: samples/hit.wav }`
- **THEN** loading SHALL preserve the required kind and typed resource reference for preparation without exposing either as a port

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

### Requirement: Resource static parameters resolve in preparation context

A resource-reference static parameter SHALL resolve through an explicit preparation context carrying authoring origins, host sample rate, and package roots. A relative literal SHALL resolve from the document or package version in which that literal was written, and name pass-through SHALL preserve that origin. Resolution SHALL validate the declared and supplied resource kinds, require the canonical target to remain beneath its canonical origin root, deduplicate immutable loaded data, enforce exact sample-rate compatibility, and produce a typed handle before runtime state is created.

#### Scenario: Package-relative resource resolves

- **WHEN** a packaged graph definition supplies a relative sample resource argument
- **THEN** preparation SHALL resolve it beneath that package's root and share the loaded immutable sample across identical resolved references

#### Scenario: Wrong resource kind is rejected

- **WHEN** a convolution definition receives a resource that is not a supported impulse response
- **THEN** preparation SHALL fail with a structured diagnostic before runtime state creation

#### Scenario: Caller override retains caller origin through pass-through

- **WHEN** a root document supplies a relative resource literal to a packaged definition through one or more static-parameter name pass-throughs
- **THEN** preparation SHALL resolve that literal from the root document where it was written rather than the receiving package root

#### Scenario: Shared resource data has disjoint runtime state

- **WHEN** multiple flattened instances resolve the same canonical resource at the same host sample rate
- **THEN** preparation SHALL share one immutable decoded allocation while constructing independent mutable processor state for every instance

### Requirement: No static expression language

Static arguments SHALL flow by literal value or by name from an enclosing definition's static parameters. Arithmetic, conditionals, functions, and any other expression forms SHALL be rejected.

#### Scenario: Name pass-through is accepted

- **WHEN** a composite forwards its own static parameter by name as a nested node's static argument
- **THEN** resolution SHALL substitute the enclosing definition's resolved value

#### Scenario: Arithmetic expression is rejected

- **WHEN** a static argument contains an expression such as `$channels + 1`
- **THEN** validation SHALL fail with a structured diagnostic before flattening
