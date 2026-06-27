#include "MidiToRustEngine.h"

#include <iostream>

MidiToRustEngine::MidiToRustEngine (RustEngineSource& sourceToUse)
    : source (sourceToUse)
{
}

void MidiToRustEngine::handleIncomingMidiMessage (juce::MidiInput* /*source*/, const juce::MidiMessage& message)
{
    if (message.isNoteOn())
    {
        source.noteOn (message.getNoteNumber(), static_cast<int> (message.getVelocity() * 127.0f));
        std::cout << "note on  " << message.getNoteNumber() << '\n';
        return;
    }

    if (message.isNoteOff())
    {
        source.noteOff (message.getNoteNumber());
        std::cout << "note off " << message.getNoteNumber() << '\n';
    }
}
