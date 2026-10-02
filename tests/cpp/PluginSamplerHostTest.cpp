#include "PluginProcessor.h"

#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <iostream>
#include <memory>
#include <thread>

namespace
{
constexpr int blockSize = 16;

std::unique_ptr<DandrumAudioProcessor> makeSampler()
{
    std::unique_ptr<juce::AudioProcessor> plugin (createPluginFilter());
    auto* sampler = dynamic_cast<DandrumAudioProcessor*> (plugin.get());
    if (sampler == nullptr)
        return {};
    plugin.release();
    auto result = std::unique_ptr<DandrumAudioProcessor> (sampler);
    result->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    result->prepareToPlay (48000.0, blockSize);
    return result;
}

std::array<float, 4> hit (DandrumAudioProcessor& sampler, int note)
{
    juce::AudioBuffer<float> buffer (2, blockSize);
    buffer.clear();
    juce::MidiBuffer midi;
    midi.addEvent (juce::MidiMessage::noteOn (1, note, static_cast<juce::uint8> (100)), 0);
    sampler.processBlock (buffer, midi);
    return { buffer.getSample (0, 0), buffer.getSample (1, 0),
             buffer.getSample (0, 1), buffer.getSample (1, 1) };
}

bool near (float actual, float expected)
{
    return std::abs (actual - expected) < 0.0001f;
}

float continuation (DandrumAudioProcessor& sampler)
{
    juce::AudioBuffer<float> buffer (2, blockSize);
    buffer.clear();
    juce::MidiBuffer noMidi;
    sampler.processBlock (buffer, noMidi);
    return buffer.getSample (0, 0);
}

bool modulatedHit (const char* id, float normalised, int note,
                   std::array<float, 4>& samples)
{
    auto sampler = makeSampler();
    if (sampler == nullptr || ! sampler->isInstrumentLoaded())
        return false;
    auto* parameter = sampler->getParameterForPublicId (id);
    if (parameter == nullptr)
        return false;
    const auto yaml = sampler->currentInstrumentYaml();
    const auto path = sampler->currentInstrumentFile();
    juce::AudioBuffer<float> empty (2, blockSize);
    empty.clear();
    juce::MidiBuffer noMidi;
    sampler->processBlock (empty, noMidi);
    parameter->setValueNotifyingHost (normalised);
    samples = hit (*sampler, note);
    return sampler->currentInstrumentYaml() == yaml
        && sampler->currentInstrumentFile() == path
        && sampler->getParameterForPublicId (id) == parameter;
}
}

int main()
{
    auto sampler = makeSampler();
    const auto packagedRoot = juce::File::getSpecialLocation (juce::File::userApplicationDataDirectory)
                                  .getChildFile ("Dandrum/Sampler Example");
    if (sampler == nullptr || ! sampler->isInstrumentLoaded()
        || sampler->demoConfiguration().instrumentId != "dandrum.advanced-drum-kit"
        || sampler->getActivePublicParameterIds().size() != 23
        || ! juce::File (juce::String (sampler->demoConfiguration().instrumentPath.string()))
                  .getParentDirectory().getChildFile ("assets/advanced-drums.wav").existsAsFile()
        || juce::File (juce::String (sampler->demoConfiguration().instrumentPath.string()))
                  .getParentDirectory() != packagedRoot)
    {
        std::cerr << "sampler VST example did not load the drum kit by default\n";
        return 1;
    }
    for (const auto* id : { "drums.pitch_ratio", "drums.start_offset", "drums.level",
                            "drums.pan", "drums.variation",
                            "drums.kick.pitch_ratio", "drums.kick.start_offset", "drums.kick.level", "drums.kick.pan",
                            "drums.snare.pitch_ratio", "drums.snare.start_offset", "drums.snare.level", "drums.snare.pan", "drums.snare.variation",
                            "drums.closed_hat.pitch_ratio", "drums.closed_hat.start_offset", "drums.closed_hat.level", "drums.closed_hat.pan",
                            "drums.open_hat.pitch_ratio", "drums.open_hat.start_offset", "drums.open_hat.level", "drums.open_hat.pan", "drums.open_hat.variation" })
    {
        const auto* parameter = sampler->getParameterForPublicId (id);
        if (parameter == nullptr || parameter->getName (128) != juce::String (id))
        {
            std::cerr << "sampler VST host parameter is not named for its public control: " << id << '\n';
            return 1;
        }
    }
    const auto preparedUi = sampler->getPreparedUiDocument();
    if (! preparedUi || preparedUi->instrumentId != "dandrum.advanced-drum-kit"
        || preparedUi->sources.size() != 1 || preparedUi->maps.size() != 1
        || preparedUi->sources[0].id != "drums"
        || preparedUi->sources[0].sampleRateHz != 48000
        || preparedUi->sources[0].frameCount != 51000
        || preparedUi->sources[0].regions.size() != 6
        || preparedUi->maps[0].selectionMode != "round_robin"
        || preparedUi->maps[0].zones.size() != 7
        || preparedUi->maps[0].zones[1].velocityHigh != 63
        || preparedUi->maps[0].zones[2].velocityLow != 64
        || preparedUi->maps[0].zones[2].roundRobinGroup != "hard_snare"
        || preparedUi->maps[0].zones[2].controlGroup != 2
        || ! preparedUi->capabilities.sampleKeyMap
        || ! preparedUi->capabilities.preparedWaveform
        || preparedUi->capabilities.synthLayer
        || preparedUi->capabilities.patternSequencer)
    {
        std::cerr << "prepared sampler UI document lost actual kit metadata or capabilities\n";
        return 1;
    }
    const auto shared = std::find_if (preparedUi->parameters.begin(), preparedUi->parameters.end(),
        [] (const auto& parameter) { return parameter.id == "drums.pitch_ratio"; });
    const auto snareControl = std::find_if (preparedUi->parameters.begin(), preparedUi->parameters.end(),
        [] (const auto& parameter) { return parameter.id == "drums.snare.pitch_ratio"; });
    if (preparedUi->parameters.size() != 23
        || shared == preparedUi->parameters.end() || snareControl == preparedUi->parameters.end()
        || shared->scope != InstrumentUiDocument::ControlScope::instrument
        || snareControl->scope != InstrumentUiDocument::ControlScope::sampleGroup
        || snareControl->controlGroup != 2)
    {
        std::cerr << "prepared sampler UI document lost shared/per-pad parameter scope\n";
        return 1;
    }
    const auto waveformJob = sampler->requestPreparedWaveform (
        preparedUi->generation, "drums", "kick", 0, 16);
    if (! waveformJob
        || sampler->requestPreparedWaveform (preparedUi->generation, "drums", "missing", 0, 16)
        || sampler->requestPreparedWaveform (preparedUi->generation, "missing", "kick", 0, 16)
        || sampler->requestPreparedWaveform (preparedUi->generation, "drums", "kick", 1, 16)
        || sampler->requestPreparedWaveform (preparedUi->generation - 1, "drums", "kick", 0, 16))
    {
        std::cerr << "prepared waveform admission ignored source, region, channel or generation\n";
        return 1;
    }
    auto waveformStatus = sampler->getPreparedWaveformJobStatus (*waveformJob);
    for (int attempt = 0; attempt < 200
         && waveformStatus && waveformStatus->state == InstrumentUiWaveformService::State::running;
         ++attempt)
    {
        std::this_thread::sleep_for (std::chrono::milliseconds (5));
        waveformStatus = sampler->getPreparedWaveformJobStatus (*waveformJob);
    }
    if (! waveformStatus || waveformStatus->state != InstrumentUiWaveformService::State::ready
        || ! waveformStatus->result || waveformStatus->result->sourceId != "drums"
        || waveformStatus->result->regionId != "kick"
        || waveformStatus->result->sampleRateHz != 48000
        || waveformStatus->result->startFrame != 0 || waveformStatus->result->endFrame != 12000
        || waveformStatus->result->buckets.size() != 16
        || waveformStatus->result->buckets.front().startFrame != 0
        || waveformStatus->result->buckets.back().endFrame != 12000)
    {
        std::cerr << "prepared waveform job lost its source-frame coordinates or rate\n";
        return 1;
    }
    const auto lowest = std::min_element (waveformStatus->result->buckets.begin(),
                                          waveformStatus->result->buckets.end(),
        [] (const auto& a, const auto& b) { return a.minimum < b.minimum; });
    const auto highest = std::max_element (waveformStatus->result->buckets.begin(),
                                           waveformStatus->result->buckets.end(),
        [] (const auto& a, const auto& b) { return a.maximum < b.maximum; });
    if (! near (lowest->minimum, -0.69995117f)
        || ! near (highest->maximum, 0.80514526f))
    {
        std::cerr << "prepared waveform lost known signed kick extrema\n";
        return 1;
    }
    const auto waveformSession = sampler->uiCommands().createSession();
    const auto sessionWaveform = sampler->requestPreparedWaveform (
        preparedUi->generation, "drums", "kick", 0, 16, waveformSession);
    const auto ownedStatus = sessionWaveform
        ? sampler->getPreparedWaveformJobStatus (*sessionWaveform) : std::nullopt;
    if (! ownedStatus || ownedStatus->sessionId != waveformSession)
    {
        std::cerr << "prepared waveform request lost its editor session owner\n";
        return 1;
    }
    sampler->cancelPreparedWaveformSession (waveformSession);
    sampler->uiCommands().closeSession (waveformSession);
    auto reloadReader = makeSampler();
    const auto oldDocument = reloadReader->getPreparedUiDocument();
    const auto oldGeneration = oldDocument ? oldDocument->generation : 0;
    const auto oldWaveformJob = reloadReader->requestPreparedWaveform (
        oldGeneration, "drums", "kick", 0, 16);
    const auto tb303 = juce::File (juce::String (
        InstrumentDemoConfiguration::tb303().instrumentPath.string()));
    if (! oldDocument || ! oldWaveformJob || ! reloadReader->reloadInstrumentFromFile (tb303))
        return 1;
    const auto replacementUi = reloadReader->getPreparedUiDocument();
    if (! replacementUi || replacementUi->generation == oldGeneration
        || replacementUi->instrumentId != "dandrum.tb303-acid"
        || ! replacementUi->sources.empty() || ! replacementUi->maps.empty()
        || replacementUi->capabilities.preparedWaveform
        || replacementUi->parameters.size() != 7
        || ! std::all_of (replacementUi->parameters.begin(), replacementUi->parameters.end(),
            [] (const auto& parameter) {
                return parameter.scope == InstrumentUiDocument::ControlScope::instrument;
            })
        || oldDocument->sources[0].regions[2].id != "snare_hard_a"
        || oldDocument->maps[0].zones[2].velocityLow != 64)
    {
        std::cerr << "UI document did not remain owned across instrument reload\n";
        return 1;
    }
    const auto retiredWaveform = reloadReader->getPreparedWaveformJobStatus (*oldWaveformJob);
    if (! retiredWaveform || retiredWaveform->state != InstrumentUiWaveformService::State::stale
        || reloadReader->requestPreparedWaveform (replacementUi->generation,
                                                   "drums", "kick", 0, 16))
    {
        std::cerr << "reload left old waveform data actionable\n";
        return 1;
    }
    auto reprepareReader = makeSampler();
    std::atomic<bool> malformedDocument { false };
    std::thread observer ([&]
    {
        for (int attempt = 0; attempt < 64; ++attempt)
        {
            const auto document = reprepareReader->getPreparedUiDocument();
            if (! document || document->sources.size() != 1 || document->maps.size() != 1
                || document->sources[0].regions.size() != 6
                || document->maps[0].zones[2].velocityLow != 64)
                malformedDocument.store (true, std::memory_order_relaxed);
        }
    });
    for (int attempt = 0; attempt < 8; ++attempt)
        reprepareReader->prepareToPlay (attempt % 2 == 0 ? 44100.0 : 96000.0, blockSize);
    observer.join();
    if (malformedDocument.load (std::memory_order_relaxed))
    {
        std::cerr << "prepared UI reader observed an incomplete host reprepare\n";
        return 1;
    }
    const auto kick = hit (*sampler, 36);
    if (! near (kick[0], -0.5f) || ! near (kick[1], -0.5f))
    {
        std::cerr << "mapped kick note did not render the bundled sample\n";
        return 1;
    }

    std::array<float, 4> pitched {}, started {}, leveled {}, panned {}, varied {};
    if (! modulatedHit ("drums.pitch_ratio", (2.0f - 0.125f) / (8.0f - 0.125f), 36, pitched)
        || ! modulatedHit ("drums.start_offset", 0.5f, 36, started)
        || ! modulatedHit ("drums.level", 0.125f, 36, leveled)
        || ! modulatedHit ("drums.pan", 1.0f, 36, panned)
        || ! modulatedHit ("drums.variation", 1.0f, 38, varied)
        || ! near (pitched[0], -0.5f) || near (pitched[2], kick[2])
        || near (started[0], kick[0])
        || ! near (leveled[0], -0.25f)
        || ! near (panned[0], 0.0f) || ! near (panned[1], -0.5f)
        || ! near (varied[0], -0.4375f))
    {
        std::cerr << "host parameter modulation did not reach all five live sample controls\n";
        return 1;
    }

    std::array<float, 4> kickPitched {}, kickStarted {}, kickLeveled {}, kickOnSnare {}, snarePanned {}, openVaried {};
    auto referenceSnare = makeSampler();
    auto referenceOpenHat = makeSampler();
    if (referenceSnare == nullptr || referenceOpenHat == nullptr)
        return 1;
    const auto snare = hit (*referenceSnare, 38);
    const auto openHat = hit (*referenceOpenHat, 46);
    if (! modulatedHit ("drums.kick.pitch_ratio", (2.0f - 0.125f) / (8.0f - 0.125f), 36, kickPitched)
        || ! modulatedHit ("drums.kick.start_offset", 0.5f, 36, kickStarted)
        || ! modulatedHit ("drums.kick.level", 0.125f, 36, kickLeveled)
        || ! modulatedHit ("drums.kick.level", 0.125f, 38, kickOnSnare)
        || ! modulatedHit ("drums.snare.pan", 1.0f, 38, snarePanned)
        || ! modulatedHit ("drums.open_hat.variation", 1.0f, 46, openVaried)
        || near (kickPitched[2], kick[2]) || near (kickStarted[0], kick[0])
        || ! near (kickLeveled[0], -0.25f)
        || ! near (kickOnSnare[0], snare[0])
        || ! near (snarePanned[0], 0.0f) || near (snarePanned[1], 0.0f)
        || near (openVaried[0], openHat[0]))
    {
        std::cerr << "host modulation did not address drum pads independently\n";
        return 1;
    }

    auto baselineTail = makeSampler();
    auto modulatedTail = makeSampler();
    hit (*baselineTail, 36);
    hit (*modulatedTail, 36);
    auto* liveLevel = modulatedTail->getParameterForPublicId ("drums.kick.level");
    liveLevel->setValueNotifyingHost (0.125f);
    const auto baselineFrame = continuation (*baselineTail);
    const auto modulatedFrame = continuation (*modulatedTail);
    if (std::abs (baselineFrame) < 0.001f || ! near (modulatedFrame, baselineFrame * 0.5f))
    {
        std::cerr << "host level change did not affect an already sounding drum voice\n";
        return 1;
    }

    auto* level = sampler->getParameterForPublicId ("drums.level");
    level->setValueNotifyingHost (0.125f);
    juce::MemoryBlock saved;
    sampler->getStateInformation (saved);
    auto restored = makeSampler();
    restored->setStateInformation (saved.getData(), static_cast<int> (saved.getSize()));
    if (! restored->isInstrumentLoaded() || ! restored->getLastLoadError().isEmpty()
        || ! near (hit (*restored, 36)[0], -0.25f))
    {
        std::cerr << "sampler state did not restore its relative sample assets and host level\n";
        return 1;
    }

    auto presetSampler = makeSampler();
    const auto projectRoot = std::filesystem::path (__FILE__).parent_path().parent_path().parent_path();
    const auto preset = juce::File (juce::String ((projectRoot
        / "examples/presets/advanced-drum-kit-quiet-left.yaml").string()));
    if (! presetSampler->loadPresetFromFile (preset)
        || ! near (hit (*presetSampler, 36)[0], -0.25f))
    {
        std::cerr << "bundled drum preset did not apply to the sampler: "
                  << presetSampler->getLastPresetError() << '\n';
        return 1;
    }
    return 0;
}
