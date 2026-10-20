# First native Trigger editor owner

This is one isolated, opt-in component of OpenSpec task 3.2. The task and full
beta remain open. Read receipt.json and handoff.json for the exact scope and
gaps. The default editor crate remains std-only; native-x11 enables the pinned
Wry/GTK/WebKit/x11rb edge. No GUI types enter the format-neutral host.

author-raw.tar.gz retains every original author evidence byte, including raw
logs, RED and final source snapshots, LLVM profiles/exports, the local-crate
instrumentation wrapper, dependency metadata, feature trees and 191 licence
texts. author-artifacts.json records each member's exact bytes, mode and SHA256;
extract into a temporary directory to inspect or reproduce the retained tools.
Archive metadata is deterministic; file contents are unchanged. SHA256SUMS
covers all portable files except itself. Guard gzip files preserve exact raw
output; guard-artifacts.json identifies the decoded bytes.

The native fixture runs against a real X11 server and WebKitGTK, not a browser
stand-in. It checks a literal 1200 by 800 hidden child, an independently expected
external-script IPC response and missing-resource 404, parent survival, child
tree/toplevel cleanup, released browser and callback weak references, invalid
parents and wrong-thread calls. A ready GLib source stops itself at 65 callbacks
so a missing 64-iteration service budget fails promptly. Checked reparent failure
occurs after actual WebKit creation and releases its registered resource closure.
Duplicate protocol rejection is a separate pre-WebKit builder error: the earlier
investigation claim was disproved and remains explicitly superseded in the report.

Original stub and external-script REDs have contemporaneous source snapshots.
The intermediate retained-IPC failure has its raw log but lacks a contemporaneous
source snapshot. The canonical buffered-destruction failure does have an exact
pre-run snapshot; GDK display synchronization fixes that existing immediate
cross-connection teardown contract. Three comparable repeats are retained.
The fixed-size independent-literal correction and shared test-fixture/lint
refactoring are GREEN oracle improvements, not invented new behavior REDs.

Native owner coverage has no exclusions: 129/134 lines, 11/13 functions and
184/203 regions in the native lane. The fast lane hits 0/134 native lines and
52/52 asset lines. No branch counters were enabled. Gaps include unsupported
GDK backend, connection/request transport loss and a private fresh-manager IPC
registration-refusal invariant. They remain visible in handoff.json; the native
owner is not claimed to have 100% coverage. It is newly implemented, not an
extracted module. Production callbacks/audio are outside this measured fixture.

The repaired source received final-source fmt, denied-warning Clippy, native and
fast LLVM runs, the registered three-lane CTest and a native library build.
The unchanged maintained source-free guard ran from a complete hash-matched RAM
input tree against reusable caches. Its tiny package proves default build and
test isolation, not production browser rendering. Commands, source identities,
resource samples and raw logs are retained. Mutation remains excluded by user.

Three source-free attempts stopped at storage resource floors; their results
are incomplete, not test failures or successes. Compact-profile attempts also
remain incomplete. After explicit user approval, only the named 30 GiB debug
cache was deleted. The RAM guard cache was preserved on the freed disk with
all file hashes/modes/symlinks checked and its logical path retained. The full
maintained guard then passed at its default build profile. Exact environments,
resource samples and cleanup/relocation receipts are retained.
Some stage logs copied after the interrupted attempt were pre-existing; their
mtime-based provenance record marks them as prior, never successful new steps.

No production React/session initialization, browser font rendering, network/CSP
policy, show/hide or resize facade, actual CLAP GUI/timer/API integration,
view-owned note/gesture cleanup, or uninterrupted native audio is proved by this
component. These remain subsequent acceptance work. The Rust licence metadata
and full available texts are evidence; system-library redistribution and final
product linkage audits remain later gates. Main and reusable caches are retained.
