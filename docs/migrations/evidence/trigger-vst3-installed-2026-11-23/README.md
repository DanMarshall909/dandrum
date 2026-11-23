# Installed Linux candidate,23November2026

Private candidate v3 is built from04c297f0623c63e9dd7e397c2d10b0133d66fd22,
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
