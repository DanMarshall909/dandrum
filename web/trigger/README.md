# Trigger standalone shell

Trigger implements the supplied eight-workspace design as an independent React
shell. The browser preview uses a silent in-memory adapter. Run the full editor
with real sample playback inside JUCE from the repository root:

```sh
./demo react-trigger
```

The native adapter initializes from the prepared Advanced Drum Kit. It supports
key/velocity mapping, forward loops, trim/fades/reverse, supported sample
selectors, bounded polyphony, reversible prepared edits, actual host parameters,
native mono/stereo WAV import, prepared waveforms and measured main-output meters.
The Voice page exposes every actual native host parameter. Unsupported voice
modules, FX/routing, modulation, analysis, zero-crossing snapping and isolated
source audition report a capability error without accepting a simulated edit.
Voice counts/playheads are not measured by this bridge and remain unavailable.
The mock Felt Kit is not presented as a real native preset.

The [engine roadmap](../../docs/trigger/native-engine-roadmap.md) records missing
capabilities, existing primitives to reuse and the control/API/DSP test boundaries.

Selected WAV files are copied beneath the instrument's authoring root in
`.tools/trigger-assets/<unique-id>/` so the Rust resource boundary can admit
them. Original files remain intact; these owned copies must remain available
for saved patches to reload. This local cache is ignored by Git.

The launcher builds `dist-native/` and embeds its script, CSS and local fonts;
the integrated editor needs no development server. Native replacement prepares
an owned YAML snapshot on the processor worker, retaining the working engine
and undo operands on failure. Browser preview instructions:

```sh
npm ci
npm run dev -- --port 8322
```

Open <http://127.0.0.1:8322> for the editor or
<http://127.0.0.1:8322/catalog.html> for the shared component catalog. Start Empty,
then load Felt Kit from Patch. Header size controls and Collapse select the four
geometries. Log exposes analysis/missing/unsupported/host mock switches.

```sh
npm run check
npm test
npm run test:coverage
npm run test:browser
npm run build
```

The browser suite uses `/usr/bin/google-chrome`. Set `TRIGGER_CHROME` to another
installed Chromium executable when needed. The `trigger-react-shell` CTest runs
check, unit, build and browser lanes; run `npm ci` and provide Chrome first.

All 70 authored React files are below 200 lines. Components receive props and
callbacks; presentation/model bindings live in `shell/`, and authoritative DTOs,
operations, commands and telemetry live in `engine/`. The received React drawing
library lives in `vendor/design-system/`; authored controls adapt it and are
reused by the shell and catalog.

See [engine contract](engine/engine-contract.md),
[feature audit](../../docs/trigger/feature-audit.md),
[platform update](../../docs/trigger/platform-spec.md) and
[verification](../../docs/trigger/verification.md). The immutable source archive
contents and font provenance are under `docs/trigger/` at the repo root.
