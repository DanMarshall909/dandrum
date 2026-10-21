# Pending automation read boundary

Base: `68668a3ffc1a6a65eebc359a66b577f3ee9c4fdb`.
`ParameterController::pending_automation` borrows the existing ledger and yields
copied events in delivery order. Reading cannot acknowledge host delivery or free
queue capacity. This is the accepted staging prerequisite for the next CLAP
automation consumer; no editor services or OpenSpec tasks are completed here.

The real packaged-controller test specifies empty, first-read allocation-free,
partial/repeated reads, distinct physical values, full/reserved-End pressure,
explicit one-head acknowledgement and ordered later events. Missing-interface
compiler RED precedes the five-line implementation. The output-name harness
warning and earlier resource-guarded baseline setup stop are separately recorded.
The accepted prior baseline matches all 124 original source hashes.

Formatting and strict all-target Cargo Clippy pass. The full normal host run
passes 152 unit/integration tests and two compile-fail doctests on final production
bytes. After strengthening only the allocation oracle, the final 16 parameter
tests, formatting and Clippy pass again; a fresh whole-suite rerun is not claimed.
One fresh exact LLVM profile from those 16 tests records seven method invocations
and all five method regions. Existing owner gaps stay visible; no whole-owner or
branch-coverage claim applies. No further extraction is justified.

`author.tar.gz` contains 95 hash-bound data files plus its manifest: exact RED,
source snapshots, actual compiler/Cargo artifact bindings, raw logs, fresh LLVM
profile/export and qualifications. `author-artifacts.json` binds the data members;
`qualification.json` explains scope. Build/cache ELFs remain outside the archive.
`root-checks.json` records the independently hash-verified actual Cargo executable
smoke test and strict scoped/canonical/spec-coverage results. Mutation is excluded.
Required CLAP automation, heartbeat, mapping/recovery and all later beta products
remain open. No cache was deleted by the agents.
