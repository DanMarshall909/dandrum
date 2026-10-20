Frontend note-lifetime evidence, 2026-10-20

Frozen lane: /tmp/dandrum-lanes/frontend, base d1662501a375710d9892371453c2614ad96518ed.
Owned source paths and final SHA256 hashes: handoff.json. Root owns integration, review, docs, Git/index and publication. No task completion or native controller/audio claim.

Behavior: retain the note helper across unchanged instance/attachment mapping, undo and generation refresh. Preserve original press generation, delayed-On/release ordering and heartbeat. Fresh presses use the current generation. Changed instance/attachment still gets a fresh helper. Current-helper errors stay visible across document generations; retired-helper and closed-engine late errors stay suppressed.

Source-bound baseline: 40 focused tests pass; baseline-sources.json, baseline.log, baseline-run.json, baseline-v8, baseline-native-live.mjs.
RED: eight selected behavior tests, five expected failures (four missing matching Off variants and hidden current-note failure), three retirement characterization passes. red.log + red-run.json (actual Node exit1) + red-sources.json. No setup failure counted as RED.
GREEN: all48 focused tests pass, with raw V8 output and source hashes in green-*.
Ordinary: npm test139pass; npm run check passes70authoredReactfiles. Raw logs, exact commands, timings and source manifest retained.
Browser slow boundary: six existing native-transport Playwright checks pass. Root/cwd failure and Chromium UNIX socket path failure retained separately as browser-setup-failed-* and browser-launch-failed-*; corrected RAM-only configs and short alias retry passed. Alias was task-created only after explicit approval and removed after checking exact target and owned process absence. All physical cache/profile/temp/output stayed in this subtree; retained cache is untouched.

Coverage: NativeLive100%lines/87.18%branches/85.71%functions in focused Node aggregate. All changed behavior branches are exercised; eleven raw unexecuted range snippets are inherited (meter guard, waveform cache/validation/admission/staleness/results and close cancellation/error callbacks). Baseline and final detailed range records retained. No new module extracted. This is focused coverage, not a whole frontend/native guarantee.

Refactor: retained existing createNotes owner and a small scalar connection snapshot, no new abstraction. Test matrix shares the same scenario while retaining distinct held/inflight and release/close outcomes. No further refactor justified.

Fast lane remains existing ordinary Node tests; slow lane is existing registered Playwright suite. Full Node driver finished about1.3s; no lane membership changed from a single duration sample. Browser selection is supplementary production entrypoint/optional-service/generation-rendering regression evidence, not another proof of the helper contract.

Mutation excluded by recorded user direction. Same-attachment external restore, per-press nonce and native controller/audio/physicalWry acceptance remain unclaimed. No running owned process, no source modification after GREEN, no Git/index/commit/publication or task checkbox changes.
