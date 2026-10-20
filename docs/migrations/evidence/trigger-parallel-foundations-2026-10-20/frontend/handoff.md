Frontend task 3.1 evidence, frozen base a116c2c

Workspace: /tmp/dandrum-lanes/frontend
Runtime: Node v22.19.0; npm 10.9.3; Vite 6.4.4.
Lockfile SHA256: 07d6d015bc9148c1f55f3146f38200f835727c0fc4bbe19cf7a415529ba342d9

Contract coordinated with protocol agent: window.__DANDRUM__ = {protocolVersion:1,backendId:'juce-free',backend:{getNativeFunction,addEventListener,removeEventListener}}. Callable bridge preserves existing camelCase methods and positional arguments. Capability rejection retains Error.code='capabilityUnavailable' and Error.method. Protocol envelope serialization/reply projection remains task 3.3. The JUCE comparison bridge is selected through the same frontend selector when __DANDRUM__ is absent.

Characterization: existing JUCE default selection, exact parameter writes, feedback and listener cleanup; string/status rejection and absolute preset path. Current pre-edit baseline measured native-engine/native-live at 100% lines. Baseline sandbox test/coverage failed existing component-guard because spawnSync was rejected with EPERM; the exact final maintained driver passes under approved subprocess execution. No guard implementation was changed.

RED/GREEN: selector-red-detail.log pins versioned backend identity/precedence (expected juce-free, observed undefined); validation-red.log pins invalid protocol/malformed descriptor refusal. selector-green-coverage.log and validation-green-coverage.log retain per-criterion coverage.

Validation, all commands cwd web/trigger:
- NODE_V8_COVERAGE=/tmp/dandrum-evidence/frontend/v8 npm run test:coverage: exit 0, 102/102, 1938.282258ms. final-coverage.log + v8/*.json. Selector 100% lines/branches/functions; NativeEngine 100% lines, 84.53% branches, 90.43% functions. Node does not instrument JSX entrypoint; its behavior is checked by browser assertions.
- npm run check: exit 0; check.log.
- npm run build: exit 0, Vite 2.00s; build.log.
- npm run build:native: exit 0, Vite 2.16s; build-native.log. Existing >500kB native chunk warning retained.
- npm run test:browser -- --config=/tmp/dandrum-evidence/frontend/playwright.config.mjs tests/browser/native-transport.spec.mjs tests/browser/native-frame.spec.mjs: exit 0, 5/5, 5.7s; browser.log.
- npm run test:browser -- --config=/tmp/dandrum-evidence/frontend/playwright.config.mjs: exit 0, 31/31, 1.4m; browser-full.log. Includes retained JUCE real viewport fixture, all silent mock editing behaviors, new v1 path, visible invalid-version/capability errors and silent preview.
- git diff --check: exit 0.

Fast lane: npm test / test:coverage, about 2s. Slow lane: existing test:browser, about 1.4m. Existing catalog slow fixture took 25.5s, remains its distinct real-browser boundary; no lane membership changed on one run.

Refactor decisions: production backend detection extracted once into a 100%-covered selector shared by NativeEngine and React launch; no new transport facade or duplicate protocol serializer was added. Tests use existing native-fixture and a table for malformed contracts. Two short window setups remain direct rather than introducing fixture infrastructure. Browser fixture intentionally models callable JS contract only; it is not calibrated proof of Rust protocol serialization or a native WebView. Native boundary calibration belongs to task 3.3.

Resources: launch disk 42GiB free, shared installed node_modules 111MiB. A first default Vite browser run could update shared .vite cache; dependencies were never installed/edited. Corrected setup uses a local node_modules directory of read-only dependency symlinks with owned .vite/.vite-temp. Owned caches/build outputs preserved for integration; raw evidence ~16MiB incl 15MiB source-bound V8. Browser runner completed and shut its task-owned port 8343 server down. No source tasks/specs/docs, Rust or CMake changed. No commits/push. Mutation excluded by recorded user direction; no mutation claims.

Intended files:
M web/trigger/engine/native-engine.mjs
M web/trigger/main.jsx
new web/trigger/engine/native-transport.mjs
new web/trigger/tests/native-transport.test.mjs
new web/trigger/tests/browser/native-transport.spec.mjs

Pending: actual native session snapshot/result projection, meter capability policy, host title/resize policy, JSON envelope/reply/event wiring and WebView destructive cleanup task 3.3. No end-to-end JUCE-free editor readiness claimed.
