## Why

Starting a Dandrum demo currently requires remembering checkout paths, build targets and commands. A maintained repository entrypoint should make the available demos discoverable and launch the requested one consistently.

## What Changes

- Add executable `./demo`: list demos without arguments and launch a named demo with forwarded arguments.
- Find demo sources in this checkout and registered Dandrum Git worktrees, preferring this checkout.
- Build native standalone demos and prepare browser-preview dependencies before launch; identify real-engine apps and mock previews in the list.
- Keep the catalog, documentation and regression checks updated when demos change.

## Capabilities

### New Capabilities

- `demo-launcher`: Discover, prepare and launch maintained developer demos.

### Modified Capabilities

None.

## Impact

Repository launcher, demo catalog, Python tests, CTest registration, README and agent maintenance guidance. Uses existing Python, Git, CMake, Node/npm and native build prerequisites; no DSP or plugin behavior changes.
