# Ignore recovery replies after editor closure

The task5.5 lifecycle contract requires a closed editor to ignore late replies.
A controlled public NativeEngine test closes document/Undo/Redo while a failure
recovery snapshot is pending, then resolves it with a different accepted patch.
The original native error must remain the rejection, the editor patch must stay
unchanged and no ready/failed patch update may follow closure.

Genuine RED preserves the exact preimage and complete output: document recovery
already passes, while Undo and Redo adopt the late patch. The smallest repair
checks `closed` again after awaiting the recovery snapshot, matching the existing
successful-command and document-recovery guards. The three-case table now passes.
No runtime, Rust source, dependency, queue or native history owner changes.
The covered refactor review found no useful extraction for this one guard.

All170 frontend tests,12Chromium entrypoint tests and30actual native CLAP checks
pass on the final current bundle. Type/component and active strict OpenSpec/spec
mapping pass; the latter remains631scenarios/392mapped/239ratchet. Node22/V8 gives
NativeEngine100%lines/87.03%branches/90.82%functions and Store100%lines/86.45%
branches/88.89%functions. This records execution, backed by the public failure/
state assertions; it does not claim all branches or whole GUI coverage.

[Binding](binding.json) and [index](raw-index.json) identify the readback-verified
[raw packet](raw.tar.gz). Its source overlays apply to Gitc9e9fd8a. The external
native replay archive retains exact pristine product/test/18resource inputs from
the maintained fixture binding. Actual native tests deliberately mutate their
private copies, so reconstruction uses source resources with the exact binding
hashes. An initial archive preflight detected that mutation and created no
archive; it is not a test or product failure. The Rust DSO remains the qualified
33af9b90 product. Native checks use private authenticated X11/WebKitGTK2.50.4.

Every named run records stopped owned workers and removed scratch, including
RED. Browser scratch uses a short path for Chromium Unix sockets. Reusable disk
caches and required replay evidence are retained; the preceding retention-budget
excess remains explicit, with about103GiB physical free space. Main stays clean
at68dd655. Mutation remains excluded. Strict engineClippy remains FAIL129 baseline
errors. Native import/mapping/Empty and recovery, portable state, installed offline,
physical input/device and named-DAW gates remain open. VST3/standalone are deferred.
No beta task/scenario closes here. Normal hooks, independent review and publication
are separate gates; this packet alone claims none of them.
