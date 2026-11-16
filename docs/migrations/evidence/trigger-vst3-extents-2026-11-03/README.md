# Numeric output extents — 3 November 2026

A real callback inconsistency was reproduced through a bounded child: process
rejected a wrapping channel extent, then error cleanup attempted a slice over
that same rejected address and the child exited SIGSEGV. Parent diagnostics are
retained; the first incompatible test-selection invocation is a separate setup
error. Crash-dump resource limit was zero; future child runs additionally disable
dumpability. No matching reentrancy crash report was observed in /var/crash.

Processing and f32/f64 cleanup now share numeric channel-end checks. A bad plane
is skipped while the other accessible plane is cleared. Tests assert rejection,
untouched invalid-plane backing storage, cleared valid storage and continuation
at literal signed source frame16(11869/32768), proving no DSP-time advancement.
Null/misaligned planes and missing pointer arrays are also exercised. The helper
does not prove arbitrary memory accessibility; valid bus/pointer-array storage
and nonwrapping writable planes remain host ABI preconditions.

Both callback boundary parents, seven audio tests, five curve tests, the real
malloc/free sentinel and strict Clippy pass. Parent-owned temporary namespaces
now clean child fixtures even after a killed/failed child is reaped. This is a
small covered numeric helper extraction; actual-cdylib source coverage and
per-instantiation counts are recorded in verification.json. Final combined
coverage, native editor/state/REAPER/validator and completion review remain open.
Binaries and full LLVM JSON remain private.
