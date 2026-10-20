# Fixed-entry native editor navigation

This is one covered criterion within still-open OpenSpec task 3.2. The Wry edge
allows only `dandrum://localhost/index.html` as its top-level document. One private
immutable ENTRY_URI supplies both initial loading and the exact navigation
predicate. No public configuration, dependency, build wiring or host-core change
is introduced. Tests keep independent literal expected URLs.

Read receipt.json and handoff.json first. Exact raw logs, contemporaneous
baseline/RED/GREEN/refactor/final source snapshots, command/cwd/environment/exit
and timing receipts, LLVM profiles/exports and source bindings are retained in
deterministic author-raw.tar.gz. author-artifacts.json lists every member's bytes,
mode and SHA256; extract to a temporary directory to inspect. Contents are
unchanged; tar metadata is canonicalized. SHA256SUMS covers portable files except
itself. No target directory or compiled executable is included.

A task-owned loopback endpoint is calibrated with independent Rust TCP reads of
known text and HTML responses. Before production changes, ordinary packaged page
JavaScript navigates to that HTML page: WebKit commits its actual HTTP URI and
sentinel title, and the listener records the browser GET. A separate baseline
also commits the packaged root alias. The genuine RED asserts the fixed URI and
fails on the actual unwanted HTTP URI/title/GET, not a setup failure or timeout.

GREEN requires both attempted destinations to retain the exact entry URI and
local title. The original ordinary page reports its attempted target, then
continues local fetch and IPC; the endpoint records only the independent health
checks. Existing module/CSS/inline dimensions/local404 fetch and enforcing
external-fetch CSP controls still pass. No native evaluated JavaScript is used
as the navigation policy oracle. The HTML sentinel has no attachment behavior,
and all HTTP requests stay on the owned loopback endpoint.

Final registered CTests pass 3/3 in 1.572698 seconds, native boundary 1.42 seconds.
All-target native Clippy with warnings denied, native library build, formatting,
whitespace, default dependency graph and native/fast LLVM checks pass against
identical frozen source hashes. The new callback executes 7 times: 5 initial entry
loads and 2 denials. Native coverage 129/134 lines, 12/14 functions, 190/209 regions retains
5 inherited uncovered lines/19 regions; raw instantiations 23/34 are separate. Fast
assets remain 52/52 lines, native owner 0/134 in that lane. No exclusions or branch
coverage claim. The covered production refactor shares the fixed entry constant;
test refactoring reuses the package and independently calibrated HTTP reader.

CMake, manifests, lockfile, lib.rs and assets.rs are unchanged; default editor
graph remains std-only. The published 4d85b25 source-free/default guard remains
applicable through the reviewed 8d002a1 unchanged boundary. That large unaffected
guard was not repeated. Existing native acquisition/destruction assertions run.

This proves representative ordinary-page top-level navigation only. Full
subresource policy, production React/eight fonts, popup/download behavior,
show/hide/size/API/order, actual CLAP main-thread/session/audio/device/DAW/VST3
proof remain subsequent work. Task 3.2 and full beta stay open. Mutation remains
excluded; all reusable caches are preserved after the separately approved debug
cache deletion. Main stays clean at 68dd655.
