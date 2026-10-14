## ADDED Requirements

### Requirement: Owned runtime can move between host threads

The engine SHALL allow a host to prepare an instrument off the render thread and transfer exclusive ownership of its runtime and preallocated buffers to another thread through the safe Rust API. Transfer SHALL preserve the prepared graph, signed output and reset lifecycle without introducing callback locks or additional DSP processing.

#### Scenario: Render after ownership transfer

- **WHEN** a host prepares a representative gain/filter instrument and moves its runtime and preallocated buffers to a render thread
- **THEN** the safe Rust API SHALL permit the transfer and render the expected literal signed stereo output through the child modules

#### Scenario: Reset after moving the runtime back

- **WHEN** the runtime is returned to the control thread after rendering, reset and transferred for another render
- **THEN** the instrument SHALL remain usable and render the expected initial signed output again with independent instance state and off-render ownership reclamation
