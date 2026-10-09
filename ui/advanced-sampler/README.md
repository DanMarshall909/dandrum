# Advanced sampler Slint editor

The native editor follows the imported [v3 design guide](../../docs/design-system/advanced-sampler/README.md).
Its current adapter is silent: note events, loading, analysis and telemetry are observable editor workflows, with no audio engine connected.

Use Slint C++ SDK **1.18.1** and a C++20 compiler. Configure the SDK through `CMAKE_PREFIX_PATH` or `Slint_DIR`:

```sh
cmake -S . -B build/advanced-sampler \
  -DDANDRUM_ADVANCED_SAMPLER_ONLY=ON \
  -DCMAKE_PREFIX_PATH=/path/to/Slint-cpp-1.18.1-Linux-x86_64 \
  -DCMAKE_BUILD_TYPE=Release
cmake --build build/advanced-sampler
./demo sampler-slint
```

Native model, theme and generated binding tests use a display-free testing backend:

```sh
ctest --test-dir build/advanced-sampler --output-on-failure
```

The pointer, keyboard and screenshot checks run on a private authenticated Xvfb display with `--headless`, clearing inherited desktop variables:

```sh
python3 scripts/check-advanced-sampler.py --headless --viewer /path/to/slint-viewer
```

Set `DANDRUM_XVFB` to a locally staged Xvfb executable when needed. CMake registers this suite when `DANDRUM_SAMPLER_TEST_RUNTIME=ON`; `DANDRUM_SAMPLER_TEST_HEADLESS=ON` is its default. Set that option to `OFF` for an explicitly requested desktop run. `--check-only` performs compilation without needing a display.

Enable both native interaction contracts and all application screenshot cases in CTest:

```sh
cmake -S . -B build/advanced-sampler \
  -DDANDRUM_ADVANCED_SAMPLER_ONLY=ON \
  -DDANDRUM_SAMPLER_TEST_RUNTIME=ON \
  -DDANDRUM_SAMPLER_TEST_HEADLESS=ON \
  -DDANDRUM_SAMPLER_VIEWER=/path/to/slint-viewer
cmake --build build/advanced-sampler
ctest --test-dir build/advanced-sampler --output-on-failure
```

The direct executable accepts `--size min|default|expanded` and `--state STATE` for operational design fixtures. These fixtures use the same native model and controls. Reference comparisons and the remaining acceptance audit are tracked in [OpenSpec](../../openspec/changes/add-advanced-sampler-slint/tasks.md); compilation or component screenshots alone do not complete that audit.

Capture all 30 states, both finishes for all eight surfaces, and all 13 pages at each of the three editor sizes from the actual compiled application:

```sh
python3 scripts/capture-advanced-sampler.py --headless --all-sizes \
  --app build/advanced-sampler/advanced-sampler/Release/dandrum-advanced-sampler
```

This produces 85 captures. Omit `--all-sizes` for the 46 state and theme cases. CTest includes the full matrix. The capture runner uses an authenticated private display and records native snapshot dimensions and hashes. `--state sample` restricts a focused capture. Add `--scale 1.25` for fractional DPI or `--scale 2` for a high-density comparison: the native renderer scales the same logical layout, and the manifest records the scale and original pixel dimensions. For example, default 1200×800 at scale 1.25 produces 1500×1000 pixels; match React devicePixelRatio to 1.25. Gallery zoom only changes viewing size. Font licenses ship beside the executable, and its footer opens the standard About Slint widget.

The runtime suite also checks pixels from the actual LP, HP, BP and Notch filter plots and the envelope release segment. Run that renderer test alone with:

```sh
ctest --test-dir build/advanced-sampler -R advanced-sampler-voice-render --output-on-failure
```

Its private-display runner retains the native test log and four snapshots in the build's `voice-render` directory. A failing native assertion or display startup fails the check; it never substitutes a compile-only result.
