# Native Trigger state evidence

This records the format-neutral state portion of OpenSpec task 2.4 on base
`749d9e028e42888a615649f7b3ff74b93b2fcad8`. The earlier parameter boundary is
already published. Actual CLAP/native/editor and full-beta acceptance remain open.

[Receipt](receipt.json) binds the eight source files and the registered checks.
[Author handoff](author/HANDOFF.md) distinguishes genuine missing-behavior REDs
from later GREEN characterization, explains the signed PCM oracle, and records
covered refactoring and coverage limits. Source hashes identify the measured
candidate; absolute original paths are provenance, not runtime resource lookups.

Raw logs, LCOV exports, the original patch and new-file snapshots use lossless
gzip with zero timestamps. `artifact-map.json` records both compressed and
uncompressed SHA-256 values. `SHA256SUMS` covers the portable artifacts, excluding
itself. To reproduce the coverage aggregation, decompress the author LCOV files
beside a copy of `author/coverage-source-summary.py` in a temporary directory and
run the script there. Reproduction needs Python's standard library only.

The first source-free invocation selected the wrong unittest class and ran no
product test. Its retained error is separate from the corrected successful guard.
The registered CMake logs enumerate all 62 host and 30 session tests. Session
coverage reports its unchanged FFI gaps separately from the fully executed
changed session owner. No mutation, cache deletion or native continuity claim
is included. Independent completion review and publication are separate gates.
