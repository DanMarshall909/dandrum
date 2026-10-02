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
    ~InstrumentHostWebBridge();

    static const char* bootstrapScript() noexcept;
    std::array<NativeFunctionEntry, 14> nativeFunctions();
    juce::WebBrowserComponent::Options addNativeFunctions (
        juce::WebBrowserComponent::Options options);
    std::optional<juce::WebBrowserComponent::Resource> provideResource (
        const juce::String& path, const std::string& pageHtml) const;
    bool publishParameterUpdates (juce::WebBrowserComponent& browser);

    void setParameterFromWeb (const juce::Array<juce::var>& arguments,
                              juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void beginGestureFromWeb (const juce::Array<juce::var>& arguments,
                              juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void endGestureFromWeb (const juce::Array<juce::var>& arguments,
                            juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getParametersForWeb (const juce::Array<juce::var>& arguments,
                              juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    void getParameterStateForWeb (const juce::Array<juce::var>& arguments,
                                  juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    void noteOnFromWeb (const juce::Array<juce::var>& arguments,
                        juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void noteOffFromWeb (const juce::Array<juce::var>& arguments,
                         juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void noteHeartbeatFromWeb (const juce::Array<juce::var>& arguments,
                               juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void reloadInstrumentFromWeb (const juce::Array<juce::var>& arguments,
                                  juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getUiJobStatusFromWeb (const juce::Array<juce::var>& arguments,
                                juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void subscribeMeterFromWeb (const juce::Array<juce::var>& arguments,
                                juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void setMeterVisibleFromWeb (const juce::Array<juce::var>& arguments,
                                 juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getMeterPacketForWeb (const juce::Array<juce::var>& arguments,
                               juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void ackMeterPacketFromWeb (const juce::Array<juce::var>& arguments,
                                juce::WebBrowserComponent::NativeFunctionCompletion completion);
    bool expireNoteSession (double nowMilliseconds) noexcept;
    juce::var parameterSnapshotForWeb() const;
    juce::var parameterStateForWeb() const;

    std::uint32_t lastSeenSurfaceGeneration() const noexcept;

private:
    DandrumAudioProcessor& processor;
    std::uint32_t lastSeenParameterSurfaceGeneration;
    std::uint64_t sessionId;
    double lastNoteHeartbeatMilliseconds = 0.0;
    bool noteSessionActive = false;
};
