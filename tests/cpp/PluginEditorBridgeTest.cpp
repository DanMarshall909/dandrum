#include "PluginEditor.h"
#include "DefaultPatch.h"

#include <atomic>
#include <chrono>
#include <cmath>
#include <csignal>
#include <iostream>
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
        juce::var result;
        bool completed = false;
        auto completion = [&] (juce::var value) { result = std::move (value); completed = true; };
        bool invoked = false;
        for (auto& [name, callback] : editor.hostBridge.nativeFunctions())
            if (name.toString() == command)
            {
                callback (arguments, completion);
                invoked = true;
                break;
            }
        if (! invoked)
        {
            if (command == "renderSoundLab")
                editor.renderSoundLabFromWeb (arguments, completion);
            else if (command == "getSoundLabAnalysis")
                editor.getSoundLabAnalysisForWeb (arguments, completion);
            else
                throw std::runtime_error ("unknown test command");
        }
        if (! completed)
            throw std::runtime_error ("native command did not complete synchronously");
        return result;
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
    int changeCount = 0;
    int lastIndex = -1;
    float lastValue = -1.0f;
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
        snapshot = PluginEditorBridgeTestProbe::invoke (editor, "getParameters");
        require (std::abs (static_cast<float> (findParameter (snapshot, "kick.tune_hz")
                                                  .getProperty ("value", {})) - 0.37f) < 0.00001f,
                 "getParameters did not reflect the value set through the bridge");

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
            dandrum::findRepositoryExample ("examples/patches/tb303-acid.yaml").string()));
        require (processor.reloadInstrumentFromFile (tb303),
                 "could not reload distinct instrument for browser refresh test");
        require (processor.getParameterSurfaceGeneration() != previousGeneration,
                 "replacement did not advance public surface generation");
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
                     && kickPageText != InstrumentDemoConfiguration::tb303().indexHtml,
                 "second demo did not serve a distinct page through the same resource provider");
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
