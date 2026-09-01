#pragma once

#include <cstdint>
#include <optional>

#include <juce_audio_processors/juce_audio_processors.h>
#include <juce_gui_extra/juce_gui_extra.h>

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
    juce::WebBrowserComponent::Options createBrowserOptions();
    std::optional<juce::WebBrowserComponent::Resource> provideResource (const juce::String& path) const;
    void setParameterFromWeb (const juce::Array<juce::var>& arguments,
                              juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getParametersForWeb (const juce::Array<juce::var>& arguments,
                              juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    void noteOnFromWeb (const juce::Array<juce::var>& arguments,
                        juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void noteOffFromWeb (const juce::Array<juce::var>& arguments,
                         juce::WebBrowserComponent::NativeFunctionCompletion completion);
    juce::var parameterSnapshotForWeb() const;
    void timerCallback() override;

    DandrumAudioProcessor& processor;
    juce::WebBrowserComponent browser;
    std::uint32_t lastSeenParameterSurfaceGeneration = static_cast<std::uint32_t> (-1);

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessorEditor)
};
