# Drum sampler VST3 example

Build the sampler instrument with:

```bash
$HOME/.local/bin/cmake -S . -B build
$HOME/.local/bin/cmake --build build --target dandrum-sampler-plugin_VST3
```

The VST3 bundle is `build/dandrum-sampler-plugin_artefacts/VST3/Dandrum Sampler.vst3`. Add that bundle to a folder scanned by your host, or point the host's plug-in scanner at the build folder. This is a separate instrument from the original Dandrum TB-303 demo, with its own VST3 identity.

The VST embeds the synthetic, redistributable `examples/patches/advanced-drum-kit.yaml` patch and `examples/patches/assets/advanced-drums.wav`. Before audio processing, it stages them in the user's application data directory under `Dandrum/Sampler Example/`; the WAV is under its `assets/` subdirectory. This keeps the default kit available when the VST is copied away from the source checkout. Patch loading and WAV decoding happen during instrument preparation, not in the audio callback. The source WAV is 48 kHz; playback keeps the same pitch and musical duration at 44.1, 48, and 96 kHz hosts.

## Play the kit

| MIDI note | Hit | Behavior |
| --- | --- | --- |
| 36 | Kick | One-shot kick |
| 38 | Snare | Velocity 1–63 selects the soft snare; 64–127 selects alternating hard snares |
| 42 | Closed hat | Cuts a sounding open hat at the note's frame |
| 46 | Open hat | Alternating open-hat hits; shares the hat choke group |

The plug-in editor has pads for these four notes. You can also sequence them from a MIDI clip or controller. Other notes have no mapped hit in this example.

## Modulate the sound from a host

The kit has five shared controls plus separate controls for the kick, snare, closed hat, and open hat. The shared IDs are `drums.pitch_ratio`, `drums.start_offset`, `drums.level`, `drums.pan`, and `drums.variation`. Each pad has `pitch_ratio`, `start_offset`, `level`, and `pan` under `drums.<pad>.`; the snare and open hat also have `variation` because they have alternate hits. For example, `drums.kick.level` changes the kick without changing the snare. Hosts display these public IDs as parameter names; their underlying VST3 slots remain stable across instrument reloads.

| Host parameter | Range | Effect |
| --- | --- | --- |
| `drums.pitch_ratio` | 0.125–8, default 1 | Playback speed and pitch, including an active hit |
| `drums.start_offset` | 0–1, default 0 | Start position within the chosen region on the next hit |
| `drums.level` | 0–4, default 1 | Output amplitude, including an active hit |
| `drums.pan` | −1–1, default 0 | Stereo pan, including an active hit |
| `drums.variation` | 0–1, default 0 | Rotate a newly triggered hit to a compatible alternate; zero preserves the authored round-robin choice |

The same ranges apply to pad controls. Shared and pad pitch and level multiply; shared and pad start offset, pan, and variation add within their supported ranges. A pad's `pitch_ratio`, `level`, and `pan` affect its active hits on the next audio block. Its `start_offset` and `variation` take effect on the next hit. The four pads use control groups 1–4 in the patch's sample map, so velocity layers and alternates for one pad inherit the same controls. A custom map can define up to eight control groups.

In Bitwig Studio, open the plug-in's parameter list and search for a pad control such as `drums.snare.pan`. Add a modulator to the plug-in, such as Steps for rhythmic variation or an LFO for pitch or pan, then map its source to that parameter and set the modulation amount. Bitwig describes its [plug-in parameter list](https://www.bitwig.com/userguide/latest/vst_plug-ins/) and [modulator mapping](https://www.bitwig.com/userguide/latest/the_unified_modulation_system/) in its user guide. The same controls accept ordinary host automation in other VST3 hosts.

The JUCE processor reads host parameter values at the start of each audio block and passes changes to the running Rust engine. The engine keeps pad control values separate even when notes overlap, while the kit still manages voices and hi-hat choking together.

The patch's source files, regions, sample map, velocity layers, round-robin groups, voice limit, and choke mode are preparation-time structure. Edit a copy of the patch and explicitly reload that file to change them; host modulation changes only the public playback controls. The source patch in `examples/patches/` remains the maintained example for development. Sample assets referenced by a custom patch must be placed relative to that patch's location so preparation can resolve them. The staged files are the bundled default and may be replaced by a later plug-in build.
