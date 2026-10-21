# G2 parameter integration author handoff

The coherent developer-only parameter boundary is implemented and frozen for independent review. This is not completion of the full JUCE-free Trigger beta or G3. Root owns maintained integration gates, evidence curation, independent review, Git/index, publication, OpenSpec/docs/tracker, and task closure. No commit, push, checkbox closure, mutation run, G3 production change, or cache deletion was performed by this author.

Base: `954c549732537b95203e4b2a71c9bfeff2f2bb9a` in `/tmp/dandrum-lanes/runtime`. The ten author-owned changed files are snapshotted in `final-source/` and enumerated by `final-source-artifact-manifest.json` (SHA256 `31034d0398f7ef7fc12f4e5c99ce42354d9f1c1dff5d335ba6aeefbfbda122ef`). Root-owned changes are excluded from that snapshot. The G1 JavaScript observer is unchanged from the published base.

## Resulting behavior

* Existing protocol admission and controller authority now serve numeric parameter state and real Begin/Set/End operations. Accepted writes report numeric generation/sequence and are distinct from host settlement. Duplicates never redispatch. The only host protocol additions are the three serialized error variants `busy`, `commandRejected`, and `duplicateRequest`; existing admission errors remain intact.
* The private, bounded automation mirror uses direct exact `rtrb=0.3.2`, capacity 64, a main producer, transferable audio/stopped consumer, and accepted-prefix credits. Queued plus accepted-uncollected events remain bounded by 64. Host rejection retains the first event and stops that callback; each callback makes at most 64 attempts including concurrent refill. Host success precedes pop and credit publication; main collects exact credits and acknowledges only the ledger prefix. Ownership survives failed activation and deactivate/reactivate. Main stopped flush validates the complete input batch before collecting/staging credits or publishing output.
* Main owns the 16 ms requested HostTimer token through registering/disabled/live/removing states. First supported GUI creation requires successful registration. Rollback failure retains disabled cleanup ownership and retries; a live token survives hide/destroy and is disabled/removed at final release. Host calls occur outside Core borrows, and borrowed host access remains on Main.
* After admitted native initialization and GTK, one view-independent weak GLib source requests bounded service every 10 ms. Configuration-only instances create no source. The source survives view destruction, collects accepted credits, and drains bounded host feedback; final cancellation happens outside the GUI borrow before owner release. There is no private GTK event loop or plugin `NativeEditor::service` loop.
* Authenticated host feedback reaches the actual rendered Voice control through existing encoding and transport. Parameter Undo restores the prior scalar through the real controller and host output. GLib/browser callbacks do not capture or invoke a borrowed host. Host timer/flush requests do not imply host acceptance or progress when host processing is suspended.

## Final source and artifact binding

Final formatting preceded the final normal build (110). There were no maintained source edits after that build. It selected the unique Cargo `compiler-artifact` production library with `profile.test=false` from `release/deps`; it did not stage the top-level convenience path.

| Artifact | SHA256 |
| --- | --- |
| Actual compiled production DSO and copied `installed/DandrumTrigger.clap` | `7b75a9d5eb22ae0126edc993be342b24432b289d124d6282f0fe11df8ae58e4d` |
| Actual `trigger_parameters-8fc57b2ef83fb6e4` test ELF | `51a02349660df11b353c18fc9f08af077c364d5a306cbbd41a7ac951c567b569` |
| Production `editor/app.js`, unmodified for parameter cases | `4b9ce5a356fea5f22703f9046c77a80ac616c00242348a12fe81f58fe4f760f2` |
| Final `gui.rs` | `d49197a6cd735d7632145022f052d83204233f55991d20815767184d225ea34b` |

`110-final-formatted-native-build-binding.json`, the final manifest, raw Cargo JSON, and per-run source hashes bind the actual selection. Parameter tests use public WebKit JavaScript evaluation against the actual live WebView and unmodified production assets. The separate G1 test ELF retains no direct WebKit/JavaScriptCore `DT_NEEDED` edge (`g1-host-link-audit.json` / final G1 binding). G1 copied-app observer tests restore the app hash after each case.

## Final author verification

Each focused run lists the actual selected test ELF first, requires full-line exact test membership, executes one exact identity, and checks the named result, executed count, summary, exit status, and source/artifact binding. Zero-test setup runs were not classified as success or behavioral RED. Native normal cases use the normal runtime stack and no preload; only the explicit calibrated allocator child uses its test interposer.

| Receipt and matching raw logs | Result |
| --- | --- |
| 110 final formatted normal native build | PASS, 51.28 s |
| 111 exact host-joined instrumented owner units | 12 named cases PASS |
| 112 final parameter native cases | 11 named outer cases PASS, 10.17 s |
| 113 final G1 native cases | 5 named cases PASS, 3.05 s; app restored each time |
| 114 final ordinary owner units | 12 named cases PASS |
| 115 public engine baselines | Both signed-gain and timed TB303 cases PASS |
| 116 formatting | PASS |
| 117/118/119 strict Clippy, lib/all tests, `-D warnings` | editor/default/plain Trigger PASS |
| 120 offline locked normal/build/dev Cargo graphs | default/plain Trigger PASS; zero forbidden native packages |

The native cases are: authenticated numeric state; stopped host-accepted gestures; rejected-output/activation/audio-return ownership; missing HostTimer; refused HostTimer; later rendered timer-driven edit; rollback/retry cleanup; view-independent credit collection; host feedback/rendered control/parameter Undo; invalid stopped-flush ownership; and calibrated DSO callback allocation/destruction audit. Names are stable in `native_all.py` and the maintained test source. Root has registered the eleven outer cases and independently owns their maintained CTest execution.

Real control and audio oracles include physical 0.5 / normalized 0.125 / CLAP ID 3, ordered B/V/E at time 0, and note 36 velocity 100 at frame 3 producing both-channel zero prefix, -0.25, -179/32768. Feedback plays note at frame 0 and changes physical value to 2 at frame 1, independently pinning old/new signed PCM (-0.25 then -716/32768), authenticated DOM normalized 0.5/text 2, equal UI sequence 1, no output echo, and unchanged structural history. Actual header parameter Undo restores physical 1, sequence 2, exact B/V/E, and fresh signed PCM -0.5, -358/32768. This does not prove structural Undo or pointer dragging.

The final allocator outer/child in 112 positively calibrate Rust cdylib allocation `(1,0)` and preexisting destruction `(0,1)`, then measure the actual plugin and Clack DSO active flush/process at `(0,0)`. There were 29,690 callbacks, 284 whole callback intervals overlapping entered host MainContext service, and 29,387 further callbacks after actual browser JavaScript completion during subsequent context pumping. Callback buffers and recorder are preallocated; callback code does not wait on the overlap handshake. This is a bounded GUI-service overlap proof, not continuous editor destroy/recreate acceptance.

## Coverage, dependency, and refactor assessment

`feedback-owner-coverage-assessment.json` and unfiltered owner/native LLVM exports retain 62/62 unique automation source code regions hit when merging exact ranges/region kind across generic monomorphs. Raw LLVM file summary still reports 61/62 regions (and 54/55 lines); that contradiction is preserved. This is aggregate source-region execution, not branch coverage or every path of every instantiation. The bounded feedback service aggregates 53/53 source code regions; source registration/cancellation have separate owner evidence. The weak-upgrade fallback remains intentionally unhit under the valid final-cancellation invariant. Existing G1 receive-guard/unavailable route gaps remain; no full GUI/adapter 100% claim is made.

Earlier adapter-only profiles did not instrument the host crate. The exact production host rustc recipe was captured in 103 with dependency identity audit, replayed with coverage in 107, and joined into the owner in 108/111. The instrumented host rlib SHA is `058f27f0712a435880b8385dec8558e05203f6f2872ca891fc0ea0e08ce2ce01`. `host-protocol-serialization-characterization.json` binds final protocol source, recipe, dependencies, and raw export. `encode_reply` executed 172 times; full protocol coverage remains partial (75.14% lines / 66.47% regions). Serde's generated enum serializer has no separately mapped regions for the three new variant declaration lines. Exact actual-dispatch JSON error-code assertions prove their behavior; those declarations are not mislabeled uncovered executable lines, and no invented per-variant numeric coverage is claimed.

Refactor review retained the bounded mirror as one small private module, whose aggregate source execution satisfies the accepted new-module gate. Feedback borrow scopes release before browser evaluation and recapture current attachment authority for each event. No further production extraction was justified. The covered audit closure was extracted into a named test helper after 106 behavioral proof to improve formatting, then rebuilt/retested in 110/112.

`dependency-direct-edge-audit.json` retains exact cached versions/checksums/licenses: rtrb 0.3.2, WebKit2GTK 2.0.2, clap-sys 0.5.0. Unique package name/version/source identities do not change from base. The WebKit direct optional edge is enabled only with `trigger-editor`; the standard CLAP ABI edge is test-only. The initial unconditional WebKit dev edge was caught by an actual default/plain development-graph RED and repaired before final graphs. Root independently reproduced it with the maintained guard against a private retained old manifest.

## Chronology qualifications retained

* An initial pure dispatch seam extraction preceded proper pre-refactor characterization. Its exact patch/baseline bytes were preserved and production GUI restored exactly to base before actual native capabilityUnavailable RED 04. No new parameter behavior preceded that native RED. The earlier seam test is not relabeled pre-refactor coverage.
* Setup-only runs include moved-GObject compilation 02; wrong native ELF/filter zero-test run 21; the invalid >64 fixture configuration in 24; source interface/API setup in 63; wrong unit filter zero-test run 69; and audit result-line framing run 104. Real output RED is 22; source-progress RED is 61; stale-feedback RED is 84; invalid stopped-flush atomicity RED is 97. Their subsequent GREEN cycles and raw logs remain distinct.
* Ownership native test 34 and nondefault audio/session preservation added to missing/refused/rollback timer cases after implementation are explicitly post-implementation characterization, not retrospective REDs.
* The original feedback negative observer needed calibration of its actual dynamic `NativeEngine.invoke` lookup. Run 90 confirms real later header Undo records its `setParameter`, and a known explicit read records `getParameterState`. The no-poll/echo claim is qualified to that observed production parameter path, not universal IPC interception.
* Run 59 accidentally used a relative target path. Only its owned group was terminated; its reusable outputs remain preserved. Exact-absolute allowed-cache guards now precede Cargo launch; relative-path rejection was calibrated without starting a builder. `59-accidental-target-inventory.json` retains setup/ownership evidence. No cache was deleted.
* Audit 104 produced provisional successful allocator metrics but failed strict captured-result framing, so it is setup evidence only. Standard capture plus `--show-output` preserves exact parent/child result lines; fresh strengthened 106 and final formatted 112 pass. No callback defect was inferred from 104.

## Ownership return and remaining work

`terminal-cache-source-release.json` records all builders absent and all author sessions complete; the three exact existing caches were returned idle and maintained author sources frozen. At release there were 25,245,839,360 free disk bytes and 12,166,606,848 MemAvailable bytes. Parent explicitly acknowledged taking cache/build/native ownership. This author performs only evidence-folder notes after release and makes no assumptions about subsequently mutable cache artifacts.

The accidental `/tmp/dandrum-lanes/runtime/cache-editor-guard` remains preserved idle, outside source/evidence staging and any portable archive. Root should curate a compact base-plus-changed-source/hash/raw-check/profile/export packet rather than archive repeated source snapshots, installed ELF files, or caches.

Root still owns final maintained driver/CMake preparation and relevant full-owner regressions, independent review, normal commit/push, and documentation/spec/tracker decisions. Required G3 heartbeat/expiry/worker/mapping/full-health/structural Undo gates, calibrated pointer behavior, and continuous GUI create/destroy with concurrent audio remain open. The parameter cases use unmodified app bytes, but they do not establish all full-beta acceptance. Qtractor closed/inactive and suspended-host progress limitations remain explicit. No G3 production was started.
