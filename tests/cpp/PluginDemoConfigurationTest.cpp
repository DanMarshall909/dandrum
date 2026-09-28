#include "PluginProcessor.h"
#include "DefaultPatch.h"

#include <cmath>
#include <iostream>
#include <memory>
#include <vector>

namespace
{
bool hasAudio (const juce::AudioBuffer<float>& buffer)
{
    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
        for (int frame = 0; frame < buffer.getNumSamples(); ++frame)
            if (std::abs (buffer.getSample (channel, frame)) > 0.000001f)
                return true;
    return false;
}
}

int main()
{
    DandrumAudioProcessor processor;
    const auto expected = juce::File (juce::String (
        dandrum::findRepositoryExample ("examples/patches/tb303-acid.yaml").string()));
    if (! processor.isInstrumentLoaded()
        || processor.currentInstrumentFile() != expected
        || ! processor.hasPublicParameter ("filter.cutoff")
        || processor.hasPublicParameter ("kick.tune_hz"))
    {
        std::cerr << "fresh TB-303 demo did not load the 303 instrument and public surface\n";
        return 1;
    }

    const auto& tb303 = processor.demoConfiguration();
    if (tb303.instrumentPath != expected.getFullPathName().toStdString()
        || ! tb303.soundLabFixturePath.has_value()
        || *tb303.soundLabFixturePath
               != dandrum::findRepositoryExample ("examples/sound-design/tb303-acid-poc.yaml")
        || ! tb303.matchSourcePath.has_value()
        || *tb303.matchSourcePath != expected.getFullPathName().toStdString()
        || tb303.title != "Dandrum TB-303"
        || tb303.indexHtml.find ("TB-303") == std::string::npos)
    {
        std::cerr << "TB-303 demo configuration does not select a coherent patch, fixture, source, title and page\n";
        return 1;
    }

    const auto kickConfig = InstrumentDemoConfiguration::kick();
    DandrumAudioProcessor kick (kickConfig);
    if (! kick.isInstrumentLoaded()
        || kick.currentInstrumentFile().getFullPathName().toStdString() != kickConfig.instrumentPath
        || ! kick.hasPublicParameter ("kick.tune_hz")
        || kick.hasPublicParameter ("filter.cutoff")
        || ! kickConfig.soundLabFixturePath.has_value()
        || *kickConfig.soundLabFixturePath
               != dandrum::findRepositoryExample ("examples/sound-design/synthetic-808-kick-poc.yaml")
        || ! kickConfig.matchSourcePath.has_value()
        || *kickConfig.matchSourcePath != kickConfig.instrumentPath
        || kickConfig.title == tb303.title
        || kickConfig.indexHtml == tb303.indexHtml)
    {
        std::cerr << "second demo configuration did not select a distinct kick experience\n";
        return 1;
    }

    constexpr int blockSize = 128;
    kick.setPlayConfigDetails (0, 2, 48000.0, blockSize);
    kick.prepareToPlay (48000.0, blockSize);
    auto* decay = kick.getParameterForPublicId ("kick.decay_ms");
    if (decay == nullptr)
        return 1;
    decay->setValueNotifyingHost (0.375f);
    juce::MemoryBlock savedState;
    kick.getStateInformation (savedState);

    DandrumAudioProcessor restored;
    restored.setPlayConfigDetails (0, 2, 48000.0, blockSize);
    restored.prepareToPlay (48000.0, blockSize);
    const auto slotsBefore = restored.getParameters();
    std::vector<juce::String> idsBefore;
    for (const auto* slot : slotsBefore)
        idsBefore.push_back (static_cast<const juce::RangedAudioParameter*> (slot)->paramID);
    restored.setStateInformation (savedState.getData(), static_cast<int> (savedState.getSize()));
    const auto slotsAfter = restored.getParameters();
    const auto* restoredDecay = restored.getParameterForPublicId ("kick.decay_ms");
    if (restored.currentInstrumentFile() != kick.currentInstrumentFile()
        || restored.currentInstrumentYaml() != kick.currentInstrumentYaml()
        || restoredDecay == nullptr || std::abs (restoredDecay->getValue() - 0.375f) > 0.00001f
        || slotsAfter.size() != 64 || slotsAfter.size() != slotsBefore.size())
    {
        std::cerr << "cross-demo state restore lost the kick instrument, values or fixed host slots\n";
        return 1;
    }
    for (int index = 0; index < slotsAfter.size(); ++index)
        if (slotsAfter[index] != slotsBefore[index]
            || static_cast<const juce::RangedAudioParameter*> (slotsAfter[index])->paramID
                   != idsBefore[static_cast<std::size_t> (index)])
        {
            std::cerr << "cross-demo state restore changed an automation slot identity\n";
            return 1;
        }

    const auto missing = juce::File::getSpecialLocation (juce::File::tempDirectory)
                             .getNonexistentChildFile ("dandrum_demo_missing", ".yaml");
    if (restored.reloadInstrumentFromFile (missing)
        || restored.getLastLoadError().isEmpty()
        || restored.currentInstrumentFile() != kick.currentInstrumentFile()
        || restored.getParameterForPublicId ("kick.decay_ms") != restoredDecay)
    {
        std::cerr << "failed cross-demo reload did not preserve the restored instrument\n";
        return 1;
    }
    juce::AudioBuffer<float> buffer (2, blockSize);
    buffer.clear();
    juce::MidiBuffer midi;
    midi.addEvent (juce::MidiMessage::noteOn (1, 36, static_cast<juce::uint8> (100)), 0);
    restored.processBlock (buffer, midi);
    if (! hasAudio (buffer))
    {
        std::cerr << "restored instrument became silent after failed reload\n";
        return 1;
    }

    kick.releaseResources();
    restored.releaseResources();
    return 0;
}
