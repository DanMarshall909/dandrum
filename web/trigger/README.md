# Trigger standalone shell

Trigger implements the supplied eight-workspace design as an independent React
shell with a silent in-memory adapter. It does not produce audio.

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
