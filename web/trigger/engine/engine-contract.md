# Trigger engine boundary

The TypeScript declarations in [contract.ts](contract.ts) define the standalone
shell's DTOs, reversible operands and adapter API. `npm run check` checks the
concrete mock and actual Store/telemetry consumer against that interface. This package produces no audio and performs no
decoding: file metadata, peaks, analysis and performance feedback are simulated.
An eventual Dandrum adapter implements this boundary without importing React.

`getPatch()` returns an owned copy of the last accepted patch. IDs are stable
within a patch. UI components receive values and callbacks from the presenter;
the editor Store invokes adapter operations. Prepared edits return promises and
emit `patch: preparing`, then `ready` with accepted data or `failed` with a
message. Validation failure retains the previous patch. Queued edits prepare in
request order. Live `setParam` and `setMacro` writes apply synchronously.

Regions use normalized positions in the asset, including fades and loop
crossfade. Rule notes and roots use MIDI integers 0–127; velocity uses 1–127.
Parameters and macro values use their declared range (the mock's parameters are
0–1). Route amounts use −1–1. Bus meters use dBFS. The mock represents routing,
voice templates and effect parameters in its model without rendering audio.

An import returns loaded or unsupported asset DTOs after progressive `asset`
events. WAV, AIFF, FLAC, MP3 and OGG are supported mock types; AAC/M4A ends
unsupported. Relinking changes an existing asset. Analysis returns a job
immediately and subsequently emits running progress, done results, failed or
cancelled. Analysis never edits regions; applying a result is a separate command.
Patch replacement invalidates jobs and old loading completions.

`mapAssets` builds regions, sounds, rules and selectors from existing loaded
resources in one prepared edit. `importSamples` combines resource import and
mapping (or selected-region replacement) so the editor records one reversible
operation. These shell convenience operations are part of the handoff adapter.
The four policies are sequential keys, root plus velocity, stack and round robin.
Explicit scientific note names use C4 = MIDI 60; explicit `note`, `key`, `midi`,
`vel`/`velocity`/`v`, dynamic and `rr` tokens are recognized. Other filename
numbers are not interpreted as pitches. The dialog shows the resolved ranges.
The parser's naming conventions are product choices, while MIDI key/root and
velocity semantics follow the [SFZ key](https://sfzformat.com/opcodes/key/),
[root](https://sfzformat.com/opcodes/pitch_keycenter/) and
[velocity](https://sfzformat.com/opcodes/lovel/) definitions.

The separate telemetry reader receives approximately 30 updates per second:
voices, active notes, playheads, selector positions, modulation activity, bus
meters and host automation. It has its own subscriptions and does not cause
editor-store rerenders. Audition and computer keys create silent simulated
voices and advancing playheads. Replacement clears playback and publishes fresh
zero-state feedback; adapter closure cancels timers and pending preparation.

The Store retains reversible entity operations, affected order operands and
per-control values and nested entity orders. It never retains entire-patch snapshots as undo entries.
Explicit pointer gestures merge their changes; rapid edits of the same control
merge through 600ms. Different controls stay separate. A queued parameter command
captures its inverse at first successful execution and retains it through redo.
Composite sample replacement owns both the region and Source-history edits.
Undo and redo wait for
successful preparation before transferring history entries. New/load/reload
clear history only after successful replacement; the presenter confirms when
discarding unsaved edits.

`applyDelta(UndoOperand[])` is a required EngineAdapter method: every structural
undo/redo replays owned entity/property/order/dictionary/metadata operands through
atomic preparation. It validates identities and paths before publishing. Native
adapters must implement the same reversible boundary; it is not mock-only.

Optional `MockDiagnostics` (`getLog`, `clearLog`, `setMockSwitch`) is injected into
Store separately. The standalone editor/catalog inject MockEngine's diagnostics;
an adapter alone needs no mutable log or switches. Failure, missing, unsupported
and host switches are diagnostics. `getJobs` and `close` remain mock-owner helpers.
The adapter owns periodic telemetry when subscribed through `on('telemetry')`;
clients never reach into a simulator to start it. Unsubscribing the mock's last
telemetry listener stops its timer. See the feature audit for callback ownership
and executable verification.


`auditionSource` previews the selected asset/region, nested selector or Lush
module directly, bypassing neighbouring trigger rules. It is a silent mock
convenience; native adapters must audition that same source identity.

Envelope seconds/linear sustain remain canonical; visible fields adapt to ms/s
and %/dB, with unchanged formatted commits preserving the original value.
`mock-wave.mjs` provides a deterministic signed fixture for zero-crossing snap,
not decoded asset data. Slice history retains derived region/rule identities,
per-slice properties and count/method. Bypass, regeneration and undo update all
owned operands. Collapsing records the source, trim, fade and reverse transform
and redirects derived regions to the simulated rendered asset.

Voice/FX policy fields are adapter DTO values. The mock simulates selection,
voice count/decay and meters; it does not synthesize sound or fully emulate host
transport, glide/stealing, convolution or dynamics. These remain native adapter
responsibilities.
