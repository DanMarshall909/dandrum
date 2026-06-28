## ADDED Requirements

### Requirement: Compiled rendering supports voice-to-global routing

The compiled offline render path SHALL correctly process voice-scoped modules that route audio or control signals to global-scoped modules. Voice-scoped modules SHALL be processed first so their outputs are available when global-scoped modules execute.

#### Scenario: Voice oscillator through global mixer produces correct output

- **WHEN** a graph has a voice-scoped `oscillator` connected to a global `audio_mixer` connected to a global `audio_output`
- **THEN** compiled rendering produces the same left/right buffers as raw Graph rendering

#### Scenario: Compiled execution order remains globals-first

- **WHEN** a graph has both global-scoped and voice-scoped modules
- **THEN** `CompiledPatch::execution_order` lists every global-scoped node before any voice-scoped node

#### Scenario: Voice nodes process before global nodes in compiled rendering

- **WHEN** `process_block_compiled` executes
- **THEN** all voice-scoped nodes SHALL be processed before any global-scoped node in each block
