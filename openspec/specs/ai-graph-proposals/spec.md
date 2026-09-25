## Purpose

Define provider-neutral, locally validated AI graph suggestions for offline sound design, with Codex CLI as the first adapter and no direct API credentials.

## Requirements

### Requirement: Graph proposals use a provider-neutral contract

Dandrum SHALL request graph ideas through a provider-neutral adapter contract carrying a canonical request, capability description, cancellation signal, and structured response. Matching and validation orchestration SHALL NOT depend on a provider-specific request or response type.

#### Scenario: A different provider satisfies the same orchestration

- **WHEN** a test provider implements the graph-proposal adapter contract
- **THEN** Dandrum SHALL submit the same canonical sound summary, allowed module catalogue, current topology, and requested search controls used for any other provider
- **AND** it SHALL consume the same structured proposal response without provider-specific branching

### Requirement: Provider context excludes reference audio and secrets

Graph-proposal requests SHALL contain derived sound features, score residuals, an allowed module catalogue, and a sanitized current topology. They SHALL NOT contain raw reference samples, local reference paths, credentials, API keys, or unrelated repository content.

#### Scenario: Proposal request is built from a match result

- **WHEN** Dandrum creates a graph-proposal request from a completed match
- **THEN** the request SHALL contain only the approved derived and structural fields
- **AND** it SHALL not contain the reference path or RIFF/WAV sample bytes

### Requirement: Every provider proposal is locally validated

Dandrum SHALL parse a provider's structured response, reject undeclared fields, and validate proposed patch YAML through the local patch, graph, and preparation rules before presenting it as valid. A proposal SHALL NOT be loaded into the active instrument automatically.

#### Scenario: Provider returns a valid patch proposal

- **WHEN** a provider returns schema-conforming patch YAML that passes local preparation
- **THEN** Dandrum SHALL mark the proposal locally validated
- **AND** it SHALL preserve the explanation and suggested bounded public search parameters

#### Scenario: Provider returns invalid or unsafe content

- **WHEN** a response is malformed, adds disallowed assets or script modules, names unavailable modules, or fails local patch/graph/preparation validation
- **THEN** Dandrum SHALL reject it with a local diagnostic
- **AND** the active patch SHALL remain unchanged

### Requirement: Codex CLI is the first adapter without direct API credentials

The initial provider SHALL invoke `codex exec` in an isolated temporary working directory using the user's existing ChatGPT login. It SHALL use ephemeral, read-only, non-repository execution with a JSON output schema, SHALL pass the prompt through standard input, and SHALL remove direct OpenAI/Codex API-key variables from the child environment.

#### Scenario: Codex adapter constructs an isolated invocation

- **WHEN** a proposal is requested through the Codex adapter
- **THEN** the child invocation SHALL include ephemeral, read-only, ignored-user-config, ignored-rules, skipped-git-check, isolated-working-directory, output-schema, and output-file arguments
- **AND** the canonical prompt SHALL be supplied on standard input
- **AND** direct provider API-key environment variables SHALL be absent

#### Scenario: Codex CLI is unavailable or not authenticated

- **WHEN** the executable is missing, the saved ChatGPT login is unavailable, the child times out, cancellation is requested, or the process returns unusable output
- **THEN** the adapter SHALL return a bounded diagnostic without exposing credentials or unbounded child output
- **AND** Dandrum SHALL remain usable without an AI proposal

### Requirement: AI work remains outside realtime processing

Provider process creation, network-backed inference, response parsing, and proposal validation SHALL NOT execute from Dandrum's realtime audio processing path.

#### Scenario: Realtime source remains provider-free

- **WHEN** the realtime source boundary is inspected by the build guard
- **THEN** it SHALL contain no provider adapter, child-process, prompt, or proposal-validation operations
