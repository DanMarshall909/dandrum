## Why

Dandrum can render and inspect a deterministic sound fixture, but reproducing a reference sound still depends on manual knob tuning and visual judgement. A small, reproducible spectral-matching loop will make the current TB-303 proof of concept useful as the first instance of a general sound-implementation workflow, while a provider-neutral proposal boundary can use AI for harder graph layouts without making AI output trusted engine input.

## What Changes

- Add deterministic offline matching of declared patch parameters against an aligned PCM WAV reference, using a multi-resolution spectral loss plus level and spectral-trajectory terms.
- Preserve the matching seed, configuration, reference identity, best parameter values, score breakdown, and rendered audition as one reproducible result.
- Add progress and cooperative cancellation without performing file IO, rendering, optimization, or analysis on the realtime audio path.
- Add a provider-neutral graph-proposal contract whose structured output must pass Dandrum's local schema, graph, and preparation validation before use.
- Add Codex CLI as the first graph-proposal adapter, using the user's existing ChatGPT login through an isolated, read-only `codex exec` child process rather than a direct API call or API key.
- Extend Sound Lab to select a reference, start/cancel a match, compare reference and candidate audio/metrics, inspect score and progress, accept the best parameter values, and request a validated AI topology proposal.

## Capabilities

### New Capabilities

- `spectral-sound-matching`: Deterministic offline parameter search, objective measurement, progress/cancellation, and reproducible match results.
- `ai-graph-proposals`: Provider-neutral structured graph proposals, local validation, and the initial Codex CLI adapter.

### Modified Capabilities

- `sound-lab-ui`: Adds reference selection, spectral-match progress and A/B inspection, accepted parameter results, and AI topology proposal status to the existing offline Sound Lab.

## Impact

- Rust workbench: reference loading/alignment, multi-resolution analysis, bounded deterministic optimization, result manifests, provider contract, Codex CLI adapter, and tests.
- Rust/C boundary: owned match/proposal job results and progress snapshots for the editor controller.
- JUCE plugin editor: background job ownership, reference file selection, native bridge methods, candidate/reference resources, and cancellation.
- Embedded web UI: match controls, progress, loss breakdown, overlaid trajectories, A/B playback, accepted values, and proposal diagnostics.
- Build/spec evidence: new behavior tests, coverage and mutation checks, strict OpenSpec validation, scenario mappings, native build/CTest, and independent completion review.
