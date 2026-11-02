# Trigger CLAP: named DAW acceptance

Task 7.3 of `add-juce-free-trigger-beta-products` was exercised on 2 November
2026. Two installed Trigger instances accepted selected WAVs, independent native
control edits, timed host automation, editor loss/reopening and fresh saved-project
restoration. The acceptance exposed two automation-boundary defects, both repaired here.
Tasks 8.1 and 8.2, including full beta audit/qualification, remain open.

## Qualified host and package

- REAPER **7.84 Linux x86_64**, revision `4b6323` (28 October 2026), portable
  evaluation; no registration or purchase. Official archive SHA-256:
  `d87b39862bec6a82bbb530731ae1fa36bba4bc9954ff72518a187420eed9ec7d`.
- Required setting: **General / Advanced / Enable IME support** (`linux_ime=1`).
  This initializes GTK before the plugin. With the default setting, creating the
  GTK editor leaves the host's windows black. That configuration remains unqualified.
- Owned Xvfb/Xfwm4 X11 session, 1600×1200, scale one; actual XTest pointer/keyboard
  events and native GTK file chooser. Existing PipeWire/JACK: 44.1 kHz, 1024 frames.
  Offline renders explicitly use 48 kHz. This is automated native input;
  [task 7.2](../trigger-clap-native-overlap-2026-11-01/README.md) retains the separate
  physical-input and callback-overlap evidence.
- Live timing observations disable anticipative FX on both tracks
  (`I_PERFFLAGS=2`), so the host parameter cache can be compared with transport
  position. Default anticipative processing exposed future values in that cache;
  it did not invalidate the independently checked offline rendering.
- Installed product SHA-256:
  `fd24495233b94466a72de31ec302fd0089f9a4293077766ee533cda77aa552d8`.
  [installed-manifest.json](installed-manifest.json) binds all 20 package files.
  The final maintained release rebuild has this same product hash. Actual
  `/proc/<pid>/maps` receipts bind both DAW processes to this installed path.
- Qtractor 1.6.5's official AppImage starts but cannot load this plugin in its
  bundled library environment (`libgtk-3.so.0: undefined symbol: atk_plug_set_child`).
  Direct loading under the same environment reproduces it. This AppImage is
  unqualified; other Qtractor packages were not assessed.

Official references: [REAPER downloads](https://www.reaper.fm/download.php),
[REAPER scripting API](https://www.reaper.fm/sdk/reascript/reascripthelp.html),
[CLAP event definitions](https://github.com/free-audio/clap/blob/main/include/clap/events.h).
The retained download identities, host configuration and raw loader failures are
in [raw-evidence.tar.gz](raw-evidence.tar.gz).

## Acceptance evidence

| Clause | Observed result and retained artifact |
| --- | --- |
| Balanced native control gesture | A real B `drums.level` drag holds **1.21** through the middle of a held pointer, then returns to host automation **0.75** after release; A remains **0.25**. `final/native-gesture-automation.tsv` and its PASS receipt retain the observations. Exact one-begin/one-end counts and public ID are independently asserted at the real DOM/rendered-browser boundary and native CLAP output boundary. There is no raw CLAP event-count capture from the uninstrumented final DAW process; this is composite evidence. |
| Timed automation and feedback | With both editors closed, A/B follow **0.5/1.5**, then **0.25/0.75**. A reopens during playback; B stays closed. `final/live-editor-lifecycle.tsv`, PASS receipt and `editor-feedback-receipt.txt` bind host values/open state at 2.902494331 seconds. `final-live-reopened-feedback.png` shows the reopened production editor. |
| Independent edits | Native chooser imports distinct literal PCM fixtures into A and B. A's complete state is byte-identical before/after B's import. The B drag leaves A's automation unchanged. Separate A-only/B-only renders match their own signed expected PCM, rather than merely differing from each other. |
| Fresh selected-sample/parameter restoration | Original selected files are hashed, sealed in the saved project, then deleted; original PID 200747 exits 0. Fresh PID 236871, with new cache/tmp directories and no instrumentation, restores two **114,028-byte** state payloads byte-for-byte, including logical document/resources and all **23 named physical values** per instance. `final/fresh-state-verification.json`, sealed states, restored states and values retain the comparison. Fresh process exits 0. |
| Signed audible behavior | All **96,000 channel samples** in each of four real REAPER renders match the independent literal PCM/time/gain oracle, including zeros outside two 256-frame note onsets. `final/render-verification.json` and WAVs retain the result; each render's checker rejects a controlled sign reversal at frame 12,000/channel zero. |

The PCM16 source patterns are declared in `final/selected-fixtures.json`. Playback
uses the existing decoder's arithmetic mean of stereo channels, duplicated into
stereo output. MIDI note 48/velocity 127 occurs at 0.25 and 0.75 seconds. Square
level envelopes change at 0.5 seconds. The first four combined channel pairs are
`-0.25, 0.5, -0.28125, 0.0625`; the second onset is half those values. This oracle
does not derive expected output from the plugin or from another render.
From the extracted archive, run `python3 scripts/verify-renders.py final` to audit the retained
WAVs. The one-run live scripts record the exact pinned fixture; they are not a
maintained, portable or CI-qualified DAW runner.

## Repairs and verification

The shell's pointer handler previously chose the nearest generic slider, whose
label was `level`, before its enclosing public `data-knob="drums.level"`. Model
only tests called `Store.beginGesture` directly and missed that path. The repair
prefers named control wrappers, preserving standalone public labels. New DOM and
rendered-browser tests require one begin before writes and one end only on release.
The original lookup fails both regressions; the fixed lookup passes.

The CLAP adapter now marks editor begin/value/end events `IS_LIVE`, as specified
for live user interaction. The native recorder first failed on empty flags, then
passed with exact ordered IDs/values and live flags. Its assertion also applies
through the wider native parameter suite. Existing undo/feedback and callback
safety tests remain the corresponding boundary evidence.

Focused LLVM measurement instruments the changed adapter crate only and loads it
through the real native test host. `deliver_automation` has 4/4 mapped regions
executed; its event-conversion closure has 26/26, including all three live-flag
writes. An unrelated generic queue instantiation has one unhit region; full
adapter/host, branch and instantiation coverage are not claimed. V8 executes the
changed pointer callback; the broader input-actions owner remains partially
covered (about 67%). No module extraction is needed for these small repairs.

[verification.json](verification.json) indexes commands and results. The raw
archive retains frontend unit/coverage, browser regressions, CTest, focused LLVM,
OpenSpec/spec coverage and the strict engine gate. Normal commit/push hooks run
separately during publication; their logs remain with the task run. Mutation remains excluded by
Dan's existing direction; no mutation score is claimed. The pre-existing engine
strict-Clippy baseline and broader owner qualification belong to task 8.1.

## Failed probes and limitations

Failures are preserved with their scope. Early DAW gesture probes either used a
weak oracle (only a transient edit was required), clicked before the async editor
was ready, or ran the host in Read mode. Those are not acceptance passes. The
final probe waits for editor preparation and requires a stable held value before
release. Temporary frontend/CLAP trace instrumentation is retained separately;
it is absent from the final DAW processes and package.

Fresh live probes also caught anticipative cache lookahead, a muted B left by a
cancelled isolated-render attempt, and normal parameter chasing to the edit cursor
when playback stops. Final live evidence explicitly unmutes both tracks, disables
anticipative FX and asserts values while playback is still running. A render
warning blocked an attempted overwrite; subsequent artifacts use distinct names.
Observer races with already closed render windows and state-busy import readiness
are retained as setup failures, not product acceptance.

GTK critical messages and Xvfb EGL/DRI3 warnings remain in raw logs. No default-host,
Wayland, other DAW, device latency or unrestricted-beta support follows from this
run. Reusable builds, installed package and qualified portable DAW tools remain
under `/tmp/dandrum-task-7-3` for task 8.1; no shared cache/dependency was deleted.

The main lesson is to test public controls through their actual DOM entrypoint,
then qualify asynchronous native readiness and host buffering before interpreting
a DAW probe. A passing model test or relative audio comparison cannot establish
those boundaries.
