## MODIFIED Requirements

### Requirement: Block processing model
The engine SHALL process patches in blocks using a shared scheduling model for offline rendering and realtime instrument rendering.

#### Scenario: Render uses block scheduler
- **WHEN** an offline render is executed with a configured block size
- **THEN** the graph SHALL be processed block by block rather than by a separate offline-only execution model

#### Scenario: Realtime render uses shared block scheduler semantics
- **WHEN** a prepared realtime instrument renders successive audio callback blocks
- **THEN** the graph SHALL advance through the same block scheduling semantics used by offline rendering for event ordering and module state progression
