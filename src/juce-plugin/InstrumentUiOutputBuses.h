#pragma once

#include "InstrumentUiDocument.h"
#include <juce_audio_processors/juce_audio_processors.h>
#include <utility>

// Capture during an owner-serialized configuration phase, never from audio.
// All names and layouts are copied; no JUCE bus pointer survives this call.
inline std::vector<InstrumentUiDocument::OutputBus> captureInstrumentUiOutputBuses (
    const juce::AudioProcessor& processor, const std::string& stereoMainMeter = {})
{
    std::vector<InstrumentUiDocument::OutputBus> result;
    const auto layout = processor.getBusesLayout();
    for (int index = 0; index < layout.outputBuses.size(); ++index)
    {
        InstrumentUiDocument::OutputBus bus;
        bus.id = "output:" + std::to_string (index);
        bus.name = processor.getBus (false, index)->getName().toStdString();
        bus.main = index == 0;
        const auto& channels = layout.outputBuses[index];
        for (int channel = 0; channel < channels.size(); ++channel)
            bus.channels.push_back (juce::AudioChannelSet::getAbbreviatedChannelTypeName (
                channels.getTypeOfChannel (channel)).toStdString());
        if (bus.main && channels == juce::AudioChannelSet::stereo())
            bus.meterBusId = stereoMainMeter;
        result.push_back (std::move (bus));
    }
    return result;
}
