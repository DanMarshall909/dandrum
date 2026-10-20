# Packaged Trigger editor assets

This is a verified asset component of task 3.2, not completion of the Wry editor
or full beta. Start with receipt.json and author/HANDOFF.md. The author directory
retains exact raw logs, command receipts, candidate source copies and hashes,
production input/output hashes, resource samples and source-free stage logs.
SHA256SUMS covers every retained file except itself.

The real production bundle was rebuilt from source bytes matching the candidate.
Four fast tests assert the local URL/package policy. The separately selected
production calibration executes through CTest against the staged full bundle;
its default ignored status is not counted as passing evidence. The tiny Python
source-free fixture proves build isolation and selected runtime behavior, not
production font rendering. Its first attempt stopped at a disk threshold; the
unchanged guard passed from a hash-matched RAM candidate with existing caches
preserved. The original interrupted logs and successful rerun both remain here.

RED logs are exact historical failures. The initial stub was reconstructed later
from its original tool write and is labelled accordingly; a contemporaneous RED
source snapshot and some historical timings are unavailable. A test-literal
Clippy correction is retained and is not behavior RED. Mutation is excluded.

Coverage is source-bound: raw LLVM LF/LH is 52/52, all 49 distinct emitted DA
records are hit, and all five unique function/closure starts are hit. Raw mangled
instances are 7/11 due duplicate generic closures. No branch coverage claim.
author/summarize-coverage.py reproduces the retained accounting.

Reproduction uses the existing asset pipeline: build web/trigger with
`npm run build:native`, configure CMake with DANDRUM_ENGINE_ONLY=ON,
DANDRUM_CLAP_PROOF=ON, DANDRUM_TRIGGER_CLAP_EDITOR=ON and
DANDRUM_TRIGGER_EDITOR_BUNDLE_DIR pointing to that complete dist-native bundle.
Build dandrum-trigger-clap-editor, then run the two trigger-editor CTests. The
exact commands/environment used here are in author/final-run-receipts.json and
author/02-registration-green-receipts.json. The source-free command and source
identity are in author/11 records. Installed dependencies/caches are prerequisites;
this receipt does not claim a dependency-free clean-machine build.

No actual browser/network policy, native parenting/lifecycle, session UI, CLAP GUI
or uninterrupted audio acceptance is claimed. Task 3.2 stays unchecked. The next
component pins/audits Wry before native adoption and proves those native boundaries.

Whitespace-bearing raw tool logs, CMake JSON and diff output are stored as
deterministic gzip files. artifact-map.json maps original names to stored files
and records both original-byte and stored-byte SHA256. Decompress with gzip to
recover the exact original content; no log whitespace was normalized. The
author/evidence-sha256.json entries refer to those original decoded bytes.
Original author files and reusable caches remain untouched.
