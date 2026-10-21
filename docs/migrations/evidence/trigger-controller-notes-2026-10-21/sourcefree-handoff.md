# Controller-notes source-free verification handoff

**Result: PASS.** The actual default-release source-free Cargo gate exited 0 with 121 ordinary tests and 2 compile-fail doctests. Every summary reported zero failed, ignored, measured, or filtered tests. The independent locked normal-dependency tree audit exited 0 and contained no Clack, Wry, GTK, WebKit, or JUCE packages.

The root worktree remained at `28caf6ca02861bebdf1a2e09e49c762682ed1cce`; its frozen source inputs and Git/index state were not changed by this verifier. Cargo executed against the retained RAM fixture, after a bounded update of only `src/trigger-host/src/parameters.rs` (SHA-256 `37b8e92e6507036084fcb99b9fabe717550d53fe2f36a2f046312e630cc3efea`) and `src/trigger-host/tests/editor_note_controller.rs` (SHA-256 `99b99000f9e35406e87d29325d4f6ae07a6354097a40a94544c7cfff68fbb08e`). The current root and fixture maps each contain 591 inputs. All 589 other fixture inputs preserve their before-run bytes, modes, and mtimes. The full root map is byte-for-byte equal before and after the run; fixture input bytes match all 591 current root inputs. The fixture has no `web`, `ui`, or `third_party` trees.

The exact complete test command was:

```text
/home/dan/.cargo/bin/cargo test --release --offline --locked --jobs 1 --manifest-path /dev/shm/dandrum-beta-20261020/editor-guard-source/build/engine-only-regression/fixture-kwjt48yz/source/src/trigger-host/Cargo.toml -vv
```

It ran from the fixture source root with `CARGO_TARGET_DIR=/tmp/dandrum-lanes/cache-editor-guard`, `CARGO_INCREMENTAL=0`, one job, offline/locked resolution, and a private `TMPDIR` under this disk evidence directory. No debug, coverage, wrapper, instrumentation, or profile override was active. Actual compiler invocations showed release `opt-level=3`, linker-plugin LTO, and one codegen unit. The command took 552.922 seconds. See `release-tests.log` and `release-tests-run.json` for the full verbose output, exact environment, exit status, elapsed time, and 0.5-second actual-host resource samples.

The actual ten direct test-binary launches are listed in `executed-artifact-bindings.json`, with each post-audit SHA-256, `rustc --test` invocation, Cargo fingerprint files, dep-info, and direct Cargo `Running` line. This binding is current to the 591-input source map. `prior-cache-binding-reference.json` points to the earlier retained 590-input proof and explicitly keeps its nine old binary names and hashes separate from the current result.

The dependency command was `cargo tree --offline --locked --manifest-path <fixture>/src/trigger-host/Cargo.toml --edges normal --prefix none`; full output is in `dependencies.log`, with the exact invocation and resource sample in `dependencies-run.json`. Both owned process groups exited. The existing release cache and refreshed fixture remain intact. The minimum sampled `/tmp` free space was 21,097,013,248 bytes and minimum `MemAvailable` was 10,435,014,656 bytes. `/dev/shm` was sampled as an unused build-output volume; its minimum free space was 859,901,952 bytes and it was not used for build output.

This proves the source-free Rust host tests and Rust normal-dependency graph for these exact source inputs. It does not prove full CMake, native, CLAP, browser, beta, task completion, or publication behavior. Root retains ownership of integration and archival.
