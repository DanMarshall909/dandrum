# Execution Flow Diagrams

## Current Bug — Global Consumer Runs Before Voice Producer

```mermaid
flowchart LR
    subgraph CompiledPatch.execution_order
        direction LR
        G1[global_a<br/>audio_mixer]
        G2[global_b<br/>audio_output]
        V1[voice_c<br/>oscillator]
    end

    subgraph process_block_compiled current
        direction TB
        Step1["1. Iterate execution_order<br/>globals first"] --> Step2["2. Process audio_mixer<br/>reads oscillator output → SILENCE"]
        Step2 --> Step3["3. Process audio_output<br/>accumulates silence → SILENT OUTPUT"]
        Step3 --> Step4["4. Process oscillator<br/>produces audio → TOO LATE, already consumed"]
    end

    V1 -.->|should feed into| G1
    G1 --> G2
    style V1 fill:#f96,stroke:#333
    style G1 fill:#69f,stroke:#333
    style G2 fill:#69f,stroke:#333
```

## Fixed — Two-Phase Voice-Then-Global Processing

```mermaid
flowchart LR
    subgraph CompiledPatch unchanged
        direction LR
        EO[execution_order<br/>globals → voices<br/>unchanged]
        VI[voice_node_indices<br/>[voice_c]]
        GI[global_node_indices<br/>[audio_mixer, audio_output]]
    end

    subgraph process_block_compiled fixed
        direction TB
        Seed["Phase 0: Seed MIDI/events<br/>into all_outputs"]
        Phase1["Phase 1: Iterate voice_node_indices<br/>Process oscillator → store in all_outputs"]
        Phase2["Phase 2: Iterate global_node_indices<br/>audio_mixer reads oscillator from all_outputs<br/>audio_output reads mixer from all_outputs"]
        Extract["Extract left/right from<br/>audio_output in all_outputs"]
    end

    Seed --> Phase1 --> Phase2 --> Extract

    VI -.-> Phase1
    GI -.-> Phase2

    style Phase1 fill:#f96,stroke:#333
    style Phase2 fill:#69f,stroke:#333
```

## Comparison — Raw Polyphonic vs Compiled (Fixed)

```mermaid
flowchart LR
    subgraph raw process_block_polyphonic
        direction TB
        RS["Build voice_seq & global_seq<br/>from raw topological_sort"]
        RP1["Phase 1: Per-voice loop<br/>voice_seq with per-voice states<br/>accumulate outputs"]
        RP2["Phase 2: global_seq<br/>consumes accumulated outputs"]
        RS --> RP1 --> RP2
    end

    subgraph compiled process_block_compiled fixed
        direction TB
        CS["Use pre-computed<br/>voice_node_indices &<br/>global_node_indices"]
        CP1["Phase 1: voice_node_indices<br/>single pass (monophonic)<br/>store in all_outputs"]
        CP2["Phase 2: global_node_indices<br/>consume from all_outputs"]
        CS --> CP1 --> CP2
    end

    raw -.->|same pattern| compiled
```

## Summary

| Diagram | Content |
|---------|---------|
| 1 — Current Bug | `execution_order` iterates globals first → `audio_mixer` reads voice oscillator's not-yet-computed output → silence |
| 2 — Fixed | `process_block_compiled` uses two phases: voice phase first (produce + store), then global phase (read stored + consume). `execution_order` stays unchanged |
| 3 — Comparison | The fixed compiled path matches the same two-phase pattern already proven in `process_block_polyphonic` |
