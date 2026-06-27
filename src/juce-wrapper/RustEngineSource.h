#pragma once

#include <juce_audio_basics/juce_audio_basics.h>
#include <juce_core/juce_core.h>

#include "RustEngineBindings.h"

class RustEngineSource final : public juce::AudioSource
{
public:
    RustEngineSource();
    ~RustEngineSource() override;

    void prepareToPlay (int samplesPerBlockExpected, double newSampleRate) override;
    void releaseResources() override;
    void getNextAudioBlock (const juce::AudioSourceChannelInfo& bufferToFill) override;

    bool loadPatch (const juce::String& yamlPath);
    void noteOn (int note, int velocity);
    void noteOff (int note);
    bool hasFinished() const;

private:
    mutable juce::CriticalSection engineLock;
    DandrumEngine* engine = nullptr;
};
