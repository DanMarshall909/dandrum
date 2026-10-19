#pragma once

#include "PluginEditor.h"

#include <array>
#include <atomic>
#include <cmath>
#include <csignal>
#include <cstdlib>
#include <string>
#include <utility>
#include <vector>

struct PluginEditorBridgeTestProbe
{
    static juce::WebBrowserComponent& browser (DandrumAudioProcessorEditor& editor)
    {
        return editor.browser;
    }
};

class TriggerWebRuntimeApplication final : public juce::JUCEApplication, private juce::Timer
{
public:
    const juce::String getApplicationName() override { return "Trigger WebView Check"; }
    const juce::String getApplicationVersion() override { return "1"; }

    void initialise (const juce::String&) override
    {
        processor = makeProcessor();
        unaffectedProcessor = makeProcessor();
        if (! signedOutputIs (*processor, 36, -0.5f)
            || ! signedOutputIs (*processor, 60, 0.0f)
            || ! signedOutputIs (*unaffectedProcessor, 36, -0.5f)
            || ! signedOutputIs (*unaffectedProcessor, 60, 0.0f))
        {
            finish (false, "two native instances did not start with independent signed output");
            return;
        }
        openEditor (sizes[0].first, sizes[0].second);
        deadline = juce::Time::getMillisecondCounterHiRes() + 30000;
        startTimer (100);
    }

    void shutdown() override
    {
        stopTimer();
        editor.reset();
        processor.reset();
        unaffectedProcessor.reset();
    }

private:
    enum class Phase { layout, admitLostReply, waitForEdit, reconnect, waitForUndo };

    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<DandrumAudioProcessor> unaffectedProcessor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    double deadline = 0;
    bool pending = false;
    bool editorDestroyed = false;
    std::uint64_t editRequest = 0;
    std::uint64_t undoRequest = 0;
    Phase phase = Phase::layout;
    std::size_t viewport = 0;
    const std::array<std::pair<int, int>, 3> sizes {{ {1180, 860}, {820, 560}, {1500, 1000} }};
    struct Mode { const char* label; int width; int height; };
    const std::array<Mode, 5> modes {{ {"Min", 820, 560}, {"Default", 1200, 800},
        {"Expanded", 1600, 1000}, {"Collapse", 900, 126}, {"Open editor", 1600, 1000} }};
    std::size_t mode = 0;
    double resizeDeadline = 0;

    static std::unique_ptr<DandrumAudioProcessor> makeProcessor()
    {
        auto result = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::sampler());
        result->setPlayConfigDetails (0, 2, 48000, 64);
        result->prepareToPlay (48000, 64);
        return result;
    }

    static float hit (DandrumAudioProcessor& target, int note)
    {
        juce::AudioBuffer<float> buffer (2, 64);
        juce::MidiBuffer empty;
        for (int block = 0; block < 1000; ++block)
        {
            buffer.clear();
            target.processBlock (buffer, empty);
        }
        buffer.clear();
        juce::MidiBuffer midi;
        midi.addEvent (juce::MidiMessage::noteOn (1, note, juce::uint8 (100)), 0);
        target.processBlock (buffer, midi);
        return buffer.getSample (0, 0);
    }

    static bool signedOutputIs (DandrumAudioProcessor& target, int note, float expected)
    {
        return std::abs (hit (target, note) - expected) < 0.0001f;
    }

    void openEditor (int width, int height)
    {
        editor.reset (processor->createEditorIfNeeded());
        if (dynamic_cast<DandrumAudioProcessorEditor*> (editor.get()) == nullptr)
        {
            finish (false, "no integrated browser editor");
            return;
        }
        editor->setSize (width, height);
        editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
        editor->setVisible (true);
    }

    DandrumAudioProcessorEditor* webEditor() const
    {
        return dynamic_cast<DandrumAudioProcessorEditor*> (editor.get());
    }

    void chooseMode()
    {
        resizeDeadline = juce::Time::getMillisecondCounterHiRes() + 4000;
        const auto script = "(() => { const button=Array.from(document.querySelectorAll('header button'))"
            ".find(button=>button.textContent.trim()===" + juce::JSON::toString (modes[mode].label)
            + "); if(button)button.click(); })()";
        PluginEditorBridgeTestProbe::browser (*webEditor()).evaluateJavascript (script);
    }

    void finish (bool success, const juce::String& message)
    {
        stopTimer();
        std::cout << (success ? "PASS: " : "FAIL: ") << message << '\n';
        setApplicationReturnValue (success ? 0 : 1);
        quit();
    }

    template<typename Callback>
    void evaluate (const juce::String& script, Callback&& callback)
    {
        auto* activeEditor = webEditor();
        if (activeEditor == nullptr)
        {
            finish (false, "native WebView was unavailable during lifecycle proof");
            return;
        }
        pending = true;
        PluginEditorBridgeTestProbe::browser (*activeEditor).evaluateJavascript (
            script, [this, callback = std::forward<Callback> (callback)] (auto result) mutable
            {
                pending = false;
                if (const auto* error = result.getError())
                {
                    finish (false, error->message);
                    return;
                }
                const auto* value = result.getResult();
                if (value != nullptr && value->isString()) callback (value->toString());
            });
    }

    void beginLifecycle()
    {
        phase = Phase::admitLostReply;
        resizeDeadline = 0;
        deadline = juce::Time::getMillisecondCounterHiRes() + 30000;
    }

    void pollLayout()
    {
        if (resizeDeadline > 0 && juce::Time::getMillisecondCounterHiRes() > resizeDeadline)
        {
            finish (false, "Native size choice did not resize the editor window");
            return;
        }
        juce::AudioBuffer<float> audio (2, 64);
        audio.clear();
        juce::MidiBuffer midi;
        processor->processBlock (audio, midi);
        const auto script = R"JS((() => {
          const root=document.querySelector('[data-testid="trigger-editor"]');
          const backend=document.querySelector('[data-backend="native"]');
          const failure=document.querySelector('[role="alert"]');
          const bounds=root?.getBoundingClientRect();
          return JSON.stringify({ready:!!root&&!!backend&&document.body.textContent.includes('Advanced Drum Kit')
            &&document.fonts.status==='loaded',
            headerBackend:!!root?.querySelector('header')?.contains(backend),
            viewportWidth:innerWidth,viewportHeight:innerHeight,
            fits:!!bounds&&bounds.left>=-0.5&&bounds.top>=-0.5&&bounds.right<=innerWidth+0.5&&bounds.bottom<=innerHeight+0.5
              &&document.documentElement.scrollWidth<=innerWidth,
            failure:failure?.textContent??'',sliders:document.querySelectorAll('[role="slider"]').length});
        })())JS";
        evaluate (script, [this] (const juce::String& value)
        {
            const auto report = juce::JSON::parse (value);
            if (report["failure"].toString().isNotEmpty())
            {
                finish (false, report["failure"].toString());
                return;
            }
            if (! static_cast<bool> (report["ready"])) return;
            if (editor->getName() != "trigger - Advanced Drum Kit")
            {
                finish (false, "native window title must follow the accepted patch name");
                return;
            }
            const int expectedWidth = viewport < sizes.size() ? sizes[viewport].first : modes[mode].width;
            const int expectedHeight = viewport < sizes.size() ? sizes[viewport].second : modes[mode].height;
            if (static_cast<int> (report["viewportWidth"]) != expectedWidth
                || static_cast<int> (report["viewportHeight"]) != expectedHeight
                || editor->getWidth() != expectedWidth || editor->getHeight() != expectedHeight)
                return;
            if (! static_cast<bool> (report["headerBackend"]))
            {
                finish (false, "native connection status must stay inside the sampler header");
                return;
            }
            if (! static_cast<bool> (report["fits"]))
            {
                finish (false, "full sampler must fit the native WebView without horizontal overflow");
                return;
            }
            if (viewport < sizes.size())
            {
                if (static_cast<int> (report["sliders"]) == 0)
                {
                    finish (false, "full sampler controls are unavailable");
                    return;
                }
                if (++viewport == sizes.size()) chooseMode();
                else editor->setSize (sizes[viewport].first, sizes[viewport].second);
            }
            else if (++mode == modes.size()) beginLifecycle();
            else chooseMode();
        });
    }

    void admitLostReply()
    {
        const auto script = R"JS((() => {
          if (!window.__dandrumLostReplyStarted) {
            window.__dandrumLostReplyStarted=true;
            void (async()=>{try{
              const native=window.__JUCE__.backend.getNativeFunction;
              const session=await native('getSamplerSession')();
              const connection=session.connection;
              void native('setInstrumentZoneKeyRange')(connection.instanceId,connection.attachmentId,
                connection.nextRequestId,connection.revision,'kit','kick',60,60);
              window.__dandrumLostReply=JSON.stringify({request:connection.nextRequestId});
            }catch(error){window.__dandrumLostReply=JSON.stringify({error:String(error?.message??error)});}})();
          }
          return window.__dandrumLostReply??'';
        })())JS";
        evaluate (script, [this] (const juce::String& value)
        {
            if (value.isEmpty()) return;
            const auto report = juce::JSON::parse (value);
            if (report["error"].toString().isNotEmpty())
            {
                finish (false, report["error"].toString());
                return;
            }
            try { editRequest = std::stoull (report["request"].toString().toStdString()); }
            catch (...) { finish (false, "lost-reply script returned an invalid request identity"); return; }
            phase = Phase::waitForEdit;
        });
    }

    void waitForEdit()
    {
        const auto status = processor->getInstrumentSessionJobStatus (editRequest);
        if (! status) return;
        if (! editorDestroyed)
        {
            editor.reset();
            editorDestroyed = true;
        }
        if (status->state == DandrumAudioProcessor::UiJobState::running) return;
        if (status->state != DandrumAudioProcessor::UiJobState::completed
            || status->revision != 1 || status->undoCount != 1)
        {
            finish (false, "admitted edit did not survive complete editor destruction exactly once");
            return;
        }
        const auto unaffected = unaffectedProcessor->getInstrumentSessionSnapshot();
        if (! signedOutputIs (*processor, 60, -0.5f) || ! signedOutputIs (*processor, 36, 0.0f)
            || ! signedOutputIs (*unaffectedProcessor, 36, -0.5f)
            || ! signedOutputIs (*unaffectedProcessor, 60, 0.0f)
            || ! unaffected || unaffected->revision != 0 || unaffected->undoCount != 0)
        {
            finish (false, "lost-reply edit changed signed output or the second instance");
            return;
        }
        openEditor (1180, 860);
        phase = Phase::reconnect;
    }

    void reconnectAndUndo()
    {
        const auto script = R"JS((() => {
          const ready=document.querySelector('[data-testid="trigger-editor"]')
            &&document.querySelector('[data-backend="native"]')&&!document.querySelector('[role="alert"]');
          if (!ready) return '';
          if (!window.__dandrumRecoveredUndoStarted) {
            window.__dandrumRecoveredUndoStarted=true;
            void (async()=>{try{
              const native=window.__JUCE__.backend.getNativeFunction;
              const session=await native('getSamplerSession')();
              const connection=session.connection;
              const kick=session.document.maps?.find(map=>map.id==='kit')?.zones?.find(zone=>zone.id==='kick');
              const undo=Array.from(document.querySelectorAll('header button'))
                .find(button=>button.getAttribute('aria-label')?.startsWith('Undo Change Key Range'));
              if(connection.revision!=='1'||connection.history?.undoCount!==1
                  ||kick?.keyLow!==60||kick?.keyHigh!==60||!undo||undo.disabled)
                throw Error('Recreated React editor did not recover mapping and native undo');
              undo.click();
              window.__dandrumRecoveredUndo=JSON.stringify({request:connection.nextRequestId});
            }catch(error){window.__dandrumRecoveredUndo=JSON.stringify({error:String(error?.message??error)});}})();
          }
          return window.__dandrumRecoveredUndo??'';
        })())JS";
        evaluate (script, [this] (const juce::String& value)
        {
            if (value.isEmpty()) return;
            const auto report = juce::JSON::parse (value);
            if (report["error"].toString().isNotEmpty())
            {
                finish (false, report["error"].toString());
                return;
            }
            try { undoRequest = std::stoull (report["request"].toString().toStdString()); }
            catch (...) { finish (false, "recreated editor returned an invalid undo request identity"); return; }
            phase = Phase::waitForUndo;
        });
    }

    void waitForUndo()
    {
        const auto status = processor->getInstrumentSessionJobStatus (undoRequest);
        if (! status || status->state == DandrumAudioProcessor::UiJobState::running) return;
        const auto unaffected = unaffectedProcessor->getInstrumentSessionSnapshot();
        if (status->state != DandrumAudioProcessor::UiJobState::completed
            || status->revision != 2 || status->undoCount != 0
            || ! signedOutputIs (*processor, 36, -0.5f) || ! signedOutputIs (*processor, 60, 0.0f)
            || ! signedOutputIs (*unaffectedProcessor, 36, -0.5f)
            || ! signedOutputIs (*unaffectedProcessor, 60, 0.0f)
            || ! unaffected || unaffected->revision != 0 || unaffected->undoCount != 0)
        {
            finish (false, "recovered React undo did not restore independent signed output");
            return;
        }
        finish (true, "packaged WebView reconnects after lost reply and recovers independent native undo");
    }

    void timerCallback() override
    {
        if (juce::Time::getMillisecondCounterHiRes() > deadline)
        {
            finish (false, "packaged WebView lifecycle proof timed out");
            return;
        }
        if (pending) return;
        switch (phase)
        {
            case Phase::layout: pollLayout(); break;
            case Phase::admitLostReply: admitLostReply(); break;
            case Phase::waitForEdit: waitForEdit(); break;
            case Phase::reconnect: reconnectAndUndo(); break;
            case Phase::waitForUndo: waitForUndo(); break;
        }
    }
};

inline int runTriggerWebRuntime (int argc, char** argv)
{
   #if JUCE_LINUX
    std::signal (SIGPIPE, SIG_IGN);
    if (std::getenv ("DISPLAY") == nullptr)
    {
        std::cout << "SKIP: Trigger WebKit lifecycle requires an X display\n";
        return 77;
    }
   #endif
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase*
    {
        return new TriggerWebRuntimeApplication();
    };
    std::vector<const char*> args (argv, argv + argc);
    args.push_back (nullptr);
    return juce::JUCEApplicationBase::main (argc, args.data());
}
