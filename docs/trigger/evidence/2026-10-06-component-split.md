# Component size and presentation refactor evidence

The user's current preference is fewer than 200 lines per authored React
component, superseding the earlier 500-line preference. There are 20 authored
React files; the largest is `web/trigger/components/AssetSidebar.jsx` at 111
lines. The root controller is 18 lines. State, pointer/keyboard actions and
presentation projections live in separate `shell/` modules. The received
compiled primitives are attributed under `vendor/design-system/` and exposed
through a two-line public entry point.

## Observable verification

- Before the refactor, the frame/navigation browser test passed through all
  eight pages, compact mode, the minimum rail and expanded overview.
- Nine screenshots (Empty plus all eight pages) captured immediately before and
  after the split are pixel-identical at the same viewport. Capture files are
  retained under `/tmp/dandrum-trigger-before-split-evidence` and
  `/tmp/dandrum-trigger-render-evidence`. This proves preservation of the
  current implementation, not final matching against the design reference.
- The parameter integration test initially failed because a visible Cutoff
  edit did not create a named undo entry. Its browser setup first had to wait
  for preset Ready; without that wait, navigation raced preset completion.
- The owning Node test now asserts the displayed Cutoff value, command label,
  undo restoration and final redo value. The browser test proves keyboard
  editing, named Undo, restored value and Ctrl+Shift+Z through real React DOM.

Commands run in `web/trigger` after the split and vendor relocation:

```text
npm test                 3 passed, exit 0
npm run test:browser     2 passed, exit 0
npm run build            passed, exit 0
```

`openspec validate add-trigger-react-shell --strict` also passed from the
worktree root. Coverage is diagnostic: engine lines ran fully, but branch
coverage and the reference-derived presentation/actions remain incomplete.
This is an implementation checkpoint; full-shell tasks remain open. No final
readiness, complete visual QA, spec synchronization, review, commit or
publication is claimed.

## Remaining work owner

The authoritative remaining scope is
`openspec/changes/add-trigger-react-shell/tasks.md`; its checkboxes remain
unmarked until the complete related behavior and checks pass. Current source
projection data and several callbacks are still demonstrations awaiting
EngineAdapter ownership. Typed contract, full operations/menus, mock telemetry,
all 24 action-reachable states, catalog, feature audit and final visual/review
checks remain part of that change.
