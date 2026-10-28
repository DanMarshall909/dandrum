# Negotiated browser wire profile

This checkpoint advances task 5.2 of `add-juce-free-trigger-beta-products`.
It adds browser support for the already implemented native profile; no beta
task or scenario closes here. Production NativeEngine initialization, document
editing/import, Empty, native Redo/recovery and portable restoration still need
integration. VST3 and standalone remain deferred.

The browser enables version 2 only after a correlated version-1 negotiation
reply matches the supported method and size profile. It uses version 2 for
`prepareInstrumentSessionPatch` and `getSamplerSession`, with independent
64 KiB decoded-document, 512 KiB request and 1 MiB reply bounds. Other calls and
events retain version 1 and their existing limits. Every pending request stores
its expected reply version, so outstanding old replies and wrong-version
responses cannot consume the wrong promise. Existing capacity, IDs and disposal
ownership remain in place. Native errors retain their explicit code/message.

The initial fast RED posts no large patch after negotiation. The corresponding
actual copied-product RED passes setup and then reports `oversized` from the
production browser transport. After the minimal transport change the real
Wry/Core test admits the meaningful >16 KiB document, publishes the complete
saved document and literal signed PCM, and restores the previous document/audio
through native Undo while the independent instance stays unchanged. The test
now uses the production backend instead of a raw-IPC/cursor workaround.

All154 frontend tests pass, with the changed transport measured at100% lines,
branches and functions by Node22/V8. Raw profiles are retained; coverage measures
execution, while literal frames, boundaries, unresolved old promises, native
errors and real copied-product assertions supply the behavioral oracles. Type
and component checks, Rust formatting and strict all-target adapter Clippy pass.
All30 actual native CLAP checks and12 Chromium native-entrypoint cases pass.
The first Chromium run failed before executing tests because the long scratch
path exceeded the Unix socket limit; a short owned disk-backed scratch path
passes without changing code or weakening checks. Owned process/scratch cleanup
is verified for successful and failed runs; caches are preserved.

[Binding](binding.json) records source and exact external replay identities;
[index](raw-index.json) hashes each [raw packet](raw.tar.gz) member, verified on
readback. Reconstruct source from immutable Git75670196 plus the three current
source overlays. The initial RED archive preserves its original test/production
source; additional negative-profile tables are visible in the final source and
preimplementation RED log. Raw native fixture bindings include complete copied
product/resource hashes. Its exact replay archive remains outside Git at the
bound durable path. Rust production is unchanged from35a79, so prior owning
Rust/LLVM evidence remains applicable; this browser behavior is measured with
V8 rather than claiming new whole-GUI LLVM coverage.

This is headless X11/WebKitGTK2.50.4 and browser component evidence. Physical
input/devices, installed offline artifacts and named-DAW acceptance remain open.
Strict engine Clippy remains FAIL129 baseline diagnostics. Mutation is excluded.
Main remains clean at68dd655. Normal-hook local checkpoint, independent review
and publication are separate recorded steps, not implied by these passing tests.
