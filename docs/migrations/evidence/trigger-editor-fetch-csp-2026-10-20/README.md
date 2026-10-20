# External-fetch CSP foundation

This is one covered criterion within still-open task 3.2. The only production
change is a fixed `Content-Security-Policy: connect-src 'self'` response header
at the concrete native editor edge. No new dependency, configuration surface,
default feature, CMake wiring or format-neutral-host code is introduced.

Start with receipt.json, handoff.json and final-candidate-hashes.json. The exact
author evidence is stored in deterministic author-raw.tar.gz; author-artifacts.json
records every member's bytes, mode and SHA256. Extract into a temporary directory
to inspect raw logs, baseline/RED/GREEN/final source snapshots, command receipts,
LLVM profiles/exports and instrumentation wrapper. File contents are unchanged;
only tar metadata is canonicalized. SHA256SUMS covers every portable file except
itself. No target directory or executable build output is included.

An independent Rust TCP client proves the owned loopback endpoint serves a
known HTTP response. The unchanged owner then completes an actual no-cors
WebKit fetch and the listener records its GET. The contemporaneous RED fails
specifically because the enforcing CSP violation is absent while that request
still succeeds. GREEN requires an enforcing connect-src event naming the exact
blocked origin and local document, a rejected fetch, and no external-attempt GET.
Normal packaged module, stylesheet, inline dimensions and local 404 fetch remain
positive controls. No native script evaluation stands in for page execution.

The only production line added is hit 10 times. Native owner coverage is130/135
lines, 11/13 functions, 185/204 regions, with no exclusions or branch claim. Its five
uncovered source lines and 19 regions are inherited from the reviewed base. The fast
lane deliberately has 0/135 native lines and 52/52 asset lines. The new test helper
was created for this criterion; no production module extraction was introduced.

Final source-bound native/fast LLVM, three registered CTests, all-target native
Clippy with warnings denied, formatting, whitespace and default-graph checks
pass. The source-free build/default boundary from published 4d85b25 remains valid
for this change: only existing cfg(native-x11) source/tests change; unchanged
CMake,manifest,lock and default graph are recorded. Its large guard was not
needlessly repeated. Existing owner lifetime and acquisition assertions still run.

This criterion covers external fetch/connect-src enforcement only. Complete
resource restrictions, navigation/new-window/download controls, actual production
React/fonts, show/hide/size/API/order lifecycle and CLAP/session/audio acceptance
remain subsequent work. Task 3.2 and the full beta stay open; main and remaining
reusable caches are preserved. Mutation remains excluded by user direction.
