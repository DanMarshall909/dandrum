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
} // namespace

int main()
{
    constexpr int blockSize = 64;

    auto processor = std::make_unique<DandrumAudioProcessor>();
    processor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    processor->prepareToPlay (48000.0, blockSize);

    juce::AudioBuffer<float> buffer (2, blockSize);
    buffer.clear();

    juce::MidiBuffer midi;
    midi.addEvent (juce::MidiMessage::noteOn (1, 60, (juce::uint8) 100), 10);
    midi.addEvent (juce::MidiMessage::noteOff (1, 60), 50);

    processor->processBlock (buffer, midi);

    if (! bufferIsFinite (buffer))
    {
        std::cerr << "plugin processBlock produced non-finite samples\n";
        return 1;
    }

    processor->releaseResources();

    return 0;
}
