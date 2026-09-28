#include "RustEngineSource.h"

#include <algorithm>
#include <cmath>

RustEngineSource::RustEngineSource()
    : engine (dandrum_engine_create())
{
}

RustEngineSource::~RustEngineSource()
{
    const juce::ScopedLock lock (engineLock);
    dandrum_kernel_destroy (kernel);
    dandrum_engine_destroy (engine);
}

void RustEngineSource::prepareToPlay (int samplesPerBlockExpected, double newSampleRate)
{
    const juce::ScopedLock lock (engineLock);
    sampleRateHz = static_cast<std::uint32_t> (juce::jmax (1.0, std::round (newSampleRate)));
    maxBlockSize = static_cast<std::size_t> (juce::jmax (1, samplesPerBlockExpected));
    dandrum_engine_prepare_realtime (engine,
                                     static_cast<float> (newSampleRate),
                                     maxBlockSize);
    if (kernel != nullptr)
    {
        const DandrumKernelBusDeclaration master { "master", 2, 2 };
        auto* replacement = dandrum_kernel_prepare_file (kernelPath.toRawUTF8(), sampleRateHz,
                                                         maxBlockSize, &master, 1);
        dandrum_kernel_destroy (kernel);
        kernel = replacement;
    }
}

void RustEngineSource::releaseResources() {}

bool RustEngineSource::loadPatch (const juce::String& yamlPath)
{
    const juce::ScopedLock lock (engineLock);
    const DandrumKernelBusDeclaration master { "master", 2, 2 };
    if (auto* prepared = dandrum_kernel_prepare_file (yamlPath.toRawUTF8(), sampleRateHz,
                                                      maxBlockSize, &master, 1))
    {
        dandrum_kernel_destroy (kernel);
        kernel = prepared;
        kernelPath = yamlPath;
        return true;
    }
    if (! dandrum_engine_load_patch (engine, yamlPath.toRawUTF8()))
        return false;

    dandrum_kernel_destroy (kernel);
    kernel = nullptr;
    kernelPath.clear();
    return true;
}

void RustEngineSource::getNextAudioBlock (const juce::AudioSourceChannelInfo& bufferToFill)
{
    auto* buffer = bufferToFill.buffer;
    buffer->clear (bufferToFill.startSample, bufferToFill.numSamples);

    if (buffer->getNumChannels() <= 0)
        return;

    auto* left = buffer->getWritePointer (0, bufferToFill.startSample);
    auto* right = buffer->getNumChannels() > 1 ? buffer->getWritePointer (1, bufferToFill.startSample) : left;

    drainPendingMidiEvents();
    if (kernel != nullptr)
    {
        if (buffer->getNumChannels() < 2)
            return;

        const auto totalFrames = static_cast<std::size_t> (bufferToFill.numSamples);
        for (std::size_t offset = 0; offset < totalFrames;)
        {
            const auto frames = std::min (maxBlockSize, totalFrames - offset);
            float* channels[] { left + offset, right + offset };
            const DandrumKernelOutputBusView master { "master", channels, 2, frames };
            if (dandrum_kernel_render (kernel, nullptr, 0, &master, 1, frames) != frames)
                break;
            offset += frames;
        }
    }
    else if (engine != nullptr)
    {
        dandrum_engine_render (engine, left, right, static_cast<std::size_t> (bufferToFill.numSamples));
    }
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
    if (kernel != nullptr)
        return true;
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
        {
            if (kernel != nullptr)
                dandrum_kernel_note_on_at (kernel, event.note, event.velocity, 0);
            else
                dandrum_engine_note_on (engine, event.note, event.velocity);
        }
        else
        {
            if (kernel != nullptr)
                dandrum_kernel_note_off_at (kernel, event.note, 0);
            else
                dandrum_engine_note_off (engine, event.note);
        }

        readIndex = (readIndex + 1) % pendingMidiCapacity;
        pendingMidiReadIndex.store (readIndex, std::memory_order_release);
    }
}
