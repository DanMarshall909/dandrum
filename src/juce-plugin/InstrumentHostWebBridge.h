#pragma once

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>

#include <juce_gui_extra/juce_gui_extra.h>

#include "PluginProcessor.h"

class InstrumentHostWebBridge final
{
public:
    using NativeFunctionEntry = std::pair<juce::Identifier, juce::WebBrowserComponent::NativeFunction>;
    explicit InstrumentHostWebBridge (DandrumAudioProcessor& processorToUse);

    static const char* bootstrapScript() noexcept;
    std::array<NativeFunctionEntry, 4> nativeFunctions();
    juce::WebBrowserComponent::Options addNativeFunctions (
        juce::WebBrowserComponent::Options options);
    std::optional<juce::WebBrowserComponent::Resource> provideResource (
        const juce::String& path, const std::string& pageHtml) const;
    bool publishParameterUpdates (juce::WebBrowserComponent& browser);

    void setParameterFromWeb (const juce::Array<juce::var>& arguments,
                              juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getParametersForWeb (const juce::Array<juce::var>& arguments,
                              juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    void noteOnFromWeb (const juce::Array<juce::var>& arguments,
                        juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void noteOffFromWeb (const juce::Array<juce::var>& arguments,
                         juce::WebBrowserComponent::NativeFunctionCompletion completion);
    juce::var parameterSnapshotForWeb() const;

    std::uint32_t lastSeenSurfaceGeneration() const noexcept;

private:
    DandrumAudioProcessor& processor;
    std::uint32_t lastSeenParameterSurfaceGeneration;
};
