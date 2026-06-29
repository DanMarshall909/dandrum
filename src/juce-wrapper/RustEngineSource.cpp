#include "RustEngineSource.h"

RustEngineSource::RustEngineSource()
    : engine (dandrum_engine_create())
{
}

RustEngineSource::~RustEngineSource()
{
    const juce::ScopedLock lock (engineLock);
    dandrum_engine_destroy (engine);
}

void RustEngineSource::prepareToPlay (int samplesPerBlockExpected, double newSampleRate)
{
    const juce::ScopedLock lock (engineLock);
    dandrum_engine_prepare_realtime (engine,
                                     static_cast<float> (newSampleRate),
                                     static_cast<std::size_t> (juce::jmax (1, samplesPerBlockExpected)));
}

void RustEngineSource::releaseResources() {}

bool RustEngineSource::loadPatch (const juce::String& yamlPath)
{
    const juce::ScopedLock lock (engineLock);
    return dandrum_engine_load_patch (engine, yamlPath.toRawUTF8());
}

void RustEngineSource::getNextAudioBlock (const juce::AudioSourceChannelInfo& bufferToFill)
{
    auto* buffer = bufferToFill.buffer;
    buffer->clear (bufferToFill.startSample, bufferToFill.numSamples);

    if (engine == nullptr || buffer->getNumChannels() <= 0)
        return;

    auto* left = buffer->getWritePointer (0, bufferToFill.startSample);
    auto* right = buffer->getNumChannels() > 1 ? buffer->getWritePointer (1, bufferToFill.startSample) : left;

    drainPendingMidiEvents();
    dandrum_engine_render (engine, left, right, static_cast<std::size_t> (bufferToFill.numSamples));
}

bool RustEngineSource::noteOn (int note, int velocity)
{
    return enqueueMidiEvent ({ PendingMidiEventType::noteOn,
                               static_cast<unsigned char> (juce::jlimit (0, 127, note)),
                               static_cast<unsigned char> (juce::jlimit (0, 127, velocity)) });
}

bool RustEngineSource::noteOff (int note)
{
    return enqueueMidiEvent ({ PendingMidiEventType::noteOff,
                               static_cast<unsigned char> (juce::jlimit (0, 127, note)),
                               0 });
}

bool RustEngineSource::hasFinished() const
{
    const juce::ScopedLock lock (engineLock);
    return dandrum_engine_is_finished (engine);
}

bool RustEngineSource::enqueueMidiEvent (PendingMidiEvent event)
{
    const auto writeIndex = pendingMidiWriteIndex.load (std::memory_order_relaxed);
    const auto nextWriteIndex = (writeIndex + 1) % pendingMidiCapacity;

    if (nextWriteIndex == pendingMidiReadIndex.load (std::memory_order_acquire))
    {
        droppedMidiEvents.fetch_add (1, std::memory_order_relaxed);
        return false;
    }

    pendingMidiEvents[writeIndex] = event;
    pendingMidiWriteIndex.store (nextWriteIndex, std::memory_order_release);
    return true;
}

void RustEngineSource::drainPendingMidiEvents()
{
    auto readIndex = pendingMidiReadIndex.load (std::memory_order_relaxed);

    while (readIndex != pendingMidiWriteIndex.load (std::memory_order_acquire))
    {
        const auto event = pendingMidiEvents[readIndex];

        if (event.type == PendingMidiEventType::noteOn)
            dandrum_engine_note_on (engine, event.note, event.velocity);
        else
            dandrum_engine_note_off (engine, event.note);

        readIndex = (readIndex + 1) % pendingMidiCapacity;
        pendingMidiReadIndex.store (readIndex, std::memory_order_release);
    }
}
