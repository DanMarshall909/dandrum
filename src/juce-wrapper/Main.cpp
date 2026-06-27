#include <juce_audio_basics/juce_audio_basics.h>
#include <juce_audio_devices/juce_audio_devices.h>

#include <atomic>
#include <cmath>
#include <iostream>

namespace
{
class BeepSource final : public juce::AudioSource
{
public:
    void prepareToPlay (int /*samplesPerBlockExpected*/, double newSampleRate) override
    {
        sampleRate = newSampleRate;
        phase = 0.0;
        remainingSamples.store (static_cast<int> (sampleRate * beepSeconds));
    }

    void releaseResources() override {}

    void getNextAudioBlock (const juce::AudioSourceChannelInfo& bufferToFill) override
    {
        auto* buffer = bufferToFill.buffer;
        buffer->clear (bufferToFill.startSample, bufferToFill.numSamples);

        if (sampleRate <= 0.0)
            return;

        auto samplesLeft = remainingSamples.load();
        if (samplesLeft <= 0)
            return;

        const auto channels = buffer->getNumChannels();
        const auto phaseDelta = juce::MathConstants<double>::twoPi * frequencyHz / sampleRate;

        for (int i = 0; i < bufferToFill.numSamples && samplesLeft > 0; ++i)
        {
            const auto renderedSamples = totalBeepSamples() - samplesLeft;
            const auto envelope = amplitudeEnvelope (renderedSamples, samplesLeft);
            const auto sample = static_cast<float> (std::sin (phase) * gain * envelope);

            for (int channel = 0; channel < channels; ++channel)
                buffer->addSample (channel, bufferToFill.startSample + i, sample);

            phase += phaseDelta;
            if (phase >= juce::MathConstants<double>::twoPi)
                phase -= juce::MathConstants<double>::twoPi;

            --samplesLeft;
        }

        remainingSamples.store (samplesLeft);
    }

    bool hasFinished() const
    {
        return remainingSamples.load() <= 0;
    }

private:
    int totalBeepSamples() const
    {
        return static_cast<int> (sampleRate * beepSeconds);
    }

    double amplitudeEnvelope (int renderedSamples, int samplesLeft) const
    {
        const auto fadeSamples = static_cast<int> (sampleRate * 0.01);
        if (fadeSamples <= 0)
            return 1.0;

        const auto fadeIn = std::min (1.0, static_cast<double> (renderedSamples) / fadeSamples);
        const auto fadeOut = std::min (1.0, static_cast<double> (samplesLeft) / fadeSamples);
        return std::min (fadeIn, fadeOut);
    }

    static constexpr double frequencyHz = 440.0;
    static constexpr double beepSeconds = 0.35;
    static constexpr double gain = 0.20;

    double sampleRate = 0.0;
    double phase = 0.0;
    std::atomic<int> remainingSamples { 0 };
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

    std::cout << "Beep.\n";

    while (! beep.hasFinished())
        juce::Thread::sleep (10);

    deviceManager.removeAudioCallback (&player);
    player.setSource (nullptr);

    return 0;
}
