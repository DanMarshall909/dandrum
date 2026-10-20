# Independent completion review — fixed-entry navigation

Verdict: **PASS**. No actionable in-scope finding remains. This approves one fixed-entry top-level navigation criterion within still-open OpenSpec task 3.2, plus the finite packet envelope. It does not complete task 3.2, synchronize specifications, or certify the beta.

Candidate manifest SHA256: `470c8e004026906072b04731a1a1cb4934dfc9d894b712bcd6107b89e990152b` (17 paths).
Base/current/integration/recorded intended upstream: `8d002a1201e59e03da94381e8411672767200853`.
Branch: `lane/trigger-runtime-20261020`; intended upstream/publication target: `origin/feat/juce-free-host-adapters`.

## Independence and method

I independently read the packet, project AGENTS.md, shared AI rules, agentic-delivery and completion-review reference, behavior-first-tdd, coverage-and-mutation, security-by-design and reflect guidance. I read the owning proposal/design/tasks and both delta specifications, the complete production owner, changed tests/fixture, existing native lifecycle assertions, asset owner/package calibration, CMake registration and active hook entrypoints. I inspected the entire staged diff, all portable receipts and source identities, raw baseline/RED/GREEN/initial-coverage/refactor/final logs, coverage exports/annotated source and orchestration scripts. I did not delegate approval, edit repository/index, run builds/tests/coverage/native sessions/network, or change caches. Writes are confined to this reviewer directory.

## Candidate reconstruction

Independent read-only checks establish:

- The manifest serialization has the stated SHA256. All 17 paths match the complete live Git status inventory in bytewise order; no unstaged/untracked extra path exists. Every canonical mode, staged blob ID, index/worktree status, SHA256 and staged-versus-worktree byte comparison matches. There is no unresolved index or special file. Cached whitespace check passes.
- HEAD, local branch, intended remote-tracking object and merge-base reproduce the packet identity. The runtime lane has no configured `@{u}`; the manifest deliberately names the intended feature upstream rather than claiming branch tracking. The finite publication procedure must explicitly use that target. Main independently remains clean at `68dd6553a2f07e71e2fe09ee1d7094d518559937`.
- The raw archive SHA256 is `996741da861ac7982b24410f42330dcce0ffaa61b4b9b0aa1532abf70f022634`. All 234 members are regular files with safe relative paths, exactly matching the artifact index order/set, byte counts, modes and SHA256. Every member also matches the retained author artifact under `/tmp/dandrum-lanes/evidence-editor-navigation` byte-for-byte and mode-for-mode. No extraction was necessary. All portable SHA256SUMS entries verify. Portable and archived handoff/source-manifest bytes agree.
- Every contemporaneous run snapshot matches its own source-hash receipt. Every final run's entire snapshot set matches the current candidate paths. Baseline and RED production native owner bytes equal base `8d002a1`; the new navigation fixture is identical through baseline/RED/GREEN/refactor/final snapshots. The GREEN-to-initial-coverage test change strengthens the independent HTML calibration with an explicit HTTP200 assertion. Owning test bytes then remain identical through refactor and final coverage.
- CMake, editor manifest/lockfile, lib.rs and assets.rs independently match both base and unchanged-boundary hashes. No build/dependency/feature/license change is hidden in this slice.

## Task-to-implementation matrix

| Owner/task | Implementation and evidence | Completion claim assessed |
|---|---|---|
| 3.2 local-only loading: fixed top-level entry criterion | Private `ENTRY_URI` plus Wry navigation callback in `src/trigger-editor/src/native_x11.rs`; initial load uses same constant | Criterion satisfied for the bounded ordinary-page navigation scope |
| 3.2 preserve usable packaged document | `tests/fixtures/navigation.mjs` and `tests/internal/local_policy.rs` exercise local module/CSS/inline style/fetch/IPC around denied navigation | Satisfied; resource policy overall remains incomplete |
| 3.2 actual parenting/lifecycle neighbors | Existing native real-parent test retains acquisition/destruction/reparent-error/callback-release assertions and calls the new criterion helper | Existing protection rerun; show/hide/resize/API/order and actual CLAP main-thread integration are not claimed complete |
| 5.1 focused verification/refactoring | Source-bound native+fast LLVM, three registered CTests, Clippy warnings denied, build/fmt/default graph; covered constant refactor | Evidence for this material slice only; full task remains open |
| 5.2 review/publication | This independent review; staged portable evidence/progress; normal-hook finite publication still pending | Review gate earned; publication must follow envelope |
| 3.3, 4.x and beta | No implementation/checkbox changes | Explicitly outstanding |

No canonical requirement/scenario text or spec-tests.map mapping changes. The scoped OpenSpec validation and canonical spec-coverage logs pass with unchanged 618 scenarios / 379 mapped / 239 backlog. Keeping task 3.2 unchecked and avoiding a claim to prove the whole copied-production-bundle scenario is correct.

## Criterion-to-test matrix

All rows use maintained native test `native_x11::tests::real_child_loads_local_content_and_drops_without_owning_the_host_parent`, via the new `top_level_document_stays_at_fixed_entry` helper unless specified otherwise.

| Observable criterion | Independent oracle | Evidence/outcome |
|---|---|---|
| Ordinary page cannot replace entry with owned HTTP HTML | Literal actual WebKit URI/title, browser-attempt IPC, endpoint ledger | Genuine RED commits external URI/title/browser GET; GREEN/final retain exact entry/local title and no browser GET |
| Ordinary page cannot navigate to packaged root alias | Actual WebKit URI and independent literal expected entry | Baseline commits `dandrum://localhost/`; GREEN/final remain exact index URI |
| Fixed entry still loads normally | Real local page readiness and actual title/URI | Both new cases and existing native cases pass; final navigation callback observed accepting initial entry |
| Document remains usable after denials | Ordinary module delayed local404 fetch and IPC with literal entry URL | Both targets produce `navigation-survived|404|dandrum://localhost/index.html` |
| Existing local content is preserved | `local|404|43px|37px` from actual module/CSS/inline style/fetch | Passes before attempts for both targets and existing CSP control |
| Existing external-fetch denial remains enforced | `external_fetch_is_blocked_by_enforcing_csp`: actual enforcing CSP violation, rejected fetch, independently healthy endpoint with calibration-only ledger | Passes baseline/RED/GREEN/refactor/final |
| Fixture cannot pass merely because endpoint is unavailable | Independent Rust TCP HTTP response/body/MIME calibration; actual unmodified-browser navigation success/title/GET | Successful baseline and precise RED demonstrate viable unwanted route |

## RED, coverage and refactoring assessment

The baseline exits0 and actually commits both unwanted destinations. Its external HTML sentinel is not an attachment/download, has a known title, and receives an additional browser GET beyond independent calibration requests. The RED exits101 at local_policy.rs126 on actual URI equality: observed `http://127.0.0.1:40319/navigation` versus required literal entry. Its actual title is `owned-navigation-sentinel`; its endpoint ledger includes the browser GET. Existing local/CSP assertions pass first. This is missing behavior, not compilation, CORS, setup, timeout or mocked denial.

GREEN exits0 and rejects both destinations while the original page continues. Initial instrumented coverage follows; the production refactor unifies duplicate entry literals as one immutable private constant. Tests retain independent expected literals. The refactor native rerun passes, and the exact final sources receive fresh final native/fast coverage plus the other registered/static checks. Shared page/response helpers are reused and calibrated, without a new extracted production module or public test seam.

Final raw LLVM summaries reproduce native owner 129/134 lines, 12/14 functions, 190/209 regions, 23/34 instantiations; fast assets 52/52 lines, 86/87 regions; fast native owner0/134. Native owner line68 callback count7 and initial load line87 count5 agree with five entry loads and two denied attempts. Real observed outcomes distinguish those decisions; the count alone is not policy proof. Zero branch/MC/DC counters are disclosed, not advertised as full branch coverage. The five inherited uncovered line locations and 19 regions remain visible; IPC-registration refusal, non-X11 backend, connection/transport errors and other inherited directions are not newly closed. The selected X11 environment cannot prove Wayland failure paths. No exclusion or production-module extraction conceals these gaps.

The instrumented binary is independently retained and hash-verified: 81,987,232 bytes; SHA256 `d76f71f9f1986e283f8d41925f5746aad889b2e652560b7d506d05f8e00534d1`, matching final export receipts. Ordinary sandbox `/dev/shm` is a different mount; a narrowly authorized host read resolved that observation. No binary or profiler was executed by this reviewer. Native and fast profiles/profdata/JSON/annotated exports are archived. Toolchain receipt pins rustc1.96.0/LLVM22.1.2, GTK3.24.33/WebKitGTK2.50.4. Mutation is excluded by user direction, with no mutation-strength claim.

Final three registered CTests pass in1.572698s, including native1.42s and fast0.08s; final native coverage command takes2.019964s including build/setup (test body1.28s). Clippy all-target native `-D warnings`, native library build, fmt and default graph pass. Host-side read-only CTest registration confirms the normal fast/package/native selections and actual staged production bundle input. Full production React/font rendering is not inferred from byte-serving calibration. The unaffected published `4d85b25` source-free/default guard is reused through `8d002a1`, expressly not rerun.

## Four test pillars

| Pillar | Assessment |
|---|---|
| Regression protection | Strong for this criterion: real baseline/RED prove the unwanted navigation; actual URI/title and network ledger are separate from JS messages; both external and alternate-local classes are tested |
| Refactoring resistance | Assertions observe final browser/document/network behavior and literal contract values, rather than constant names, callback counts or call order; accessing the real WebKit view is appropriate boundary observation |
| Fast feedback | Exact equality requires no elaborate unit policy owner. Real WebKit behavior cannot be proved by a mocked handler; the existing named native boundary lane is appropriate. GREEN/refactor bodies1.28/1.27s are comparable; routine fast tests remain separate. No evidence warrants moving membership |
| Maintainability | Two targets share a readable table loop and existing owned endpoint/page helpers. Finite service deadlines, socket timeouts and joined worker bound failures; typed actual URI/title diagnostics identify causes. One existing native runner preserves GTK thread constraints |

## Architecture, security and alternatives

The security decision is concentrated at the concrete Wry navigation edge: accept only exact fixed entry, using the same private constant for loading. Inspecting pinned Wry confirms its callback routes NavigationAction decisions to explicit WebKit use/ignore, while unrelated policy types remain outside this callback. No engine/session/audio dependencies, public knobs, parsing capability, token/secret handling or unsafe owned code are added. Existing RAII child and callback destruction order is preserved. Endpoint traffic is owned loopback only, without external network use.

A URL prefix/origin allowlist would permit the demonstrated root-alias replacement. A separate configurable policy module or test seam would add unsupported capability and another owner for a single equality. CSP connect-src already governs fetch but the real baseline shows it does not meet navigation policy. The chosen one-line callback plus shared constant is the smaller in-scope solution. Popup/download, arbitrary subresources, production React/eight fonts, show/hide/size, CLAP lifecycle and actual audio/DAW/platform acceptance remain separate unproved boundaries. No universal WebView security claim is approved.

## Publication and resource envelope

The packet's finite envelope is accepted: copy these exact two review artifacts; change only receipt.review with PASS/initial manifest/report/verdict hashes; replace the one specified progress sentence; regenerate SHA256SUMS deterministically; supply the complete final19path manifest and exact five metadata path changes to this same reviewer for addendum before commit. Source/test/generated/spec drift is outside this envelope. Addendum verification is still required before publication.

Normal active executable hooks are `.githooks/pre-commit` (strict engine coverage driver) and `.githooks/pre-push` (engine tests then changed-engine-source mutation selector). `core.hooksPath=.githooks` is verified. No engine source is changed here; no mutation bypass is needed or approved. Mandatory hook results remain a publication gate, not a result of this read-only review. Require exact final tree verification, ff-only authorized feature integrations, fetch without merge and unchanged intended upstream8d002a1, then normal non-force push. Any failed gate, upstream movement or unexpected content invalidates this PASS.

Author resource receipt records physical free16,431,566,848 bytes, host tmpfs free10,919,817,216 bytes, target1.9G and no owned Cargo/rustc/editor processes. These are recorded samples rather than permanent reservations. I started no workers/native sessions and release review ownership now. Preserve all caches, evidence, warm targets and worktrees under the bounded publication plan; no deletion or main merge is approved.

## Findings and reusable lessons

Actionable findings: none.

Supported lesson candidate: browser refusal tests need a healthy target that the unchanged real browser demonstrably reaches, plus actual consumer/network observations. This avoids mistaking setup/CSP/CORS/network failures for navigation policy. Evidence: baseline/RED above. Proposed owner: project native-boundary testing guidance. Downside: extra real-browser fixture cost. Disposition: the existing TDD calibration rule already covers the principle; retain the concrete example in this evidence only, with no standing-guidance edit.

Supported operational observation: host and sandbox `/dev/shm` differ. A missing sandbox cache path does not prove cache deletion; resolve mount visibility using an authorized bounded read before drawing that conclusion. Owner: this runtime evidence. Disposition: documented here, no general policy change.
