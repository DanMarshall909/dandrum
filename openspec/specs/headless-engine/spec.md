## Purpose

Specify the frontend-independent engine behavior and offline rendering entry points.

## Requirements

### Requirement: Engine runs without a GUI

The engine SHALL load and process instruments without requiring a graphical user interface, graphical toolkit, window
system, or plugin host.

#### Scenario: Load patch in headless process

- **WHEN** a valid YAML patch file is loaded by a non-GUI process
- **THEN** the engine SHALL construct the instrument graph without opening any window or requiring GUI services

#### Scenario: Engine core is frontend independent

- **WHEN** the engine package is used by a CLI frontend
- **THEN** the engine SHALL not depend on CLI-specific argument parsing, terminal IO, GUI code, or plugin APIs

### Requirement: Offline rendering entry point

The engine SHALL provide an offline rendering path that processes a loaded patch from input events into a WAV audio
output.

#### Scenario: Render patch to WAV

- **WHEN** a valid patch and input event sequence are rendered offline
- **THEN** the engine SHALL produce a WAV file containing the rendered audio

#### Scenario: Rendering is deterministic

- **WHEN** the same patch, assets, render settings, and input events are rendered twice
- **THEN** the audio output SHALL be identical within the engine's defined sample format

### Requirement: Block processing model

The engine SHALL process patches in blocks using the same scheduling model intended for future realtime use.

#### Scenario: Render uses block scheduler

- **WHEN** an offline render is executed with a configured block size
- **THEN** the graph SHALL be processed block by block rather than by a separate offline-only execution model

### Requirement: Owned runtime can move between host threads

The engine SHALL allow a host to prepare an instrument off the render thread and transfer exclusive ownership of its runtime and preallocated buffers to another thread through the safe Rust API. Transfer SHALL preserve the prepared graph, signed output and reset lifecycle without introducing callback locks or additional DSP processing.

#### Scenario: Render after ownership transfer

- **WHEN** a host prepares a representative gain/filter instrument and moves its runtime and preallocated buffers to a render thread
- **THEN** the safe Rust API SHALL permit the transfer and render the expected literal signed stereo output through the child modules

#### Scenario: Reset after moving the runtime back

- **WHEN** the runtime is returned to the control thread after rendering, reset and transferred for another render
- **THEN** the instrument SHALL remain usable and render the expected initial signed output again with independent instance state and off-render ownership reclamation
