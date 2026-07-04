I have a Rust modular synth DSP engine that runs in a JUCE audio callback. It's headless now
but will eventually become a DAW plugin (AU/VST3).

Current architecture:
- RealtimeGraphProcessor owns a CompiledPatch (index-based routing from YAML patches)
- Each audio block, the engine routes signals through modules in topological order
- Module outputs are `HashMap<String, Vec<f32>>` per module (audio/control port -> buffer)
- Inputs are gathered by summing from upstream module outputs into fresh `vec![0.0; frames]`
- 25 process_* functions (oscillator, ADSR, filter, sampler, echo, reverb, etc.), each allocating
  `Vec::with_capacity(frames)` for outputs and returning ModuleOutputs
- Polyphonic path creates per-voice HashMaps that get accumulated
- Sampler had a `sample.clone()` per block (already fixed to borrow)
- All buffers are small (64-512 f32), ~40-80 allocs per block for typical patches

Question: I want to eliminate all audio-callback allocation for plugin compliance.
I'm considering replacing ModuleOutputs with a pre-allocated arena:

  struct ModuleCtx<'a> {
      inputs: &'a [&'a [f32]],     // zero-copy from arena
      outputs: &'a mut [&'a mut [f32]],  // write destination
      events: &'a [BlockEvent],
      event_out: Vec<BlockEvent>,   // still needs dynamic sizing
  }

The arena would be sized: total_output_ports × voices × block_size, allocated once at prepare().

Tradeoffs I see:
- Arena replaces HashMap lookups with index offsets ✓
- No per-block allocation ✓
- Need memset entire arena per block (stale data) or only the used slots (tracking overhead)
- Events don't fit fixed-size well (script modules can emit variable event counts)
- ~25 process_* functions need signature changes
- dispatch.rs routing loop changes significantly

Is the arena approach the right abstraction? Would you use a different pattern?
What about events — fixed ring buffer with overflow, or SmallVec, or keep Vec?
