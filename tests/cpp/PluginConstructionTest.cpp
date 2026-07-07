#include "PluginProcessor.h"
#include "DefaultPatch.h"

#include <cmath>
#include <iostream>
#include <memory>
#include <thread>
#include <vector>

namespace
{
bool bufferIsFinite (const juce::AudioBuffer<float>& buffer)
{
    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
    {
        const auto* samples = buffer.getReadPointer (channel);
        for (int i = 0; i < buffer.getNumSamples(); ++i)
            if (! std::isfinite (samples[i]))
                return false;
    }

    return true;
}

bool bufferHasSignal (const juce::AudioBuffer<float>& buffer)
{
    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
    {
        const auto* samples = buffer.getReadPointer (channel);
        for (int i = 0; i < buffer.getNumSamples(); ++i)
            if (std::abs (samples[i]) > 0.000001f)
                return true;
    }

    return false;
}

float tailRms (const juce::AudioBuffer<float>& buffer, int fromSample)
{
    double sumSquares = 0.0;
    int count = 0;

    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
    {
        const auto* samples = buffer.getReadPointer (channel);
        for (int i = fromSample; i < buffer.getNumSamples(); ++i)
        {
            sumSquares += static_cast<double> (samples[i]) * samples[i];
            ++count;
        }
    }

    return count == 0 ? 0.0f : static_cast<float> (std::sqrt (sumSquares / count));
}

juce::RangedAudioParameter* findParameter (DandrumAudioProcessor& processor, const juce::String& publicParameterId)
{
    return processor.getParameterForPublicId (publicParameterId);
}

float renderTailRms (DandrumAudioProcessor& processor, int blockSize, int numBlocks, int noteNumber)
{
    juce::AudioBuffer<float> full (2, blockSize * numBlocks);
    full.clear();

    for (int block = 0; block < numBlocks; ++block)
    {
        juce::AudioBuffer<float> blockBuffer (2, blockSize);
        blockBuffer.clear();
        juce::MidiBuffer midi;
        if (block == 0)
            midi.addEvent (juce::MidiMessage::noteOn (1, (juce::uint8) noteNumber, (juce::uint8) 100), 0);

        processor.processBlock (blockBuffer, midi);

        for (int channel = 0; channel < 2; ++channel)
            full.copyFrom (channel, block * blockSize, blockBuffer, channel, 0, blockSize);
    }

    return tailRms (full, (numBlocks - 1) * blockSize);
}

float renderKickTailRms (float normalizedDecayValue, int blockSize, int numBlocks)
{
    auto processor = std::make_unique<DandrumAudioProcessor>();
    processor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    processor->prepareToPlay (48000.0, blockSize);

    auto* decayParam = findParameter (*processor, "kick.decay_ms");
    if (decayParam != nullptr)
        decayParam->setValueNotifyingHost (normalizedDecayValue);

    const auto result = renderTailRms (*processor, blockSize, numBlocks, 36);
    processor->releaseResources();
    return result;
}

juce::File defaultPatchFile()
{
    return juce::File (juce::String (dandrum::defaultPatchPath().string()));
}

juce::File examplePresetFile (const juce::String& name)
{
    const auto path = dandrum::defaultPatchPath().parent_path().parent_path() / "presets" / name.toStdString();
    return juce::File (juce::String (path.string()));
}

// Writes a copy of the bundled default instrument with a target line replaced,
// so reload tests can prove a genuinely different instrument was loaded (not
// just the same file re-read). Returns an invalid (default-constructed) File
// if targetLine isn't found in the bundled patch, rather than silently
// writing out an unmodified copy — callers must not mistake "no match" for
// "modified".
juce::File writeModifiedKickPatch (const juce::String& targetLine, const juce::String& newLine)
{
    const auto original = defaultPatchFile();
    auto content = original.loadFileAsString();
    if (! content.contains (targetLine))
        return {};

    content = content.replace (targetLine, newLine);

    auto modified = juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getChildFile ("dandrum_modified_kick_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    modified.replaceWithText (content);
    return modified;
}

juce::File writePatchWithAdditionalPublicParameter()
{
    const auto original = defaultPatchFile();
    auto content = original.loadFileAsString();
    const juce::String oldText = "      maps_to: kick.click\n    - name: kick.sub_decay_ms";
    const juce::String newText =
        "      maps_to: kick.click\n"
        "    - name: kick.extra_click\n"
        "      type: number\n"
        "      default: 0.25\n"
        "      min: 0\n"
        "      max: 1\n"
        "      maps_to: kick.click\n"
        "    - name: kick.sub_decay_ms";

    if (! content.contains (oldText))
        return {};

    content = content.replace (oldText, newText);
    auto modified = juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getChildFile ("dandrum_added_public_parameter_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    modified.replaceWithText (content);
    return modified;
}

// A minimal but valid instrument with no preset_surface parameters at all, so
// reloading to it drops every public parameter the default instrument exposed
// from the dynamic editor/public-id surface.
juce::File writePatchWithNoPublicParameters()
{
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile ("dandrum_no_public_parameters_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    file.replaceWithText (
        "metadata:\n"
        "  name: No Public Parameters\n"
        "instrument:\n"
        "  id: dandrum.no-public-parameters\n"
        "  preset_schema_version: 1\n"
        "render:\n"
        "  sample_rate_hz: 48000\n"
        "  block_size_frames: 64\n"
        "  duration_frames: 128\n"
        "modules:\n"
        "  - id: osc\n"
        "    type: oscillator\n"
        "  - id: mixer\n"
        "    type: audio_mixer\n"
        "  - id: out\n"
        "    type: audio_output\n"
        "    inputs:\n"
        "      - name: left\n"
        "        signal_type: audio\n"
        "      - name: right\n"
        "        signal_type: audio\n"
        "connections:\n"
        "  - from: osc.audio\n"
        "    to: mixer.inputs\n"
        "  - from: mixer.mix\n"
        "    to: out.left\n"
        "  - from: mixer.mix\n"
        "    to: out.right\n");
    return file;
}

juce::File writePresetFile (const juce::String& filePrefix, const juce::String& yaml)
{
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile (filePrefix + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    file.replaceWithText (yaml);
    return file;
}

bool nearlyEqual (float actual, float expected, float tolerance)
{
    return std::abs (actual - expected) <= tolerance;
}
} // namespace

int main()
{
    constexpr int blockSize = 64;

    auto processor = std::make_unique<DandrumAudioProcessor>();
    if (! processor->isInstrumentLoaded())
    {
        std::cerr << processor->getLastLoadError() << '\n';
        return 1;
    }

    if (! processor->hasPublicParameter ("kick.tune_hz"))
    {
        std::cerr << "default plugin instrument did not expose kick.tune_hz\n";
        return 1;
    }

    juce::MemoryBlock state;
    processor->getStateInformation (state);
    if (state.getSize() == 0)
    {
        std::cerr << "plugin state serialization produced an empty state block\n";
        return 1;
    }

    auto restored = std::make_unique<DandrumAudioProcessor>();
    restored->setStateInformation (state.getData(), static_cast<int> (state.getSize()));
    if (! restored->hasPublicParameter ("kick.tune_hz"))
    {
        std::cerr << "restored plugin did not keep the public parameter mapping\n";
        return 1;
    }

    processor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    processor->prepareToPlay (48000.0, blockSize);

    juce::AudioBuffer<float> buffer (2, blockSize);
    buffer.clear();

    juce::MidiBuffer midi;
    midi.addEvent (juce::MidiMessage::noteOn (1, 60, (juce::uint8) 100), 10);

    processor->processBlock (buffer, midi);

    if (! bufferIsFinite (buffer))
    {
        std::cerr << "plugin processBlock produced non-finite samples\n";
        return 1;
    }

    if (! bufferHasSignal (buffer))
    {
        std::cerr << "plugin processBlock rendered silence after default instrument note-on\n";
        return 1;
    }

    processor->releaseResources();

    // Prove that changing a public parameter (via the same APVTS path the
    // generic JUCE knobs use) actually reaches the running instrument, rather
    // than only updating the host-facing parameter value cosmetically.
    constexpr int decayBlockSize = 128;
    constexpr int decayNumBlocks = 40; // ~106ms at 48kHz
    const auto shortDecayTailRms = renderKickTailRms (0.0f, decayBlockSize, decayNumBlocks);
    const auto longDecayTailRms = renderKickTailRms (1.0f, decayBlockSize, decayNumBlocks);

    if (! (longDecayTailRms > shortDecayTailRms * 2.0f))
    {
        std::cerr << "changing kick.decay_ms did not audibly change the render: short="
                   << shortDecayTailRms << " long=" << longDecayTailRms << '\n';
        return 1;
    }

    // Public parameter changes must never mutate or reload the loaded YAML.
    {
        auto yamlCheckProcessor = std::make_unique<DandrumAudioProcessor>();
        yamlCheckProcessor->setPlayConfigDetails (0, 2, 48000.0, decayBlockSize);
        yamlCheckProcessor->prepareToPlay (48000.0, decayBlockSize);
        const auto yamlBefore = yamlCheckProcessor->currentInstrumentYaml();

        auto* tuneParam = findParameter (*yamlCheckProcessor, "kick.tune_hz");
        if (tuneParam == nullptr)
        {
            std::cerr << "kick.tune_hz parameter missing\n";
            return 1;
        }
        tuneParam->setValueNotifyingHost (0.75f);

        juce::AudioBuffer<float> yamlCheckBuffer (2, decayBlockSize);
        yamlCheckBuffer.clear();
        juce::MidiBuffer yamlCheckMidi;
        yamlCheckMidi.addEvent (juce::MidiMessage::noteOn (1, 36, (juce::uint8) 100), 0);
        yamlCheckProcessor->processBlock (yamlCheckBuffer, yamlCheckMidi);

        if (yamlCheckProcessor->currentInstrumentYaml() != yamlBefore)
        {
            std::cerr << "changing a public parameter mutated the loaded instrument YAML\n";
            return 1;
        }
        yamlCheckProcessor->releaseResources();
    }

    // Muting should silence the processor even with an active note.
    auto mutedProcessor = std::make_unique<DandrumAudioProcessor>();
    mutedProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    mutedProcessor->prepareToPlay (48000.0, blockSize);
    mutedProcessor->setMuted (true);
    if (! mutedProcessor->isMuted())
    {
        std::cerr << "setMuted(true) did not update isMuted()\n";
        return 1;
    }

    juce::AudioBuffer<float> mutedBuffer (2, blockSize);
    mutedBuffer.clear();
    juce::MidiBuffer mutedMidi;
    mutedMidi.addEvent (juce::MidiMessage::noteOn (1, 60, (juce::uint8) 100), 0);
    mutedProcessor->processBlock (mutedBuffer, mutedMidi);

    if (bufferHasSignal (mutedBuffer))
    {
        std::cerr << "processBlock produced signal while the processor was muted\n";
        return 1;
    }

    mutedProcessor->setMuted (false);
    mutedProcessor->releaseResources();

    // Reload success + parameter carry-over: an explicit value set before
    // reload should still apply after the swap, overriding whatever default
    // the replacement YAML declares (design.md: "parameters that still exist
    // keep their current value"). A live parameter change after the reload
    // should also still reach the (new) running engine.
    constexpr int reloadBlockSize = 128;
    constexpr int reloadNumBlocks = 40; // ~106ms at 48kHz

    auto reloadProcessor = std::make_unique<DandrumAudioProcessor>();
    reloadProcessor->setPlayConfigDetails (0, 2, 48000.0, reloadBlockSize);
    reloadProcessor->prepareToPlay (48000.0, reloadBlockSize);

    auto* decayParam = findParameter (*reloadProcessor, "kick.decay_ms");
    if (decayParam == nullptr)
    {
        std::cerr << "kick.decay_ms parameter missing on the default instrument\n";
        return 1;
    }
    decayParam->setValueNotifyingHost (0.0f); // normalized minimum: shortest decay

    const auto shortBeforeReloadTailRms = renderTailRms (*reloadProcessor, reloadBlockSize, reloadNumBlocks, 36);

    // The replacement file declares a much longer default (1900ms), so if
    // carry-over were broken and the new file's own default won instead, this
    // render would come out clearly longer/louder than shortBeforeReloadTailRms.
    const auto longDefaultFile = writeModifiedKickPatch ("decay_ms: 650", "decay_ms: 1900");
    if (! reloadProcessor->reloadInstrumentFromFile (longDefaultFile))
    {
        std::cerr << "reloadInstrumentFromFile failed unexpectedly: "
                   << reloadProcessor->getLastLoadError() << '\n';
        return 1;
    }

    if (! reloadProcessor->isInstrumentLoaded()
        || reloadProcessor->currentInstrumentFile() != longDefaultFile
        || ! reloadProcessor->currentInstrumentYaml().contains ("decay_ms: 1900"))
    {
        std::cerr << "reloadInstrumentFromFile did not update loaded-instrument bookkeeping\n";
        return 1;
    }

    if (! reloadProcessor->getLastReloadWarning().isEmpty())
    {
        std::cerr << "reloadInstrumentFromFile warned about dropped parameters when none were dropped: "
                   << reloadProcessor->getLastReloadWarning() << '\n';
        return 1;
    }

    const auto carriedOverTailRms = renderTailRms (*reloadProcessor, reloadBlockSize, reloadNumBlocks, 36);
    // The carried-over 50ms decay is fully silent this far into the tail, so its
    // tail RMS is ~0 and can't support a multiplicative bound. A broken carry-over
    // would instead apply the replacement file's 1900ms default and leave a
    // clearly audible tail here, comparable to the long-decay render measured
    // above. Bound against a small fraction of that known-audible level.
    const auto carriedOverTailCeiling = juce::jmax (shortBeforeReloadTailRms * 2.0f, longDecayTailRms * 0.1f);
    if (! (carriedOverTailRms < carriedOverTailCeiling))
    {
        std::cerr << "explicit kick.decay_ms value was not carried over across reload: before="
                   << shortBeforeReloadTailRms << " afterReload=" << carriedOverTailRms
                   << " ceiling=" << carriedOverTailCeiling << '\n';
        return 1;
    }

    // The parameter bridge should still be live on the freshly-reloaded
    // engine: a further explicit change should audibly take effect.
    decayParam = findParameter (*reloadProcessor, "kick.decay_ms");
    if (decayParam == nullptr)
    {
        std::cerr << "kick.decay_ms parameter disappeared after reload\n";
        return 1;
    }
    decayParam->setValueNotifyingHost (1.0f); // normalized maximum: longest decay
    const auto longAfterReloadTailRms = renderTailRms (*reloadProcessor, reloadBlockSize, reloadNumBlocks, 36);
    if (! (longAfterReloadTailRms > carriedOverTailRms * 1.3f))
    {
        std::cerr << "kick.decay_ms changes stopped taking effect after reload: short="
                   << carriedOverTailRms << " long=" << longAfterReloadTailRms << '\n';
        return 1;
    }

    reloadProcessor->releaseResources();

    // A replacement instrument that introduces a new public parameter should
    // expose it through an unused fixed host slot and initialise it from the
    // YAML-declared default value.
    {
        auto addProcessor = std::make_unique<DandrumAudioProcessor>();
        addProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        addProcessor->prepareToPlay (48000.0, blockSize);

        const auto addedParameterFile = writePatchWithAdditionalPublicParameter();
        if (addedParameterFile == juce::File())
        {
            std::cerr << "writePatchWithAdditionalPublicParameter could not patch the default instrument\n";
            return 1;
        }
        if (! addProcessor->reloadInstrumentFromFile (addedParameterFile))
        {
            std::cerr << "reloadInstrumentFromFile failed for added-parameter patch: "
                      << addProcessor->getLastLoadError() << '\n';
            return 1;
        }

        auto* extraClick = findParameter (*addProcessor, "kick.extra_click");
        if (extraClick == nullptr)
        {
            std::cerr << "replacement instrument did not expose newly-added public parameter kick.extra_click\n";
            return 1;
        }
        if (! nearlyEqual (extraClick->getValue(), 0.25f, 0.002f))
        {
            std::cerr << "kick.extra_click was not initialised from YAML default: " << extraClick->getValue() << '\n';
            return 1;
        }

        addProcessor->releaseResources();
        addedParameterFile.deleteFile();
    }

    // Reload failure: the previous instrument keeps running unchanged.
    auto failedReloadProcessor = std::make_unique<DandrumAudioProcessor>();
    failedReloadProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    failedReloadProcessor->prepareToPlay (48000.0, blockSize);

    const juce::File missingFile ("/nonexistent/path/to/dandrum_missing_instrument.yaml");
    if (failedReloadProcessor->reloadInstrumentFromFile (missingFile))
    {
        std::cerr << "reloadInstrumentFromFile unexpectedly succeeded for a missing file\n";
        return 1;
    }
    if (failedReloadProcessor->getLastLoadError().isEmpty())
    {
        std::cerr << "reloadInstrumentFromFile failure did not set an error message\n";
        return 1;
    }

    juce::AudioBuffer<float> stillWorkingBuffer (2, blockSize);
    stillWorkingBuffer.clear();
    juce::MidiBuffer stillWorkingMidi;
    stillWorkingMidi.addEvent (juce::MidiMessage::noteOn (1, 36, (juce::uint8) 100), 0);
    failedReloadProcessor->processBlock (stillWorkingBuffer, stillWorkingMidi);

    if (! bufferIsFinite (stillWorkingBuffer) || ! bufferHasSignal (stillWorkingBuffer))
    {
        std::cerr << "processor stopped rendering correctly after a failed reload attempt\n";
        return 1;
    }

    failedReloadProcessor->releaseResources();

    // writeModifiedKickPatch must fail loudly (an invalid File) rather than
    // silently writing an unmodified copy when the target line isn't found,
    // so a future drift in the bundled patch can't silently defang the
    // reload/carry-over test above.
    const auto noSuchTargetFile = writeModifiedKickPatch ("decay_ms: this-value-does-not-exist", "decay_ms: 1900");
    if (noSuchTargetFile != juce::File())
    {
        std::cerr << "writeModifiedKickPatch did not report failure for an unmatched target line\n";
        return 1;
    }

    // reloadInstrumentFromFile must not silently prepare a candidate engine
    // with an invalid (zero) sample rate when called before the host has ever
    // called prepareToPlay.
    {
        auto unpreparedProcessor = std::make_unique<DandrumAudioProcessor>();
        if (unpreparedProcessor->reloadInstrumentFromFile (defaultPatchFile()))
        {
            std::cerr << "reloadInstrumentFromFile succeeded before prepareToPlay was ever called\n";
            return 1;
        }
        if (unpreparedProcessor->getLastLoadError().isEmpty())
        {
            std::cerr << "reloadInstrumentFromFile rejected a pre-prepareToPlay call without setting an error\n";
            return 1;
        }
    }

    // Concurrent reloadInstrumentFromFile calls must never destroy an engine
    // another in-flight reload is still using: two threads hammering reload
    // on the same processor must leave it in a valid, still-rendering state.
    {
        auto racingProcessor = std::make_unique<DandrumAudioProcessor>();
        racingProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        racingProcessor->prepareToPlay (48000.0, blockSize);

        const auto defaultFile = defaultPatchFile();
        const auto alternateFile = writeModifiedKickPatch ("decay_ms: 650", "decay_ms: 700");

        constexpr int threadCount = 4;
        constexpr int iterationsPerThread = 20;
        std::vector<std::thread> threads;
        for (int t = 0; t < threadCount; ++t)
        {
            threads.emplace_back ([&, t]
            {
                for (int i = 0; i < iterationsPerThread; ++i)
                {
                    const auto& file = ((t + i) % 2 == 0) ? defaultFile : alternateFile;
                    racingProcessor->reloadInstrumentFromFile (file);
                }
            });
        }
        for (auto& thread : threads)
            thread.join();

        if (! racingProcessor->isInstrumentLoaded())
        {
            std::cerr << "processor was left without a loaded instrument after concurrent reloads\n";
            return 1;
        }

        juce::AudioBuffer<float> racingBuffer (2, blockSize);
        racingBuffer.clear();
        juce::MidiBuffer racingMidi;
        racingMidi.addEvent (juce::MidiMessage::noteOn (1, 36, (juce::uint8) 100), 0);
        racingProcessor->processBlock (racingBuffer, racingMidi);

        if (! bufferIsFinite (racingBuffer))
        {
            std::cerr << "processor produced non-finite samples after concurrent reloadInstrumentFromFile calls\n";
            return 1;
        }

        racingProcessor->releaseResources();
        alternateFile.deleteFile();
    }

    // Reloading to an instrument that drops previously-live public parameters
    // must be visibly reconciled (design.md), not silently absorbed.
    {
        auto dropProcessor = std::make_unique<DandrumAudioProcessor>();
        dropProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        dropProcessor->prepareToPlay (48000.0, blockSize);

        const auto noPublicParametersFile = writePatchWithNoPublicParameters();
        if (! dropProcessor->reloadInstrumentFromFile (noPublicParametersFile))
        {
            std::cerr << "reloadInstrumentFromFile failed unexpectedly for a patch with no public parameters: "
                       << dropProcessor->getLastLoadError() << '\n';
            return 1;
        }

        if (dropProcessor->hasPublicParameter ("kick.tune_hz"))
        {
            std::cerr << "reload kept kick.tune_hz visible after the replacement instrument removed it\n";
            return 1;
        }

        if (dropProcessor->getLastReloadWarning().isEmpty())
        {
            std::cerr << "reloadInstrumentFromFile did not warn about parameters dropped by the new instrument\n";
            return 1;
        }

        dropProcessor->releaseResources();
        noPublicParametersFile.deleteFile();
    }

    // Compatible presets should apply as public value changes for the loaded
    // instrument. They must not replace or mutate the immutable instrument YAML.
    {
        auto presetProcessor = std::make_unique<DandrumAudioProcessor>();
        presetProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        presetProcessor->prepareToPlay (48000.0, blockSize);
        const auto yamlBefore = presetProcessor->currentInstrumentYaml();
        const auto presetFile = examplePresetFile ("tight-808-kick.yaml");

        if (! presetProcessor->loadPresetFromFile (presetFile))
        {
            std::cerr << "compatible preset failed to load: " << presetProcessor->getLastPresetError() << '\n';
            return 1;
        }

        if (presetProcessor->currentPresetName() != "Tight 808 Kick" || presetProcessor->currentPresetYaml().isEmpty())
        {
            std::cerr << "loaded preset identity/content was not retained\n";
            return 1;
        }

        auto* decay = findParameter (*presetProcessor, "kick.decay_ms");
        if (decay == nullptr || ! nearlyEqual (decay->getValue(), (420.0f - 50.0f) / (2000.0f - 50.0f), 0.01f))
        {
            std::cerr << "preset value for kick.decay_ms was not applied to mutable parameter state\n";
            return 1;
        }

        if (presetProcessor->currentInstrumentYaml() != yamlBefore)
        {
            std::cerr << "loading a preset mutated the loaded instrument YAML\n";
            return 1;
        }

        presetProcessor->releaseResources();
    }

    // Incompatible or structural presets should be reported, not applied.
    {
        auto rejectProcessor = std::make_unique<DandrumAudioProcessor>();
        rejectProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        rejectProcessor->prepareToPlay (48000.0, blockSize);

        const auto wrongInstrumentPreset = writePresetFile (
            "dandrum_wrong_instrument_preset_",
            "name: Wrong\n"
            "instrument:\n"
            "  id: dandrum.other\n"
            "  preset_schema_version: 1\n"
            "values:\n"
            "  kick.decay_ms: 420\n");
        if (rejectProcessor->loadPresetFromFile (wrongInstrumentPreset))
        {
            std::cerr << "preset targeting another instrument was applied\n";
            return 1;
        }
        if (rejectProcessor->getLastPresetError().isEmpty())
        {
            std::cerr << "wrong-instrument preset did not report an error\n";
            return 1;
        }

        const auto structuralPreset = writePresetFile (
            "dandrum_structural_preset_",
            "name: Structural\n"
            "instrument:\n"
            "  id: dandrum.synthetic-808-kick\n"
            "  preset_schema_version: 1\n"
            "modules: []\n");
        if (rejectProcessor->loadPresetFromFile (structuralPreset))
        {
            std::cerr << "structural preset was applied\n";
            return 1;
        }
        if (! rejectProcessor->getLastPresetError().contains ("structural"))
        {
            std::cerr << "structural preset rejection did not explain the structural field: "
                      << rejectProcessor->getLastPresetError() << '\n';
            return 1;
        }

        rejectProcessor->releaseResources();
        wrongInstrumentPreset.deleteFile();
        structuralPreset.deleteFile();
    }

    // State persistence should embed enough instrument and preset information
    // to restore without depending only on the original absolute file paths.
    {
        auto stateProcessor = std::make_unique<DandrumAudioProcessor>();
        stateProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        stateProcessor->prepareToPlay (48000.0, blockSize);

        const auto restoredLongDefaultFile = writeModifiedKickPatch ("decay_ms: 650", "decay_ms: 1750");
        if (! stateProcessor->reloadInstrumentFromFile (restoredLongDefaultFile))
        {
            std::cerr << "state restore setup failed to reload modified instrument: "
                      << stateProcessor->getLastLoadError() << '\n';
            return 1;
        }
        if (! stateProcessor->loadPresetFromFile (examplePresetFile ("tight-808-kick.yaml")))
        {
            std::cerr << "state restore setup failed to load preset: "
                      << stateProcessor->getLastPresetError() << '\n';
            return 1;
        }

        juce::MemoryBlock savedState;
        stateProcessor->getStateInformation (savedState);

        auto restoredStateProcessor = std::make_unique<DandrumAudioProcessor>();
        restoredStateProcessor->setStateInformation (savedState.getData(), static_cast<int> (savedState.getSize()));

        if (! restoredStateProcessor->currentInstrumentYaml().contains ("decay_ms: 1750"))
        {
            std::cerr << "state restore did not restore embedded instrument YAML\n";
            return 1;
        }
        if (restoredStateProcessor->currentPresetName() != "Tight 808 Kick"
            || restoredStateProcessor->currentPresetYaml().isEmpty())
        {
            std::cerr << "state restore did not restore preset identity/content\n";
            return 1;
        }
        if (! restoredStateProcessor->hasPublicParameter ("kick.decay_ms"))
        {
            std::cerr << "state restore did not rebuild the public parameter surface\n";
            return 1;
        }

        stateProcessor->releaseResources();
        restoredStateProcessor->releaseResources();
        restoredLongDefaultFile.deleteFile();
    }

    longDefaultFile.deleteFile();
    return 0;
}
