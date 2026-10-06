## Context

Native JUCE apps and React packages are spread across registered Git worktrees. The root checkout has reusable native build artifacts. A browser preview can be silent even when its corresponding embedded editor drives the Rust engine.

## Goals / Non-Goals

**Goals:** One maintained command, discoverable names, explicit preview labels, checkout-independent execution, dependency preparation, argument forwarding and useful failure exits.

**Non-Goals:** Merge other feature branches, install system packages, alter audio behavior, open browser windows automatically, or manage background servers.

## Decisions

- Use an executable Python standard-library entrypoint and a small JSON catalog. Python is already used in repository checks; shell parsing and external launcher packages add unnecessary dependencies.
- Resolve sources from the script's checkout first, then Git's registered worktrees in pathname order. Read the catalog from the launcher checkout. Never fetch, switch branches or change another worktree's source files. Print the selected checkout before preparing its disposable build/dependency outputs.
- Native entries build only their CMake target in that checkout's `build/`, then run the standalone artifact. React embedded entries explicitly enable the WebView backend and bootstrap the sampler package dependency required by that checkout's CMake configuration. On Linux prefer distro build tools in child PATH to avoid mixing Homebrew and system GUI libraries; explicit CC/CXX still apply.
- Read the configured CMake cache after configuration: preserve a single-config build type, prefer Release among available multi-config types, or use the first available type. JUCE puts that configuration immediately after `<target>_artefacts`, before the plugin's `Standalone` directory. Resolve only the configuration just built so a stale default artifact cannot win. Calibrate this layout against vendored JUCE's actual target-file generation.
- Browser entries install locked dependencies if absent, then run the package's dev command in the foreground. Ctrl+C stops the foreground process; Vite prints the URL. Forward all arguments after the name, permitting one optional `--` separator.
- A CTest maintenance guard compares shipped JUCE app/plugin targets and local React dev packages with catalog coverage. Adding a demo must update the catalog and its behavior evidence, README where appropriate, and the owning specification.
- Use fast command-planning tests plus separate real CMake/npm boundary tests. They validate success, preparation failure and argument preservation without building the audio engine just to test subprocess orchestration.

## Risks / Trade-offs

- [Several compatible worktrees] → Prefer the launcher checkout, then sort by path; display the chosen source. A selected worktree's edits are used as they stand.
- [Cold native build or missing system libraries] → Display commands and propagate errors; link documented prerequisites. Preserve reusable build output.
- [No compatible checkout] → List the demo as unavailable and fail launch with its required source identity.
- [Catalog drift] → Enforce inventory checks in CTest and retain explicit agent maintenance guidance.

## Migration Plan

Add the command without changing existing launch commands. Removal of the entrypoint and catalog reverses the feature; existing build/dependency caches remain useful.
