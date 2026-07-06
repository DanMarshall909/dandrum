#include "PluginProcessor.h"

#include <cmath>
#include <iostream>
#include <memory>

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

    return 0;
}
