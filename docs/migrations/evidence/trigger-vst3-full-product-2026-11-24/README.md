# Trigger VST3 Linux whole-product qualification

OpenSpec task 6.1 qualifies product source `c62fd4924b981b6b17a1f2211c9f2530afa1646d` and the same 361-file installed bundle used for [REAPER task 5.2](../trigger-vst3-reaper-2026-11-24/README.md). The VST3 binary SHA-256 is `16f00ecec644d9ece503886d42d360b33cf8dfac39cd326d8d853ab8c5841406`. Commit `c5d2941b` adds the REAPER evidence and task checkbox. The product sources, manifests, locks and editor inputs match the candidate.

## Test results

| Check | Result |
| --- | --- |
| VST3 ordinary and native | 89 ordinary tests and three explicit X11/WebKit journeys passed. The native allocator audit measured 704 complete callbacks with zero allocations or frees and 351 signed renders. Ordinary parent tests launch the three private deadline/allocator children. |
| Configured fast and slow regression | 47 distinct CTest cases passed across the ordinary, bare X11 and PipeWire/JACK groups, including the actual 256 MiB shared state, engine and session suites, installed assets, dynamic CLAP loading and the native/editor fixture lanes. |
| Production React | 314 package tests passed. Type and component checks passed for the current frontend inputs. |
| Installed VST3 | The copied-bundle dynamic host passed with real native editing, stopped service, recreation, signed audio and independent instances. Task 5.2 already exercised the same binary in actual REAPER. |

The shared-state slow case consumes the full 268,435,456-byte encoded payload across four WAVs and restores 536,870,560 decoded bytes. It checks signed output before and after fresh restore, byte-for-byte resave, a final-byte truncation after valid prior content, atomic capacity refusal, and reclamation across owner lifetimes. The standalone execution of the same CTest-built binary peaked at 1,400,052 KiB RSS. The configured case passed separately.

The installed CLAP regression uses the CMake-installed binary `ef88c0fb42dad7d3f10c59bf3692054808c4c2c7efd17bb665b89b16fbea312d`. Its standalone native host completed 376 audited callbacks, received 18 MIDI events, and balanced both gestures in the task-owned 44.1 kHz / 1024-frame PipeWire/JACK session. The 24 parameter scenarios also exercised selection, instance isolation, ownership, callback safety, Undo and view loss through the configured private fixtures.

## Validator and installed dependencies

The pinned Steinberg SDK 3.8.0 build 66 validator ran with `-e` and no filters. It returned 255 with **535 passes and two failures**. The two float32 cases are block and sample accuracy with all 23 parameters changing every sample. Each supplies 1,472 automation points in a 64-frame block. The agreed shared array capacity is 1,024 points plus notes total. The owner test accepts exactly 1,024, rejects 1,025, clears the accessible output, and preserves the signed playback cursor for the next call. Those validator workloads remain unsupported. The product advertises stereo float32 output. Mono, surround and float64 processing are outside the qualified configuration.

The bundle audit rehashed the complete manifest and matched 203 resolved normal/build Rust packages, four JavaScript runtime packages and 344 notice text payloads, including font and supplemental notices. The manifest and lock digests matched the notice inventory. Both installed native products resolved their ELF dependency closures on this machine without JUCE or RPATH/RUNPATH. The exact VST3 bundle also passed the native dynamic-host journey from an unrelated directory with the checkout masked and the network namespace without routes. These private artifacts establish this Linux configuration. Native Windows and macOS and public binary distribution remain separate milestones.

## Scenario evidence

| OpenSpec scenario | Assertions and evidence |
| --- | --- |
| Installed bundle is discovered outside the checkout | Current source-hidden native product journey and [REAPER module maps](../trigger-vst3-reaper-2026-11-24/verification.json) bind host execution to the retained bundle. `discovery` verifies stable classes, all public IDs and explicit missing/invalid-resource rejection. |
| Processor works without a controller | `factory_context_services_processor_without_controller_or_editor_and_releases_registration`, `processor_only_partial_streams_restore_all_named_values_and_portable_signed_sound_fresh` and `stopped_normalized_points_retain_exact_endpoint_without_playing_or_advancing` drive the real processor with isolated ownership. |
| Controller disconnection preserves one instrument authority | `separate_components_exchange_projection_through_standard_interface_only_proxies`, the processor-owner history and reply-capacity cases, and `connection` stale/reentrant/failed delivery cases prove one session across independently revoked projections. |
| Timed notes and public values render literal signed output | The `audio` tests compare signed stereo values at known offsets and a fractional clock, including two independent processors and rejected reconfiguration. The `discovery` owner checks normalized/physical conversions, defaults and stable IDs. |
| Curves retain exact timing across block partitions | `standard_parameter_queues_render_every_signed_ramp_and_jump_sample`, `standard_host_curves_are_invariant_under_equivalent_block_partitioning` and `equal_frame_notes_keep_host_order_and_sample_the_new_onset_control` use independent literal signed samples. The REAPER render calibration rejects shifted and stepped curves. |
| Stopped flush changes values without advancing sound | `stopped_normalized_points_retain_exact_endpoint_without_playing_or_advancing` and `stopped_processor_feedback_updates_its_projection_and_retries_refused_delivery` check sound, cursor, feedback and history across stopped delivery. |
| Invalid or excessive callback batches fail atomically | `curves` and `audio` verify output clearing, pending transition preservation and the exact 1,024/1,025 boundary. The calibrated `callback_audit` and the actual native overlap drive complete processing and reset calls with allocator interception. |
| Fresh VST3 instance restores selected samples and all values | `selected_payload_reencodes_and_restores_after_source_instance_and_file_loss` and `controller_component_state_projects_all_values_without_playable_session_or_host_edits` prove resource bytes, literal output, legacy-reader support and projection-only restore. The fresh REAPER process independently restores all 46 values and both instruments. |
| Invalid streams preserve working playback and history | `invalid_state_and_failed_streams_preserve_nonempty_undo_redo_values_and_signed_cursor` and the processor-only partial-stream case exercise short successful I/O, zero progress, failure, and atomic state/history preservation. The 256 MiB slow case proves the shared upper boundary. |
| Actual native editor destruction retains Undo and service | `production_native_selection_retains_decoded_content_after_source_and_view_loss` runs real React and GTK selection with note/gesture teardown and signed Undo/Redo. The REAPER breakpoint witness proves admitted work finishes with transport stopped and the native editor absent. |
| Failed creation and missing service preserve a usable instrument | `view_native` exercises failed partial parenting/service, parent survival and successful reattachment. `service` and `connected_view_without_host_loop_fails_without_native_creation` prove a clean capability refusal and explicit registration recovery. |
| Host content scaling preserves the complete editor | The VST3 `view` tests negotiate valid factors and retain old geometry on invalid input or refused resizing, including reentrant host frame release. The configured shared-editor geometry run and [installed DPI result](../trigger-vst3-dpi-2026-11-23/README.md) prove both layers and the live editor. |
| REAPER restores independent instances with timed automation | [Task 5.2](../trigger-vst3-reaper-2026-11-24/README.md) proves two native instances, four independent signed renders, retained Undo/Redo and byte-exact portable restore. The current native overlap, CLAP regressions and full validator outcomes are retained with this audit. |

## Coverage and refactoring review

| Owning production code | Ordinary lines | Native lines | Combined lines | Changed lines exercised |
| --- | ---: | ---: | ---: | ---: |
| VST3 adapter cdylib | 3,345 / 3,932 | 2,841 / 3,932 | 3,706 / 3,932 | 802 / 874 |
| Shared Trigger editor | 52 / 510 | 433 / 510 | 440 / 510 | 167 / 168 |

Coverage was exported from 36 fresh VST3 and three fresh shared-editor LLVM profiles against their identified objects. Changed lines are added or replaced executable lines since the VST3 foundation merge `88328948`. The [verification record](verification.json) retains every instrumented changed-line gap. Examples include startup cleanup after worker creation fails, rejected requests from a retiring view and an X11 visibility request error. The reports measure line and region execution. Branch and mutation evidence were outside the measured scope.

Both ABI adapters use the shared Trigger session/runtime, portable state, parameter policy and editor framing and commands. VST3 keeps host services, COM frame negotiation and scoped projection transport in its adapter. CLAP keeps timer, GUI and host gesture ABI integration. The shared editor owns X11 presentation and file acquisition. The new view code and the selection and gesture handlers follow those boundaries. Splitting the largest controller file would move lifecycle and acknowledgment handling across a new boundary, so this qualification leaves the code as reviewed and records the uncovered paths for a future scoped change.

The first configured run hit managed Cargo disk admission refusals. After a separately approved 12 GiB preflight for verified warm profiles and the three remaining debug checks, all configured cases ran. The work preserved the 8 GiB disk/RAM floors and left the repository's 17 GiB default policy in place. A scratch monitor initially double-counted managed exports. It recovered the archived inactive producer marker with the cache lease held and reran the affected cases with disjoint accounting.

The shared geometry case failed on the window-managed display. It passed on a bare X11 display, where the fixture controls its host-parent dimensions. Shared editor coverage runs use a fresh process per GTK case after the initial two-case invocation exposed GTK's thread-affinity constraint. These original attempts remain in the local logs.

Private raw logs, JUnit, coverage profiles and receipts are retained under `.git/codex/vst3-full-product-20261124/`. Combined review and publication are tracked in [issue #45](https://github.com/DanMarshall909/dandrum/issues/45) and [OpenSpec tasks 6.2–6.3](../../../../openspec/changes/prove-trigger-vst3-linux/tasks.md).
