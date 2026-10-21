# Fractional-rate contract check

Read-only, 2026-10-21; no implementation, execution, downloads, cache/Git/spec/tracker changes. The reported official-validator result is 33 successes, 10 skips, one process-varying-sample-rates failure at fractional1234.57Hz. This review neither waives that failure nor decides publication/acceptance. Task4.3/5.1/full-beta validation remains open; no exclusions are proposed.

**Contract finding:** the inspected primary CLAP activation declaration accepts double sample_rate and returns bool, documented as true on success (plugin.h:60–73, retained official CLAP pin195b42a004144fab0b3cf95e9c067187d15365b7). It promises constant rate/frame bounds during activation; it does not explicitly require success for every positive f64 rate. Fractional rates are representable legitimate host requests, not malformed ABI input. Returning activation failure is a supported outcome.

Actual Clack27ca explicitly documents that activate may fail for any reason, leaving the plugin unactivated (plugin/src/plugin.rs:188–204). Its wrapper forwards the original f64 and maps Result success/failure to the ABI bool (instance.rs:259–276). The actually linked clap-sys0.5.0 binding confirms f64→bool (src/plugin.rs:30–39). Thus the validator's required acceptance of1234.57 is a compatibility/test criterion beyond an explicit universal-rate mandate found in these inspected declarations. Its failure remains real evidence, not an ABI violation proved solely by this result. The validator's own test source was not inspected here.

**Changing this is not an adapter-cast fix.** TriggerHostConfiguration.sample_rate_hz is u32 (trigger-host/lib.rs:27–29), propagated into RenderSettings, PreparationContext and persistent Session preparation/reconfiguration. PreparedRuntime requires matching integer metadata, then passes rate as f32 to RealtimeGraphProcessor (runtime.rs:80–84,126–130). Genuine fractional support would require a deliberate retained host-rate/precision contract through these owners, validation, configuration equality and rollback, serialization/shared consumers, and DSP construction—not rounding/truncating the request.

Asset handling also separates native WAV rate from host rate: LoadedSample metadata remains integer WAV Hz; ResourceResolver's key includes integer host Hz and matching-rate policy (preparation.rs:101–108,161–174). Matching-rate loaders currently take u32, while sampler lowering computes native-source/host playback scale as f32 (1827,1945–1946; similarly slicer/granular paths). Preserving fractional host timing requires auditing these policies/ratios and signed multi-frame/interpolation oracles, without relabelling native WAV metadata. Broader source traversal and implementation are outside this check.

The current accepted integer-only task4.1 policy explains the failure. Whether to expand that policy or retain an explicit unmet validator criterion is the root's unresolved decision.

Source SHA256 (root-relative unless named):
- CLAP plugin.h: `d2a178137349749eb1dd9c1d6a437300309044626dcc14b0f6e31995f1c5f25b`
- clap-sys0.5.0 plugin.rs: `88c9496747f2f418f6905be9d4a2eddcbf78f56d4faf2255aff0acc966f021d5`
- Clack plugin/src/plugin.rs: `7bf93473b1ca0caef8f57db8754311fd71f5d2185e2d921e4a0626758c5964f4`
- Clack plugin/src/plugin/instance.rs: `764afc9f332bd7d778aed5b210742e6fea8526ddf187ad3dad2d260570eb0aef`
- src/trigger-host/src/lib.rs: `d16daf1630f43184909847e03107d402a8d54f3bf2fe6085a8bfd966c1aa80f9`
- src/trigger-host/src/runtime.rs: `41cb146824e21aa3c5814885b5561cd00d8152b5390338af25af26bc96f62592`
- src/rust-engine/src/patch.rs: `177747327e15f9e8f52f7a1947a2d8d1a3b0d0aaed2212bd538c17523403f128`
- src/rust-engine/src/preparation.rs: `1179b659082e4751e8ef7833dc582900d6da95520c25b22ef1d8ac2079f2f8cf`
- src/rust-engine/src/audio_loading.rs: `9e456cafe2eef0d62da62a4ad60c0c9859c27c74bf88727f4725fa8d8609c25b`
- src/instrument-session/src/lib.rs: `b010be63d2a6c61e7362e80418acdb53ab35bf15f82d242a3d0c9c4fbd647cf4`
