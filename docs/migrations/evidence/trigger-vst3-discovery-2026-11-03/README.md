# Installed VST3 discovery and shared public values

Task 2.2 of `prove-trigger-vst3-linux` uses the actual pinned-binding cdylib,
loaded by `dlopen` from a temporary standard `.vst3/Contents/x86_64-linux`
bundle. Processor and controller have distinct fixed IDs. Factory metadata,
standard COM identity/reference ownership, rejection outputs and module hooks
are exercised through the exported ABI. The controller reads metadata without
constructing a session or decoding samples. Only the processor opens the sound
owner, and missing resources fail without a checkout fallback or partial owner.

Seven passing dynamic tests cover all 23 literal parameter IDs/names/ranges,
non-default values, endpoint/text conversion, malformed/nonfinite input,
reordered aliases, incompatible surfaces, missing resources, wrong bundle
layout, duplicate initialization, termination and rejected foreign-thread calls.
The processor's first successful-init attempt exposed an invalid event-capacity
request: the prepared graph supports 64 queued events here, not 1,024. The host
curve-point limit will have separate bounded storage in its later criterion.

Repeated module load/unload completed every component termination but crashed
at test-thread exit. Promoting only the verified executing object with GNU
`RTLD_NODELETE` eliminates that failure, consistent with the existing Rust/GTK
thread-local destructor residency requirement. All COM owners still release;
this retains code, not a global instrument or application loader reference.
The failed trials remain in the text archive.

Shared descriptor construction was characterized before extraction; its new
functions and conversion closures are fully executed by owning shared-host
coverage. The complete current shared-host fast suite and 11 CLAP editor unit
tests pass. Strict VST3 Clippy passes. VST factory code measures 128/128 lines
and 197/197 regions; explicit ID mapping measures 20/20 lines and 26/26 regions.
Component and Linux loader coverage is partial and recorded in verification.json;
audio/state/view implementations and defensive broken-loader cases are not
claimed by this checkpoint. Branch/MC-DC coverage is unavailable.

The accepted output-growth cap remains 32 GiB plus 2 GiB scratch. The managed
Cargo retention ceiling is raised to 80 GiB so its existing 8 GiB per-job
admission estimate cannot evict the 34.14 GiB cache present at task start.
This retention setting adds no authority to exceed the incremental cap.
Available RAM/free disk remain above the agreed 8 GiB floors. Reusable outputs,
dependencies and private binaries are preserved. Full processor messaging,
audio/curves/state/editor, official validator, native REAPER acceptance,
independent completion review and publication remain required by later tasks.
