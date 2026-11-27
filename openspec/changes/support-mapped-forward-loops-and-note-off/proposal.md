## Why

Native Trigger accepts forward-loop settings, but a mapped sample plays through its region once and becomes silent. A signed render through the prepared sample map reproduced the mismatch. Trigger also exposes gated playback without a native mapped note-off contract.

## What Changes

- Make mapped forward loops repeat their declared frame window, including reverse playback and bounded crossfades, until note-off.
- Add explicit gated playback for mapped regions. One-shot regions retain their current note-off-independent tail.
- Keep invalid playback edits transactional and make accepted modes survive native undo, redo, and state restore.
- Prove signed audio and voice lifetime through the mapped engine and a native host. Keep ping-pong, sustain, automatic loop discovery, time stretching, and CC64 delivery outside this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `advanced-sampling-options`: specify mapped forward-loop, gated and one-shot playback, boundaries, and safe voice retirement.
- `trigger-native-webview`: specify transactional playback-mode edits and round-trip behavior.

## Impact

Rust sample declarations, preparation and mapped rendering; native UI snapshot projection; Trigger's document compiler and editor state; focused Rust, JavaScript and native host tests.
