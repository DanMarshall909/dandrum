# Stopped normalized endpoints — 3 November 2026

The real VST3 zero-sample pitch flush lost normalized endpoint precision by
converting through physical units. The next ramp's independently asserted signed
sample was0.1679762 instead of0.16797613; this semantic RED is retained.

The common runtime now validates a bounded unique normalized scalar batch,
applies it through the existing stopped scalar-prefix policy and preserves exact
normalized endpoints only when admission succeeds. Both physical/discrete and
normalized entrypoints share one infallible application helper after whole-batch
validation. VST3 retains only a fixed root-index array; its copied descriptor
strings/ranges and scalar conversion path were removed. CLAP retains physical
step behavior. No DSP rendering or native-session edit occurs in this path.

Six actual ABI curve tests pass, including a stopped point plus note followed by
a ramp: the stopped note does not play, first PCM stays-.5, and the next sample
matches the raw normalized endpoint. Owning tests reject malformed/duplicate/
excess scalars without consuming queued UI work; accepted flush settles that work
and leaves the complete native SessionSnapshot(YAML/revision/Undo/Redo) unchanged.
The next note begins at literal signed-1 stereo, proving unchanged DSP time.
Real malloc/free callback sentinels, complete current dynamic regressions, owning
fast suite and strict Clippy pass. Admission-closed restoration is also tested.

These are accumulated TDD coverage logs, not final fresh-profile/full-lane
completion evidence. Host feedback/gestures remain task3.2 work. Portable IBStream
state, actual production editor destruction/recreation, installed REAPER,
unfiltered validator and combined independent Astra review/publication remain
mandatory. All binaries/full LLVM JSON remain private.
