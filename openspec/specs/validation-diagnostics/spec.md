## Purpose

Specify structured diagnostic records for all validation and runtime errors in the engine.

## Requirements

### Requirement: Structured diagnostic records

All validation and runtime errors in the engine SHALL produce structured diagnostic records rather than unstructured
strings.

#### Scenario: Validation error returns structured diagnostic

- **WHEN** graph validation fails due to incompatible port types
- **THEN** the validator SHALL return a structured diagnostic record containing error code, severity, YAML path, module
  ID, port name, expected type, actual type, and message

### Requirement: Stable error codes

Every diagnostic SHALL include a stable, unique error code in dot-separated namespace format (e.g.,
`validation.type_mismatch`, `graph.cycle_detected`). Error codes SHALL NOT change between releases.

#### Scenario: Error code is stable

- **WHEN** the same validation error occurs in different engine versions
- **THEN** the error code SHALL be identical

### Requirement: Diagnostic severity levels

Diagnostics SHALL support three severity levels: error (prevents rendering), warning (render proceeds but behaviour may
be unexpected), and info (advisory).

#### Scenario: Error severity prevents render

- **WHEN** a diagnostic has severity `error`
- **THEN** the engine SHALL NOT start rendering

#### Scenario: Warning allows render

- **WHEN** a diagnostic has severity `warning`
- **THEN** the engine MAY start rendering with warnings reported

### Requirement: YAML source location

Every diagnostic originating from YAML parsing or patch validation SHALL include the file path, line number, and column
range of the relevant source location.

#### Scenario: Error reports YAML location

- **WHEN** a YAML patch contains an invalid module parameter value
- **THEN** the diagnostic SHALL report the file path, line, and column of the invalid value

### Requirement: Port-level diagnostics

Diagnostics that reference a module port SHALL include the module ID and port name.

#### Scenario: Connection error references port

- **WHEN** a connection references a nonexistent port on an existing module
- **THEN** the diagnostic SHALL include the module ID and the unresolved port name

### Requirement: Type and value reporting

Diagnostics involving type or value mismatches SHALL report the expected type/value and the actual type/value.

#### Scenario: Type mismatch reports expected and actual

- **WHEN** a connection between incompatible signal types is validated
- **THEN** the diagnostic SHALL report the expected signal type and the actual signal type

### Requirement: Suggested fix

Where safe to compute automatically, diagnostics SHALL include a suggested fix.

#### Scenario: Missing connection suggests add

- **WHEN** a module's required input port is unconnected
- **THEN** the diagnostic MAY suggest connecting it to a compatible default source

### Requirement: Diagnostics collection interface

The engine SHALL expose a collection interface to retrieve all diagnostics from loading, validation, and graph
construction.

#### Scenario: Diagnostics collected after load

- **WHEN** a patch file is loaded with validation warnings
- **THEN** the diagnostics collection SHALL contain all warnings and errors from parsing and validation
