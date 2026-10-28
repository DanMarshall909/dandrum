# Selected-sample and editor foundation completion audit

This is the clause-to-evidence audit for tasks 5.1–5.5 of
`add-juce-free-trigger-beta-products`. The complete product source is published
commit `a6303b0762830c12057edab2a59817985c6573bc`, tree
`ef8d6c6b88215e3b70916e7d4108a8b335bd403b`. This audit changes no behavior,
requirements, scenario mapping or dependency. The sole live workflow tracker
remains `.git/codex/deferred-work.md` in the shared repository metadata.

| Task and clauses | Owning executable proof | Portable evidence |
|---|---|---|
| 5.1 Immutable encoded/decoded identity and metadata; original replacement/removal; instance isolation; memory bounds; off-audio lifetime/reclamation | `selected_samples::selected_runtime_pins_bytes_until_control_reclamation_without_callback_destruction`; `pending_work_is_charged_until_its_cross_thread_completion_is_consumed`; `retained_results_and_prepared_consumers_stay_charged_and_reuse_content`; `batch_failure_cancellation_and_foreign_completion_preserve_capacity`; `file_count_encoded_decoded_and_resource_limits_reject_atomically`. Real cross-thread signed PCM, callback allocation/free sentinels, absence of original files/root and four real 64 MiB source probes exercise the actual engine/catalog/session owners. | [Snapshot foundation](../trigger-selected-snapshot-foundation-2026-10-23/README.md), including resolved independent review findings and exact LLVM/native/resource-budget evidence. |
| 5.2 Negotiated bounded protocol; v1 compatibility; invalid/stale/conflicting/busy outcomes; unadmitted versus admitted ownership | `protocol.rs` tests `negotiated_patch_frames_admit_real_larger_documents_and_retain_exact_replay_identity`, `negotiated_wire_document_and_reply_bounds_are_independent_of_version_one`, `document_replay_byte_pressure_retires_old_documents_without_evicting_small_requests`, `transport_retries_never_repeat_admission_conflicting_or_retired_ids_are_rejected`. `selected_admission.rs` tests view-owned choosing, stale/foreign callback refusal, bounded recoverable outcomes, shared-worker admission, worker loss and admitted view loss. Actual Wry ingress tests exercise negotiated >16 KiB documents and reject forged/unnegotiated/oversized frames. | [Bounded patch admission](../trigger-bounded-patch-admission-2026-10-27/README.md), [snapshot foundation](../trigger-selected-snapshot-foundation-2026-10-23/README.md). |
| 5.3 Native full-document/history publication; fixed public identities and physical values; compatible defaults versus true layout rejection; Empty; once-only import; signed Undo/Redo | Adapter-library `whole_patch_publication_survives_view_loss_and_native_undo_preserves_physical_values` changes authored defaults while retaining nondefault live values, maps MIDI 36→60 and checks signed output/history/retries. `whole_patch_rejects_unsupported_profile_before_owning_a_job` rejects incompatible interfaces/resources/profile limits before admission. Host `silent_empty_document_keeps_product_parameter_scopes_values_and_callback_ownership` and actual native `copied_react_empty_retains_controls_and_imports_after_recreation` prove silent resource-free Empty, 23 stable public controls and fresh-editor selected import. `copied_react_import_maps_selected_pcm_once_through_native_history` proves one import and UI Undo/Redo. | [Bounded patch admission](../trigger-bounded-patch-admission-2026-10-27/README.md), [fixed descriptions](../trigger-empty-control-descriptors-2026-10-29/README.md), [native Empty](../trigger-native-empty-2026-10-29/README.md), [native Import](../trigger-native-selected-import-2026-10-28/README.md). |
| 5.4 Actual GTK acquisition and production frontend; decoded metadata; cancel/invalid/stale-view cleanup; normal nonoverlapping mapping | Actual native `copied_native_selection_retains_real_metadata_across_source_and_editor_loss` and `copied_react_import_maps_selected_pcm_once_through_native_history` use real GTK file selection and production React/Wry, truthful 32000 Hz/stereo/4-frame metadata and a literal 60-byte WAV. Native acquisition tests cover malformed/unreadable file shapes, non-UTF-8 names, symlinks and a bounded FIFO child. Production `native-selection.test.mjs` and `native-selected-import.test.mjs` prove invalid metadata, foreign owners, late views, next-unused-note mapping, full-keyboard refusal and once-only disposal via Cancel/Escape/replacement; Chromium and actual native cancellation calibrate the entrypoints. | [Snapshot foundation](../trigger-selected-snapshot-foundation-2026-10-23/README.md), [native Import](../trigger-native-selected-import-2026-10-28/README.md), [import disposal](../trigger-native-import-disposal-2026-10-28/README.md). |
| 5.5 Import/mapping/status/recovery/Undo across complete editor destruction and independent instances; selected signed waveform; no late callback/resource leak | Actual native `copied_react_recovers_selected_import_after_lost_reply_and_recreation` withholds the admission reply under real runtime backpressure, removes the original WAV, destroys the Wry child and observes automatic recovery by a fresh production consumer. One host block permits publication once; UI Undo/Redo retain 23 controls and physical gain 2, literal selected [-0.25,-0.25], kit [-1,-1], unmapped silence and an unchanged second instance. `native-recovery.test.mjs` covers closed continuations, failed-job recovery and the final permitted probe. Controller tests retain/release owned outcomes independently of views; native destruction assertions verify owned child/window cleanup. | [Pending-ID discovery](../trigger-pending-request-discovery-2026-10-29/README.md), [production recovery](../trigger-editor-pending-recovery-2026-10-29/README.md), [actual native recovery](../trigger-native-pending-recovery-2026-10-29/README.md), [final-probe repair](../trigger-recovery-final-probe-2026-10-29/README.md). |

## Current combined checks

[Binding](binding.json) identifies a fresh complete current-source owner run and
hashes every member of [raw owner logs](owners.tar.gz), read back after creation.
Session: 41 tests. Host: 192 tests and two doctests. Editor adapter library:
23 tests, plus one deliberately ignored FIFO child launched by its passing parent
`native_acquisition_rejects_fifo_without_waiting_for_a_writer`; it cannot be
counted as acquisition evidence without its completion marker. All suites pass.
The owned supervisor reports exit 0, no remaining workers and scratch removed.
No full-owner coverage percentage is inferred from this ordinary test run.

The final-probe packet binds the current 281 Node tests/types/components/V8,
33 native CLAP checks (119.70 s), 14 Chromium checks, exact copied production
bundle and unfiltered official validator: 34 successes, 10 explicit unsupported
capability skips, zero failures. Nine source-free CMake checks pass on the same
unchanged Rust/CMake boundary. Formatting and strict host/editor-adapter Clippy,
active and all 52 canonical strict OpenSpec checks, and the unchanged spec map
(631 scenarios, 392 mapped, 239 ratchet) pass. The exact candidate and all repair
interactions received independent PASS before normal publication. Each linked
packet retains genuine RED evidence separately from later characterizations,
setup failures and historical predecessor calibration.

## Explicit remaining limits

These task-5 component outcomes do not qualify installed/offline execution,
actual desktop input, uninterrupted selected audio, audio-device playback or
named-DAW behavior. Tasks 6.1–8.2 retain portable selected/Empty state restoration,
installed/native/DAW proof, final audits and scenario synchronization. No beta
scenario closes merely because its component task is complete.

Strict engine Clippy remains FAIL with 129 unchanged baseline diagnostics; this
is not a waiver or passing engine lint gate. Snapshot/catalog/private defensive
coverage gaps and partial frontend branch/function coverage remain explicit in
the original packets. No new behavior or extracted module is introduced here.
Mutation stays excluded. Main stays clean at 68dd655. VST3/standalone remain
later increments. Reusable disk-backed caches are preserved; only verified
run-owned scratch was removed. The estimated 64 GiB cache-retention budget remains
exceeded by the measured roughly 80 GiB ordinary/LLVM/native targets; that
measurement grants no cache deletion authority.
