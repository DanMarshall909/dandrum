The controller-owned quiescent service and atomic render-configuration boundary are implemented and frozen for fresh root-owned review and normal publication. HEAD remains `301641faa72e57837df3d49c6c4aa6563882d6c4`; no author Git/index/design/docs edits.

**Use the renewed artifacts:** `author-freeze-watermark.json` (manifest SHA256 `4239394e6e21e63a3e2994caebc32f10c8f8f3a727f29a4d8112a1a7d85ed5a4`), `coverage-qualification-watermark.json`, `completion.json`, and `terminal-resources-watermark.json`. Original `author-freeze.json` and all earlier gates/proofs remain preserved under their original names. `watermark-only-test.diff` records the root-authorized assertion change. The renewed host test hash is `f725fcc7d035abad53727a478e386e869cf62a04d4abee3b44ac363a8e77f05e`; every production/Session-test byte, mode and mtime matches the original freeze.

Candidate paths are precisely the five files in the renewed freeze: Session `src/lib.rs` (owned preparation visibility only), host `src/parameters.rs` and `src/runtime.rs`, new Session `tests/render_configuration.rs`, and new host `tests/render_reconfiguration.rs`. Drafts and phase/source snapshots in this evidence directory are historical evidence, not candidate test routes.

The public APIs are `ParameterController::service_quiescent(&mut self, &mut RuntimeAudio) -> Result<QuiescentReport, ParameterError>` and `reconfigure_quiescent(&mut self, &mut RuntimeAudio, TriggerHostConfiguration) -> Result<ReconfigurationReport, ParameterError>`; the report is `Unchanged` or `Changed`. Foreign identity precedes bounds/Busy; failures consume no ownership work. Equal complete configuration preserves DSP position without I/O/allocation/destruction. A narrow precommit activation lease prepares the entire runtime before Session mutation, validates retained layout/event capacity, publishes Session/runtime/control configuration infallibly, and destroys old owners on calling control.

Final gates:

| Gate | Evidence phase | Result |
|---|---|---|
| Host fmt / all-target strict Clippy | `17-watermark-fmt-check`, `18-watermark-clippy` | PASS; 0.50s Clippy |
| Full host + compile-fail docs | `19-watermark-full-host` | 138 + 2 PASS, 3.02s |
| Full host LLVM | `20-watermark-host-cov` | 138 PASS, 29.14s |
| Host raw remerge/export proof | `21-watermark-host-proof` | 12 raw profiles / 12 actual executed binary SHAs + fingerprints; 3.02s |
| Full Session ordinary / LLVM | `07-full-session`, `10-full-session-cov` | 33 PASS; 5.53s / 8.04s |
| Session raw remerge/export proof | `11-session-profile-proof` | 3 raw / 3 actual binary SHAs + fingerprints |
| Session fmt / all-target strict Clippy | `07-session-fmt-check`, `12-final-session-clippy` | PASS |
| OpenSpec strict / diff check | `13-openspec-validate-explicit-node`, `14-diff-check` | PASS |

Session proof reuse after the host-only assertion is verified in `session-final-proof-reuse.json`. Both merged profdata files reproduce byte for byte; all export paths, segments, functions, regions and integer counters match exactly. Derived percentages differ by about 1.42e-14 and Cargo adds metadata; originals are retained.

There are 10 new host public tests and 3 new Session characterization tests. The earlier atomic focused16 was 9 new host integration tests plus 7 existing runtime-quiescent integration tests. Nonzero UI settlement/reset cause/restore epoch, revision different from epoch, queues/feedback/gestures/history/outcomes, same-request watermark rejection, worker/pending/retired Busy, E65 shrink rejection, resource failure, editor arbitration, separate audio ownership return and signed PCM/fractional continuation are publicly asserted. Callback allocation/destruction audits are zero; control replacement has a signed negative live-allocation witness.

Qualification: three new defensive propagation regions remain unhit (runtime constructor error exit and the two Session-error arm regions under exclusive same-owner commit). Root accepted preserving them without artificial seams. No blanket 100% or branch/MC/DC claim. Ordinary-note FIFO relies the published runtime proof plus untouched endpoint/queue inspection. Author oracle/setup errors are disclosed in `completion.json`; raw failures remain and are not relabelled RED. Mutation is excluded; new source-free/default-release/CMake/CLAP/native acceptance is deferred. Task4.1 and beta remain open.

**Terminal cache/output handback:** all author Cargo/test/LLVM/profile processes are terminal; frozen bytes/modes/mtimes verified exactly. Both `/tmp/dandrum-lanes/cache-runtime` and `/tmp/dandrum-lanes/cache-host-snapshot-cov` are returned to root. No further author target access or source edits are planned absent a root/reviewer request. Final actual resources: disk16.91GB, MemAvailable12.06GB; RAM is an unused output volume. All task outputs/TMPDIR/profiles used disk, original floors retained, no cache cleanup/move. `evidence-inventory.json` binds the stable packet files; root owns archive curation, review, docs and publication.
