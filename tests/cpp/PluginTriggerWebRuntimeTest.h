#pragma once

#include "PluginEditor.h"
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
        processor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::sampler());
        processor->setPlayConfigDetails (0, 2, 48000, 64);
        processor->prepareToPlay (48000, 64);
        editor.reset (processor->createEditor());
        editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
        editor->setVisible (true);
        deadline = juce::Time::getMillisecondCounterHiRes() + 20000;
        startTimer (100);
    }
    void shutdown() override
    {
        stopTimer();
        editor.reset();
        processor.reset();
    }
private:
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    double deadline = 0;
    bool pending = false;
    void finish (bool success, const juce::String& message)
    {
        stopTimer();
        std::cout << (success ? "PASS: " : "FAIL: ") << message << '\n';
        setApplicationReturnValue (success ? 0 : 1);
        quit();
    }
    void timerCallback() override
    {
        if (juce::Time::getMillisecondCounterHiRes() > deadline)
        {
            finish (false, "embedded full shell did not become ready");
            return;
        }
        if (pending) return;
        juce::AudioBuffer<float> audio (2, 64);
        audio.clear();
        juce::MidiBuffer midi;
        processor->processBlock (audio, midi);
        auto* webEditor = dynamic_cast<DandrumAudioProcessorEditor*> (editor.get());
        if (webEditor == nullptr) { finish (false, "no integrated browser editor"); return; }
        pending = true;
        const auto script = R"JS((() => {
          const root=document.querySelector('[data-testid="trigger-editor"]');
          const backend=document.querySelector('[data-backend="native"]');
          const failure=document.querySelector('[role="alert"]');
          return JSON.stringify({ready:!!root&&!!backend&&document.body.textContent.includes('Advanced Drum Kit')
            &&document.querySelectorAll('[role="slider"]').length>0&&document.fonts.status==='loaded',
            failure:failure?.textContent??'',sliders:document.querySelectorAll('[role="slider"]').length});
        })())JS";
        PluginEditorBridgeTestProbe::browser (*webEditor).evaluateJavascript (script,
            [this] (auto evaluation)
            {
                pending = false;
                if (const auto* error = evaluation.getError())
                {
                    finish (false, error->message);
                    return;
                }
                const auto* value = evaluation.getResult();
                if (value == nullptr || ! value->isString()) return;
                const auto report = juce::JSON::parse (value->toString());
                if (report["failure"].toString().isNotEmpty())
                    finish (false, report["failure"].toString());
                else if (static_cast<bool> (report["ready"]))
                    finish (true, "full local React shell, native session, controls and fonts loaded in production WebKit");
            });
    }
};

inline int runTriggerWebRuntime (int argc, char** argv)
{
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase*
    {
        return new TriggerWebRuntimeApplication();
    };
    std::vector<const char*> args (argv, argv + argc);
    args.push_back (nullptr);
    return juce::JUCEApplicationBase::main (argc, args.data());
}
