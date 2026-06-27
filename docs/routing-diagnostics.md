# Routing Diagnostics

## Feedback

Instantaneous audio and control feedback is rejected during graph validation because it creates an undefined processing order. Add an explicit boundary in the cycle:

- Audio feedback: use `audio_delay_one_sample` or `block_delay`.
- Control feedback: use `control_delay`, a smoothing stage, or a tick/block boundary.
- Event/script feedback: events are queued for a future block rather than recursively processed in the same step.

Cycle diagnostics include the participating `module.port -> module.port` path so the missing boundary can be found directly.

## Many-To-One Routing

Multiple sources cannot implicitly feed a single-value input. Route those sources through an explicit mixer or summing module first, such as `control_mixer` for control signals or `audio_mixer` for audio signals.
