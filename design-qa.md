# Advanced sampler visual verification

final result: blocked

This progress checkpoint contains the Slint/React layout, state and theme
refinements across the 16 audited groups. The Snare output-caption regression
now passes. Unsupported-file feedback still needs amber warning styling, and
exact publication verification is running. The wider functional audit and real
audio integration remain incomplete.

## Comparison evidence

Source visual truth: [Advanced Sampler v3](docs/design-system/advanced-sampler/reference/Advanced%20Sampler%20v3.dc.html),
SHA-256 a72240e95d4c944aac66709c824fcc2b51f7d9cbe8d1da250d6f6c596cd89119.
Source HTML, supplied fonts and original reference screenshots are unchanged.

Generated evidence is retained locally under
build/advanced-sampler-evidence/fidelity-fixes-cycle-4/: index.html,
comparison-manifest.json, root-review.json, pixel-metrics.json, pairs/,
react/ and slint/, plus supplemental scale folders. It is outside Git.

The matrix contains 88 matched pairs: 13 workspace pages at three sizes,
30 operational/design states, all eight surfaces in both finishes, and three
supplemental Sample density checks. Root opened 85 distinct combined pairs;
three pairs reuse direct inspection only because BOTH original hashes match
an inspected pair. React is left and native Slint is right.

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
unresized. Shared gallery zoom is a viewing control, separate from application
scaling. Each case records logical size, physical pixels, scale and both hashes.
Raw RGB measurements describe differences; they are not an acceptance score.

The reviewed seventh executable is preserved at
build/advanced-sampler-evidence/fidelity-reviewed-seventh-binary/dandrum-advanced-sampler,
SHA-256 748a594ec909575ce60bad676cf0b0cfe9f92ada7cc96529f1e42f80ebc838e5.
Its coherent configuration passed all 46 headless CTest groups in 154.10 seconds.
A source freeze records 73 unchanged production paths through its captures.
The separate visual publication configuration registers 43 tests, excluding
three platform/file-drop checks; its final execution and capture hashes remain
pending. The first exact-scope run passed 41/43: the new Snare regression was
RED and the fractional-key render exposed a verification-script error. Setting
SLINT_SCALE_FACTOR during compilation baked fixed scaling into generated
headers. The corrected run regenerates those headers with that environment
variable unset, so launch-time scaling remains adjustable. The key pixel
assertions are unchanged.

## Findings and corrections

Every workspace was rechecked at minimum, default and expanded sizes.
In the evidence column, workspace names identify their min/default/expanded
pairs. Named tests below are CTest groups or their checked-in test owners.

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
| 15. States | Error/progress feedback and Freeze missing or misplaced. Restored banners/progress/actions and dimmed confirmation modal. | missing/unsupported/loading/running/failed/freeze; App Binding/Visual State/Domain. Rejected replacements retain prior cache; confirmation does not freeze or claim real audio. |
| 16. Themes | Material depth/grain coverage and segments drifted. Restored well shading, grain across drawer/keys, neutral selected segments and primary glow. | All 16 theme pairs; Theme Binding and Visual Material Render inspect actual composed pixels for Aluminium, Brushed, Ember, Graphite, Midnight, Forest, Camo, Paper in Soft/Flat. |

Current actionable finding: **[P2] Unsupported-file warning** uses the native
red error banner where the React reference uses amber. Add a composed-pixel
regression and correct the warning border/wash while preserving the existing
cached source. Missing-file error feedback should retain its error treatment.

The Snare Plugin Out caption changed from “stereo” to “Out 5/6”. Its new
regression reproduced the failure before the projection fix and now passes
with all 28 Visual Projection assertions. Its final expanded capture is pending.

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

Focused native caption/heading probes, Wave Render snapshots and composed
Visual Material Render evidence remain under build/advanced-sampler-evidence/.
The final output-caption correction requires a focused combined comparison.
Full combined pairs establish page composition and viewport resilience.

## Comparison history

The initial audit found the 16 groups above with zero correction cycles.
Cycle 1 applied main layout/state/material corrections; renderer and interaction
checks then exposed variable-wave seams and source feedback. Cycle 2 exposed
80% key separator loss, round-robin/Freeze overlaps, cached-source suppression
and FX percentage formatting. Their regressions and fixes remain in evidence.

Cycle 3 was reviewed across all 88 pairs after those corrections. It exposed
clipped icon captions, overlapping minimum Layers headings and a Hard label
outside its band. ActionButton width and Layers geometry were corrected.
Final selectors distinguish the Velocity table heading from the AXIS tab and
locate Perform in the header. The earlier broad causal-RED claim for all three
geometry findings was overstated; corrected selectors now pass.

Cycle 4 was fully reviewed and its coherent 46-group headless run passed.
Publication source review found and corrected the Snare physical-output
caption; its exact readout regression passes. A further unsupported-state
comparison identified the amber warning mismatch recorded above. Final captures must come
from one verified binary and be directly opened or proven byte-identical to
opened originals.

## Verification and constraints

Strict validation of add-advanced-sampler-slint passes. Spec coverage passes
with 584 scenarios, 345 mapped tests and 239 unchanged ratchet entries. No
wider OpenSpec task is marked complete by this visual report. The broader
sampler audit retains 28 pending controls and real audio integration.

Publication includes the preserved ReadOnlyKeyMap.slint required by the visual
pages. Fifteen platform/file-drop/guide partial files and their CMake wiring
remain outside the change, with their hashes preserved. No platform completion
or general accessibility certification is claimed. Rust DSP is unchanged.

Implementation checklist:

- Correct the unsupported-file amber warning and verify composed pixels.
- Recheck the corrected Snare physical-output caption in the final capture.
- Complete the exact visual publication build and all 43 headless checks.
- Recheck final matched originals and focused routing evidence.
- Verify gallery zoom, wipe, difference, groups and image links.
- Publish this progress checkpoint on main at the user’s request and record
  held work; final visual acceptance remains blocked by the items above.

Follow-up polish: browser/native font rasterization and minor optical icon
alignment can be refined separately after the substantive visual pass.
