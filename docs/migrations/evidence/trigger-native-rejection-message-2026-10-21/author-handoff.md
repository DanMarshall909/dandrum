# Native rejection presentation follow-up

## Scope and source freeze

- Checkout: `/tmp/dandrum-lanes/frontend`
- Package cwd: `/tmp/dandrum-lanes/frontend/web/trigger`
- Base HEAD: `ca36647a619034c96bc03f1ab2ffc4e166d356d6`
- Final allowed changes: `web/trigger/engine/native-engine.mjs`, `web/trigger/tests/native-engine.test.mjs`
- Final SHA-256: `native-engine.mjs` `73f2f68dedc9cdb6a86e77ae9b3741fb26aab9dea7faf1234fe980cc6b96f4ee`; `native-engine.test.mjs` `50603050a00e0343c73af2efd99f1695316e37033acca148db86c3dda7e078e0`
- `git status --short` contains only those two modified paths. No index, branch, docs, specs, package, dependency or other source changes. `git diff --check` passes.

## Behavior and design

The production `NativeEngine.report` now presents a string `Error.message`, then a supplied string `message`, then a supplied string native `code`, then `String(error)`. It still returns without publishing after engine close. This addresses native WebView replies shaped exactly as `{code: 'capabilityUnavailable'}` while retaining messages from object, Error and string failures.

The new public-path test creates the real `createWebViewTransport`, initializes the production `NativeEngine` through its backend, rejects a `setParameter` command, and checks both caller rejection and the emitted `patch` failure. It covers code-only native reply, reply with message, Error thrown by posting and string thrown by posting. Existing same-attachment current-error test and close/instance/attachment-retirement tests still pass. No helper extraction is justified: this is one short presentation decision at the existing report boundary.

## Verification

Node `v22.19.0`, npm `10.9.3`; installed dependencies reused. No browser test was rerun and no browser selection changed.

- TDD RED on actual host with old reporter: `node --test --test-reporter=spec --test-name-pattern='native command failures preserve' tests/native-engine.test.mjs` — exit 1; the code-only and message cases emitted `[object Object]`; Error and string cases passed. Raw: `red-host.log`.
- Focused GREEN on actual host with final source: same command — exit 0, 5/5 tests including four cases. Raw: `green-host-focused.log`.
- Owning NativeEngine suite plus Node V8 coverage: `node --experimental-test-coverage --test-reporter=spec tests/native-engine.test.mjs` — exit 0, 40 tests passed. `native-engine.mjs` 100% line coverage, 82.20% branch, 87.10% function; `native-engine.test.mjs` 100% line, 99.17% branch, 99.47% function. Coverage remains incomplete elsewhere in these existing files; a pre-change coverage capture was not taken, so those gaps are not attributed to this change. The new reporter outcomes are exercised by all four cases and existing close filtering. Raw: `native-engine-coverage-host.log`.
- Full package `npm test` on actual host — exit 0, 151/151 tests. Raw: `full-npm-test-host.log`.
- `npm run check` — exit 0; TypeScript and 70 React-file component check pass. Raw: `npm-check-host.log`.
- `npm run build -- --outDir /tmp/dandrum-lanes/evidence-frontend-rejection-message/preview-build` — exit 0, 247 modules, 3.45s. Raw: `preview-build.log`.
- `npm run build:native -- --outDir /tmp/dandrum-lanes/evidence-frontend-rejection-message/native-build` — exit 0, 241 modules, 3.60s. Raw: `native-build.log`.
- Baseline same-attachment current/late note error filters passed before the behavior test was added. Raw: `baseline-focused.log`.

The first full `npm test` in the default sandbox exited 1 at `component-guard.test.mjs:13`. A child-process probe established the environment difference: `spawnSync(process.execPath, ...)` returned `EPERM` with empty stdout/stderr in the sandbox; no `NODE_OPTIONS` or `NODE_TEST_CONTEXT` was set. The exact component test passed on actual host using `require_escalated`, and the full 151-test suite passed there. No component guard/test files were changed. The sandbox failure log is `full-npm-test.log`; the child probe and direct component failure are `component-guard.log` and `check-components.log`.

## Resource and preservation notes

Before and after checks: `/tmp` filesystem 24 GiB available; `/dev/shm` 16 GiB available; memory about 12 GiB available. Each owned build output is about 1.6 MiB. Existing package `dist/` and `dist-native/` outputs were left untouched. No reusable cache or dependency was removed. No mutation test or Rust task was run.

## Corrections and retrospective baseline evidence

The earlier resource sentence above reporting `/dev/shm` with 16 GiB available was a sandbox namespace sample. It was not the actual-host sample and must not be used as host headroom evidence. The actual-host `require_escalated` sample at 2026-10-21 00:26:09 UTC recorded `/dev/shm` 16 GiB total, 15 GiB used, 1.1 GiB available; `/tmp` 478 GiB total, 430 GiB used, 24 GiB available; memory 12 GiB available. See `actual-host-resource-process.log`. The same sample found no active test, build, Vite, Cargo or rustc task process; no old cache or build was touched. Output builds were directed to the owned `/tmp` evidence directory, each about 1.6 MiB.

The initial implementation did not capture baseline coverage before editing. That project requirement was missed and is not represented as completed chronologically. At root's request, I captured an honest retrospective baseline: `git show ca36647a619034c96bc03f1ab2ffc4e166d356d6:<path>` supplied the exact two original preimages; the saved copies are under `baseline-source/` with SHA-256 `fdb765ac93834b14922d1577475c8d5e2a1078f03fb0fa6fd8a4410f81efec16` (`native-engine.mjs`) and `71fd785bce9da4ee62e98e27d809d97232000171249688f4abb78062d16e0be6` (`native-engine.test.mjs`). Final preimages are preserved under `final-source/` and match the final hashes above.

For the retrospective run, both current files first matched their saved final SHA-256 values. A guarded swap installed only the exact baseline bytes, then ran from `/tmp/dandrum-lanes/frontend/web/trigger` on the actual host: `node --experimental-test-coverage --test-reporter=spec tests/native-engine.test.mjs`. It exited 0 with 35 tests passing in 1.923 seconds. Baseline `native-engine.mjs` coverage was 100% lines, 81.32% branches and 87.10% functions; the whole measured file set was 85.22% lines, 75.70% branches and 76.52% functions. Raw report: `baseline-native-engine-coverage-retrospective.log`. A guarded `finally` restore verified both final hashes exactly after the run.

The exact old reporter was also rerun against the new public behavior test on the actual host, producing `red-host.log`: the code-only and supplied-message object cases failed with `[object Object]`, while the Error-thrown and string-thrown cases passed. This gives a truthful characterization of those two existing fallbacks against the old reporter; it does not claim the new scenarios existed in the exact HEAD baseline suite. The final-source focused suite and full package suite were rerun after restoration: `final-green-focused-host.log` is 5/5, `final-full-npm-test-host.log` is 151/151, and `final-npm-check-host.log` passes. The final sources still hash to `73f2f68dedc9cdb6a86e77ae9b3741fb26aab9dea7faf1234fe980cc6b96f4ee` and `50603050a00e0343c73af2efd99f1695316e37033acca148db86c3dda7e078e0`.

Receipt details (host/environment, command, cwd, baseline and final source hashes, exit codes and durations) are in `baseline-coverage-receipt.txt` and `actual-host-resource-process.log`. No repository source outside the original two files changed during the retrospective swap.

Post-verification actual-host sample at 2026-10-21 00:27:28 UTC is preserved in `actual-host-post-check-resource-process.log`: it again records `/dev/shm` 1.1 GiB free, `/tmp` 24 GiB free, and memory 12 GiB available. Node v22.19.0 reported `NODE_OPTIONS` and `NODE_TEST_CONTEXT` unset. No owned test/build worker remained active.
