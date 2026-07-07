#pragma once

#include <cstdint>
#include <memory>
#include <vector>

#include <juce_audio_processors/juce_audio_processors.h>

#include "PluginProcessor.h"

class DandrumAudioProcessorEditor final : public juce::AudioProcessorEditor,
                                          private juce::Timer
{
public:
    explicit DandrumAudioProcessorEditor (DandrumAudioProcessor& processorToUse);
    ~DandrumAudioProcessorEditor() override;

    void paint (juce::Graphics& g) override;
    void resized() override;

private:
    struct ParameterControl
    {
        juce::String publicId;
        juce::RangedAudioParameter* parameter = nullptr;
        std::unique_ptr<juce::Label> label;
        std::unique_ptr<juce::Slider> slider;
    };

    void rebuildControlsIfNeeded();
    void rebuildControls();
    void updateStatusLabel();
    void timerCallback() override;

    DandrumAudioProcessor& processor;
    juce::Label statusLabel;
    juce::TextButton loadPresetButton { "Load Preset..." };
    std::unique_ptr<juce::FileChooser> presetChooser;
    std::vector<ParameterControl> parameterControls;
    std::uint32_t lastSeenParameterSurfaceGeneration = static_cast<std::uint32_t> (-1);

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessorEditor)
};
