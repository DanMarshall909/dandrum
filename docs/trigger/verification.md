# Trigger shell verification

Candidate source: branch `work/trigger-react-shell-2026-10-06`, based on
`origin/main` d02ee1b6. Node 22.19.0, npm 10.9.3, local Google Chrome. Package lock
is committed; npm ci reproduces dependency installation. Verification follows
implementation because the user explicitly waived test-first TDD for this shell.

## Executable checks

Commands run in `web/trigger`, unless noted:

| Command | Result and retained evidence |
|---|---|
| `npm run check` | TypeScript adapter and actual Store/telemetry consumer boundary, props-only engine import guard, JSX syntax and <200-line authored-component convention pass; 70 files, largest 99 lines |
| `npm test` | 51 behavior tests pass, no skips; ~1s; review-ctest-package-output log |
| `npm run test:browser` | 23 browser tests (catalog sweep 97×9 states, editing, all pages, 24 action states, runtime, preset/recovery, four sizes, clipped-control and precision regressions); full package CTest includes this lane |
| `npm run build` | Multi-page editor/catalog production build passes; shared React/vendor chunks; no Design Components runtime or legacy SamplerApp |
| `ctest --test-dir build -R '^trigger-react-shell$' --output-on-failure` (repo root) | Repository test entry point runs check/unit/build/browser, with actual test discovery |
| `openspec validate add-trigger-react-shell --strict` (repo root) | Valid delta change |
| `scripts/check-spec-coverage` (repo root) | Synced main baseline passes: 569 scenarios, 330 mapped, 239 unrelated ratchet entries; all 10 new capability scenarios map to the complete shell gate |
| `scripts/check-rust-coverage` (repo root) | Normal pre-commit coverage gate passes: 1,075 Rust tests, strict engine-file coverage unchanged; native-coverage-preflight log |
| Geometry browser probe | All 8 pages at Min/Default/Expanded plus Compact: exact outer dimensions, no internal horizontal auto-scroll, no visible text below 11px; zero page errors |
| Screenshot/action capture | All 24 supplied reference states and 24 ordinary-action states retained; no state picker in delivered app |

Raw logs/screenshots are retained under `evidence/final/`. The definitive full
CTest log is `trigger-review-ctest.log`. The shell's normal pipeline fails
explicitly if npm dependencies or the configured browser are unavailable.
Browser tests use an installed Chrome by default (`TRIGGER_CHROME` overrides it).

## Coverage and fault evidence

`npm run test:coverage` uses Node's coverage collector. The report includes tests
and reference projections; its aggregate percentage is not browser coverage.
The final measured aggregate is 88.37% lines / 77.79% branches / 70.94% functions;
the raw report is retained separately. Entity deltas, Store, signed fixture,
unit conversion and new slice-history/envelope-value helpers have 100% line
execution in this lane. Remaining domain guards and browser-only callbacks are
visible in the report; JSX browser execution is not instrumented. No 100% whole-
application coverage claim is made.

Five deliberate faults were injected separately in an isolated copy: breaking
the 600ms merge boundary, shifting pitch roots, omitting Slice history undo,
resetting candidate values during order undo, and removing missing-source
fallback. Every fault was rejected by its named owning behavior test; clean
baseline passed before/after and source hashes were checked. See
`evidence/final/fault-probes/report.json` and logs. This is bounded calibration,
not a full StrykerJS mutation score. The component guard separately rejects
oversized components, forbidden engine imports and invalid JSX with specific
diagnostics. No Rust behavior changed; its normal commit/push hooks remain active.

## Native environment and measurement limits

Initial CMake configure used the Homebrew linker and failed to resolve system
fontconfig/freetype dependencies. The resulting initial CTest found no tests;
that output was not treated as a pass. Configure succeeded with system GCC/G++:

```sh
PATH=/usr/bin:/bin:/home/dan/.local/bin:/home/dan/.cargo/bin:/home/dan/.nvm/versions/node/v22.19.0/bin \
/home/dan/.local/bin/cmake -S . -B build \
  -DCMAKE_C_COMPILER=/usr/bin/gcc -DCMAKE_CXX_COMPILER=/usr/bin/g++
```

The package's CTest subsequently discovered and ran its full gate. Native DSP,
C++/JUCE rendering, true host automation, real decoder/file access and disk preset
persistence are outside this standalone handoff. Browser directory chooser
matching is implemented but not automated as a native OS folder-selection test.
Visual source comparisons are manual evidence, not a pixel-difference score;
the user's 11px minimum and truthful live data cause documented differences.
See `visual-comparisons.md` and the unmodified comparison board.

## Review repairs

The first independent review returned FAIL for three reproduced P2 findings:
queued parameter inverse values, composite replacement Source-history operands,
and undeclared adapter dependencies. Its original verdict, matrices and failing
behavior probes are retained under `evidence/final/review-round1/`. The repairs
capture inverse values at execution, include history in composite replacement,
declare UndoOperand/applyDelta and inject optional MockDiagnostics separately.
The actual Store and telemetry consumer now use TypeScript checking; a restricted
adapter test proves edit/undo/redo/subscription without MockEngine internals.
Four added Node tests cover these behaviors and diagnostic injection.

The full refreshed CTest passes in 82.64s with 51 Node and 23 browser tests. An
in-memory consumer fault produces the specific compiler error for undeclared
`EngineAdapter.log`; baseline/restored compile clean and source bytes are unchanged
(`trigger-consumer-fault.log`). This is guard calibration, not a full mutation score.
No authored test setup failure is counted as behavior-fault evidence. Final
exact-candidate review passed before publication.

## Independent review and finalization

The exact candidate is frozen using a path/mode/content/index manifest and
upstream identity in the external review packet. Independent review passed for that candidate plus its finite envelope;
the manifest, verdict and matrices are retained at `/tmp/dandrum-trigger-final-review/`. The only proposed
content finalization is the exact new-spec/test-map synchronization, task 4.2
and verification status; source/test edits require a refreshed candidate and review. Existing
worktrees, warm dependencies/build caches and the active 8322 preview are retained.
