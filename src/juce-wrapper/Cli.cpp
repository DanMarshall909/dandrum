#include "Cli.h"

#include <iostream>

void printAudioDeviceInfo (juce::AudioDeviceManager& deviceManager)
{
    if (auto* device = deviceManager.getCurrentAudioDevice())
    {
        const auto sampleRate = device->getCurrentSampleRate();
        const auto bufferSize = device->getCurrentBufferSizeSamples();
        const auto outputLatency = device->getOutputLatencyInSamples();
        const auto bufferMs = 1000.0 * static_cast<double> (bufferSize) / sampleRate;
        const auto outputLatencyMs = 1000.0 * static_cast<double> (outputLatency) / sampleRate;

        std::cout << "Audio device: " << device->getName() << '\n';
        std::cout << "Sample rate: " << sampleRate << " Hz\n";
        std::cout << "Buffer size: " << bufferSize << " samples (" << bufferMs << " ms)\n";
        std::cout << "Reported output latency: " << outputLatency << " samples (" << outputLatencyMs << " ms)\n";
    }
}

void printMidiInputs()
{
    const auto devices = juce::MidiInput::getAvailableDevices();

    if (devices.isEmpty())
    {
        std::cout << "No MIDI inputs found.\n";
        return;
    }

    std::cout << "MIDI inputs:\n";

    for (int i = 0; i < devices.size(); ++i)
        std::cout << "  [" << i << "] " << devices[i].name << '\n';
}
