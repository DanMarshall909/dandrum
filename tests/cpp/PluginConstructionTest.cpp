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

juce::RangedAudioParameter* findParameter (DandrumAudioProcessor& processor, const juce::String& parameterId)
{
    for (auto* parameter : processor.getParameters())
    {
        auto* hosted = dynamic_cast<juce::HostedAudioProcessorParameter*> (parameter);
        if (hosted != nullptr && hosted->getParameterID() == parameterId)
            return dynamic_cast<juce::RangedAudioParameter*> (parameter);
    }

    return nullptr;
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

// Writes a copy of the bundled default instrument with a target line replaced,
// so reload tests can prove a genuinely different instrument was loaded (not
// just the same file re-read). Returns an invalid (default-constructed) File
// if targetLine isn't found in the bundled patch, rather than silently
// writing out an unmodified copy — callers must not mistake "no match" for
// "modified".
juce::File writeModifiedKickPatch (const juce::String& targetLine, const juce::String& newLine)
{
    const juce::File original (juce::String (dandrum::defaultPatchPath().string()));
    auto content = original.loadFileAsString();
    if (! content.contains (targetLine))
        return {};

    content = content.replace (targetLine, newLine);

    auto modified = juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getChildFile ("dandrum_modified_kick_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    modified.replaceWithText (content);
    return modified;
}

// A minimal but valid instrument with no preset_surface parameters at all, so
// reloading to it drops every public parameter the default instrument exposed
// (design.md requires that surface-changing reload be visibly reconciled,
// not silently absorbed).
juce::File writePatchWithNoPublicParameters()
{
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile ("dandrum_no_public_parameters_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    file.replaceWithText (
        "metadata:\n"
        "  name: No Public Parameters\n"
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
        std::cerr << "restored plugin did not keep the public parameter layout\n";
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
    if (! (carriedOverTailRms < shortBeforeReloadTailRms * 2.0f))
    {
        std::cerr << "explicit kick.decay_ms value was not carried over across reload: before="
                   << shortBeforeReloadTailRms << " afterReload=" << carriedOverTailRms << '\n';
        return 1;
    }

    // The parameter bridge should still be live on the freshly-reloaded
    // engine: a further explicit change should audibly take effect.
    decayParam->setValueNotifyingHost (1.0f); // normalized maximum: longest decay
    const auto longAfterReloadTailRms = renderTailRms (*reloadProcessor, reloadBlockSize, reloadNumBlocks, 36);
    if (! (longAfterReloadTailRms > carriedOverTailRms * 1.3f))
    {
        std::cerr << "kick.decay_ms changes stopped taking effect after reload: short="
                   << carriedOverTailRms << " long=" << longAfterReloadTailRms << '\n';
        return 1;
    }

    reloadProcessor->releaseResources();
    longDefaultFile.deleteFile();

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
        if (unpreparedProcessor->reloadInstrumentFromFile (juce::File (juce::String (dandrum::defaultPatchPath().string()))))
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

        const juce::File defaultFile (juce::String (dandrum::defaultPatchPath().string()));
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
    // must be visibly reconciled (design.md), not silently absorbed: the host
    // parameter surface stays fixed, but a warning should say what no longer
    // has a live engine target.
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

        if (! dropProcessor->hasPublicParameter ("kick.tune_hz"))
        {
            std::cerr << "reload changed the fixed APVTS parameter surface (kick.tune_hz disappeared)\n";
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

    return 0;
}
