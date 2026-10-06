# Shell editing checkpoint

The user explicitly waived test-first TDD for this shell on 2026-10-06.
Implementation now proceeds directly, followed by meaningful behavior and
browser verification. Final verification and independent review remain required.

The supplied React drawing primitives remain reused through
`web/trigger/components/design-system/index.jsx`. `ParameterKnob` adapts precise
dragging, Shift ×0.1, keyboard steps/Page/Home/End, typing, hover timing, reset and
cleanup. `ChoiceMenu` supplies choice state and in-window popup placement around
the received MenuButton drawing. Waveform editing composes the supplied waveform
with accessible region, fade and loop handles. The original source remains
immutable. One attributed Toggle compatibility correction supplies its visible
label as an accessible name.

The current accepted-patch store owns parameter, region, mapping and import
operations. Mapping no longer retains a separate demo-zone model. Clipboard
replacement trims both note and velocity rectangles, preserving unrelated
layers. Four import policies offer an interpretation preview; each accepted
import is one reversible operation. Common explicit filename tokens are parsed;
the parser avoids treating unrelated digits as pitches. TypeScript DTOs and the
public adapter surface are retained in `engine/contract.ts` and documented in
`engine/engine-contract.md`.

Patch new/reload confirmation, successful replacement history clearing,
separate 30Hz telemetry, keyboard notes, visual audition, docked log filtering
and mock switches are connected. Replacement now publishes cleared feedback and
continues telemetry. Selector simulation handles stack, candidate mute/solo,
round robin per note and unavailable-asset fallback. Tree and browser rows are
projected from patch resources; group rename/delete/duplicate use commands.

## Recorded verification

In `/tmp/dandrum-trigger-shell/web/trigger`:

- `npm test`: 23 Node behavior tests passed, exit 0.
- `npm run check`: TypeScript public-boundary check passed, exit 0; a subsequent
  component-size/import-boundary driver is included in the same command.
- `npm run build`: passed, exit 0. Runtime, received design system and application
  now build as separate chunks; the earlier oversized-chunk warning is removed.
- `npm run test:browser`: 8 tests passed, exit 0, including full-range/fine knob
  drag, typing/reset, separate gesture undo, waveform editing across navigation,
  loop mode, all import choices and undo back to Empty.
- `openspec validate add-trigger-react-shell --strict`: passed, exit 0.

A browser geometry probe measured exact outer sizes 820×560, 1200×800,
1600×1000 and 900×126, with zero header overflow at all four. Default screenshots
for Empty and all eight pages were captured in
`/tmp/dandrum-trigger-render-evidence`; the Sample capture was inspected. This is
not a complete source-to-implementation comparison. Reference rendering exposes
its nominal content geometry plus a two-pixel border; the app now uses the
plan's exact outer geometry. Minimum 11px typography, new controls and actual
data/telemetry differ from the original constant-state demonstrations and still
require the final comparative audit.

## Remaining scope

All OpenSpec task checkboxes remain open because the full criteria are broader
than this checkpoint. Sample history, comprehensive region/value binding,
analysis application/slice editing, complete Mapping drop operations, selectors
and crossfade UI, full Voice/Modulation/Macro/Routing/Effects bindings and menus,
all 24 action-reachable states, component catalog, feature audit, platform
document update, responsive reference capture, complete visual QA and independent
candidate review remain. No complete-shell, final review, commit or publication
claim is made.
