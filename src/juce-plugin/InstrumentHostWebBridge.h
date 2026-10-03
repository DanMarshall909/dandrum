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
    std::array<NativeFunctionEntry, 27> nativeFunctions();
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
    void getPreparedDocumentForWeb (const juce::Array<juce::var>& arguments,
                                    juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    void requestWaveformFromWeb (const juce::Array<juce::var>& arguments,
                                 juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getWaveformJobStatusFromWeb (const juce::Array<juce::var>& arguments,
                                     juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    void cancelWaveformFromWeb (const juce::Array<juce::var>& arguments,
                                juce::WebBrowserComponent::NativeFunctionCompletion completion);
    // Every ready reply carries at most 16 columns; images stay in the renderer.
    static constexpr std::size_t spectralColumnsPerPage = 16;
    void requestSpectrogramFromWeb (const juce::Array<juce::var>& arguments,
                                    juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void getSpectrogramJobStatusFromWeb (const juce::Array<juce::var>& arguments,
                                        juce::WebBrowserComponent::NativeFunctionCompletion completion) const;
    void cancelSpectrogramFromWeb (const juce::Array<juce::var>& arguments,
                                   juce::WebBrowserComponent::NativeFunctionCompletion completion);
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
    void ackMeterClipFromWeb (const juce::Array<juce::var>& arguments,
                             juce::WebBrowserComponent::NativeFunctionCompletion completion);
    void subscribeLiveAnalysisFromWeb (const juce::Array<juce::var>&,
                                      juce::WebBrowserComponent::NativeFunctionCompletion);
    void setLiveAnalysisVisibleFromWeb (const juce::Array<juce::var>&,
                                       juce::WebBrowserComponent::NativeFunctionCompletion);
    void getLiveAnalysisPacketForWeb (const juce::Array<juce::var>&,
                                     juce::WebBrowserComponent::NativeFunctionCompletion);
    void ackLiveAnalysisPacketFromWeb (const juce::Array<juce::var>&,
                                      juce::WebBrowserComponent::NativeFunctionCompletion);
    void unsubscribeLiveAnalysisFromWeb (const juce::Array<juce::var>&,
                                        juce::WebBrowserComponent::NativeFunctionCompletion);
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
