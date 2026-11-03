# Stopped-flush callback reentry — 3 November 2026

The actual dynamically loaded COM processor accepted a stopped zero-sample
process call, entered the host event-list getter, then deadlocked when that
getter reentered processor termination. An owned child prints and flushes the
entry witness; its parent enforces a three-second deadline, kills/reaps the child
and retains the failure. This is the semantic RED, distinct from the first
coverage invocation's incompatible-option setup error.

Termination now checks the audio endpoint's atomic in-use flag before mutating
or destroying ownership. Reentrant termination rejects; the original flush
returns successfully; subsequent termination succeeds after callback ownership
returns. The parent passes in under a quarter second. Seven signed audio tests
and the calibrated malloc/free callback parent/child remain green; strict Clippy
passes. The shared child supervisor retains deadline diagnostics and always
reaps its owned child. Both intentionally ignored children are launched by their
ordinary parent tests; neither is claimed as skipped acceptance.

This repairs a concrete callback lifecycle boundary. Full point-queue/native
callback overlap evidence, automation, state, production editor destruction and
recreation, installed REAPER, official validator and exact combined Astra review
remain required. Binaries and full LLVM JSON remain private.
