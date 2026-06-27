# Dandrum

Headless-first OSS virtual instrument experiment.

## First Sound

The first milestone is deliberately tiny: prove the JUCE wrapper can open the default audio device while Rust owns the sample generation.

Native Linux dependencies for JUCE:

```bash
sudo apt install -y libasound2-dev libx11-dev libxext-dev libxinerama-dev libxrandr-dev libxcursor-dev libxrender-dev libfreetype6-dev libfontconfig1-dev libgl1-mesa-dev libcurl4-openssl-dev
```

```bash
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build
./build/dandrum-beep_artefacts/dandrum-beep
```

This uses JUCE as the wrapper/host side. The current binary links a Rust static library from `src/rust-engine/` and calls it from the JUCE audio callback.

## MIDI Input

JUCE owns MIDI device IO and forwards note events into the Rust engine.

```bash
./build/dandrum-beep_artefacts/dandrum-beep --list-midi-inputs
./build/dandrum-beep_artefacts/dandrum-beep --midi-input 0
```

The default no-argument command plays a Rust-generated test note and exits. MIDI mode stays open until Ctrl+C.

For test harnesses without a physical MIDI device, inject a synthetic JUCE MIDI note through the same MIDI handler path:

```bash
./build/dandrum-beep_artefacts/dandrum-beep --test-midi-note 60
```
