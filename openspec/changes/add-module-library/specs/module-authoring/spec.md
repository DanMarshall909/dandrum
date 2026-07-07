## ADDED Requirements

### Requirement: Defined modules use module terminology, not composite terminology

The engine SHALL refer to a YAML-assembled graph building block (one declared via `module_definitions` and expanded into constituent primitives) as a **defined module**, distinguished from a **primitive/built-in module** by category, not by the term "composite." Internal identifiers, source files, comments, tests, documentation, and new spec prose SHALL use module terminology rather than composite terminology.

#### Scenario: Defined vs primitive modules are distinguished by category

- **WHEN** the module catalogue is inspected
- **THEN** each module SHALL be identifiable as either a built-in/primitive module or a defined module
- **AND** the distinction SHALL NOT rely on the term "composite"

#### Scenario: Public YAML terminology is unchanged

- **WHEN** a patch declares a defined module via `module_definitions` and references it by `type`
- **THEN** the existing `module_definitions` and `type` fields SHALL continue to work unchanged

### Requirement: The composite-to-module rename preserves behaviour

Renaming the internal composite concept to module terminology SHALL NOT change engine behaviour: rendered output, public YAML semantics, and FFI/ABI symbol contracts SHALL be identical before and after the rename.

#### Scenario: Rendered output is unchanged

- **WHEN** a patch containing a defined module is rendered before and after the rename
- **THEN** the rendered audio SHALL be byte-identical

#### Scenario: FFI symbols are unchanged

- **WHEN** the crate's exported FFI symbols are compared before and after the rename
- **THEN** the exported symbol names and their contracts SHALL be identical