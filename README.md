# Dandrum

Headless-first OSS virtual instrument experiment.

## First Sound

The first milestone is deliberately tiny: prove the JUCE audio wrapper can open the default audio device and emit a short beep.

Native Linux dependencies for JUCE:

```bash
sudo apt install -y libasound2-dev libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxcursor-dev libxrender-dev libfreetype6-dev libfontconfig1-dev libgl1-mesa-dev libcurl4-openssl-dev
```

```bash
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build
./build/dandrum-beep_artefacts/dandrum-beep
```

This uses JUCE as the wrapper/host side. The Rust audio engine is intentionally deferred until the audio pipeline is proven.
