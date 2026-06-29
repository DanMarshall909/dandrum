#pragma once

#include <array>
#include <atomic>
#include <cstddef>

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
    bool noteOn (int note, int velocity);
    bool noteOff (int note);
    bool hasFinished() const;

private:
    enum class PendingMidiEventType
    {
        noteOn,
        noteOff
    };

    struct PendingMidiEvent
    {
        PendingMidiEventType type = PendingMidiEventType::noteOff;
        unsigned char note = 0;
        unsigned char velocity = 0;
    };

    bool enqueueMidiEvent (PendingMidiEvent event);
    void drainPendingMidiEvents();

    static constexpr std::size_t pendingMidiCapacity = 256;
    std::array<PendingMidiEvent, pendingMidiCapacity> pendingMidiEvents {};
    std::atomic<std::size_t> pendingMidiReadIndex { 0 };
    std::atomic<std::size_t> pendingMidiWriteIndex { 0 };
    std::atomic<std::size_t> droppedMidiEvents { 0 };

    mutable juce::CriticalSection engineLock;
    DandrumEngine* engine = nullptr;
};
