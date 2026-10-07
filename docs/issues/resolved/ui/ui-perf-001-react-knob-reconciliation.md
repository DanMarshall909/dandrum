# React knob reconciliation during asynchronous refresh

Status: Resolved

This record holds the concrete findings repaired during `optimize-react-plugin-ui`; the change's task list owns delivery status. Evidence is retained in [the performance validation directory](../../../resource-measurements/2026-10-07-react-ui-performance/validation/).

## Findings and decision log

On 7 October 2026, read-only review and composed React tests exposed four assumptions in the shared knob after writes stopped waiting for full snapshots:

1. Releasing a drag could restore the previous snapshot, including an interim snapshot from an earlier move. The knob now records its latest accepted value-write generation and sequence, retaining that value until a snapshot covers it. Gesture boundaries alone do not prove a value.
2. Cancelled, invalid or unchanged typed edits could restore the raw old snapshot. They now use the same admission-aware reconciliation. Tests include Escape, unchanged Enter, invalid input, blur and an accepted default reset while reads stall.
3. A snapshot arriving while a later keyboard edit waited for the prior drag's closure could steal that edit's display. Pending discrete commits now own their display through flush; interaction tokens prevent earlier closure callbacks from reclaiming it.
4. Completion review F1 found that the previous gesture's rejected `endGesture` could restore its value during a later active drag. The first repair captured native submission ownership; the next completion review reproduced the same failure when the old closure was still queued behind a stalled value write. The serializer now captures ownership at begin/change/commit/end submission, before any wait, and carries it through queued and coalesced operations. Only that originating owner can apply rejection reconciliation; the adapter consumes the opaque owner without forwarding it to the native bridge. The supported `noGesture` status reproduces the failure, and a composed test asserts the later admitted 0.85 value stays visible after the earlier 0.75 drag's closure fails.

The tests use the actual shared hook, knob, gesture serializer and prepared parameter admission with stalled native reads/closure and explicit host automation. Each failure was reproduced before its correction; the repaired suites pass. The first three findings received a separate read-only repair review. F1 was found in the first completion review; the second review returned FAIL for its queued-origin variant. Both variants now have composed RED/GREEN evidence, with serializer ownership ordering and adapter isolation assertions. The entire repaired candidate requires another completion review. This record does not claim physical pointer verification.

## Lesson candidates

The narrow owner is `web/shared/host-knob.mjs` and its component tests: a transport completion boundary change requires checking release, cancellation and later-interaction ownership through the actual component. Both asynchronous success and failure callbacks need the originating interaction's ownership, retained from queuing before serializer waits. Helper-only ordering tests did not establish those composed outcomes. The maintained regression cases record this lesson; no standing rule or memory change is proposed.
