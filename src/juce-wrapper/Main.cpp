#include <juce_audio_devices/juce_audio_devices.h>

#include "Cli.h"
#include "MidiToRustEngine.h"
#include "RustEngineSource.h"

#include <atomic>
#include <csignal>
#include <iostream>
#include <memory>

namespace
{
std::atomic<bool> shouldQuit { false };

void handleSignal (int)
{
    shouldQuit.store (true);
}

void waitForEngineToFinish (const RustEngineSource& engineSource)
{
    while (! engineSource.hasFinished())
        juce::Thread::sleep (10);
}
} // namespace

int main (int argc, char* argv[])
{
    std::signal (SIGINT, handleSignal);
    std::signal (SIGTERM, handleSignal);

    const juce::StringArray args (argv + 1, argc - 1);

    if (args.contains ("--list-midi-inputs"))
    {
        printMidiInputs();
        return 0;
    }

    juce::AudioDeviceManager deviceManager;
    const auto error = deviceManager.initialiseWithDefaultDevices (0, 2);

    if (error.isNotEmpty())
    {
        std::cerr << "Failed to initialize audio device: " << error << '\n';
        return 1;
    }

    printAudioDeviceInfo (deviceManager);

    RustEngineSource engineSource;
    juce::AudioSourcePlayer player;
    player.setSource (&engineSource);
    deviceManager.addAudioCallback (&player);

    const auto patchArgIndex = args.indexOf ("--patch");
    if (patchArgIndex >= 0 && patchArgIndex + 1 < args.size())
    {
        const auto patchPath = args[patchArgIndex + 1];
        if (! engineSource.loadPatch (patchPath))
        {
            std::cerr << "Failed to load patch: " << patchPath << '\n';
            return 1;
        }
        std::cout << "Loaded patch: " << patchPath << '\n';
    }

    const auto exitCode = [&]() -> int {
        const auto testMidiNoteArgIndex = args.indexOf ("--test-midi-note");
        if (testMidiNoteArgIndex >= 0 && testMidiNoteArgIndex + 1 < args.size())
        {
            const auto note = juce::jlimit (0, 127, args[testMidiNoteArgIndex + 1].getIntValue());
            MidiToRustEngine syntheticMidi (engineSource);

            std::cout << "Synthetic MIDI test note: " << note << '\n';
            syntheticMidi.handleIncomingMidiMessage (nullptr, juce::MidiMessage::noteOn (1, note, static_cast<juce::uint8> (110)));
            juce::Thread::sleep (180);
            syntheticMidi.handleIncomingMidiMessage (nullptr, juce::MidiMessage::noteOff (1, note));
            waitForEngineToFinish (engineSource);
            return 0;
        }

        if (args.contains ("--test-midi-scale"))
        {
            MidiToRustEngine syntheticMidi (engineSource);
            const int cMajor[] { 60, 62, 64, 65, 67, 69, 71, 72 };

            std::cout << "Synthetic MIDI C major scale\n";
            for (const auto note : cMajor)
            {
                std::cout << "Scale note: " << note << '\n';
                syntheticMidi.handleIncomingMidiMessage (nullptr, juce::MidiMessage::noteOn (1, note, static_cast<juce::uint8> (110)));
                juce::Thread::sleep (180);
                syntheticMidi.handleIncomingMidiMessage (nullptr, juce::MidiMessage::noteOff (1, note));
                juce::Thread::sleep (40);
            }

            waitForEngineToFinish (engineSource);
            return 0;
        }

        const auto midiArgIndex = args.indexOf ("--midi-input");
        if (midiArgIndex >= 0 && midiArgIndex + 1 < args.size())
        {
            const auto devices = juce::MidiInput::getAvailableDevices();
            const auto requestedIndex = args[midiArgIndex + 1].getIntValue();

            if (requestedIndex < 0 || requestedIndex >= devices.size())
            {
                std::cerr << "Invalid MIDI input index: " << requestedIndex << "\n";
                printMidiInputs();
                return 1;
            }

            MidiToRustEngine midiCallback (engineSource);
            const auto midiIdentifier = devices[requestedIndex].identifier;
            deviceManager.setMidiInputDeviceEnabled (midiIdentifier, true);
            deviceManager.addMidiInputDeviceCallback (midiIdentifier, &midiCallback);

            std::cout << "Opened MIDI input: " << devices[requestedIndex].name << '\n';
            std::cout << "Play MIDI notes. Press Ctrl+C to quit.\n";

            while (! shouldQuit.load())
                juce::Thread::sleep (50);

            deviceManager.removeMidiInputDeviceCallback (midiIdentifier, &midiCallback);
            return 0;
        }

        std::cout << "Rust engine test note. Use --test-midi-note <note>, --test-midi-scale, --list-midi-inputs, or --midi-input <index>.\n";
        engineSource.noteOn (60, 110);
        waitForEngineToFinish (engineSource);
        return 0;
    }();

    deviceManager.removeAudioCallback (&player);
    player.setSource (nullptr);
    return exitCode;
}
