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

void RustEngineSource::prepareToPlay (int /*samplesPerBlockExpected*/, double newSampleRate)
{
    const juce::ScopedLock lock (engineLock);
    dandrum_engine_prepare (engine, static_cast<float> (newSampleRate));
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

    const juce::ScopedLock lock (engineLock);
    dandrum_engine_render (engine, left, right, static_cast<std::size_t> (bufferToFill.numSamples));
}

void RustEngineSource::noteOn (int note, int velocity)
{
    const juce::ScopedLock lock (engineLock);
    dandrum_engine_note_on (engine, static_cast<unsigned char> (juce::jlimit (0, 127, note)),
                            static_cast<unsigned char> (juce::jlimit (0, 127, velocity)));
}

void RustEngineSource::noteOff (int note)
{
    const juce::ScopedLock lock (engineLock);
    dandrum_engine_note_off (engine, static_cast<unsigned char> (juce::jlimit (0, 127, note)));
}

bool RustEngineSource::hasFinished() const
{
    const juce::ScopedLock lock (engineLock);
    return dandrum_engine_is_finished (engine);
}
