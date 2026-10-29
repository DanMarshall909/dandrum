# Bounded portable input through actual CLAP

This task6.1/6.2 component fixes the CLAP consumer's65,536-byte eager-read cap for
already-supported version3 frames. The base is
`524be5c40766897bee1aa994cd62303533df33e6`. Default packaged saves still emitv2;
packaged-onlyv3 writing/full payload qualification and complete beta acceptance
remain open. No task or beta scenario closes here.

The first genuine RED loads a fixed v2 input, verifies all23 independently named
nondefault physical values and two literal signed frames, then proves the larger
v3 frame valid through the format-neutral controller. The real copied CLAP
rejects that frame;0.23s/one test/exit101. Its102,044-byte WAV alone exceeds the
legacy limit. A second real-CLAP RED shows an unsupported version attempting
foreign payload I/O before rejection;0.04s/one test/exit101. Both preimplementation
source sets, commands and raw output are retained.

The existing codec now returns an opaque bounded StateHeader. Main parses it
without borrowing the controller, briefly preflights existing native metadata/
admission, releases the borrow, then reads exactly the validated payload extent
and checks EOF. BufferedState owns only transient input bytes. Header/body bounds
come from the same version/metadata/manifest validation; no second codec, decoder,
catalog, history, framework or input-limit policy is introduced.

Foreign reads can reenter the plugin. Final load revalidates current native
values/revision/generation and uses the existing catalog reservation/decoder and
atomic publication. A job admitted during I/O can cause final Busy after input
consumption without retiring that job. Preexisting native acquisition/structural
backpressure refuses before body reads. Legacy JSON retains its exact65KiB cap
and existing policy. save_state/default writer behavior is unchanged.

GREEN restores a fresh copied CLAP after its packaged WAV is removed, with actual
11-byte host reads that reenter parameter getters. All23nondefault values match,
MIDI36 is silent and MIDI60 renders literal[-.14536858,-.1444931] on both channels
after offset5. The source WAV's four relevant PCM words are asserted independently.
Another instance retains exact state and packaged[-.5,-.5]. Truncated/corrupt/
trailing bodies and a real host error inside the body preserve accepted data;
refused candidates request different YAML/values so rollback assertions are not
vacuous. Unsupported versions, metadata limits, foreign product and incompatible
public values refuse before any foreign body attempt.

A native owner test separately retains an actual catalog reservation, then admits
a real structural request after successful preflight. Both initial and final
Busy leave state/values unchanged, and public job status still reports running.
These are post-GREEN characterizations of the preserved ownership policy, not
invented historical REDs.

Production refactoring shares state_restore_values/restore_state between direct
and buffered loading. The complete extracted metadata checker has38executed
annotations. Tests share a local input encoder and a fixed documented v2 fixture
owned by the format-neutral host; expected values/waveforms are independent of
the writer under test. Existing adapter audit/observer fixtures remain unchanged.

Complete200host+2doctests/41session/23editor-adapter tests pass. All19actual headless
CLAP dynamic tests pass, including the allocator-audit parent that explicitly
launches its ignored child. Formatting, strict all-target host/session/editor-
adapter Clippy, active strictOpenSpec and unchanged map631/392/239 pass. Final
strict/dynamic rerun after strengthened test-only oracles passes. Engine strict
Clippy remains FAIL129 unchanged source/baseline diagnostics, unwaived.

Final LLVM binds8objects/11fresh profiles:6owner executables, actual instrumented
headless DSO and dynamic host. Added annotations18codec/32controller/6catalog/
14adapter all execute; the extracted metadata checker is38/38 annotated. Broad
owners/branches/instantiations remain partial. Execution is not itself an assertion.
Exact ELF/profile/source replay is sealed before hook cache reuse; the ordinary
headless473c3a59 artifact/resources are archived separately.

[Binding](binding.json), [raw index](raw-index.json) and [raw evidence](raw.tar.gz)
retain10source overlays, RED/GREEN, source-bound coverage and owned cleanup. No
cache/large generated audio fixture enters Git. Owned workers/scratch are cleaned;
reusable disk caches stay. The64GiB retention estimate remains exceeded without
eviction authority. Normal hooks and independent exact-candidate review/publication
remain separate gates in the external clap-portable-stream-review record.

No rebuilt GUI/official/installed/offline/native-input/continuous-selected-audio/
namedDAW proof is claimed. PriorGUI2fe/d157 official34success/10explicitSKIP/0FAIL
remains prior-product evidence. Main stays clean68dd655; mutation excluded;
CLAP-only beta10/17active and VST3/standalone deferred.
