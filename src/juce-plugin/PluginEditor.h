#pragma once

#include <memory>
#include <vector>

#include <juce_audio_processors/juce_audio_processors.h>

#include "PluginProcessor.h"

class DandrumAudioProcessorEditor final : public juce::AudioProcessorEditor
{
public:
    explicit DandrumAudioProcessorEditor (DandrumAudioProcessor& processorToUse);
    ~DandrumAudioProcessorEditor() override;

    void paint (juce::Graphics& g) override;
    void resized() override;

private:
    struct ParameterControl
    {
        juce::RangedAudioParameter* parameter = nullptr;
        std::unique_ptr<juce::Label> label;
        std::unique_ptr<juce::Slider> slider;
    };

    DandrumAudioProcessor& processor;
    juce::Label statusLabel;
    std::vector<ParameterControl> parameterControls;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessorEditor)
};