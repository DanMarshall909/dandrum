## Context

Every stateful DSP processor exposes `reset()`, but nothing calls it, so the methods
carry `#[allow(dead_code)]`. Investigation found two distinct, both-legitimate reasons
the code is unreachable, and they need different handling:

- Voice-scoped modules (`ExecutionScope::Voice`, e.g. `filter`, `envelope_follower`)
  have their `PerModuleState` built once per voice slot and reused across notes
  (`polyphony::build_polyphonic_states_from_compiled`); note-on does not reset it.
- Global effects (reverb, echo, dynamics, convolution, spectral, saturator,
  frequency_splitter) are shared across the whole graph; resetting them per note would
  destroy their tails, and there is currently no engine-level lifecycle reset that
  would legitimately clear them.

## Decisions

- **Split reset by scope.** A per-voice reset runs on note allocation and touches only
  voice-scoped state. A separate engine reset/panic clears everything, including global
  effect tails. The two never conflict: normal note activity must not reset global
  effects.
- **Dispatch through `PerModuleState`.** Add reset dispatch on `PerModuleState` (per-voice
  variants) and a graph-level cascade for global state, rather than reaching into each
  processor from the allocation path, keeping the match in one place.
- **FFI additive only.** Existing `dandrum_*` exports stay byte-for-byte; the host reset is
  a new symbol.

## Trade-offs / Risks

- Resetting voice state on retrigger is an audible behaviour change (removes rare
  retrigger clicks) — acceptable and generally desirable, but it must be scoped to
  voice modules so it does not silence reverb/echo tails mid-performance.
- Deciding "which modules carry resettable state" must be explicit; a stateless module
  gaining a no-op reset is fine, but a stateful module missing from the dispatch would
  silently keep bleeding — task 1.1 makes the audit explicit.

## Out of scope

- Oscillator phase reset on note-on (free-running oscillators are a separate, deliberate
  design choice and not part of the `reset()` family addressed here).
