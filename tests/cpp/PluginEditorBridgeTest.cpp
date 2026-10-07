#include "PluginEditor.h"
#include "PluginWebRuntimeCheck.h"
#include "PluginLiveBridgeCheck.h"
#include "PluginParameterPublicationCheck.h"

#include <algorithm>
#include <array>
#include <bit>
#include <atomic>
#include <chrono>
#include <cmath>
#include <csignal>
#include <fstream>
#include <iostream>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>

#if JUCE_LINUX
// Exercise the documented bool read-failure boundary through the real worker
// and bridge; successful calls still read the independently retained Rust PCM.
static std::atomic<bool> rejectSpectralRead { false };
extern "C" bool __real_dandrum_kernel_prepared_source_copy_channel (
    const DandrumKernelWaveformSource*, std::uint16_t, std::uint64_t, float*, std::size_t);
extern "C" bool __wrap_dandrum_kernel_prepared_source_copy_channel (
    const DandrumKernelWaveformSource* source, std::uint16_t channel,
    std::uint64_t start, float* output, std::size_t count)
{
    return ! rejectSpectralRead.load()
        && __real_dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
}
#endif

struct PluginEditorBridgeTestProbe
{
    static juce::WebBrowserComponent& runtimeBrowser (DandrumAudioProcessorEditor& editor)
    {
        return editor.browser;
    }
    static juce::var invoke (DandrumAudioProcessorEditor& editor,
                             const juce::String& command,
                             const juce::Array<juce::var>& arguments = {})
    {
        const auto options = editor.createBrowserOptions();
        const auto& functions = options.getNativeFunctions();
        const auto found = functions.find (juce::Identifier (command));
        if (found == functions.end())
            throw std::runtime_error ("browser command is not registered: " + command.toStdString());
        juce::var result;
        bool completed = false;
        auto completion = [&] (juce::var value) { result = std::move (value); completed = true; };
        found->second (arguments, completion);
        if (! completed)
            throw std::runtime_error ("native command did not complete synchronously");
        return result;
    }

    static bool hasCommand (DandrumAudioProcessorEditor& editor, const juce::String& command)
    {
        return editor.createBrowserOptions().getNativeFunctions().count (juce::Identifier (command)) == 1;
    }

    static void refresh (DandrumAudioProcessorEditor& editor) { editor.timerCallback(); }
    static bool expireNotesAfterDisconnect (DandrumAudioProcessorEditor& editor)
    {
        return editor.hostBridge.expireNoteSession (std::numeric_limits<double>::max());
    }
    static bool expireNotesBeforeDeadline (DandrumAudioProcessorEditor& editor)
    {
        return editor.hostBridge.expireNoteSession (
            juce::Time::getMillisecondCounterHiRes() + 1000.0);
    }

    static bool publishUnchangedSurface (DandrumAudioProcessorEditor& editor)
    {
        return editor.hostBridge.publishParameterUpdates (editor.browser);
    }

    static std::uint32_t seenSurfaceGeneration (const DandrumAudioProcessorEditor& editor)
    {
        return editor.hostBridge.lastSeenSurfaceGeneration();
    }

    static std::optional<juce::WebBrowserComponent::Resource> resource (
        const DandrumAudioProcessorEditor& editor, const juce::String& path)
    {
        return editor.provideResource (path);
    }

    static bool hasSoundLab (const DandrumAudioProcessorEditor& editor)
    {
        return editor.soundLabController != nullptr;
    }

    static void setReferenceFile (DandrumAudioProcessorEditor& editor, juce::File file)
    {
        editor.soundLabReferenceFile = std::move (file);
    }

    static juce::String matchedPatch (const DandrumAudioProcessorEditor& editor)
    {
        const auto snapshot = editor.soundLabController->snapshot();
        return snapshot.match != nullptr ? juce::String (snapshot.match->patchYaml) : juce::String();
    }
};

namespace
{
void require (bool condition, const std::string& message)
{
    if (! condition)
        throw std::runtime_error (message);
}

struct HostListener final : juce::AudioProcessorListener
{
    void audioProcessorParameterChanged (juce::AudioProcessor*, int index, float value) override
    {
        lastIndex = index;
        lastValue = value;
        ++changeCount;
    }
    void audioProcessorChanged (juce::AudioProcessor*, const ChangeDetails&) override {}
    void audioProcessorParameterChangeGestureBegin (juce::AudioProcessor*, int index) override
    {
        lastGestureIndex = index;
        ++beginCount;
    }
    void audioProcessorParameterChangeGestureEnd (juce::AudioProcessor*, int index) override
    {
        lastGestureIndex = index;
        ++endCount;
    }
    int changeCount = 0;
    int lastIndex = -1;
    float lastValue = -1.0f;
    int beginCount = 0;
    int endCount = 0;
    int lastGestureIndex = -1;
};

struct SnapshotListener final : juce::AudioProcessorListener
{
    explicit SnapshotListener (DandrumAudioProcessor& source) : processor (source) {}
    void audioProcessorParameterChanged (juce::AudioProcessor*, int, float) override
    {
        sawDocument = processor.getPreparedUiDocument().has_value();
    }
    void audioProcessorChanged (juce::AudioProcessor*, const ChangeDetails&) override {}
    DandrumAudioProcessor& processor;
    bool sawDocument = false;
};

juce::var findParameter (const juce::var& snapshot, const juce::String& id)
{
    const auto* items = snapshot.getArray();
    if (items == nullptr)
        return {};
    for (const auto& item : *items)
        if (item.getProperty ("id", {}).toString() == id)
            return item;
    return {};
}
}

#include "PluginReactLayoutCheck.h"

int main (int argc, char** argv)
{
    if (argc >= 2 && juce::String (argv[1]) == "--live-bridge")
        return liveBridgeCheck::run (false);
    if (argc >= 2 && juce::String (argv[1]) == "--live-bridge-baseline")
        return liveBridgeCheck::run (true);
   #if JUCE_LINUX
    std::signal (SIGPIPE, SIG_IGN);
   #endif

    if (argc >= 2 && juce::String (argv[1]) == "--react-layout")
        return reactLayoutCheck::main<PluginEditorBridgeTestProbe> (argc, argv);

    if (argc >= 2 && juce::String (argv[1]) == "--parameter-publication")
        return parameterPublicationCheck::run<PluginEditorBridgeTestProbe>();

    if (argc >= 2 && (juce::String (argv[1]) == "--web-runtime"
                     || juce::String (argv[1]) == "--juce-gtkwebkitfork-child"))
        return packagedWebRuntime::main<false, PluginEditorBridgeTestProbe> (argc, argv);

    try
    {
        DandrumAudioProcessor processor (InstrumentDemoConfiguration::kick());
        processor.setPlayConfigDetails (0, 2, 48000.0, 64);
        processor.prepareToPlay (48000.0, 64);
        HostListener listener;
        processor.addListener (&listener);
        DandrumAudioProcessorEditor editor (processor);

        for (const auto* command : { "getParameters", "getParameterState", "getPreparedDocument", "setParameter",
                                     "beginGesture", "endGesture",
                                     "requestWaveform", "getWaveformJobStatus", "cancelWaveform",
                                     "requestSpectrogram", "getSpectrogramJobStatus", "cancelSpectrogram",
                                     "noteOn", "noteOff",
                                     "subscribeMeter", "setMeterVisible", "getMeterPacket",
                                     "ackMeterPacket", "ackMeterClip",
                                     "renderSoundLab", "chooseSoundLabReference", "matchSoundLab",
                                     "cancelSoundLab", "acceptSoundLabMatch", "requestGraphProposal",
                                     "getSoundLabAnalysis" })
            require (PluginEditorBridgeTestProbe::hasCommand (editor, command),
                     std::string ("configured editor did not register ") + command);

        const auto meterGeneration = processor.getParameterSurfaceGeneration();
        require (static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "subscribeMeter", { static_cast<int> (meterGeneration) })),
                 "web meter subscription was rejected");
        juce::AudioBuffer<float> meterBuffer (2, 64);
        juce::MidiBuffer emptyMidi;
        processor.processBlock (meterBuffer, emptyMidi);
        PluginEditorBridgeTestProbe::refresh (editor);
        const auto meterPacket = PluginEditorBridgeTestProbe::invoke (editor, "getMeterPacket");
        require (meterPacket.isObject()
                     && static_cast<int> (meterPacket.getProperty ("generation", {}))
                            == static_cast<int> (meterGeneration)
                     && static_cast<int> (meterPacket.getProperty ("observed_samples", {})) == 64
                     && meterPacket.getProperty ("sequence", {}).isString()
                     && meterPacket.getProperty ("stream_id", {}).isString()
                     && meterPacket.getProperty ("end_sample", {}).isString()
                     && meterPacket.getProperty ("peak", {}).getArray() != nullptr
                     && meterPacket.getProperty ("rms", {}).getArray() != nullptr
                     && meterPacket.getProperty ("clipped", {}).getArray() != nullptr,
                 "web meter packet lost generation, interval or exact sequence");
        require (meterPacket.getProperty ("display_peak", {}).getArray() != nullptr
                     && meterPacket.getProperty ("display_rms", {}).getArray() != nullptr
                     && meterPacket.getProperty ("display_clipped", {}).getArray() != nullptr
                     && meterPacket.getProperty ("display_complete", {}).isBool()
                     && meterPacket.getProperty ("display_valid", {}).isBool(),
                 "web meter packet omitted shared display values");
        require (PluginEditorBridgeTestProbe::invoke (editor, "getMeterPacket").isVoid(),
                 "web meter sent a second unacknowledged packet");
        require (! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "ackMeterPacket", { "bad", static_cast<int> (meterGeneration) })),
                 "malformed web meter acknowledgement was accepted");
        require (! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "ackMeterClip", { 0, static_cast<int> (meterGeneration), "0" })),
                 "web meter accepted acknowledgement without a clip occurrence");
        require (! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "ackMeterPacket", { 9007199254740993.0,
                                                  static_cast<int> (meterGeneration) })),
                 "floating-point meter sequence was accepted with lost precision");
        require (! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "ackMeterPacket",
                     { meterPacket.getProperty ("sequence", {}),
                       static_cast<int> (meterGeneration + 1) })),
                 "stale web meter generation acknowledged a packet");
        require (static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "ackMeterPacket",
                     { meterPacket.getProperty ("sequence", {}), static_cast<int> (meterGeneration) })),
                 "web meter acknowledgement failed");
        require (static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "setMeterVisible", { false, static_cast<int> (meterGeneration) })),
                 "web meter hide was rejected");
        processor.processBlock (meterBuffer, emptyMidi);
        PluginEditorBridgeTestProbe::refresh (editor);
        require (PluginEditorBridgeTestProbe::invoke (editor, "getMeterPacket").isVoid(),
                 "hidden web meter retained a visual packet");
        require (static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                     editor, "setMeterVisible", { true, static_cast<int> (meterGeneration) })),
                 "web meter reopen was rejected");
        processor.processBlock (meterBuffer, emptyMidi);
        PluginEditorBridgeTestProbe::refresh (editor);
        require (PluginEditorBridgeTestProbe::invoke (editor, "getMeterPacket").isObject(),
                 "reopened web meter did not receive current data");

        auto snapshot = PluginEditorBridgeTestProbe::invoke (editor, "getParameters");
        const auto defaultIds = processor.getActivePublicParameterIds();
        require (snapshot.getArray() != nullptr
                     && snapshot.getArray()->size() == defaultIds.size(),
                 "getParameters did not return the active surface");
        for (const auto& id : defaultIds)
        {
            const auto item = findParameter (snapshot, id);
            require (item.isObject() && item.getProperty ("name", {}).toString()
                                            == processor.getPublicParameterDisplayName (id),
                     "parameter snapshot lost public identity or display name");
        }

        const auto kickDocument = PluginEditorBridgeTestProbe::invoke (editor, "getPreparedDocument");
        const auto kickDefaults = kickDocument.getProperty ("parameters", {});
        for (const auto& [id, expected] : std::array {
                 std::pair { "kick.tune_hz", 0.28f },
                 std::pair { "kick.decay_ms", 600.0f / 1950.0f },
                 std::pair { "kick.punch", 0.7f },
                 std::pair { "kick.click", 1.0f } })
        {
            const auto prepared = findParameter (kickDefaults, id);
            require (prepared.getProperty ("normalisedDefaultValue", {}).isDouble()
                         && std::abs (static_cast<float> (prepared.getProperty (
                             "normalisedDefaultValue", {})) - expected) < 0.00001f,
                     std::string ("prepared reset default does not match loaded kick control: ") + id);
            require (std::abs (processor.getParameterForPublicId (id)->getDefaultValue()) < 0.00001f,
                     "UI reset metadata changed the stable host slot's default");
        }

        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter", { "kick.tune_hz" })
                     .toString().contains ("expects"),
                 "setParameter accepted missing value");
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter", { juce::var ("missing.id"), juce::var (0.5) })
                     .toString().contains ("Unknown public parameter"),
                 "setParameter accepted unknown public id");
        auto* parameter = processor.getParameterForPublicId ("kick.tune_hz");
        require (parameter != nullptr, "default instrument is missing kick.tune_hz");
        const auto parameterIndex = processor.getParameters().indexOf (parameter);
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter", { juce::var ("kick.tune_hz"), juce::var (0.37) }).isVoid(),
                 "setParameter rejected active public id");
        require (std::abs (parameter->getValue() - 0.37f) < 0.00001f
                     && listener.changeCount == 1 && listener.lastIndex == parameterIndex
                     && std::abs (listener.lastValue - 0.37f) < 0.00001f,
                 "setParameter failed to notify the stable host slot");
        require (listener.beginCount == 1 && listener.endCount == 1
                     && listener.lastGestureIndex == parameterIndex,
                 "current setParameter call did not bracket its own host gesture");
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                     { juce::var ("kick.tune_hz"), juce::var (0.38) }).isVoid()
                     && listener.beginCount == 2 && listener.endCount == 2,
                 "current bridge did not create a separate gesture for each parameter update");
        const auto commandGeneration = processor.getParameterSurfaceGeneration();
        const auto acceptedChanges = listener.changeCount;
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                     { juce::var ("kick.tune_hz"), juce::var (0.8),
                       juce::var (static_cast<int> (commandGeneration - 1)) })
                     .toString().contains ("stale"),
                 "browser command accepted an obsolete instrument generation");
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                     { juce::var ("kick.tune_hz"),
                       juce::var (std::numeric_limits<double>::quiet_NaN()),
                       juce::var (static_cast<int> (commandGeneration)) })
                     .toString().contains ("finite"),
                 "browser command accepted a non-finite parameter value");
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                     { juce::var ("kick.tune_hz"), juce::var (1.25),
                       juce::var (static_cast<int> (commandGeneration)) })
                     .toString().contains ("range"),
                 "browser command clamped an out-of-range parameter value");
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                     { juce::var ("kick.tune_hz"), juce::var ("0.5"),
                       juce::var (static_cast<int> (commandGeneration)) })
                     .toString().contains ("finite")
                     && PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                         { juce::var ("kick.tune_hz"), juce::var (0.5), juce::var (1.25) })
                         .toString().contains ("generation"),
                 "browser command accepted a coercible value or fractional generation");
        require (listener.changeCount == acceptedChanges
                     && std::abs (parameter->getValue() - 0.38f) < 0.00001f,
                 "rejected browser commands changed the host slot");
        snapshot = PluginEditorBridgeTestProbe::invoke (editor, "getParameters");
        require (std::abs (static_cast<float> (findParameter (snapshot, "kick.tune_hz")
                                                  .getProperty ("value", {})) - 0.38f) < 0.00001f,
                 "getParameters did not reflect the value set through the bridge");
        const auto editedControl = findParameter (PluginEditorBridgeTestProbe::invoke (
            editor, "getPreparedDocument").getProperty ("parameters", {}), "kick.tune_hz");
        require (std::abs (static_cast<float> (editedControl.getProperty (
                     "normalisedValue", -1000.0)) - 0.38f) < 0.00001f
                     && std::abs (static_cast<float> (editedControl.getProperty (
                         "normalisedDefaultValue", -1000.0)) - 0.28f) < 0.00001f,
                 "live parameter editing overwrote the prepared reset default");
        SnapshotListener reentrant (processor);
        processor.addListener (&reentrant);
        const auto nativeResult = processor.uiCommands().setParameter (
            { commandGeneration, "kick.tune_hz", 0.39 });
        const auto webResult = PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
            { juce::var ("kick.tune_hz"), juce::var (0.40),
              juce::var (static_cast<int> (commandGeneration)) });
        require (nativeResult.status == InstrumentUiCommandStatus::accepted
                     && webResult.getProperty ("status", {}).toString() == "accepted"
                     && static_cast<juce::int64> (webResult.getProperty ("sequence", {}))
                            == static_cast<juce::int64> (nativeResult.sequence + 1)
                     && listener.lastIndex == parameterIndex
                     && std::abs (parameter->getValue() - 0.40f) < 0.00001f,
                 "native and Web commands did not share the host slot and admission sequence");
        processor.removeListener (&reentrant);
        require (reentrant.sawDocument,
                 "synchronous host listener could not read a document during parameter admission");
        const auto changesAfterAdmission = listener.changeCount;
        const std::array rejectedResults {
                 processor.uiCommands().setParameter ({ commandGeneration - 1, "kick.tune_hz", 0.5 }),
                 processor.uiCommands().setParameter ({ commandGeneration, "kick.tune_hz",
                     std::numeric_limits<double>::infinity() }),
                 processor.uiCommands().setParameter ({ commandGeneration, "kick.tune_hz", -0.1 }),
                 processor.uiCommands().setParameter ({ commandGeneration, "missing.id", 0.5 }) };
        const std::array expectedStatuses {
            InstrumentUiCommandStatus::staleGeneration,
            InstrumentUiCommandStatus::invalidValue,
            InstrumentUiCommandStatus::invalidValue,
            InstrumentUiCommandStatus::unknownControl };
        for (std::size_t index = 0; index < rejectedResults.size(); ++index)
            require (rejectedResults[index].status == expectedStatuses[index]
                         && rejectedResults[index].sequence == nativeResult.sequence + 1,
                     "native rejection advanced the shared command sequence");
        require (listener.changeCount == changesAfterAdmission,
                 "rejected native command changed the host slot");
        const auto admittedSequence = processor.uiCommands().lastAdmittedSequence();
        processor.removeListener (&listener);
        std::thread hostAutomation ([parameter] { parameter->setValueNotifyingHost (0.42f); });
        hostAutomation.join();
        processor.addListener (&listener);
        const auto observed = processor.getUiParameterState();
        const auto webObserved = PluginEditorBridgeTestProbe::invoke (editor, "getParameterState");
        const auto* webValues = webObserved.getProperty ("parameters", {}).getArray();
        require (observed.generation == commandGeneration
                     && observed.admittedCommandSequence == admittedSequence
                     && std::any_of (observed.parameters.begin(), observed.parameters.end(), [] (const auto& value) {
                         return value.id == "kick.tune_hz"
                             && std::abs (value.normalisedValue - 0.42f) < 0.00001f;
                     })
                     && static_cast<juce::int64> (webObserved.getProperty ("sequence", {}))
                            == static_cast<juce::int64> (admittedSequence)
                     && webValues != nullptr
                     && std::abs (static_cast<float> (findParameter (
                         webObserved.getProperty ("parameters", {}), "kick.tune_hz")
                         .getProperty ("value", {})) - 0.42f) < 0.00001f,
                 "timer-observed host automation did not reach native and Web parameter state");

        const auto nativeSession = processor.uiCommands().createSession();
        const auto beginsBeforeDrag = listener.beginCount;
        const auto endsBeforeDrag = listener.endCount;
        require (processor.uiCommands().beginGesture (
                     { commandGeneration, "missing.id", nativeSession }).status
                     == InstrumentUiCommandStatus::unknownControl,
                 "native gesture accepted an unknown public control");
        const auto nativeBegin = processor.uiCommands().beginGesture (
            { commandGeneration, "kick.tune_hz", nativeSession });
        require (processor.uiCommands().beginGesture (
                     { commandGeneration, "kick.tune_hz", nativeSession }).status
                     == InstrumentUiCommandStatus::gestureActive
                     && processor.uiCommands().setParameter (
                         { commandGeneration, "kick.decay_ms", 0.5, nativeSession }).status
                            == InstrumentUiCommandStatus::gestureActive
                     && processor.uiCommands().endGesture (
                         { commandGeneration, "kick.decay_ms", nativeSession }).status
                            == InstrumentUiCommandStatus::unknownControl,
                 "a second control entered an active editor gesture");
        const auto nativeDragA = processor.uiCommands().setParameter (
            { commandGeneration, "kick.tune_hz", 0.45, nativeSession });
        const auto nativeDragB = processor.uiCommands().setParameter (
            { commandGeneration, "kick.tune_hz", 0.55, nativeSession });
        const auto nativeEnd = processor.uiCommands().endGesture (
            { commandGeneration, "kick.tune_hz", nativeSession });
        require (nativeBegin.status == InstrumentUiCommandStatus::accepted
                     && nativeDragA.status == InstrumentUiCommandStatus::accepted
                     && nativeDragB.status == InstrumentUiCommandStatus::accepted
                     && nativeEnd.status == InstrumentUiCommandStatus::accepted
                     && listener.beginCount == beginsBeforeDrag + 1
                     && listener.endCount == endsBeforeDrag + 1
                     && std::abs (parameter->getValue() - 0.55f) < 0.00001f,
                 "native continuous drag did not preserve one host gesture");
        const auto beginsBeforeWebDrag = listener.beginCount;
        const auto endsBeforeWebDrag = listener.endCount;
        const auto webBegin = PluginEditorBridgeTestProbe::invoke (editor, "beginGesture",
            { juce::var ("kick.tune_hz"), juce::var (static_cast<int> (commandGeneration)) });
        require (PluginEditorBridgeTestProbe::invoke (editor, "beginGesture",
                     { juce::var ("kick.tune_hz"),
                       juce::var (static_cast<int> (commandGeneration)) })
                     .toString().contains ("active"),
                 "Web adapter admitted a duplicate gesture");
        const auto webDragA = PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
            { juce::var ("kick.tune_hz"), juce::var (0.60),
              juce::var (static_cast<int> (commandGeneration)) });
        const auto webDragB = PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
            { juce::var ("kick.tune_hz"), juce::var (0.70),
              juce::var (static_cast<int> (commandGeneration)) });
        const auto webEnd = PluginEditorBridgeTestProbe::invoke (editor, "endGesture",
            { juce::var ("kick.tune_hz"), juce::var (static_cast<int> (commandGeneration)) });
        require (webBegin.getProperty ("status", {}).toString() == "accepted"
                     && webDragA.getProperty ("status", {}).toString() == "accepted"
                     && webDragB.getProperty ("status", {}).toString() == "accepted"
                     && webEnd.getProperty ("status", {}).toString() == "accepted"
                     && listener.beginCount == beginsBeforeWebDrag + 1
                     && listener.endCount == endsBeforeWebDrag + 1
                     && std::abs (parameter->getValue() - 0.70f) < 0.00001f,
                 "Web continuous drag did not preserve one host gesture");
        require (PluginEditorBridgeTestProbe::invoke (editor, "endGesture",
                     { juce::var ("kick.tune_hz"),
                       juce::var (static_cast<int> (commandGeneration)) })
                     .toString().contains ("No active gesture"),
                 "Web adapter accepted a duplicate gesture end");
        require (processor.uiCommands().endGesture (
                     { commandGeneration, "kick.tune_hz", nativeSession }).status
                     == InstrumentUiCommandStatus::noGesture,
                 "duplicate gesture end was accepted");
        const auto beginsBeforeClose = listener.beginCount;
        const auto endsBeforeClose = listener.endCount;
        {
            DandrumAudioProcessorEditor closingEditor (processor);
            require (PluginEditorBridgeTestProbe::invoke (closingEditor, "beginGesture",
                         { juce::var ("kick.tune_hz"),
                           juce::var (static_cast<int> (commandGeneration)) })
                         .getProperty ("status", {}).toString() == "accepted",
                     "editor could not begin a gesture before closing");
        }
        require (listener.beginCount == beginsBeforeClose + 1
                     && listener.endCount == endsBeforeClose + 1,
                 "editor teardown left an open host gesture");

        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36 })
                     .toString().contains ("expects"),
                 "noteOn accepted missing velocity");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOff")
                     .toString().contains ("expects"),
                 "noteOff accepted missing note");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { -1, 0.8 })
                     .toString().contains ("valid")
                     && PluginEditorBridgeTestProbe::invoke (editor, "noteOn",
                         { 36, std::numeric_limits<double>::quiet_NaN() })
                            .toString().contains ("valid")
                     && PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36, 1.5 })
                            .toString().contains ("valid")
                     && PluginEditorBridgeTestProbe::invoke (editor, "noteOff", { "36" })
                            .toString().contains ("valid"),
                 "browser note commands accepted invalid MIDI inputs");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn",
                     { 36, 0.8, static_cast<int> (commandGeneration - 1) })
                         .toString().contains ("stale")
                     && PluginEditorBridgeTestProbe::invoke (editor, "noteOff",
                         { 36, static_cast<int> (commandGeneration - 1) })
                            .toString().contains ("stale"),
                 "browser note commands accepted an obsolete instrument generation");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36, 0.8 }).isVoid()
                     && PluginEditorBridgeTestProbe::invoke (editor, "noteOff", { 36 }).isVoid(),
                 "playable keyboard commands did not publish note intent");
        for (int i = 0; i < 256; ++i)
            require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36, 0.8 }).isVoid(),
                     "bounded editor note intent rejected a same-session update");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOff", { 36 }).isVoid()
                     && processor.getDroppedMidiEventCount() == 0,
                 "editor note release was lost under producer pressure");
        {
            DandrumAudioProcessorEditor closingNoteEditor (processor);
            require (PluginEditorBridgeTestProbe::invoke (
                         closingNoteEditor, "noteOn", { 36, 0.8 }).isVoid(),
                     "second editor could not admit its note");
        }
        juce::AudioBuffer<float> afterEditorClose (2, 64);
        juce::MidiBuffer noHostNotes;
        processor.processBlock (afterEditorClose, noHostNotes);
        for (int channel = 0; channel < afterEditorClose.getNumChannels(); ++channel)
            for (int frame = 0; frame < afterEditorClose.getNumSamples(); ++frame)
                require (std::abs (afterEditorClose.getSample (channel, frame)) < 0.000001f,
                         "closed editor left a note queued for the audio callback");
        require (PluginEditorBridgeTestProbe::hasCommand (editor, "noteHeartbeat"),
                 "browser note session has no keepalive command");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36, 0.8 }).isVoid()
                     && PluginEditorBridgeTestProbe::invoke (editor, "noteHeartbeat").isVoid()
                     && ! PluginEditorBridgeTestProbe::expireNotesBeforeDeadline (editor)
                     && PluginEditorBridgeTestProbe::expireNotesAfterDisconnect (editor),
                 "stalled browser note session did not expire");
        processor.processBlock (afterEditorClose, noHostNotes);
        for (int channel = 0; channel < afterEditorClose.getNumChannels(); ++channel)
            for (int frame = 0; frame < afterEditorClose.getNumSamples(); ++frame)
                require (std::abs (afterEditorClose.getSample (channel, frame)) < 0.000001f,
                         "stalled browser left its note gated after expiry");

        const auto previousGeneration = processor.getParameterSurfaceGeneration();
        const auto reloadSession = processor.uiCommands().createSession();
        require (processor.uiCommands().beginGesture (
                     { previousGeneration, "kick.tune_hz", reloadSession }).status
                     == InstrumentUiCommandStatus::accepted,
                 "could not begin the gesture used to test reload cleanup");
        require (PluginEditorBridgeTestProbe::invoke (editor, "beginGesture",
                     { juce::var ("kick.decay_ms"),
                       juce::var (static_cast<int> (previousGeneration)) })
                     .getProperty ("status", {}).toString() == "accepted",
                 "Web editor could not begin the gesture used to test refresh cleanup");
        const auto endsBeforeReload = listener.endCount;
        const auto tb303 = juce::File (juce::String (
            InstrumentDemoConfiguration::tb303().instrumentPath.string()));
        require (processor.reloadInstrumentFromFile (tb303),
                 "could not reload distinct instrument for browser refresh test");
        require (processor.getParameterSurfaceGeneration() != previousGeneration,
                 "replacement did not advance public surface generation");
        require (processor.uiCommands().endGesture (
                     { previousGeneration, "kick.tune_hz", reloadSession }).status
                     == InstrumentUiCommandStatus::staleGeneration
                     && listener.endCount == endsBeforeReload + 1,
                 "gesture did not close its original stable host slot after reload");
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                     { juce::var (processor.getActivePublicParameterIds()[0]), juce::var (0.9),
                       juce::var (static_cast<int> (previousGeneration)) })
                     .toString().contains ("stale"),
                 "delayed browser command from the old instrument changed a replacement slot");
        PluginEditorBridgeTestProbe::refresh (editor);
        require (listener.endCount == endsBeforeReload + 2,
                 "browser refresh did not end its old-generation host gesture");
        require (PluginEditorBridgeTestProbe::seenSurfaceGeneration (editor)
                     == processor.getParameterSurfaceGeneration(),
                 "browser refresh did not consume the new public surface generation");
        require (! PluginEditorBridgeTestProbe::publishUnchangedSurface (editor),
                 "unchanged public surface caused another browser refresh");
        snapshot = PluginEditorBridgeTestProbe::invoke (editor, "getParameters");
        require (snapshot.getArray() != nullptr
                     && snapshot.getArray()->size() == processor.getActivePublicParameterIds().size()
                     && ! processor.getActivePublicParameterIds().isEmpty()
                     && findParameter (snapshot, processor.getActivePublicParameterIds()[0]).isObject()
                     && ! findParameter (snapshot, "kick.tune_hz").isObject(),
                 "browser refresh did not expose replacement instrument controls");

        require (PluginEditorBridgeTestProbe::invoke (editor, "renderSoundLab")
                     .toString().contains ("does not match"),
                 "Sound Lab rendered the kick fixture after the active instrument changed to TB-303");
        require (PluginEditorBridgeTestProbe::invoke (editor, "matchSoundLab")
                     .toString().contains ("does not match"),
                 "Sound Lab matched the kick fixture after the active instrument changed to TB-303");
        require (processor.reloadInstrumentFromFile (
                     juce::File (juce::String (InstrumentDemoConfiguration::kick().instrumentPath.string()))),
                 "could not restore kick before its Sound Lab render");
        PluginEditorBridgeTestProbe::refresh (editor);

        const auto html = PluginEditorBridgeTestProbe::resource (editor, "/index.html");
        require (html.has_value() && html->mimeType == "text/html" && ! html->data.empty(),
                 "editor failed to serve its configured page");
        require (! PluginEditorBridgeTestProbe::resource (editor, "/sound-lab.wav?generation=0"),
                 "editor served Sound Lab audio before a render");
        require (PluginEditorBridgeTestProbe::invoke (editor, "renderSoundLab")
                     .getProperty ("status", {}).toString() == "accepted",
                 "Sound Lab render command failed to start");
        require (PluginEditorBridgeTestProbe::invoke (editor, "renderSoundLab")
                     .toString().contains ("already rendering"),
                 "Sound Lab accepted a concurrent render command");
        juce::var analysis;
        for (int i = 0; i < 400; ++i)
        {
            analysis = PluginEditorBridgeTestProbe::invoke (editor, "getSoundLabAnalysis");
            if (analysis.getProperty ("state", {}).toString() != "rendering")
                break;
            std::this_thread::sleep_for (std::chrono::milliseconds (25));
        }
        require (analysis.getProperty ("state", {}).toString() == "ready",
                 "Sound Lab did not finish rendering through the native command");
        const auto audioUrl = analysis.getProperty ("audio_url", {}).toString();
        const auto audio = PluginEditorBridgeTestProbe::resource (editor, audioUrl);
        require (audio.has_value() && audio->mimeType == "audio/wav" && audio->data.size() > 44,
                 "Sound Lab did not serve the current generation WAV");
        require (! PluginEditorBridgeTestProbe::resource (editor, "/sound-lab.wav?generation=0")
                     && ! PluginEditorBridgeTestProbe::resource (editor, "/sound-lab.wav")
                     && ! PluginEditorBridgeTestProbe::resource (editor, "/unknown"),
                 "editor served a stale or unknown resource");

        DandrumAudioProcessor kick (InstrumentDemoConfiguration::kick());
        kick.setPlayConfigDetails (0, 2, 48000.0, 64);
        kick.prepareToPlay (48000.0, 64);
        DandrumAudioProcessorEditor kickEditor (kick);
        const auto kickPage = PluginEditorBridgeTestProbe::resource (kickEditor, "/index.html");
        const auto kickPageText = kickPage.has_value()
            ? std::string (reinterpret_cast<const char*> (kickPage->data.data()), kickPage->data.size())
            : std::string();
        require (kickPage.has_value() && kickPage->mimeType == "text/html"
                     && kickPageText.find ("Dandrum 808 Kick") != std::string::npos
                     && kickPageText.find ("<span>KICK</span>") != std::string::npos
                     && kickPageText.find ("/shared-instrument-ui.js") != std::string::npos
                     && kickPageText.find ("soundLabStatus") != std::string::npos
                     && kickPageText.find ("/sound-lab-ui.js") != std::string::npos,
                 "second demo did not serve a distinct page through the same resource provider");
        require (PluginEditorBridgeTestProbe::resource (kickEditor, "/sound-lab-ui.js").has_value(),
                 "configured kick demo did not serve shared Sound Lab behavior");
        const auto sharedScript = PluginEditorBridgeTestProbe::resource (kickEditor, "/shared-instrument-ui.js");
        require (sharedScript.has_value() && sharedScript->mimeType == "text/javascript"
                     && ! sharedScript->data.empty(),
                 "second demo did not serve the shared playable control behavior");
        const auto kickParameters = PluginEditorBridgeTestProbe::invoke (kickEditor, "getParameters");
        require (findParameter (kickParameters, "kick.tune_hz").isObject()
                     && ! findParameter (kickParameters, "filter.cutoff").isObject(),
                 "second demo did not expose kick metadata through the same bridge");
        require (PluginEditorBridgeTestProbe::invoke (kickEditor, "renderSoundLab")
                     .getProperty ("status", {}).toString() == "accepted",
                 "second demo could not start its configured Sound Lab render");
        juce::var kickAnalysis;
        for (int i = 0; i < 400; ++i)
        {
            kickAnalysis = PluginEditorBridgeTestProbe::invoke (kickEditor, "getSoundLabAnalysis");
            if (kickAnalysis.getProperty ("state", {}).toString() != "rendering")
                break;
            std::this_thread::sleep_for (std::chrono::milliseconds (25));
        }
        require (kickAnalysis.getProperty ("state", {}).toString() == "ready"
                     && std::abs (static_cast<double> (kickAnalysis.getProperty ("duration_seconds", {})) - 1.0) < 0.001,
                 "second demo Sound Lab did not use its own one-second kick fixture");

        DandrumAudioProcessor acid (InstrumentDemoConfiguration::tb303());
        acid.setPlayConfigDetails (0, 2, 48000.0, 64);
        acid.prepareToPlay (48000.0, 64);
        DandrumAudioProcessorEditor acidEditor (acid);
        const auto acidPage = PluginEditorBridgeTestProbe::resource (acidEditor, "/index.html");
        const auto acidScript = PluginEditorBridgeTestProbe::resource (acidEditor, "/app.js");
        const auto acidStyles = PluginEditorBridgeTestProbe::resource (acidEditor, "/app.css");
        const auto acidPageText = acidPage.has_value()
            ? std::string (reinterpret_cast<const char*> (acidPage->data.data()), acidPage->data.size())
            : std::string();
        require (acidPage && acidPage->mimeType == "text/html"
                     && acidPageText.find ("id=\"root\"") != std::string::npos
                     && acidPageText.find ("/app.js") != std::string::npos
                     && acidPageText.find ("/app.css") != std::string::npos
                     && acidPageText.find ("http://") == std::string::npos
                     && acidPageText.find ("https://") == std::string::npos
                     && acidScript && acidScript->mimeType == "text/javascript"
                     && ! acidScript->data.empty()
                     && acidStyles && acidStyles->mimeType == "text/css"
                     && ! acidStyles->data.empty(),
                 "TB-303 React assets were not packaged for offline WebView serving");
        const auto acidState = PluginEditorBridgeTestProbe::invoke (acidEditor, "getParameterState");
        const auto acidValues = acidState.getProperty ("parameters", {});
        const auto* acidParameters = acidValues.getArray();
        require (acidParameters != nullptr && acidParameters->size() == 7,
                 "TB-303 React panel did not receive seven live public controls");
        for (const auto* id : { "filter.cutoff", "filter.resonance",
                                "filter.envelope_modulation", "filter.decay_ms",
                                "accent.brightness", "amp.release_ms", "slide.time_ms" })
            require (findParameter (acidValues, id).isObject(),
                     std::string ("TB-303 public control missing from host state: ") + id);
        const auto acidDocument = PluginEditorBridgeTestProbe::invoke (
            acidEditor, "getPreparedDocument");
        const auto acidSources = acidDocument.getProperty ("sources", {});
        require (acidDocument.isObject()
                     && acidDocument.getProperty ("instrumentId", {}).toString()
                            == "dandrum.tb303-acid"
                     && acidSources.getArray() != nullptr && acidSources.getArray()->isEmpty()
                     && ! static_cast<bool> (acidDocument.getProperty ("capabilities", {})
                           .getProperty ("preparedWaveform", {}))
                     && PluginEditorBridgeTestProbe::invoke (
                         acidEditor, "requestWaveform",
                         { "drums", "kick", 0, 16,
                           static_cast<int> (acid.getParameterSurfaceGeneration()) })
                         .toString().contains ("unavailable"),
                 "TB-303 Web document invented prepared sample content");
        require (std::abs (static_cast<float> (findParameter (
                     acidDocument.getProperty ("parameters", {}), "filter.cutoff")
                     .getProperty ("normalisedDefaultValue", -1000.0))
                     - (0.4f - 0.02f) / (0.9f - 0.02f)) < 0.00001f,
                 "TB-303 reset default ignored the prepared cutoff range");

        DandrumAudioProcessor samplerWeb (InstrumentDemoConfiguration::sampler());
        samplerWeb.setPlayConfigDetails (0, 2, 48000.0, 64);
        samplerWeb.prepareToPlay (48000.0, 64);
        require (samplerWeb.isInstrumentLoaded(), "sampler Web document fixture did not prepare");
        DandrumAudioProcessorEditor samplerEditor (samplerWeb);
        const auto samplerPage = PluginEditorBridgeTestProbe::resource (samplerEditor, "/index.html");
        const auto samplerScript = PluginEditorBridgeTestProbe::resource (samplerEditor, "/app.js");
        const auto samplerStyles = PluginEditorBridgeTestProbe::resource (samplerEditor, "/app.css");
        const auto samplerPageText = samplerPage
            ? std::string (reinterpret_cast<const char*> (samplerPage->data.data()),
                           samplerPage->data.size()) : std::string();
        const auto samplerScriptText = samplerScript
            ? std::string (reinterpret_cast<const char*> (samplerScript->data.data()),
                           samplerScript->data.size()) : std::string();
        require (samplerPage && samplerPage->mimeType == "text/html"
                     && samplerPageText.find ("id=\"root\"") != std::string::npos
                     && samplerPageText.find ("/app.js") != std::string::npos
                     && samplerPageText.find ("/app.css") != std::string::npos
                     && samplerPageText.find ("http://") == std::string::npos
                     && samplerPageText.find ("https://") == std::string::npos
                     && samplerScript && samplerScript->mimeType == "text/javascript"
                     && samplerScriptText.find ("Prepared key map") != std::string::npos
                     && samplerScriptText.find ("getPreparedDocument") != std::string::npos
                     && samplerStyles && samplerStyles->mimeType == "text/css"
                     && ! samplerStyles->data.empty(),
                 "sampler React assets were not packaged for offline WebView serving");
        const auto samplerDocument = PluginEditorBridgeTestProbe::invoke (
            samplerEditor, "getPreparedDocument");
        const auto samplerSources = samplerDocument.getProperty ("sources", {});
        const auto samplerMaps = samplerDocument.getProperty ("maps", {});
        const auto samplerParameters = samplerDocument.getProperty ("parameters", {});
        const auto outputBuses = samplerDocument.getProperty ("outputBuses", {});
        require (outputBuses.isArray() && outputBuses.size() == 1,
                 "prepared Web document does not enumerate its actual host output bus");
        const auto outputBus = outputBuses[0];
        const auto outputChannels = outputBus.getProperty ("channels", {});
        require (outputBus.getProperty ("id", {}).toString() == "output:0"
                     && outputBus.getProperty ("name", {}).toString() == "Output"
                     && static_cast<bool> (outputBus.getProperty ("main", {}))
                     && outputChannels.isArray() && outputChannels.size() == 2
                     && outputChannels[0].toString() == "L"
                     && outputChannels[1].toString() == "R"
                     && outputBus.getProperty ("meterBusId", {}).toString() == "master"
                     && ! outputBus.hasProperty ("feeds"),
                 "prepared Web output binding lost actual channel names or invented routing metadata");
        require (samplerDocument.isObject()
                     && static_cast<int> (samplerDocument.getProperty ("generation", {}))
                            == static_cast<int> (samplerWeb.getParameterSurfaceGeneration())
                     && samplerDocument.getProperty ("instrumentId", {}).toString()
                            == "dandrum.advanced-drum-kit"
                     && samplerSources.getArray() != nullptr && samplerSources.getArray()->size() == 1
                     && samplerMaps.getArray() != nullptr && samplerMaps.getArray()->size() == 1
                     && samplerParameters.getArray() != nullptr
                     && samplerParameters.getArray()->size() == 23
                     && static_cast<bool> (samplerDocument.getProperty ("capabilities", {})
                           .getProperty ("preparedWaveform", {})),
                 "Web document lost prepared sampler identity, controls or capabilities");
        const auto source = samplerSources.getArray()->getReference (0);
        const auto regions = source.getProperty ("regions", {});
        const auto map = samplerMaps.getArray()->getReference (0);
        const auto zones = map.getProperty ("zones", {});
        const auto soft = findParameter (zones, "snare_soft");
        const auto hard = findParameter (zones, "snare_hard_a");
        const auto snareParameter = findParameter (samplerParameters, "drums.snare.pitch_ratio");
        require (std::abs (static_cast<float> (snareParameter.getProperty (
                     "normalisedDefaultValue", -1000.0)) - 1.0f / 9.0f) < 0.00001f,
                 "sample-group reset default ignored the prepared pitch range");
        require (source.getProperty ("id", {}).toString() == "drums"
                     && static_cast<int> (source.getProperty ("sampleRateHz", {})) == 48000
                     && source.getProperty ("frameCount", {}).toString() == "51000"
                     && regions.getArray() != nullptr && regions.getArray()->size() == 6
                     && regions.getArray()->getReference (0).getProperty ("startFrame", {}).toString() == "0"
                     && regions.getArray()->getReference (0).getProperty ("endFrame", {}).toString() == "12000"
                     && map.getProperty ("selectionMode", {}).toString() == "round_robin"
                     && map.getProperty ("selectionSeed", {}).toString() == "2026"
                     && static_cast<int> (soft.getProperty ("velocityHigh", {})) == 63
                     && static_cast<int> (hard.getProperty ("velocityLow", {})) == 64
                     && hard.getProperty ("roundRobinGroup", {}).toString() == "hard_snare"
                     && static_cast<int> (hard.getProperty ("controlGroup", {})) == 2
                     && snareParameter.getProperty ("scope", {}).toString() == "sampleGroup"
                     && static_cast<int> (snareParameter.getProperty ("controlGroup", {})) == 2,
                 "Web document did not preserve source frames, selection semantics or scoped controls");
        const auto samplerGeneration = samplerWeb.getParameterSurfaceGeneration();
        require (PluginEditorBridgeTestProbe::invoke (
                     samplerEditor, "requestWaveform",
                     { "drums", "kick", 0, 0, static_cast<int> (samplerGeneration) })
                         .toString().contains ("bucket")
                     && PluginEditorBridgeTestProbe::invoke (
                         samplerEditor, "requestWaveform",
                         { "drums", "missing", 0, 16, static_cast<int> (samplerGeneration) })
                         .toString().contains ("unavailable")
                     && PluginEditorBridgeTestProbe::invoke (
                         samplerEditor, "requestWaveform",
                         { "drums", "kick", 0, 16, static_cast<int> (samplerGeneration - 1) })
                         .toString().contains ("stale"),
                 "Web waveform admission accepted invalid buckets, region or generation");
        const auto waveformAccepted = PluginEditorBridgeTestProbe::invoke (
            samplerEditor, "requestWaveform",
            { "drums", "kick", 0, 16, static_cast<int> (samplerGeneration) });
        const auto waveformId = waveformAccepted.getProperty ("job_id", {}).toString();
        require (waveformAccepted.getProperty ("status", {}).toString() == "accepted"
                     && waveformAccepted.getProperty ("job_id", {}).isString()
                     && waveformId.isNotEmpty(),
                 "Web waveform request did not return an exact accepted job ID");
        juce::var waveformStatus;
        for (int attempt = 0; attempt < 200; ++attempt)
        {
            waveformStatus = PluginEditorBridgeTestProbe::invoke (
                samplerEditor, "getWaveformJobStatus", { waveformId });
            if (waveformStatus.getProperty ("state", {}).toString() != "running")
                break;
            std::this_thread::sleep_for (std::chrono::milliseconds (5));
        }
        const auto waveform = waveformStatus.getProperty ("result", {});
        const auto waveformBuckets = waveform.getProperty ("buckets", {});
        float minimum = 1.0f;
        float maximum = -1.0f;
        if (const auto* buckets = waveformBuckets.getArray())
            for (const auto& bucket : *buckets)
            {
                minimum = std::min (minimum, static_cast<float> (bucket.getProperty ("minimum", {})));
                maximum = std::max (maximum, static_cast<float> (bucket.getProperty ("maximum", {})));
            }
        require (waveformStatus.getProperty ("state", {}).toString() == "ready"
                     && waveformStatus.getProperty ("job_id", {}).toString() == waveformId
                     && waveform.getProperty ("sourceId", {}).toString() == "drums"
                     && waveform.getProperty ("regionId", {}).toString() == "kick"
                     && static_cast<int> (waveform.getProperty ("sampleRateHz", {})) == 48000
                     && waveform.getProperty ("startFrame", {}).toString() == "0"
                     && waveform.getProperty ("endFrame", {}).toString() == "12000"
                     && waveform.getProperty ("contentRevision", {}).toString().length() == 64
                     && waveformBuckets.getArray() != nullptr
                     && waveformBuckets.getArray()->size() == 16
                     && waveformBuckets.getArray()->getReference (0)
                            .getProperty ("startFrame", {}).toString() == "0"
                     && waveformBuckets.getArray()->getLast()
                            .getProperty ("endFrame", {}).toString() == "12000"
                     && std::abs (minimum + 0.69995117f) < 0.0001f
                     && std::abs (maximum - 0.80514526f) < 0.0001f,
                 "Web waveform job lost source coordinates, content identity or signed extrema");
        require (PluginEditorBridgeTestProbe::invoke (
                     samplerEditor, "getWaveformJobStatus", { "999999" })
                         .toString().contains ("Unknown")
                     && PluginEditorBridgeTestProbe::invoke (
                         samplerEditor, "getWaveformJobStatus", { 1.5 })
                         .toString().contains ("valid")
                     && ! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                         samplerEditor, "cancelWaveform", { waveformId })),
                 "Web waveform status or cancellation accepted an invalid or finished job");
        const auto requestSpectrum = [&] (const juce::Array<juce::var>& args)
        { return PluginEditorBridgeTestProbe::invoke (samplerEditor, "requestSpectrogram", args); };
        for (const auto& args : std::vector<juce::Array<juce::var>> {
                 {}, { "drums", "kick", -1, static_cast<int> (samplerGeneration) },
                 { "drums", "kick", 65536, static_cast<int> (samplerGeneration) },
                 { "drums", "kick", 0, 1.5 }, { "drums", "missing", 0, static_cast<int> (samplerGeneration) },
                 { "drums", "kick", 0, static_cast<int> (samplerGeneration - 1) } })
            require (requestSpectrum (args).isString(), "Web spectral admission accepted invalid input");
        const auto spectralAccepted = requestSpectrum ({ "drums", "kick", 0, static_cast<int> (samplerGeneration) });
        const auto spectralId = spectralAccepted.getProperty ("job_id", {}).toString();
        require (spectralAccepted.getProperty ("status", {}).toString() == "accepted" && spectralId.isNotEmpty(),
                 "Web spectral request did not return acceptance");
        juce::var spectralStatus;
        for (int attempt = 0; attempt < 200; ++attempt)
        {
            spectralStatus = PluginEditorBridgeTestProbe::invoke (samplerEditor, "getSpectrogramJobStatus", { spectralId, 0 });
            if (spectralStatus.getProperty ("state", {}).toString() != "running") break;
            std::this_thread::sleep_for (std::chrono::milliseconds (5));
        }
        const auto typedSpectrum = samplerWeb.getPreparedSpectrumJobStatus (std::stoull (spectralId.toStdString()));
        require (typedSpectrum && typedSpectrum->result && spectralStatus.getProperty ("state", {}).toString() == "ready",
                 "Web spectral job did not become ready");
        const auto spectralData = spectralStatus.getProperty ("result", {});
        const auto settings = spectralData.getProperty ("settings", {});
        const auto frequencies = spectralData.getProperty ("frequencyHz", {});
        require (spectralData.getProperty ("sourceId", {}).toString() == "drums"
                     && spectralData.getProperty ("regionId", {}).toString() == "kick"
                     && spectralData.getProperty ("startFrame", {}).toString() == "0"
                     && spectralData.getProperty ("endFrame", {}).toString() == "12000"
                     && static_cast<int> (spectralData.getProperty ("sampleRateHz", {})) == 48000
                     && static_cast<int> (spectralData.getProperty ("channel", {})) == 0
                     && spectralData.getProperty ("contentRevision", {}).toString().length() == 64
                     && settings.getProperty ("window", {}).toString() == "periodicHann"
                     && settings.getProperty ("scaling", {}).toString() == "oneSidedPeakDbFS"
                     && settings.getProperty ("channelPolicy", {}).toString() == "selectedChannel"
                     && static_cast<int> (settings.getProperty ("fftSize", {})) == 1024
                     && settings.getProperty ("hopFrames", {}).toString() == "256"
                     && std::abs (static_cast<double> (settings.getProperty ("floorDbFS", {})) + 120.0) < 0.000001
                     && frequencies.getArray() && frequencies.getArray()->size() == 513
                     && std::abs (static_cast<double> ((*frequencies.getArray())[64]) - 3000.0) < 0.000001
                     && std::abs (static_cast<double> (frequencies.getArray()->getLast()) - 24000.0) < 0.000001,
                 "Web spectral metadata lost declared source coordinates or FFT settings");
        std::size_t transferred = 0;
        for (int offset = 0; offset < 47; offset += 16)
        {
            const auto page = PluginEditorBridgeTestProbe::invoke (samplerEditor, "getSpectrogramJobStatus", { spectralId, offset })
                                  .getProperty ("result", {});
            const auto columns = page.getProperty ("columns", {});
            require (static_cast<int> (page.getProperty ("columnOffset", {})) == offset
                         && static_cast<int> (page.getProperty ("totalColumns", {})) == 47
                         && columns.getArray() && columns.getArray()->size() == std::min (16, 47 - offset),
                     "Web spectral numeric publication exceeded or lost its bounded page");
            for (const auto& column : *columns.getArray())
            {
                const auto& expected = typedSpectrum->result->columns[transferred++];
                const auto magnitude = column.getProperty ("magnitudeDbFS", {});
                require (column.getProperty ("startFrame", {}).toString() == juce::String (std::to_string (expected.startFrame))
                             && column.getProperty ("endFrame", {}).toString() == juce::String (std::to_string (expected.endFrame))
                             && magnitude.getArray() && magnitude.getArray()->size() == 513,
                         "Web spectral page lost exact column coordinates or bin count");
                for (int bin = 0; bin < 513; ++bin)
                    require (std::bit_cast<std::uint32_t> (static_cast<float> ((*magnitude.getArray())[bin]))
                                 == std::bit_cast<std::uint32_t> (expected.magnitudeDbFS[static_cast<std::size_t> (bin)]),
                             "Web spectral page changed a shared numeric magnitude");
            }
        }
        require (transferred == 47, "Web spectral pages lost columns");
        for (const auto& args : std::vector<juce::Array<juce::var>> {
                 {}, { spectralId }, { "999999", 0 }, { 1.5, 0 }, { spectralId, -1 },
                 { spectralId, 1.5 }, { spectralId, 47 }, { spectralId, 1025 } })
            require (PluginEditorBridgeTestProbe::invoke (samplerEditor, "getSpectrogramJobStatus", args).isString(),
                     "Web spectral status accepted invalid page coordinates");
        require (! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (samplerEditor, "cancelSpectrogram", { spectralId })),
                 "Web cancelled a finished spectral job");
        juce::String closedSpectralJob;
        {
            DandrumAudioProcessorEditor closingEditor (samplerWeb);
            const auto accepted = PluginEditorBridgeTestProbe::invoke (closingEditor, "requestSpectrogram",
                { "drums", "hat_open", 0, static_cast<int> (samplerGeneration) });
            closedSpectralJob = accepted.getProperty ("job_id", {}).toString();
            require (accepted.getProperty ("status", {}).toString() == "accepted"
                         && ! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (samplerEditor, "cancelSpectrogram", { closedSpectralJob })),
                     "one Web editor cancelled another spectral job");
        }
        const auto closedSpectrum = PluginEditorBridgeTestProbe::invoke (samplerEditor, "getSpectrogramJobStatus", { closedSpectralJob, 0 });
        require (closedSpectrum.getProperty ("state", {}).toString() == "cancelled"
                     || closedSpectrum.getProperty ("state", {}).toString() == "ready", "closed Web spectral job lost terminal status");

        juce::String closedEditorJob;
        {
            DandrumAudioProcessorEditor closingEditor (samplerWeb);
            const auto accepted = PluginEditorBridgeTestProbe::invoke (
                closingEditor, "requestWaveform",
                { "drums", "hat_open", 0, 512, static_cast<int> (samplerGeneration) });
            closedEditorJob = accepted.getProperty ("job_id", {}).toString();
            require (accepted.getProperty ("status", {}).toString() == "accepted"
                         && closedEditorJob.isNotEmpty()
                         && ! static_cast<bool> (PluginEditorBridgeTestProbe::invoke (
                             samplerEditor, "cancelWaveform", { closedEditorJob })),
                     "one Web editor cancelled another editor's waveform request");
        }
        const auto closedEditorStatus = PluginEditorBridgeTestProbe::invoke (
            samplerEditor, "getWaveformJobStatus", { closedEditorJob });
        require (closedEditorStatus.getProperty ("state", {}).toString() == "cancelled"
                     || closedEditorStatus.getProperty ("state", {}).toString() == "ready",
                 "closing a Web editor lost its queryable waveform terminal status");

        auto slicerConfiguration = InstrumentDemoConfiguration::sampler();
        slicerConfiguration.instrumentPath = juce::File (juce::String (DANDRUM_SOURCE_ROOT))
            .getChildFile ("examples/patches/advanced-break-slicer.yaml")
            .getFullPathName().toStdString();
        DandrumAudioProcessor slicerWeb (slicerConfiguration);
        slicerWeb.setPlayConfigDetails (0, 2, 44100.0, 64);
        slicerWeb.prepareToPlay (44100.0, 64);
        require (slicerWeb.isInstrumentLoaded(), "slice Web document fixture did not prepare");
        DandrumAudioProcessorEditor slicerEditor (slicerWeb);
        const auto sliceDocument = PluginEditorBridgeTestProbe::invoke (
            slicerEditor, "getPreparedDocument");
        const auto sliceSources = sliceDocument.getProperty ("sources", {});
        const auto slices = sliceSources.getArray()->getReference (0).getProperty ("slices", {});
        require (slices.getArray() != nullptr && slices.getArray()->size() == 4
                     && slices.getArray()->getReference (1).getProperty ("id", {}).toString() == "hat"
                     && slices.getArray()->getReference (1).getProperty ("startFrame", {}).toString() == "6000"
                     && slices.getArray()->getReference (1).getProperty ("endFrame", {}).toString() == "12000",
                 "Web document lost prepared slice identities and frame coordinates");

        const auto sourceRoot = std::filesystem::path (DANDRUM_SOURCE_ROOT);
        const auto documentFixture = std::filesystem::temp_directory_path()
            / ("dandrum-web-document-" + std::to_string (
                std::chrono::steady_clock::now().time_since_epoch().count()));
        std::filesystem::create_directories (documentFixture / "assets");
        struct DocumentFixtureGuard
        {
            std::filesystem::path path;
            ~DocumentFixtureGuard() { std::filesystem::remove_all (path); }
        } documentFixtureGuard { documentFixture };
        std::filesystem::copy_file (sourceRoot / "examples/patches/assets/advanced-drums.wav",
                                    documentFixture / "assets/advanced-drums.wav");
        auto detailedYaml = juce::File (juce::String (
            (sourceRoot / "examples/patches/advanced-drum-kit.yaml").string())).loadFileAsString();
        detailedYaml = detailedYaml.replace (
            "{ id: kick, start_frame: 0, end_frame: 12000 }",
            "{ id: kick, start_frame: 0, end_frame: 12000, root_note: 60, gain_db: -3, pan: 0.25, fade_in_ms: 2, fade_out_ms: 3, loop: { mode: forward, start_frame: 100, end_frame: 10000, crossfade_ms: 1 } }");
        detailedYaml = detailedYaml.replace ("selection_seed: 2026",
                                             "selection_seed: 9007199254740993");
        detailedYaml = detailedYaml.replace (
            "id: kick, region: drums.kick, key_range: [36, 36], velocity_range: [1, 127], control_group: 1",
            "id: kick, region: drums.kick, key_range: [36, 36], velocity_range: [1, 127], control_group: 1, weight: 2, gain_db: -6, pan: -0.5, pitch_semitones: 12");
        require (juce::File (juce::String ((documentFixture / "instrument.yaml").string()))
                     .replaceWithText (detailedYaml),
                 "could not write prepared document metadata fixture");
        auto detailedConfiguration = InstrumentDemoConfiguration::sampler();
        detailedConfiguration.instrumentPath = (documentFixture / "instrument.yaml").string();
        DandrumAudioProcessor detailedWeb (detailedConfiguration);
        detailedWeb.setPlayConfigDetails (0, 2, 48000.0, 64);
        detailedWeb.prepareToPlay (48000.0, 64);
        require (detailedWeb.isInstrumentLoaded(),
                 "prepared document metadata fixture did not prepare");
        DandrumAudioProcessorEditor detailedEditor (detailedWeb);
       #if JUCE_LINUX
        rejectSpectralRead.store (true);
        struct RestoreSpectralReader { ~RestoreSpectralReader() { rejectSpectralRead.store (false); } } restoreReader;
        const auto failedAdmission = PluginEditorBridgeTestProbe::invoke (detailedEditor, "requestSpectrogram",
            { "drums", "kick", 0, static_cast<int> (detailedWeb.getParameterSurfaceGeneration()) });
        const auto failedId = failedAdmission.getProperty ("job_id", {}).toString();
        juce::var failedSpectrum;
        for (int attempt = 0; attempt < 200; ++attempt)
        {
            failedSpectrum = PluginEditorBridgeTestProbe::invoke (detailedEditor, "getSpectrogramJobStatus", { failedId, 0 });
            if (failedSpectrum.getProperty ("state", {}).toString() != "running") break;
            std::this_thread::sleep_for (std::chrono::milliseconds (5));
        }
        require (failedSpectrum.getProperty ("state", {}).toString() == "failed"
                     && failedSpectrum.getProperty ("error", {}).toString().isNotEmpty()
                     && failedSpectrum.getProperty ("result", {}).isVoid(), "spectral read failure did not cross the Web status boundary");
        rejectSpectralRead.store (false);
       #endif
        const auto detailedDocument = PluginEditorBridgeTestProbe::invoke (
            detailedEditor, "getPreparedDocument");
        const auto detailedSources = detailedDocument.getProperty ("sources", {});
        const auto detailedRegions = detailedSources.getArray()->getReference (0)
            .getProperty ("regions", {});
        const auto detailedRegion = detailedRegions.getArray()->getReference (0);
        const auto detailedLoop = detailedRegion.getProperty ("loop", {});
        const auto detailedMaps = detailedDocument.getProperty ("maps", {});
        const auto detailedMap = detailedMaps.getArray()->getReference (0);
        const auto detailedZones = detailedMap.getProperty ("zones", {});
        const auto detailedKick = findParameter (detailedZones, "kick");
        require (detailedMap.getProperty ("selectionSeed", {}).toString()
                            == "9007199254740993"
                     && static_cast<int> (detailedRegion.getProperty ("rootNote", {})) == 60
                     && std::abs (static_cast<double> (detailedRegion.getProperty ("gainDb", {})) + 3.0) < 0.0001
                     && std::abs (static_cast<double> (detailedRegion.getProperty ("pan", {})) - 0.25) < 0.0001
                     && std::abs (static_cast<double> (detailedRegion.getProperty ("fadeInMs", {})) - 2.0) < 0.0001
                     && std::abs (static_cast<double> (detailedRegion.getProperty ("fadeOutMs", {})) - 3.0) < 0.0001
                     && detailedLoop.getProperty ("mode", {}).toString() == "forward"
                     && detailedLoop.getProperty ("startFrame", {}).toString() == "100"
                     && detailedLoop.getProperty ("endFrame", {}).toString() == "10000"
                     && std::abs (static_cast<double> (detailedLoop.getProperty ("crossfadeMs", {})) - 1.0) < 0.0001
                     && static_cast<int> (detailedKick.getProperty ("weight", {})) == 2
                     && std::abs (static_cast<double> (detailedKick.getProperty ("gainDb", {})) + 6.0) < 0.0001
                     && std::abs (static_cast<double> (detailedKick.getProperty ("pan", {})) + 0.5) < 0.0001
                     && std::abs (static_cast<double> (detailedKick.getProperty ("pitchSemitones", {})) - 12.0) < 0.0001,
                 "Web document lost loop, fade, optional zone data or a 64-bit map seed");
        require (samplerWeb.reloadInstrumentFromFile (tb303),
                 "could not reload sampler Web document to TB-303");
        const auto replacementDocument = PluginEditorBridgeTestProbe::invoke (
            samplerEditor, "getPreparedDocument");
        const auto replacementSources = replacementDocument.getProperty ("sources", {});
        require (static_cast<int> (replacementDocument.getProperty ("generation", {}))
                            > static_cast<int> (samplerGeneration)
                     && replacementDocument.getProperty ("instrumentId", {}).toString()
                            == "dandrum.tb303-acid"
                     && replacementSources.getArray() != nullptr
                     && replacementSources.getArray()->isEmpty(),
                 "Web document did not replace sampler facts after instrument reload");
        const auto staleSpectrum = PluginEditorBridgeTestProbe::invoke (samplerEditor, "getSpectrogramJobStatus", { spectralId, 0 });
        require (staleSpectrum.getProperty ("state", {}).toString() == "stale"
                     && staleSpectrum.getProperty ("result", {}).isVoid(), "Web spectral result survived instrument replacement");
        const auto staleWaveform = PluginEditorBridgeTestProbe::invoke (
            samplerEditor, "getWaveformJobStatus", { waveformId });
        require (staleWaveform.getProperty ("state", {}).toString() == "stale"
                     && staleWaveform.getProperty ("result", {}).isVoid(),
                 "Web waveform reply kept previous instrument PCM visible after reload");

        const auto referenceFile = juce::File::getSpecialLocation (juce::File::tempDirectory)
                                       .getNonexistentChildFile ("dandrum_kick_match_reference", ".wav");
        struct ReferenceFileGuard
        {
            juce::File file;
            ~ReferenceFileGuard() { file.deleteFile(); }
        } referenceGuard { referenceFile };
        SoundLabController referenceRenderer;
        require (referenceRenderer.startRender (*InstrumentDemoConfiguration::kick().soundLabFixturePath),
                 "could not render the configured kick reference");
        for (int i = 0; i < 400 && referenceRenderer.state() == SoundLabController::State::rendering; ++i)
            std::this_thread::sleep_for (std::chrono::milliseconds (25));
        const auto reference = referenceRenderer.snapshot();
        require (reference.state == SoundLabController::State::ready && reference.data != nullptr
                     && referenceFile.replaceWithData (reference.data->wavBytes.data(),
                                                       reference.data->wavBytes.size()),
                 "could not write the configured kick reference WAV");
        PluginEditorBridgeTestProbe::setReferenceFile (kickEditor, referenceFile);
        require (PluginEditorBridgeTestProbe::invoke (kickEditor, "matchSoundLab")
                     .getProperty ("status", {}).toString() == "accepted",
                 "configured kick Sound Lab did not start matching");
        juce::var matched;
        for (int i = 0; i < 400; ++i)
        {
            matched = PluginEditorBridgeTestProbe::invoke (kickEditor, "getSoundLabAnalysis");
            if (matched.getProperty ("state", {}).toString() != "matching")
                break;
            std::this_thread::sleep_for (std::chrono::milliseconds (25));
        }
        require (matched.getProperty ("state", {}).toString() == "matched"
                     && matched.getProperty ("best_parameters", {}).getArray() != nullptr,
                 "configured kick Sound Lab did not produce a match");
        const auto referenceAudio = matched.getProperty ("reference_audio_url", {}).toString();
        const auto candidateAudio = matched.getProperty ("candidate_audio_url", {}).toString();
        require (PluginEditorBridgeTestProbe::resource (kickEditor, referenceAudio).has_value()
                     && PluginEditorBridgeTestProbe::resource (kickEditor, candidateAudio).has_value()
                     && ! PluginEditorBridgeTestProbe::resource (
                         kickEditor, "/sound-lab-candidate.wav?generation=0"),
                 "matched Sound Lab audio did not honor its generation");
        auto matchedPatch = PluginEditorBridgeTestProbe::matchedPatch (kickEditor);
        require (kick.reloadInstrumentFromFile (tb303),
                 "could not replace the active instrument before match rejection");
        require (PluginEditorBridgeTestProbe::invoke (kickEditor, "acceptSoundLabMatch")
                     .toString().contains ("does not match"),
                 "Sound Lab accepted kick results into a different active instrument");
        require (! PluginEditorBridgeTestProbe::resource (kickEditor, "/sound-lab-ui.js"),
                 "mismatched active instrument still exposed Sound Lab behavior");
        const auto mismatchedPage = PluginEditorBridgeTestProbe::resource (kickEditor, "/index.html");
        const auto mismatchedPageText = mismatchedPage.has_value()
            ? std::string (reinterpret_cast<const char*> (mismatchedPage->data.data()),
                           mismatchedPage->data.size())
            : std::string();
        require (mismatchedPageText.find ("soundLabStatus") == std::string::npos,
                 "mismatched active instrument still exposed the Sound Lab panel");
        juce::AudioBuffer<float> replacedAudio (2, 64);
        replacedAudio.clear();
        juce::MidiBuffer replacedMidi;
        replacedMidi.addEvent (juce::MidiMessage::noteOn (1, 60, static_cast<juce::uint8> (100)), 0);
        kick.processBlock (replacedAudio, replacedMidi);
        bool replacedInstrumentAudible = false;
        for (int frame = 0; frame < replacedAudio.getNumSamples(); ++frame)
            replacedInstrumentAudible |= std::abs (replacedAudio.getSample (0, frame)) > 0.0000001f;
        require (replacedInstrumentAudible,
                 "mismatched active instrument stopped processing audio");
        require (kick.reloadInstrumentFromFile (juce::File (juce::String (
                     InstrumentDemoConfiguration::kick().instrumentPath.string()))),
                 "could not restore the configured kick before match acceptance");
        require (PluginEditorBridgeTestProbe::invoke (kickEditor, "acceptSoundLabMatch")
                     .toString().contains ("stale"),
                 "an old match remained actionable after restoring the same instrument");
        require (PluginEditorBridgeTestProbe::invoke (kickEditor, "matchSoundLab")
                     .getProperty ("status", {}).toString() == "accepted",
                 "could not start a fresh match after instrument reload");
        for (int i = 0; i < 400; ++i)
        {
            matched = PluginEditorBridgeTestProbe::invoke (kickEditor, "getSoundLabAnalysis");
            if (matched.getProperty ("state", {}).toString() != "matching")
                break;
            std::this_thread::sleep_for (std::chrono::milliseconds (25));
        }
        matchedPatch = PluginEditorBridgeTestProbe::matchedPatch (kickEditor);
        require (matchedPatch.isNotEmpty()
                     && matched.getProperty ("state", {}).toString() == "matched"
                     && PluginEditorBridgeTestProbe::invoke (kickEditor, "acceptSoundLabMatch").isVoid()
                     && kick.currentInstrumentYaml() == matchedPatch
                     && kick.currentInstrumentFile().getFullPathName().toStdString()
                            == InstrumentDemoConfiguration::kick().matchSourcePath->string(),
                 "accepted match did not reload the configured source patch");
        const auto bestParameters = matched.getProperty ("best_parameters", {});
        for (const auto& best : *bestParameters.getArray())
        {
            const auto* matchedParameter = kick.getParameterForPublicId (best.getProperty ("id", {}).toString());
            require (matchedParameter != nullptr
                         && std::abs (matchedParameter->getValue()
                                      - static_cast<float> (best.getProperty ("normalized", {}))) < 0.00001f,
                     "accepted match did not apply a best public parameter value");
        }

        auto withoutLab = InstrumentDemoConfiguration::kick();
        withoutLab.soundLabFixturePath.reset();
        withoutLab.matchSourcePath.reset();
        DandrumAudioProcessor plain (withoutLab);
        plain.setPlayConfigDetails (0, 2, 48000.0, 64);
        plain.prepareToPlay (48000.0, 64);
        DandrumAudioProcessorEditor plainEditor (plain);
        require (! PluginEditorBridgeTestProbe::hasSoundLab (plainEditor),
                 "fixture-free demo still created a Sound Lab integration");
        for (const auto* command : { "getParameters", "setParameter", "noteOn", "noteOff" })
            require (PluginEditorBridgeTestProbe::hasCommand (plainEditor, command),
                     std::string ("fixture-free editor lost ") + command);
        for (const auto* command : { "renderSoundLab", "chooseSoundLabReference", "matchSoundLab",
                                     "cancelSoundLab", "acceptSoundLabMatch", "requestGraphProposal",
                                     "getSoundLabAnalysis" })
            require (! PluginEditorBridgeTestProbe::hasCommand (plainEditor, command),
                     std::string ("fixture-free editor registered ") + command);
        require (! PluginEditorBridgeTestProbe::resource (plainEditor, "/sound-lab-ui.js"),
                 "fixture-free demo exposed Sound Lab behavior");
        const auto plainPage = PluginEditorBridgeTestProbe::resource (plainEditor, "/index.html");
        const auto plainPageText = plainPage.has_value()
            ? std::string (reinterpret_cast<const char*> (plainPage->data.data()), plainPage->data.size())
            : std::string();
        require (plainPageText.find ("soundLabStatus") == std::string::npos,
                 "fixture-free demo exposed a Sound Lab panel");
        require (PluginEditorBridgeTestProbe::invoke (plainEditor, "getParameters").getArray() != nullptr
                     && PluginEditorBridgeTestProbe::invoke (plainEditor, "noteOn", { 36, 0.7 }).isVoid(),
                 "fixture-free demo lost shared parameter or playable-note controls");
        require (! PluginEditorBridgeTestProbe::resource (plainEditor, "/sound-lab.wav?generation=0"),
                 "fixture-free demo exposed a Sound Lab audio resource");
        plain.releaseResources();

        auto customConfiguration = InstrumentDemoConfiguration::kick();
        customConfiguration.instrumentId = "dandrum.custom-instrument";
        customConfiguration.title = "Dandrum Custom Instrument";
        customConfiguration.soundLabFixturePath.reset();
        customConfiguration.matchSourcePath.reset();
        DandrumAudioProcessor custom (customConfiguration);
        DandrumAudioProcessorEditor customEditor (custom);
        const auto customPage = PluginEditorBridgeTestProbe::resource (customEditor, "/index.html");
        const auto customPageText = customPage.has_value()
            ? std::string (reinterpret_cast<const char*> (customPage->data.data()), customPage->data.size())
            : std::string();
        require (customPageText.find ("id=\"controls\"") != std::string::npos
                     && customPageText.find ("id=\"keys\"") != std::string::npos
                     && customPageText.find ("/shared-instrument-ui.js") != std::string::npos,
                 "unknown instrument identity left the Web editor without a generic page");

        DandrumAudioProcessor changing (InstrumentDemoConfiguration::kick());
        changing.setPlayConfigDetails (0, 2, 48000.0, 64);
        changing.prepareToPlay (48000.0, 64);
        std::atomic<bool> finished { false };
        std::atomic<bool> reloadsSucceeded { true };
        std::thread reloader ([&]
        {
            const auto kickFile = juce::File (juce::String (InstrumentDemoConfiguration::kick().instrumentPath.string()));
            for (int n = 0; n < 24; ++n)
                if (! changing.reloadInstrumentFromFile ((n % 2 == 0) ? tb303 : kickFile))
                    reloadsSucceeded.store (false);
            finished.store (true);
        });
        bool coherent = true;
        do
        {
            const auto values = changing.getPublicParameterSnapshot();
            if (values.size() != 6 && values.size() != 7)
                coherent = false;
            for (const auto& value : values)
                if ((values.size() == 6 && ! value.id.startsWith ("kick."))
                    || (values.size() == 7 && value.id.startsWith ("kick.")))
                    coherent = false;
        } while (! finished.load());
        reloader.join();
        require (reloadsSucceeded.load() && coherent,
                 "editor parameter snapshots mixed two instrument surfaces during reload");

        std::atomic<bool> holdFirstReload { true }, releaseFirstReload { false };
        DandrumAudioProcessor uiJobs (InstrumentDemoConfiguration::kick(), {}, [&]
        {
            if (holdFirstReload.exchange (false))
                while (!releaseFirstReload.load()) std::this_thread::sleep_for (std::chrono::milliseconds (1));
        });
        struct ReleaseHeldReload
        {
            std::atomic<bool>& released;
            ~ReleaseHeldReload() { released = true; }
        } releaseHeldReload { releaseFirstReload };
        uiJobs.setPlayConfigDetails (0, 2, 48000.0, 64);
        uiJobs.prepareToPlay (48000.0, 64);
        const auto uiGeneration = uiJobs.getParameterSurfaceGeneration();
        juce::int64 failedJobId = 0;
        {
            DandrumAudioProcessorEditor firstEditor (uiJobs);
            require (PluginEditorBridgeTestProbe::invoke (firstEditor,
                         "reloadInstrument", { juce::var ("relative.yaml"),
                                                juce::var (static_cast<int> (uiGeneration)) })
                         .toString().contains ("absolute"),
                     "Web reload accepted a relative path");
            const auto accepted = PluginEditorBridgeTestProbe::invoke (firstEditor,
                "reloadInstrument", { juce::var ("/nonexistent/dandrum-ui-job.yaml"),
                                       juce::var (static_cast<int> (uiGeneration)) });
            require (accepted.getProperty ("status", {}).toString() == "accepted",
                     "Web reload did not acknowledge the job before preparation");
            failedJobId = static_cast<juce::int64> (accepted.getProperty ("job_id", {}));
            require (failedJobId > 0, "Web reload did not return a job ID");
            require (PluginEditorBridgeTestProbe::invoke (firstEditor,
                         "reloadInstrument", { juce::var (tb303.getFullPathName()),
                                                juce::var (static_cast<int> (uiGeneration)) })
                         .toString().contains ("already running"),
                     "Web reload admitted two concurrent jobs");
        }
        {
            DandrumAudioProcessorEditor reopened (uiJobs);
            require (uiJobs.isMuted() && PluginEditorBridgeTestProbe::invoke (
                         reopened, "getUiJobStatus", { juce::var (failedJobId) })["state"].toString() == "running",
                     "Closing/reopening an editor stranded its pending processor-owned rebuild");
            releaseFirstReload = true;
            require (PluginEditorBridgeTestProbe::invoke (
                         reopened, "getUiJobStatus", { juce::var (-1) })
                         .toString().contains ("valid job ID")
                         && PluginEditorBridgeTestProbe::invoke (
                             reopened, "getUiJobStatus", { juce::var (9999) })
                             .toString().contains ("Unknown UI job"),
                     "Web job status did not reject invalid or unknown IDs");
            juce::var failedJob;
            for (int attempt = 0; attempt < 200; ++attempt)
            {
                failedJob = PluginEditorBridgeTestProbe::invoke (
                    reopened, "getUiJobStatus", { juce::var (failedJobId) });
                if (failedJob.getProperty ("state", {}).toString() != "running")
                    break;
                std::this_thread::sleep_for (std::chrono::milliseconds (5));
            }
            require (failedJob.getProperty ("state", {}).toString() == "failed"
                         && uiJobs.getParameterSurfaceGeneration() == uiGeneration,
                     "reopened editor could not recover failure without replacing the instrument");
            const auto successful = PluginEditorBridgeTestProbe::invoke (reopened,
                "reloadInstrument", { juce::var (tb303.getFullPathName()),
                                       juce::var (static_cast<int> (uiGeneration)) });
            const auto successfulJobId = successful.getProperty ("job_id", {});
            require (successful.getProperty ("status", {}).toString() == "accepted",
                     "Web reload did not accept the valid replacement");
            juce::var completedJob;
            for (int attempt = 0; attempt < 200; ++attempt)
            {
                completedJob = PluginEditorBridgeTestProbe::invoke (
                    reopened, "getUiJobStatus", { successfulJobId });
                if (completedJob.getProperty ("state", {}).toString() != "running")
                    break;
                std::this_thread::sleep_for (std::chrono::milliseconds (5));
            }
            require (completedJob.getProperty ("state", {}).toString() == "completed"
                         && uiJobs.hasPublicParameter ("filter.cutoff"),
                     "Web reload job did not install the prepared instrument");
            const auto stale = PluginEditorBridgeTestProbe::invoke (reopened,
                "reloadInstrument", { juce::var (tb303.getFullPathName()),
                                       juce::var (static_cast<int> (uiGeneration)) });
            require (stale.toString().contains ("stale"),
                     "Web reload admitted an obsolete instrument generation");
        }

        DandrumAudioProcessor durableAnalysis (InstrumentDemoConfiguration::kick());
        durableAnalysis.setPlayConfigDetails (0, 2, 48000.0, 64);
        durableAnalysis.prepareToPlay (48000.0, 64);
        juce::int64 analysisJobId = 0;
        {
            DandrumAudioProcessorEditor firstAnalysisEditor (durableAnalysis);
            const auto accepted = PluginEditorBridgeTestProbe::invoke (
                firstAnalysisEditor, "renderSoundLab");
            require (accepted.getProperty ("status", {}).toString() == "accepted",
                     "analysis did not return an accepted job before completion");
            analysisJobId = static_cast<juce::int64> (accepted.getProperty ("job_id", {}));
        }
        {
            DandrumAudioProcessorEditor reopened (durableAnalysis);
            juce::var result;
            for (int attempt = 0; attempt < 200; ++attempt)
            {
                result = PluginEditorBridgeTestProbe::invoke (
                    reopened, "getSoundLabAnalysis",
                    { juce::var (static_cast<double> (analysisJobId)) });
                if (result.getProperty ("state", {}).toString() != "rendering")
                    break;
                std::this_thread::sleep_for (std::chrono::milliseconds (5));
            }
            require (result.getProperty ("state", {}).toString() == "ready"
                         && static_cast<juce::int64> (result.getProperty ("job_id", {})) == analysisJobId,
                     "analysis job status did not survive editor reconnection");
            require (PluginEditorBridgeTestProbe::invoke (
                         reopened, "getSoundLabAnalysis", { juce::var (analysisJobId + 1) })
                         .toString().contains ("Unknown analysis job"),
                     "analysis accepted an unrelated job ID");
            require (durableAnalysis.reloadInstrumentFromFile (
                         juce::File (juce::String (InstrumentDemoConfiguration::kick().instrumentPath.string()))),
                     "could not reload while testing stale analysis status");
            result = PluginEditorBridgeTestProbe::invoke (reopened, "getSoundLabAnalysis");
            require (result.getProperty ("state", {}).toString() == "stale",
                     "analysis result from an older instrument generation remained actionable");
        }

        auto signedConfiguration = InstrumentDemoConfiguration::kick();
        signedConfiguration.instrumentPath = juce::File (juce::String (DANDRUM_SOURCE_ROOT))
            .getChildFile ("tests/fixtures/plugin-ui-knob.yaml")
            .getFullPathName().toStdString();
        signedConfiguration.soundLabFixturePath.reset();
        signedConfiguration.matchSourcePath.reset();
        DandrumAudioProcessor signedProcessor (signedConfiguration);
        signedProcessor.setPlayConfigDetails (0, 2, 48000.0, 64);
        signedProcessor.prepareToPlay (48000.0, 64);
        require (signedProcessor.isInstrumentLoaded(),
                 "Web signed knob fixture did not prepare");
        HostListener signedListener;
        signedProcessor.addListener (&signedListener);
        DandrumAudioProcessorEditor signedEditor (signedProcessor);
        const auto signedGeneration = signedProcessor.getParameterSurfaceGeneration();
        const auto expectedSlot = signedProcessor.getParameters().indexOf (
            signedProcessor.getParameterForPublicId ("fixture.level"));
        require (expectedSlot >= 0, "Web signed knob fixture has no stable host slot");
        require (PluginEditorBridgeTestProbe::invoke (
                     signedEditor, "beginGesture",
                     { juce::var ("fixture.level"),
                       juce::var (static_cast<int> (signedGeneration)) })
                     .getProperty ("status", {}).toString() == "accepted",
                 "Web signed knob gesture did not begin");
        juce::AudioBuffer<float> signedOutput (2, 64);
        juce::MidiBuffer signedMidi;
        const auto renderSigned = [&] (float expectedLeft)
        {
            signedOutput.clear();
            signedProcessor.processBlock (signedOutput, signedMidi);
            for (int channel = 0; channel < 2; ++channel)
                for (int frame = 0; frame < signedOutput.getNumSamples(); ++frame)
                    if (std::abs (signedOutput.getSample (channel, frame)
                                   - (channel == 0 ? expectedLeft : 0.0f)) > 0.00001f)
                        return false;
            return true;
        };
        require (PluginEditorBridgeTestProbe::invoke (
                     signedEditor, "setParameter",
                     { juce::var ("fixture.level"), juce::var (0.25),
                       juce::var (static_cast<int> (signedGeneration)) })
                     .getProperty ("status", {}).toString() == "accepted"
                     && renderSigned (-0.5f),
                 "Web knob did not render signed left -0.5/right 0 output");
        require (PluginEditorBridgeTestProbe::invoke (
                     signedEditor, "setParameter",
                     { juce::var ("fixture.level"), juce::var (0.75),
                       juce::var (static_cast<int> (signedGeneration)) })
                     .getProperty ("status", {}).toString() == "accepted"
                     && renderSigned (0.5f),
                 "Web knob did not render signed left +0.5/right 0 output");
        require (PluginEditorBridgeTestProbe::invoke (
                     signedEditor, "endGesture",
                     { juce::var ("fixture.level"),
                       juce::var (static_cast<int> (signedGeneration)) })
                     .getProperty ("status", {}).toString() == "accepted"
                     && signedListener.beginCount == 1 && signedListener.endCount == 1
                     && signedListener.lastGestureIndex == expectedSlot,
                 "Web signed knob schedule did not preserve the public host slot and one gesture");
        signedProcessor.removeListener (&signedListener);
        signedProcessor.releaseResources();

        processor.removeListener (&listener);
        processor.releaseResources();
        kick.releaseResources();
        changing.releaseResources();
    }
    catch (const std::exception& error)
    {
        std::cerr << error.what() << '\n';
        return 1;
    }
    return 0;
}
