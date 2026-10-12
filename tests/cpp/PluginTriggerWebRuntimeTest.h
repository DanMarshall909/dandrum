#pragma once

#include "PluginEditor.h"
#include <array>
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
        processor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::sampler());
        processor->setPlayConfigDetails (0, 2, 48000, 64);
        processor->prepareToPlay (48000, 64);
        editor.reset (processor->createEditorIfNeeded());
        editor->setSize (sizes[0].first, sizes[0].second);
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
    std::size_t viewport = 0;
    const std::array<std::pair<int, int>, 3> sizes {{ {1180, 860}, {820, 560}, {1500, 1000} }};
    struct Mode { const char* label; int width; int height; };
    const std::array<Mode, 5> modes {{ {"Min", 820, 560}, {"Default", 1200, 800},
        {"Expanded", 1600, 1000}, {"Collapse", 900, 126}, {"Open editor", 1600, 1000} }};
    std::size_t mode = 0;
    double resizeDeadline = 0;
    void chooseMode()
    {
        resizeDeadline = juce::Time::getMillisecondCounterHiRes() + 4000;
        auto& browser = PluginEditorBridgeTestProbe::browser (
            *static_cast<DandrumAudioProcessorEditor*> (editor.get()));
        const auto script = "(() => { const button=Array.from(document.querySelectorAll('header button'))"
            ".find(button=>button.textContent.trim()===" + juce::JSON::toString (modes[mode].label)
            + "); if(button)button.click(); })()";
        browser.evaluateJavascript (script);
    }
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
        if (resizeDeadline > 0 && juce::Time::getMillisecondCounterHiRes() > resizeDeadline)
        {
            finish (false, "Native size choice did not resize the editor window");
            return;
        }
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
          const bounds=root?.getBoundingClientRect();
          return JSON.stringify({ready:!!root&&!!backend&&document.body.textContent.includes('Advanced Drum Kit')
            &&document.fonts.status==='loaded',
            headerBackend:!!root?.querySelector('header')?.contains(backend),
            viewportWidth:innerWidth,viewportHeight:innerHeight,
            fits:!!bounds&&bounds.left>=-0.5&&bounds.top>=-0.5&&bounds.right<=innerWidth+0.5&&bounds.bottom<=innerHeight+0.5
              &&document.documentElement.scrollWidth<=innerWidth,
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
                {
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
                    else if (++mode == modes.size())
                        finish (true, "native header fits and React size choices resize the real editor window");
                    else chooseMode();
                }
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
