#pragma once

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
    DandrumAudioProcessor& processor;
    juce::Label statusLabel;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessorEditor)
};
