## MODIFIED Requirements

### Requirement: Script modules are graph modules

Script modules SHALL be represented as named graph definitions marked `implementation: script`, with declared event/control ports and typed language/source static parameters. Nodes SHALL instantiate those definitions through the same `type`, `static`, and `defaults` shape used for every other graph definition; instance-specific ad-hoc port declarations SHALL NOT be supported. Compilation SHALL lower the named definition to the script primitive while preserving its declared interface and typed construction values.

#### Scenario: Script module participates in routing

- **WHEN** a script-backed definition declares an event input and control output and a node instantiates that definition
- **THEN** other compatible modules SHALL connect to those ports using ordinary patch connections

#### Scenario: Repeated script instances share one interface definition

- **WHEN** a graph instantiates the same script-backed definition more than once
- **THEN** every instance SHALL expose the definition's declared ports while retaining disjoint mutable script state
