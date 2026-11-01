## Context

See proposal.md for scope. The investigation is integrated at `68dd655`; task prerequisites were published at `d2d06cf`, with 18 engine CTests and 1,091 Rust tests passing. Pinned Clack `27ca28391b57a61bad4bb5099288a7bf9247350d` accepts the compiler-checked `Send` runtime. The source Windows machine had about 2.66 GiB free on C:. Home-PC continuation uses native Linux storage; record current physical build-volume capacity and preserve reusable caches on either machine.

## Goals / Non-Goals

**Goals:** actual exported ABI/dynamic host proof for two immutable included engine fixtures, public root controls and host-owned exclusive render lifecycle.

**Non-Goals:** production framework selection, new DSP, authoring/session/journal duplication, state envelope or concurrent state-restore claims, stopped parameter flush proof, GUI/WebView/device/other-format support, cutover or complete J2. These remain required follow-ups in the full issue #24 receipt.

## Decisions

- Isolate src/clap-adapter as an experimental Rust cdylib/rlib with engine path dependency and Clack plugin/extensions pinned to the exact reviewed revision. Only audio-ports/note-ports/params plus plugin support are needed; dev host uses libloading with unnecessary default/static entry support disabled. Keep the engine manifest/API independent of Clack. Do not change the project licence; record upstream MIT/Apache attribution and exact dependency receipt without calling the production licence gate resolved.
- Include the existing TB-303 document as a compiled resource and add a minimal gain child-module fixture. Choose experimental IDs distinct from unreleased JUCE products and document explicit adapter numeric mappings, including root alias metadata. No second instrument model or authoring journal is introduced; #23 remains the shared session/SDK owner.
- Prepare root input/control/output planes off audio in resolved root-port order. Public parameter aliases resolve to root controls; do not bypass root summing/mappings by setting compiled child slots. Preallocate capacity at activation; segment rendering by bounded host event timestamps, preserving same-offset order. Compare signed gain math independently and characterize a literal TB-303 sample baseline before adapter implementation, alongside a direct engine parity oracle.
- Advertise MIDI1 only for TB-303, no note IDs/expression/per-note modulation/MIDI2. Forward note number, velocity and block-relative timing to the existing engine; test overlap behavior actually supported by the mono instrument. No advertised capabilities without implementation.
- Tests dynamically load the built .clap file with PluginEntry::load, keeping the entry alive until instances/processors are stopped/deactivated/destroyed. A static load_from_clack entry is not acceptable evidence. Establish current public engine baseline, then compile a minimal adapter scaffold with no working exported factory and confirm discovery/rendering RED; implement against that test.
- Validate blocks, event bounds/order/count, parameter IDs/finite/ranges and buffer layouts before indexing. Ordinary malformed callback input must use deterministic silent/error-status fallback and deferred atomic diagnostics, never PluginError/panic logging: the pinned wrapper error logger allocates String/CString or writes stderr. Do not advertise 64-bit audio or in-place support until proved; valid separate float32 stereo is the first proof. Callback allocation/I/O/reclamation hardening still needs dedicated subsequent evidence; no broad RT claim from a clean render alone.
- Add an explicit default-OFF CMake plugin selection before the engine-only return. Package an experimental .clap artifact and register only the dynamic proof for this opt-in selection. Existing 18 engine checks/default comparison/demos stay valid; no launchable demo is added. Share CMake's task rust-target cache; seed compatible dependency resolution from retained engine Cargo.lock, retain exact plugin lockfile, and match the engine root release profile (`lto = true`, `codegen-units = 1`) and avoid additional target/profile trees. A package-specific `strip = "debuginfo"` override for only the new adapter dev/test artifacts is allowed to reduce linked debug size while preserving engine/dependency profiles and cache reuse. Run baseline already verified once; rerun only affected build/guard/plugin checks. Stop new compile work before the physical build volume falls below 1 GiB; do not delete preserved outputs.

## Risks / Trade-offs

- [Headless sound mistaken for completed migration] → Receipt retains state/flush/host-gesture, parented production React/lifecycle, native second OS/VST3, session and device gates as pending. Official validator failures are inspected and resolved or explicitly scoped; do not hide them with exclusions.
- [Unsafe foreign-library ownership] → Limit the required unsafe loader to the verified locally built artifact and lifetime-safe host wrappers; plugin implementation uses safe owned data, not unsafe Send or runtime mutex.
- [Scope-specific fixture IDs mistaken for release ABI] → Experimental naming/README and feature selection distinguish them; release identities remain separately designed and validated.
- [Cache/disk growth] → Use the exact existing cache/lock/profile, bounded jobs and live C: monitoring. No broad compilation or cleanup after already passed regression evidence.

## Migration Plan

Public baseline, genuine dynamic-loader RED, bounded adapter GREEN, independent build/dependency receipt and official validator run, scoped acceptance mapping/review/publication. Continue state/stopped-flush/hardening and parented React proofs without selecting a production stack from this slice alone.

## Verified implementation findings

The actual validator rate matrix includes768kHz and fractional rates. For these two resource-free included fixtures, preparation metadata uses the rounded integer rate while the existing floating-point DSP runtime receives the actual host rate; owned-asset resampling is not proved by this slice. Admission validates fixed event counts, missing entries, timestamp order, global finite/ranged parameters and each same-offset note batch against the actual prepared queue before mutation. Silent rejection returns Continue so held instruments are not accidentally put to sleep. A release-profile mismatch caused an initial separate dependency build; profiles are now aligned with the retained engine and both prior caches remain preserved.
