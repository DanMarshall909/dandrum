#pragma once

#include <juce_audio_processors/juce_audio_processors.h>

#include "RustEngineBindings.h"

class DandrumAudioProcessor final : public juce::AudioProcessor
{
public:
    DandrumAudioProcessor();
    ~DandrumAudioProcessor() override;

    using juce::AudioProcessor::processBlock;

    void prepareToPlay (double sampleRate, int samplesPerBlock) override;
    void releaseResources() override;
    bool isBusesLayoutSupported (const BusesLayout& layouts) const override;
    void processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages) override;

    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override;

    const juce::String getName() const override;
    bool acceptsMidi() const override;
    bool producesMidi() const override;
    double getTailLengthSeconds() const override;

    int getNumPrograms() override;
    int getCurrentProgram() override;
    void setCurrentProgram (int index) override;
    const juce::String getProgramName (int index) override;
    void changeProgramName (int index, const juce::String& newName) override;

    void getStateInformation (juce::MemoryBlock& destData) override;
    void setStateInformation (const void* data, int sizeInBytes) override;

    bool isInstrumentLoaded() const noexcept;
    const juce::String& getLastLoadError() const noexcept;

private:
    bool loadDefaultInstrument();
    void renderSilence (juce::AudioBuffer<float>& buffer) const;

    DandrumEngine* engine = nullptr;
    bool instrumentLoaded = false;
    juce::String lastLoadError;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessor)
};
