#pragma once

#include <juce_audio_devices/juce_audio_devices.h>

#include "RustEngineSource.h"

class MidiToRustEngine final : public juce::MidiInputCallback
{
public:
    explicit MidiToRustEngine (RustEngineSource& sourceToUse);

    void handleIncomingMidiMessage (juce::MidiInput* source, const juce::MidiMessage& message) override;

private:
    RustEngineSource& source;
};
