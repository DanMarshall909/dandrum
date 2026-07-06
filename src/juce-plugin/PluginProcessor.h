#pragma once

#include <atomic>
#include <vector>

#include <juce_audio_processors/juce_audio_processors.h>

#include "RustEngineBindings.h"

struct DandrumPublicParameterDescriptor
{
    juce::String id;
    juce::String name;
    float defaultValue = 0.0f;
    float minValue = 0.0f;
    float maxValue = 1.0f;
};

class DandrumAudioProcessor final : public juce::AudioProcessor,
                                    private juce::AudioProcessorValueTreeState::Listener
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
    bool hasPublicParameter (juce::StringRef parameterId) const;
    const std::vector<DandrumPublicParameterDescriptor>& getPublicParameterDescriptors() const noexcept;
    juce::AudioProcessorValueTreeState& getParameterState() noexcept;
    bool setPublicNumericParameter (juce::StringRef parameterId, float value);
    bool reloadDefaultInstrumentWithMute();

private:
    static juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout (
        const std::vector<DandrumPublicParameterDescriptor>& descriptors);

    bool loadDefaultInstrument();
    void renderSilence (juce::AudioBuffer<float>& buffer) const;
    void parameterChanged (const juce::String& parameterID, float newValue) override;
    void applyStoredParameterValuesToEngine();

    std::vector<DandrumPublicParameterDescriptor> publicParameterDescriptors;
    juce::AudioProcessorValueTreeState parameters;
    DandrumEngine* engine = nullptr;
    bool instrumentLoaded = false;
    std::atomic<bool> mutedForReplacement { false };
    juce::String lastLoadError;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessor)
};