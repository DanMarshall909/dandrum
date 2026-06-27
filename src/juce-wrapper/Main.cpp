#include <juce_audio_basics/juce_audio_basics.h>
#include <juce_audio_devices/juce_audio_devices.h>

#include <iostream>

extern "C"
{
struct DandrumEngine;

DandrumEngine* dandrum_engine_create();
void dandrum_engine_destroy (DandrumEngine* engine);
void dandrum_engine_prepare (DandrumEngine* engine, float sampleRate);
std::size_t dandrum_engine_render (DandrumEngine* engine, float* left, float* right, std::size_t numSamples);
bool dandrum_engine_is_finished (const DandrumEngine* engine);
}

namespace
{
class BeepSource final : public juce::AudioSource
{
public:
    BeepSource()
        : engine (dandrum_engine_create())
    {
    }

    ~BeepSource() override
    {
        dandrum_engine_destroy (engine);
    }

    void prepareToPlay (int /*samplesPerBlockExpected*/, double newSampleRate) override
    {
        dandrum_engine_prepare (engine, static_cast<float> (newSampleRate));
    }

    void releaseResources() override {}

    void getNextAudioBlock (const juce::AudioSourceChannelInfo& bufferToFill) override
    {
        auto* buffer = bufferToFill.buffer;
        buffer->clear (bufferToFill.startSample, bufferToFill.numSamples);

        if (engine == nullptr || buffer->getNumChannels() <= 0)
            return;

        auto* left = buffer->getWritePointer (0, bufferToFill.startSample);
        auto* right = buffer->getNumChannels() > 1 ? buffer->getWritePointer (1, bufferToFill.startSample) : left;
        dandrum_engine_render (engine, left, right, static_cast<std::size_t> (bufferToFill.numSamples));
    }

    bool hasFinished() const
    {
        return dandrum_engine_is_finished (engine);
    }

private:
    DandrumEngine* engine = nullptr;
};
} // namespace

int main()
{
    juce::AudioDeviceManager deviceManager;
    const auto error = deviceManager.initialiseWithDefaultDevices (0, 2);

    if (error.isNotEmpty())
    {
        std::cerr << "Failed to initialize audio device: " << error << '\n';
        return 1;
    }

    BeepSource beep;
    juce::AudioSourcePlayer player;
    player.setSource (&beep);
    deviceManager.addAudioCallback (&player);

    std::cout << "Rust engine sound.\n";

    while (! beep.hasFinished())
        juce::Thread::sleep (10);

    deviceManager.removeAudioCallback (&player);
    player.setSource (nullptr);

    return 0;
}
