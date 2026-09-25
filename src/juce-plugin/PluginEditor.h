#pragma once

#include <cstdint>
#include <memory>
#include <optional>

#include <juce_audio_processors/juce_audio_processors.h>
#include <juce_gui_extra/juce_gui_extra.h>

#include "PluginProcessor.h"
#include "SoundLabController.h"

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
    void renderSoundLabFromWeb (const juce::Array<juce::var>& arguments,
                                juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void chooseSoundLabReferenceFromWeb (
        const juce::Array<juce::var>& arguments,
        juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void matchSoundLabFromWeb (const juce::Array<juce::var>& arguments,
                               juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void cancelSoundLabFromWeb (const juce::Array<juce::var>& arguments,
                                juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void acceptSoundLabMatchFromWeb (
        const juce::Array<juce::var>& arguments,
        juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void requestGraphProposalFromWeb (
        const juce::Array<juce::var>& arguments,
        juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getSoundLabAnalysisForWeb (const juce::Array<juce::var>& arguments,
                                    juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    juce::var parameterSnapshotForWeb() const;
    juce::var soundLabSnapshotForWeb() const;
    void timerCallback() override;

    DandrumAudioProcessor& processor;
    // Declared before the browser so it outlives browser-owned native callbacks.
    SoundLabController soundLabController;
    juce::File soundLabReferenceFile;
    std::unique_ptr<juce::FileChooser> soundLabFileChooser;
    juce::WebBrowserComponent browser;
    std::uint32_t lastSeenParameterSurfaceGeneration = static_cast<std::uint32_t> (-1);
    std::uint64_t lastSeenSoundLabGeneration = static_cast<std::uint64_t> (-1);

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessorEditor)
};
