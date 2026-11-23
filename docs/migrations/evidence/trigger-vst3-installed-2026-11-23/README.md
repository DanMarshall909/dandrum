# Installed Linux candidate,23November2026

Current private candidate v5 is built froma199fe4e6ec81ad4aca8392aa11c7671c08a2720,
with the existing production React assets. `candidate.json` records all361
packaged file identities, complete resources/notices and source/tool identity.
No product/test/host binaries are published. Native ldd resolves every dependency
and has no JUCE linkage. This qualifies this machine, not a Linux minimum.

`installed-native-source-hidden` runs the actual production native test executable
from an unrelated directory under bwrap, masking the checkout with empty tmpfs
and unsharing network/PID namespaces. Executable assertions require absent source,
a different network namespace and no routes. Fixtures copy only the explicit
installed candidate binary and complete Resources, with no development fallback.
The real React/native selection, deleted-source recovery, stopped/editor-absent
completion, history/recreation, signed audio and independent-instance journey
passes. This run is uninstrumented; separate calibrated callback runs retain their
own scope and do not substitute for installed REAPER. GTK emits isolated-runtime
GVFS warnings but all native assertions pass.

The unfiltered pinned SDK3.8.0_build66 extensive validator (`validator -e bundle`)
reports535passed/2failed, with no setup/teardown errors. Both failures are the
all23-parameter/every-sample single-precision cases, exceeding the explicit
1024-total-point-plus-note bound in designD4. They remain a concrete compatibility
limit, not a clean validator PASS. Existing signed capacity tests prove1024
accepted/1025 rejected atomically with silence and accepted cursor preservation.
Capacities were not enlarged to satisfy the validator. Earlier v1/v2 failures and
the diagnosed lifecycle/context repairs are retained separately in private raw
logs and the native-view evidence. No validator filters/exclusions were applied.

REAPER acceptance, current combined gates, independent completion review and
publication remain pending. Native Windows/macOS milestones remain mandatory.

Private receipts/raw logs: `.git/codex/trigger-vst3-resume-20261123/`, notably
`candidate-v3-package`, `official-candidate-v3-validator`,
`installed-native-fixture-build`, `installed-native-source-hidden` and
`installed-fixture-clippy`. The source-hidden wrapper and package builder are
one-off proof tools; ordinary adapter tests retain their source-fixture default.

Current-source requalification: `installed-native-v5-device-source-hidden` passes
the complete native journey, including physical click/Ctrl+A/key entry, on a
dedicated Xvfb display with checkout hidden and network routes absent. The fixture
retains host device access (`--dev-bind /dev /dev`) for the existing native display
stack. The initial isolated run with a replacement minimal/dev fails the paint
oracle and is retained as `installed-native-v5-source-hidden`; it is not a pass.
This is source/network separation evidence, not device sandbox qualification.
The unfiltered v5 extensive validator again reports535passed/2failed at the same
explicit capacity limitation, with no setup/teardown errors. Real REAPER v5 is
active; task5.2 remains open until all host assertions and reload actually pass.

## Partial actual REAPER checkpoint

`reaper-observations.json` and the two value tables record actual product runs.
The native first-instance level edit2.5 reaches normalized0.625 while the second
instance stays0.25. A distinct second instrument is imported. After both original
WAVs are removed, editor recreation retains native Undo (mapping disappears) and
Redo; the fresh-process second render proves the restored mapping/resources.
All46 named values match before/after fresh-process restoration to1e-12, with
fresh XDG cache/data/temp directories. Both processes exit0 and owned workers stop.

The first instance's64 signed stereo ramp/pan samples match exactly before save.
The settled second-instance render matches its independent PCM/pan/gain oracle
exactly. An immediate-unmute trial has a host fade and is retained as failed.
REAPER serializes the64frame envelope point as0.00133333seconds(63.99984frames).
The fresh first render differs from the original ideal by five PCM24LSBs; using
the independently queried restored host time gives error below one PCM24LSB.
This host timing quantization is explicit, not a bit-exact timing roundtrip claim.
Both restored renders have the expected signed waveforms and exact silent tails.

This does not close5.2. Actual live44.1k is not qualified (the observed JACK setup
is48k/1024); interrupted chooser trials do not establish mapping admission before
view loss; rendering-process module maps were not retained. Full configured/slow
gates and independent review/publication remain open. The first long executor
ended143 while its owned launcher continued; ownership was reverified and a
replacement resource monitor retained its clean terminal result. Both exact
process receipts are retained. No replay/project/product binaries are published.

The subsequent [native DPI repair](../trigger-vst3-dpi-2026-11-23/README.md)
records the final scaled candidate, exact actual-REAPER mapping, preserved
named values, current native/CLAP/offline checks and retained variant failure.
Its single-instance DPI proof does not complete the remaining Linux qualification.
