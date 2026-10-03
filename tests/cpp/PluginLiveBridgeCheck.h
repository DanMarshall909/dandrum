#pragma once

#include "InstrumentHostWebBridge.h"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <thread>
#include <vector>

namespace liveBridgeCheck
{
inline void require (bool value, const char* message)
{
    if (! value) throw std::runtime_error (message);
}
inline juce::var invoke (InstrumentHostWebBridge& bridge, const char* name,
                         const juce::Array<juce::var>& arguments = {})
{
    const auto functions = bridge.nativeFunctions();
    const auto found = std::find_if (functions.begin(), functions.end(),
        [name] (const auto& entry) { return entry.first == juce::Identifier (name); });
    require (found != functions.end(), "Live analysis command is not registered");
    juce::var result;
    bool completed = false;
    found->second (arguments, [&] (juce::var reply) { result = std::move (reply); completed = true; });
    require (completed, "Live bridge command waited for audio or analysis");
    return result;
}
template<class Predicate> void await (Predicate predicate)
{
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds (3);
    while (! predicate())
    {
        require (std::chrono::steady_clock::now() < deadline, "Live bridge worker did not deliver");
        std::this_thread::sleep_for (std::chrono::milliseconds (1));
    }
}
inline void render (DandrumAudioProcessor& processor, int frames, float expected)
{
    juce::AudioBuffer<float> audio (2, frames);
    juce::MidiBuffer midi;
    audio.clear();
    processor.processBlock (audio, midi);
    for (int n = 0; n < frames; ++n)
        require (std::abs (audio.getSample (0, n) - expected) < 0.000001f
                    && std::abs (audio.getSample (1, n)) < 0.000001f,
                 "Live bridge changed the real signed audio fixture");
}
inline void assertPacket (const juce::var& packet, std::uint32_t generation,
                          std::uint64_t start, float expected)
{
    const auto field = [&] (const char* name) { return packet.getProperty (name, {}); };
    require (packet.isObject() && static_cast<int> (field ("generation")) == static_cast<int> (generation)
                && field ("bus").toString() == "master"
                && static_cast<int> (field ("sampleRateHz")) == 96000
                && field ("startFrame").toString() == juce::String (std::to_string (start))
                && field ("endFrame").toString() == juce::String (std::to_string (start + 1024))
                && static_cast<int> (field ("channelMask")) == 3,
             "Live bridge lost output stream identity, actual rate or frame bounds");
    for (const auto* id : { "sequence", "streamId", "selectionId", "captureSequence", "startFrame", "endFrame" })
        require (field (id).isString(), "Live bridge rounded a 64-bit identity or position");
    const auto settings = field ("settings");
    require (settings.getProperty ("window", {}).toString() == "periodicHann"
                && settings.getProperty ("scaling", {}).toString() == "oneSidedPeakDbFS"
                && settings.getProperty ("channelPolicy", {}).toString() == "selectedChannel"
                && static_cast<int> (settings.getProperty ("fftSize", {})) == 1024
                && settings.getProperty ("hopFrames", {}).toString() == "256"
                && std::abs (static_cast<double> (settings.getProperty ("floorDbFS", {})) + 120.0) < 0.000001,
             "Live bridge lost declared measurement settings");
    const auto frequencyValue = field ("frequencyHz");
    const auto* frequency = frequencyValue.getArray();
    const auto channelValue = field ("channels");
    const auto* channels = channelValue.getArray();
    require (frequency && frequency->size() == 513 && channels && channels->size() == 2
                && std::abs (static_cast<double> ((*frequency)[64]) - 6000.0) < 0.000001,
             "Live bridge exceeded or lost its bounded numeric packet");
    for (int channel = 0; channel < 2; ++channel)
    {
        const auto& data = (*channels)[channel];
        const auto scopeValue = data.getProperty ("scope", {});
        const auto magnitudeValue = data.getProperty ("magnitudeDbFS", {});
        const auto* scope = scopeValue.getArray();
        const auto* magnitudes = magnitudeValue.getArray();
        require (static_cast<int> (data.getProperty ("channel", {})) == channel
                    && scope && scope->size() == 128 && magnitudes && magnitudes->size() == 513,
                 "Live bridge swapped channels or lost numeric bounds");
        const auto signedValue = channel == 0 ? expected : 0.0f;
        for (int n = 0; n < 128; ++n)
        {
            const auto& bucket = (*scope)[n];
            const auto begin = start + static_cast<std::uint64_t> (n) * 8;
            require (bucket.getProperty ("startFrame", {}).toString() == juce::String (std::to_string (begin))
                        && bucket.getProperty ("endFrame", {}).toString() == juce::String (std::to_string (begin + 8))
                        && std::abs (static_cast<double> (bucket.getProperty ("minimum", {})) - signedValue) < 0.000001
                        && std::abs (static_cast<double> (bucket.getProperty ("maximum", {})) - signedValue) < 0.000001,
                     "Live bridge changed literal signed scope or bucket coordinates");
        }
        const auto dc = channel == 0 ? 20.0 * std::log10 (std::abs (expected)) : -120.0;
        require (std::abs (static_cast<double> ((*magnitudes)[0]) - dc) < 0.0002,
                 "Live bridge changed independently known DC magnitude");
    }
}
inline int run (bool baseline)
{
    try
    {
        auto configuration = InstrumentDemoConfiguration::sampler();
        configuration.instrumentPath = std::filesystem::path (DANDRUM_SOURCE_ROOT) / "tests/fixtures/plugin-ui-knob.yaml";
        DandrumAudioProcessor processor (configuration);
        require (processor.isInstrumentLoaded(), "Live bridge fixture did not load");
        processor.setFileWatchEnabled (false);
        processor.setRateAndBufferSizeDetails (96000, 64);
        processor.prepareToPlay (96000, 64);
        const auto generation = processor.getParameterSurfaceGeneration();
        processor.getParameterForPublicId ("fixture.level")->setValueNotifyingHost (0.75f);
        render (processor, 64, 0.5f);
        const auto nativeSession = processor.uiCommands().createSession();
        require (processor.subscribeLiveAnalysis (nativeSession, generation, 3), "Native live admission failed");
        InstrumentHostWebBridge bridge (processor);
        require (static_cast<bool> (invoke (bridge, "subscribeMeter", { static_cast<int> (generation) })),
                 "Existing bridge meter admission regressed");
        if (! baseline)
        {
            for (const auto& args : std::vector<juce::Array<juce::var>> {
                     {}, { 0, 3 }, { static_cast<int> (generation - 1), 3 },
                     { static_cast<int> (generation), 0 }, { static_cast<int> (generation), 4 },
                     { static_cast<int> (generation), 1.5 }, { static_cast<int> (generation), "3" },
                     { 1.5, 3 }, { "1", 3 } })
                require (! static_cast<bool> (invoke (bridge, "subscribeLiveAnalysis", args)),
                         "Live bridge admitted malformed/stale selection");
            require (static_cast<bool> (invoke (bridge, "subscribeLiveAnalysis", { static_cast<int> (generation), 3 }))
                        && ! static_cast<bool> (invoke (bridge, "subscribeLiveAnalysis", { static_cast<int> (generation), 3 })),
                     "Live bridge did not enforce one admitted subscription");
        }
        render (processor, 1024, 0.5f);
        std::optional<InstrumentUiLiveService::Packet> native;
        await ([&] { native = processor.takeLiveAnalysisPacket (nativeSession); return native.has_value(); });
        require (native->analysis.startFrame == 64 && native->analysis.endFrame == 1088
                    && native->analysis.sampleRateHz == 96000
                    && std::abs (native->analysis.channel[0].scope[0].minimum - 0.5f) < 0.000001f,
                 "Native live fixture lost original signed measurement");
        if (baseline)
        {
            require (processor.unsubscribeLiveAnalysis (nativeSession), "Native baseline teardown failed");
            std::cout << "LIVE_BRIDGE_BASELINE PASS\n";
            return 0;
        }
        // A current payload is waiting: these rejects must not consume it.
        for (const auto& args : std::vector<juce::Array<juce::var>> {
                 {}, { "1" }, { 1.5 }, { static_cast<int> (generation - 1) } })
        {
            require (invoke (bridge, "getLiveAnalysisPacket", args).isVoid(),
                     "Live bridge admitted a malformed/stale packet request");
            require (! static_cast<bool> (invoke (bridge, "unsubscribeLiveAnalysis", args)),
                     "Live bridge let a stale request close current demand");
        }
        const auto first = invoke (bridge, "getLiveAnalysisPacket", { static_cast<int> (generation) });
        assertPacket (first, generation, 64, 0.5f);
        require (static_cast<bool> (first.getProperty ("gap", {})), "Live bridge lost the initial gap");
        const auto sequence = first.getProperty ("sequence", {});
        for (const auto& args : std::vector<juce::Array<juce::var>> {
                 {}, { false }, { false, static_cast<int> (generation - 1) },
                 { "false", static_cast<int> (generation) }, { false, "1" } })
            require (! static_cast<bool> (invoke (bridge, "setLiveAnalysisVisible", args)),
                     "Live bridge admitted malformed/stale visibility");
        for (const auto& args : std::vector<juce::Array<juce::var>> {
                 {}, { sequence }, { 1, static_cast<int> (generation) },
                 { "0", static_cast<int> (generation) }, { "18446744073709551616", static_cast<int> (generation) },
                 { sequence, static_cast<int> (generation - 1) }, { "999999", static_cast<int> (generation) } })
            require (! static_cast<bool> (invoke (bridge, "ackLiveAnalysisPacket", args)),
                     "Live bridge admitted a malformed/stale acknowledgement");
        processor.getParameterForPublicId ("fixture.level")->setValueNotifyingHost (0.25f);
        for (int n = 0; n < 6; ++n)
        {
            const auto consumed = processor.getLiveAnalysisStatistics().consumedChunks;
            render (processor, 1024, -0.5f);
            await ([&] { return processor.getLiveAnalysisStatistics().consumedChunks >= consumed + 4; });
            require (invoke (bridge, "getLiveAnalysisPacket", { static_cast<int> (generation) }).isVoid(),
                     "Stalled browser acknowledgement allowed another live payload");
        }
        require (static_cast<bool> (invoke (bridge, "ackLiveAnalysisPacket", { sequence, static_cast<int> (generation) })),
                 "Live bridge rejected exact current acknowledgement");
        const auto recent = invoke (bridge, "getLiveAnalysisPacket", { static_cast<int> (generation) });
        assertPacket (recent, generation, 6208, -0.5f);
        assertPacket (first, generation, 64, 0.5f);
        require (static_cast<bool> (invoke (bridge, "setLiveAnalysisVisible", { false, static_cast<int> (generation) })),
                 "Live bridge could not hide its view");
        require (processor.unsubscribeLiveAnalysis (nativeSession), "Native live teardown failed");
        render (processor, 256, -0.5f);
        require (invoke (bridge, "getLiveAnalysisPacket", { static_cast<int> (generation) }).isVoid(),
                 "Hidden live view retained delivery");
        require (static_cast<bool> (invoke (bridge, "setLiveAnalysisVisible", { true, static_cast<int> (generation) })),
                 "Live bridge could not resume its view");
        render (processor, 1024, -0.5f);
        juce::var resumed;
        await ([&] { resumed = invoke (bridge, "getLiveAnalysisPacket", { static_cast<int> (generation) }); return resumed.isObject(); });
        assertPacket (resumed, generation, 7488, -0.5f);
        require (static_cast<bool> (resumed.getProperty ("gap", {})), "Live bridge resumed across a hidden gap");
        require (static_cast<bool> (invoke (bridge, "unsubscribeLiveAnalysis", { static_cast<int> (generation) }))
                    && ! static_cast<bool> (invoke (bridge, "unsubscribeLiveAnalysis", { static_cast<int> (generation) })),
                 "Live bridge retained a closed subscription");
        // Actual bridge destruction must release slots; there are only four.
        for (int n = 0; n < 8; ++n)
        {
            InstrumentHostWebBridge closing (processor);
            require (static_cast<bool> (invoke (closing, "subscribeLiveAnalysis", { static_cast<int> (generation), 1 })),
                     "Closed live bridge leaked a fixed subscription slot");
            if (n == 0)
            {
                render (processor, 1024, -0.5f);
                juce::var mono;
                await ([&] { mono = invoke (closing, "getLiveAnalysisPacket", { static_cast<int> (generation) }); return mono.isObject(); });
                const auto channels = mono.getProperty ("channels", {});
                require (static_cast<int> (mono.getProperty ("channelMask", {})) == 1
                            && channels.getArray() && channels.getArray()->size() == 1
                            && static_cast<int> ((*channels.getArray())[0].getProperty ("channel", {})) == 0,
                         "Live bridge published an unsubscribed channel");
            }
        }
        std::cout << "LIVE_BRIDGE signed_native_web=PASS bounded_ack=PASS hidden_resume=PASS teardown=PASS\n";
        return 0;
    }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
}
