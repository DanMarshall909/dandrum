# Advanced sampler visual verification

final result: passed

The 16 groups from the original Slint/React visual audit have been corrected
and rechecked. All 43 visual-publication headless checks pass, and the final
matrix contains 88 reviewed pairs with matching resolution and render density.
No actionable P0/P1/P2 finding remains in these 16 groups. Font rasterization
and minor optical icon differences remain P3 polish. This result covers visual
fidelity; the broader audit retains 28 pending controls and real audio integration.

## Comparison evidence

Source visual truth: [Advanced Sampler v3](docs/design-system/advanced-sampler/reference/Advanced%20Sampler%20v3.dc.html),
SHA-256 a72240e95d4c944aac66709c824fcc2b51f7d9cbe8d1da250d6f6c596cd89119.
Source HTML, supplied fonts and original reference screenshots are unchanged.

Final evidence is retained locally under
build/advanced-sampler-evidence/fidelity-fixes-cycle-6/: index.html, report.md,
comparison-manifest.json, root-review.json, review-notes.json, pixel-metrics.json,
gallery-verification.json, pairs/, react/, slint/ and supplemental scale folders.
Raw capture artifacts remain outside Git.

The matrix contains 88 matched pairs: 13 workspace pages at three sizes,
30 operational/design states, all eight surfaces in both finishes, and three
supplemental Sample density checks. Root directly opened all 12 changed pairs
in cycle6. The other 76 pairs reuse prior review only after BOTH original hashes
were verified equal to reviewed cycle5 originals. Cycle5 opened 10 changed
pairs and verified 78 exact reuses from cycle4; cycle4 opened 85 distinct pairs
and reused three exact duplicates. React is left and native Slint is right.

| Logical editor size | Original pixels in BOTH renderers | Capture scale |
| --- | --- | --- |
| 820×560 minimum | 820×560 | 1× |
| 1200×800 default | 1200×800 | 1× |
| 1600×1000 expanded | 1600×1000 | 1× |
| 860×610 Performance | 860×610 | 1× |
| 1200×800 Sample | 960×640 | 0.8× |
| 1200×800 Sample | 1500×1000 | 1.25× |
| 1200×800 Sample | 2400×1600 | 2× |

Chromium DPR and Slint scale factor match BEFORE capture. Original PNGs are
unresized. Shared gallery zoom is separate from application scaling. Every
original hash and PNG dimension was reverified; each case records logical size,
physical pixels and render density. Raw RGB differences are descriptive, not
an acceptance score or a claim that native data matches simulated React data.

The final executable is preserved at
build/advanced-sampler-evidence/fidelity-reviewed-ninth-binary/dandrum-advanced-sampler,
SHA-256 cfc3864a6a09a6f163d4289637877e06b7868ebe60c7137f0670f3c23234cf68.
All 88 captures come from this binary. Its exact visual-publication configuration
passed 43/43 headless CTest groups in 131.81 seconds. The source freeze verifies
73 production paths unchanged through capture. The test log is
build/advanced-sampler-evidence/fidelity-warning-publication-ctest.log.

## Findings and corrections

Every workspace was rechecked at minimum, default and expanded sizes.
Workspace names below identify their min/default/expanded pairs. Named tests
are CTest groups or their checked-in test owners.

| Item | Original finding and correction | Post-fix evidence and behavioral checks |
| --- | --- | --- |
| 1. Empty | Browse crossed the invitation border. Corrected invitation bounds, primary material and instrument scope. | page-empty pairs; Visual State/Utility and composed material pixels. Truly empty native pads remain empty. |
| 2. Pads | BANK/ROWS collided and pad population differed. Separated toolbar groups and projected active kit sound owners consistently. | page-pads pairs; Visual Performance/Projection. Snare retains native bands/alternates; unrelated library zones remain in the model. |
| 3. Macros | Lower controls and assignment captions were obscured. Restored compact rows, assignments and the read-only map. | page-macros pairs; Visual Performance, macro gesture/context-menu and disabled-material checks. |
| 4. Sample | HISTORY overlapped rows and waveform/metadata drifted. Pinned history, corrected metadata/miniature overview and continuous waveform/zoom viewport. | page-sample and all scale pairs; Wave Render signed extrema, reversal, spectral pixels, miniature bounds and zoom assertions; Visual Projection. |
| 5. Slices | Selection and lower controls differed. Restored source-derived break peaks, selected slice, compact rows, metadata height and analysis feedback. | page-slices, running/failed/transients; Source Pages/Visual Domain. Retry, Cancel, Apply and Discard use actual job state. |
| 6. Mapping | Grid/keyboard proportions and selection differed. Restored rulers, selected zone and viewport proportions. | page-mapping; Domain Render/Visual Projection. Complete native zone inventory and Fit bounds remain available. |
| 7. Layers | Tall sections hid candidates. Restored compact selectors/rows, continuous gain curves, direct editable fields, aligned headings and Hard label inside its band. | page-vel and rr; Visual Domain/Domain Render/Model. Physical gain gesture, order, toggles and Undo. |
| 8. Voice | Vertical panels hid Filter/response/ADSR. Restored horizontal cards, response layout, compact idle knobs and envelope headers. | page-voice; Voice Binding/Visual Domain. Envelope edits/Undo use production callbacks; Edit source/Perform captions fit. |
| 9. Modulation | Split editor hid routes. Restored compact route density, signed percentages, semantic slot colors and actual default-route metadata. | page-mod; Visual Domain/Projection. Amount edit/Undo and default-route detach/Undo. |
| 10. Routing | Outputs/sends and expanded meters differed. Restored compact native rows, separate output buses, owner feeds and decibel controls. Snare Plugin Out now projects Out 5/6, covered by the exact-caption regression. | page-routing; Domain Pages/Visual Domain/Projection. Physical output/send controls and Undo, one-decimal dB, zero-send −∞, independent fixture channel values. |
| 11. FX | Tall cards hid sends and Inspector drifted. Restored compact processors, group/send rows and normalized percentage captions. | page-fx; Visual State/Domain. Actual chain gain/mute/output, cancelled gesture and Undo. |
| 12. Overview | Local/Mixed/Reset absent; fade values raw percentages. Projected actual ownership, mixed source fades in milliseconds and reset actions. | page-overview; Visual Utility/Projection. Reset restores inheritance; Undo restores the prior definition. |
| 13. Diagnostics | Headings overlapped trace/reason rows; initial hit state differed. Corrected dock geometry and no-hit initial state. | page-diag; Visual State/Utility. One real native note creates a selectable event; no synthetic screenshot hit. |
| 14. Performance | Keyboard toolbar crossed controls and keys/pads drifted. Corrected grouping, key tops and drawer proportions. | perf, pads, macros; Visual Performance/Key Render. Actual short/long key separation checked at 0.8×/1×/1.25×/2×. |
| 15. States | Error/progress feedback and Freeze missing or misplaced. Restored banners/progress/actions and dimmed confirmation modal. | missing/unsupported/loading/running/failed/freeze; App Binding/Visual State/Domain. Composed pixels prove unsupported amber, missing red and retained cached waveform; rejected replacements retain prior cache; confirmation does not freeze or claim real audio. |
| 16. Themes | Material depth/grain coverage and segments drifted. Restored well shading, grain across drawer/keys, neutral selected segments and primary glow. | All 16 theme pairs; Theme Binding and Visual Material Render inspect actual composed pixels for Aluminium, Brushed, Ember, Graphite, Midnight, Forest, Camo, Paper in Soft/Flat. |


Unsupported-file feedback now uses the guide's amber warning border and wash.
The new composed-pixel assertion reproduced RED before the production change
and passes in the final 43-test run. It also proves missing-file feedback remains
red and the rejected import preserves the cached source and signed waveform.
The unresized unsupported-banner-focus.png comparison was directly opened.

Snare Plugin Out now reads “Out 5/6”. Its exact-caption regression reproduced
RED before the projection fix and passes all 28 Visual Projection assertions.
The final expanded pair matches the reviewed cycle5 originals exactly; the
routing-output-focus.png comparison clearly shows Out 1/2, Out 3/4 and Out 5/6.
The other 11 changed cycle6 pairs contain 8–33 changed native icon/toggle edge
pixels per pair; all were directly reviewed with no new actionable layout issue.

## Required fidelity surfaces

- **Fonts/typography:** bundled Barlow display, Barlow Semi Condensed UI and
  JetBrains Mono values. Headers, hierarchy, idle knob labels, columns,
  wrapping and truncation inspected. Corrected icon captions and Layers
  headings have explicit geometry checks. Browser/native antialiasing and
  minor optical weight differences remain P3 polish.
- **Spacing/layout:** frame, Inspector, rail, drawer and Performance boundaries
  checked at all maintained sizes. Compact rows recover previously hidden
  workspace content. Wide tables scroll at minimum width; additional real
  native owners can require more rows than the reference fixture.
- **Colors/tokens:** all surfaces/finishes compared. Actual native pixels check
  neutral segments, accent Play toggles, Soft/Flat wells, primary glow and
  Brushed grain in the composed drawer/key caps.
- **Image quality/assets:** original screenshots stay unresized; supplied
  fonts and maintained icon assets retained. Waveform/spectral views are
  native data visualizations with signed waveform and direct pixel assertions.
- **Copy/content:** native filenames, durations, channel counts, owner counts,
  units, routing values and asynchronous errors remain authoritative. The
  silent adapter/audio capability remains visible. Reference-only simulated
  activity, fixed MIDI assignments and fabricated take counts are not copied.


The gallery passed a headless Chromium check of all 88 cases and 264 original/
paired image links, group counts, shared 25–200% zoom, wipe, RGB difference and
2400×1600 density metadata. No browser or console errors were observed. Focused
routing and warning crops preserve original pixel scale. Full pairs establish
composition and viewport behavior.

## Comparison history and verification limits

Six refinement capture cycles followed the original audit. Earlier cycles
corrected compact layout, waveform seams, fractional key separators, round-robin
and Freeze overlaps, cached-source suppression, FX percentages, clipped captions
and Layers heading/band placement. Correct selectors distinguish the Velocity
heading from the AXIS tab and locate Perform in the header; the earlier broad
claim that all three geometry fixes had causal RED proof was overstated.

Cycle4 passed its coherent 46-group configuration. The visual publication
configuration excludes three partial platform/file-drop checks and registers
43 tests. Its initial 41/43 run exposed the Snare caption failure and a verifier
error: SLINT_SCALE_FACTOR set during compilation baked scaling into generated
headers. Regeneration with compile-time scaling unset retained the unchanged
fractional-key assertions and restored adjustable runtime density.

Cycle5 passed 43/43 and reviewed 88 captures. The first amber verification then
passed 42/43 because one generated scene object was older than its generated
C++ source. The final build settled all four scene objects, proved their
freshness, passed 43/43 and captured cycle6. Its wrapper exited 1 after successful
tests/captures because a restoration variable was shadowed. The exact preserved
CMake backup was restored independently, and all 15 held file hashes plus the
73 production hashes were reverified. Failed-attempt logs and this distinction
are retained; the wrapper is not claimed to have exited successfully.

Strict add-advanced-sampler-slint validation passes. Spec coverage passes with
584 scenarios, 345 mapped tests and 239 unchanged ratchet entries. OpenSpec
implementation tasks, including the broader functional acceptance task, remain
unchanged. A three-cycle-after-functional-completion requirement cannot yet be
claimed because broader functional/audio work is unfinished.

## Publication and held work

The visual checkpoint is on main at
[20000e9](https://github.com/DanMarshall909/dandrum/commit/20000e9d6745b0f9afee732962fd1dc9a7d6b8ac).
This focused follow-up contains the warning color, composed regression, scaling
documentation and final review. ReadOnlyKeyMap.slint is already published as a
required visual dependency. Fifteen platform/file-drop/guide partial files and
their working CMake wiring remain outside this follow-up, preserved by
fidelity-held-untracked-sha.json and the shared deferred-work ledger. Rust DSP
and reference assets are unchanged. Platform completion and general accessibility
certification are outside this result.

The current native silent editor was also launched on the user's desktop;
xwininfo confirmed its 1200×800 window is viewable. This desktop launch is
separate from the headless matched screenshot and interaction evidence.

Visual implementation checklist:

- [x] Correct unsupported amber; prove missing red and retained cache.
- [x] Recheck Snare physical-output caption in final matched captures.
- [x] Complete exact visual publication build and all 43 headless checks.
- [x] Review all final matched originals and focused routing/warning evidence.
- [x] Verify gallery zoom, wipe, difference, groups and image links.
- [x] Preserve held work and report the broader functional/audio limitations.
