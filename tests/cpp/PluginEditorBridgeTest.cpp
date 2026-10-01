#include "PluginEditor.h"

#include <array>
#include <atomic>
#include <chrono>
#include <cmath>
#include <csignal>
#include <iostream>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>

struct PluginEditorBridgeTestProbe
{
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

int main()
{
   #if JUCE_LINUX
    std::signal (SIGPIPE, SIG_IGN);
   #endif

    try
    {
        DandrumAudioProcessor processor (InstrumentDemoConfiguration::kick());
        processor.setPlayConfigDetails (0, 2, 48000.0, 64);
        processor.prepareToPlay (48000.0, 64);
        HostListener listener;
        processor.addListener (&listener);
        DandrumAudioProcessorEditor editor (processor);

        for (const auto* command : { "getParameters", "setParameter", "noteOn", "noteOff",
                                     "renderSoundLab", "chooseSoundLabReference", "matchSoundLab",
                                     "cancelSoundLab", "acceptSoundLabMatch", "requestGraphProposal",
                                     "getSoundLabAnalysis" })
            require (PluginEditorBridgeTestProbe::hasCommand (editor, command),
                     std::string ("configured editor did not register ") + command);

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

        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36 })
                     .toString().contains ("expects"),
                 "noteOn accepted missing velocity");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOff")
                     .toString().contains ("expects"),
                 "noteOff accepted missing note");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36, 0.8 }).isVoid()
                     && PluginEditorBridgeTestProbe::invoke (editor, "noteOff", { 36 }).isVoid(),
                 "playable keyboard commands did not enter the MIDI queue");
        bool rejected = false;
        for (int i = 0; i < 256; ++i)
            rejected = PluginEditorBridgeTestProbe::invoke (editor, "noteOn", { 36, 0.8 })
                           .toString().contains ("queue is full") || rejected;
        require (rejected && processor.getDroppedMidiEventCount() > 0,
                 "full MIDI queue was not reported to the browser");
        require (PluginEditorBridgeTestProbe::invoke (editor, "noteOff", { 36 })
                     .toString().contains ("queue is full"),
                 "full MIDI queue did not report a dropped note-off");

        const auto previousGeneration = processor.getParameterSurfaceGeneration();
        const auto tb303 = juce::File (juce::String (
            InstrumentDemoConfiguration::tb303().instrumentPath.string()));
        require (processor.reloadInstrumentFromFile (tb303),
                 "could not reload distinct instrument for browser refresh test");
        require (processor.getParameterSurfaceGeneration() != previousGeneration,
                 "replacement did not advance public surface generation");
        require (PluginEditorBridgeTestProbe::invoke (editor, "setParameter",
                     { juce::var (processor.getActivePublicParameterIds()[0]), juce::var (0.9),
                       juce::var (static_cast<int> (previousGeneration)) })
                     .toString().contains ("stale"),
                 "delayed browser command from the old instrument changed a replacement slot");
        PluginEditorBridgeTestProbe::refresh (editor);
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
        require (PluginEditorBridgeTestProbe::invoke (editor, "renderSoundLab").isVoid(),
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
                     && kickPageText.find ("/sound-lab-ui.js") != std::string::npos
                     && kickPageText != InstrumentDemoConfiguration::tb303().indexHtml,
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
        require (PluginEditorBridgeTestProbe::invoke (kickEditor, "renderSoundLab").isVoid(),
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
        require (PluginEditorBridgeTestProbe::invoke (kickEditor, "matchSoundLab").isVoid(),
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
        const auto matchedPatch = PluginEditorBridgeTestProbe::matchedPatch (kickEditor);
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
        require (matchedPatch.isNotEmpty()
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
