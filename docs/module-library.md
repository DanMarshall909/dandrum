# Module library

A **defined module** is a graph assembled in YAML from primitives and other defined modules. It can live inside a patch's `module_definitions`, or in a reusable package. The public YAML keys remain `module_definitions` and `type`.

Patch roots and defined modules use the same graph shape: declared `ports`,
`modules`, and `connections`. A root audio port names a host bus; the port's
channel count must match the host declaration. A module's public port maps to
an internal port with `maps_to` or `maps_from`. `static_params` declare
construction-time arguments that an instance supplies in `static`; control
port values belong in `defaults` and can be replaced by a cable. A module
input that accepts multiple cables declares `multiplicity: summing`; otherwise
one incoming cable is allowed.

## Package and reference

A package is a folder with an entry YAML file of the same name. Resources named inside the entry resolve relative to that folder:

```text
my_kit/
  my_kit.yaml
  samples/hat.wav
```

Set a module instance's `type` to an exact macro-qualified entry path. The [drum voice example](../examples/patches/module-drum-voice.yaml) uses the bundled package, while the [two voice example](../examples/patches/module-drum-voice-duo.yaml) instantiates that same package twice:

```yaml
modules:
  - id: voice
    type: $LIB/1.0.0/drum_voice/drum_voice.yaml
```

`$LIB` is the immutable standard library. The bundled `1.0.0` release includes `drum_voice` and `drum_machine`; preparation checks its CRC and extracts it when needed. By default it lives at `<home>/.dandrum/lib`. Set `DANDRUM_MODULE_LIBRARY_ROOT` to use another directory. The engine never seeds it from the audio callback.

`$USER_LIB` is a mutable directory for your packages. It defaults to `<home>/.dandrum/modules`; set `DANDRUM_USER_LIBRARY_ROOT` to override it. A user package can be referenced as `type: $USER_LIB/my_kit/my_kit.yaml`. The engine does not seed or replace files there.

Pin a version such as `1.0.0` when a patch must reproduce the same package. `$LIB/latest/drum_voice/drum_voice.yaml` resolves to the newest numeric version found locally, so it can change after a library upgrade. Unknown macros and paths containing `..` are preparation errors.

## Resources and note voices

Sample and impulse-response paths are typed static resource arguments. For
example, a sampler instance can use:

```yaml
modules:
  - id: sample
    type: sampler
    static:
      sample: { kind: sample, path: samples/hat.wav }
```

The resource resolves relative to the YAML file that declares it, including a
package entry. The loader rejects absolute paths and paths that escape the
package. Repeated uses of one resolved resource share decoded sample data,
while each module keeps its own playback state.

For independent overlapping notes, wrap a defined module in `poly` with a
fixed `max_voices` and an `allocation` policy. See the
[polyphonic chords patch](../examples/patches/polyphonic-chords.yaml). A `poly`
region has its own note input and voice state; sibling regions allocate notes
independently. For a cycle inside a graph, route every feedback path through
`feedback_delay`. An ordinary delay effect does not establish a legal feedback
boundary.

## Render the example

From the repository root:

```bash
CARGO_TARGET_DIR="$PWD/build/rust-target" $HOME/.cargo/bin/cargo run \
  --manifest-path src/rust-engine/Cargo.toml \
  --bin dandrum-cli -- render examples/patches/module-drum-voice.yaml \
  --output /tmp/dandrum-library-drum-voice.wav --duration-frames 4800
```

The CLI supplies render settings, seeds `$LIB` during preparation, and writes
the example's `left` and `right` root outputs as a stereo WAV. Inline
definitions remain supported;
[`event-routing-drum-machine.yaml`](../examples/patches/event-routing-drum-machine.yaml)
uses a named `drum_voice` definition and a stereo `master` root output. Extra
named audio outputs are written to separate WAV files.
