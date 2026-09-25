## Context

The existing Rust sound workbench deterministically renders a declarative fixture, analyzes overlapping Hann-window frames, and can inspect an external 16-bit PCM WAV. The JUCE Sound Lab already owns an offline worker and presents a coherent render artifact through the embedded web editor. The maintained TB-303 patch, however, has no public parameter surface, no objective function, and no optimization loop.

This proof-of-concept must remain useful when later reference libraries contain stock and Devil Fish TB-303 recordings, MS-20 recordings, and software-reference renders. It therefore needs instrument-neutral matching and proposal contracts rather than a one-off “303 detector.” AI is advisory: it can suggest more complex module layouts, but only deterministic local code may decide whether a patch is structurally valid.

## Goals / Non-Goals

**Goals:**

- Recover declared public parameter values from aligned reference audio with deterministic, measurable offline search.
- Make the spectral objective and its component scores reusable independently of the optimizer.
- Preserve reproducibility and provenance in one coherent result.
- Support AI topology suggestions through a provider-neutral Rust trait and canonical data model.
- Use Codex CLI with saved ChatGPT authentication as the initial provider, without requiring an API key or linking an OpenAI SDK.
- Extend the existing Sound Lab worker/bridge/resource pattern for progress, cancellation, comparison, acceptance, and proposals.

**Non-Goals:**

- Claiming physical TB-303 accuracy or replacing future hardware capture work.
- Differentiable DSP, sample-by-sample waveform imitation, or automatic primitive implementation.
- Sending audio files to an AI provider.
- Applying AI-generated topology without explicit future user action.
- Running optimization or AI inference in a DAW audio callback.

## Decisions

### Match only declared public parameters

The fixture gains a `matching` section naming public numeric parameter IDs, the comparison region, seed, evaluation budget, spectral windows, and weights. Candidate values are applied to a cloned patch through the same declared target mapping used by presets. Undeclared YAML values never become optimizer degrees of freedom.

This gives stable IDs, explicit bounds, and a direct acceptance path. Searching arbitrary numeric YAML scalars was rejected because it would tune render settings and implementation details, make result manifests brittle, and bypass the public instrument model.

The maintained TB-303 patch will expose cutoff base, resonance, filter-envelope amount/decay, accent brightness, amplifier release, and slide time. The first fixture searches the four controls most relevant to brightness and spectral collapse; the remaining controls are available for later fixture families.

### Use an independent objective and a small seeded evolution search

The objective compares one declared, sample-aligned region. A single least-squares gain is fitted for the whole candidate region and frozen for all terms. Three Hann-window STFT resolutions are accumulated into logarithmic frequency bands and compared in log magnitude. Existing overlapping analysis supplies RMS-envelope and centroid-trajectory errors. Silent frames are masked consistently and each component remains separately visible.

The optimizer operates in normalized `[0, 1]` coordinates. It evaluates defaults first, then a deterministic seeded population around the best point while reducing search radius after stalled generations. This lightweight evolution strategy is sufficient for the proof and avoids choosing a permanent optimizer dependency before measurements justify CMA-ES or Bayesian search. The objective is a closure, so a later optimizer can replace it without changing rendering, UI, or result formats.

Optimization evaluates candidates serially to bound memory and CPU use. Cancellation is checked between renders. The initial UI budget is deliberately small; future batch/studio tooling can use larger budgets outside a plugin process.

### Keep one owned match artifact across Rust and JUCE

Rust owns a synchronous match operation and returns an opaque handle containing either a diagnostic or a coherent result. A progress callback reports evaluation counts and best score and returns whether work should continue. Read-only getters expose a bounded JSON manifest plus reference/candidate WAV bytes. Best parameter IDs and absolute/normalized values are exposed as typed getters so acceptance does not parse JSON.

`SoundLabController` continues to own the only background `std::jthread`. Its state expands to rendering, matching, matched/cancelled, proposing, proposal-ready, and error. The callback updates cheap progress under a mutex and increments the existing generation counter. A stop request is translated to cooperative Rust cancellation. This retains the testable UI-independent controller seam.

### Accept through the existing public parameter surface

Acceptance explicitly loads the fixture's patch as the active instrument, then applies each absolute best value through the JUCE host parameter mapped to its public ID. The fixture and repository YAML remain unchanged. This makes “accept” audible and host-visible while preserving the distinction between a patch definition and a preset-like result.

### Define one provider-neutral proposal model

`GraphProposalProvider` exposes provider ID/capabilities and one cancellable `propose(request)` operation. `GraphProposalRequest` contains aggregate reference/candidate features, residual scores, an allowed primitive catalogue, a sanitized list of current modules/cables/public controls, and constraints. `GraphProposalResponse` contains patch YAML, an explanation, and suggested public parameter IDs. Orchestration sees only these types.

Provider output is parsed with unknown-field rejection. Before a response is marked valid, Dandrum rejects assets and script modules, parses YAML through `load_patch_str`, validates it, and runs normal preparation. This deliberately treats model output as untrusted input. It is never loaded automatically.

### Invoke Codex as an isolated local process adapter

`CodexCliGraphProposalProvider` invokes `codex exec` in a fresh temporary non-repository directory with `--ephemeral`, `--sandbox read-only`, `--ignore-user-config`, `--ignore-rules`, `--skip-git-repo-check`, `--output-schema`, and `--output-last-message`. The canonical prompt is written to stdin. `OPENAI_API_KEY` and `CODEX_API_KEY` are removed from the child environment so the adapter relies on the existing ChatGPT login rather than silently using direct API credentials.

The process receives no repository directory and no reference audio/path. Stdout/stderr are redirected to bounded temporary files, the response is size-limited, and timeout/cancellation kills the child. A command-runner trait permits exhaustive unit tests without invoking Codex or the network. The provider may later be replaced by another CLI, a local Ollama/LM Studio mode, or another implementation of the same graph-proposal trait.

### Extend the current bridge rather than embedding an optimizer in JavaScript

JUCE owns file selection and background lifecycle. JavaScript receives serializable state, renders progress and comparison plots, and chooses reference/candidate resource URLs. Dynamic resources remain in-memory and generation-addressed. The web layer never opens paths, spawns providers, or calculates the objective.

## Risks / Trade-offs

- **A small evolutionary budget can miss a good parameter combination.** → Preserve seed/history/components, expose the budget in the fixture, and treat this as an extensible proof rather than a fidelity claim.
- **Multi-resolution FFT work can take tens of seconds in a plugin.** → Compare one declared representative region, process candidates serially, show evaluation progress, and support cancellation.
- **Least-squares gain can hide global loudness differences.** → Report the fitted gain and preserve RMS trajectory differences; later calibrated reference suites can pin output gain instead.
- **A Codex call still uses a remote service even without direct API calls.** → Label the provider accurately, require existing ChatGPT login, keep the core provider-neutral, and make absence/failure non-fatal.
- **Model output can contain hostile paths or executable script text.** → Send no repository context, run Codex in an empty read-only sandbox, reject assets/scripts, strictly parse the response, and validate locally without applying it.
- **Accepting changes the plugin's active instrument.** → Make acceptance explicit, route it through the existing replacement transaction, and never edit source YAML.
- **The current PCM loader accepts 16-bit WAV only.** → State the format in diagnostics; higher-resolution studio ingest and conversion can be a later capability without changing the matcher contract.

## Migration Plan

1. Add the fixture matching declaration and TB-303 public parameter surface without changing its default sound.
2. Land and test the standalone objective, search, and result model plus a workbench CLI command.
3. Add the provider-neutral contract and tested Codex CLI adapter.
4. Add FFI/controller lifecycle, then extend the Sound Lab bridge and page.
5. Run full Rust/native verification, coverage, focused mutation testing, strict OpenSpec validation, scenario mapping, and independent completion review.

Rollback is removal of the new matching/proposal modules and UI bridge additions; the existing render/analyze commands and Sound Lab render remain compatible.

## Open Questions

- Hardware capture may justify calibrated gain, pitch/slide trackers, resonance-peak metrics, and larger optimizers; those will be added from measured fixtures rather than assumed now.
- Proposal persistence and visual graph editing are intentionally deferred until a validated proposal can be reviewed in a proper patch-authoring workflow.
