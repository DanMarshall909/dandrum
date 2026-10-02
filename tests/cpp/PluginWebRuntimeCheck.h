#pragma once

#include "PluginEditor.h"

#include <cstdlib>
#include <functional>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <vector>

namespace packagedWebRuntime
{
inline bool available()
{
   #if JUCE_LINUX
    if (std::getenv ("DISPLAY") == nullptr)
    {
        std::cout << "SKIP: WebKit runtime requires an X display\n";
        return false;
    }
   #endif
    return true;
}

inline void require (bool condition, const juce::String& message)
{
    if (! condition) throw std::runtime_error (message.toStdString());
}

// Report through the native integration used by the production apps.
// Unicode evaluation results can be truncated by vendored Linux JUCE's IPC
// framing and misalign its callback FIFO. This report uses ASCII fields and
// native Promise IDs. Full plugin and DAW lifecycle checks remain separate.
constexpr auto runtimeScript = R"JS(
(() => {
  const delay = () => new Promise(resolve => setTimeout(resolve, 25));
  const waitFor = async predicate => {
    const deadline = Date.now() + 15000;
    while (!predicate()) {
      if (Date.now() > deadline) throw Error('Runtime readiness timed out');
      await delay();
    }
  };
  const run = async () => {
    const native = window.__JUCE__.backend.getNativeFunction;
    const report = native('reportPackagedRuntime');
    try {
      await waitFor(() => document.querySelector('.control-value:not(:disabled),.knob-value:not(:disabled)'));
      const sampler = !!document.querySelector('.sampler');
      await document.fonts.ready;
      const faces = [
        ['Barlow',500],['Barlow',600],['Barlow',700],
        ['Barlow Semi Condensed',500],['Barlow Semi Condensed',600],['Barlow Semi Condensed',700],
        ['JetBrains Mono',500],['JetBrains Mono',600]];
      const fonts = await Promise.all(faces.map(async ([family,weight]) => {
        const result = await document.fonts.load(`${weight} 14px "${family}"`);
        return {family,weight,matched:result.length,statuses:result.map(face => face.status),
                loaded:result.length === 1 && result[0].status === 'loaded'};
      }));
      const state = await native('getParameterState')();
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      await report('fonts', {
        fonts, parameters: state.parameters.map(p => p.id),
        errors: [...document.querySelectorAll('[role=alert]')].map(e => e.textContent),
        family: getComputedStyle(document.querySelector('main')).fontFamily,
        valueFamily: getComputedStyle(document.querySelector('.control-value,.knob-value')).fontFamily,
        resources: performance.getEntriesByType('resource').map(r => r.name.startsWith('data:') ? 'data:font' : r.name),
        width: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        layout: ['html','body','.stage','.machine-frame','.machine'].map(selector => {
          const e=document.querySelector(selector); if (!e) return {selector};
          const r=e.getBoundingClientRect(), s=getComputedStyle(e);
          return {selector,left:r.left,right:r.right,width:r.width,client:e.clientWidth,scroll:e.scrollWidth,
                  margin:s.margin,padding:s.padding,transform:s.transform};
        })
      });
      const selector = sampler ? 'input[type=range][aria-label="drums.pitch_ratio"]'
                               : '[role=slider][aria-label="CUT OFF FREQ"]';
      await waitFor(() => {
        const e = document.querySelector(selector);
        return e && Math.abs(Number(e.value ?? e.getAttribute('aria-valuenow')) - 0.37) < 0.001;
      });
      await report('hostValue', {});
      await waitFor(() => document.documentElement.clientWidth > 0 && document.documentElement.clientWidth <= 820);
      // Let ResizeObserver and React commit the layout at the new viewport size.
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      await report('compact', {
        hintClear: sampler || document.querySelector('.hint').getBoundingClientRect().top + 0.5
                             >= document.querySelector('.machine').getBoundingClientRect().bottom,
        width: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth
      });
    } catch (error) { await report('error', {message:String(error)}); }
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', run, {once:true});
  else void run();
})();
)JS";

class Check final : private juce::Timer
{
public:
    Check (DandrumAudioProcessor& host, bool isSampler) : processor (host), sampler (isSampler) {}
    void start (juce::WebBrowserComponent& view, std::function<void()> publish)
    {
        browser = &view;
        pump = std::move (publish);
        startTimer (50);
    }
    void cancel() { finished = true; stopTimer(); }
    int result = 1;

    void report (const juce::Array<juce::var>& args, juce::WebBrowserComponent::NativeFunctionCompletion completion)
    {
        if (finished) { completion (false); return; }
        try
        {
            require (args.size() == 2 && args[0].isString() && args[1].isObject(), "Invalid runtime report");
            const auto name = args[0].toString();
            const auto& data = args[1];
            require (name != "error", data.getProperty ("message", {}).toString());
            if (phase == Phase::fonts && name == "fonts")
            {
                fontReport = data;
                std::cout << "Font load report: " << juce::JSON::toString (data, true) << std::endl;
                acceptFonts (data);
                auto* parameter = processor.getParameterForPublicId (sampler ? "drums.pitch_ratio" : "filter.cutoff");
                require (parameter != nullptr, "Runtime fixture has no expected host control");
                parameter->setValueNotifyingHost (0.37f);
                phase = Phase::hostValue;
            }
            else if (phase == Phase::hostValue && name == "hostValue")
            {
                browser->setSize (820, 560);
                phase = Phase::compact;
            }
            else if (phase == Phase::compact && name == "compact")
            {
                require (static_cast<int> (data.getProperty ("width", {})) > 0
                             && static_cast<int> (data.getProperty ("width", {})) <= 820
                             && static_cast<int> (data.getProperty ("scrollWidth", {}))
                                 <= static_cast<int> (data.getProperty ("width", {})), "Compact page overflows horizontally");
                require (static_cast<bool> (data.getProperty ("hintClear", {})), "Compact panel overlaps its hint");
                std::cout << (sampler ? "SAMPLER" : "TB303") << " WebKit asset/adapter runtime: "
                          << juce::JSON::toString (fontReport, true) << std::endl;
                completion (true);
                if (const auto* preview = std::getenv ("DANDRUM_WEB_RUNTIME_PREVIEW"))
                    if (juce::String (preview) == "1")
                    {
                        phase = Phase::preview;
                        deadline = juce::Time::getMillisecondCounterHiRes() + 15000.0;
                        return;
                    }
                finish (true);
                return;
            }
            else throw std::runtime_error ("Unexpected runtime report phase");
            completion (true);
        }
        catch (const std::exception& error) { completion (false); finish (false, error.what()); }
    }

private:
    enum class Phase { fonts, hostValue, compact, preview };
    DandrumAudioProcessor& processor;
    juce::WebBrowserComponent* browser = nullptr;
    std::function<void()> pump;
    bool sampler;
    bool finished = false;
    Phase phase = Phase::fonts;
    double deadline = juce::Time::getMillisecondCounterHiRes() + 20000.0;
    juce::var fontReport;

    void finish (bool success, const juce::String& error = {})
    {
        finished = true;
        stopTimer();
        result = success ? 0 : 1;
        if (! success) std::cerr << "Packaged WebKit runtime failed: " << error << '\n';
        juce::JUCEApplicationBase::getInstance()->setApplicationReturnValue (result);
        juce::MessageManager::getInstance()->stopDispatchLoop();
    }
    void timerCallback() override
    {
        if (juce::Time::getMillisecondCounterHiRes() > deadline)
        {
            if (phase == Phase::preview) finish (true);
            else finish (false, "WebKit timed out in phase " + juce::String (static_cast<int> (phase)));
            return;
        }
        pump();
    }
    void acceptFonts (const juce::var& data)
    {
        const auto fontList = data.getProperty ("fonts", {});
        const auto* fonts = fontList.getArray();
        require (fonts != nullptr && fonts->size() == 8, "WebKit did not enumerate eight font faces");
        for (const auto& font : *fonts)
            require (static_cast<bool> (font.getProperty ("loaded", {})),
                     "Embedded font failed to load: " + font.getProperty ("family", {}).toString());
        const auto parameters = data.getProperty ("parameters", {});
        require (parameters.isArray() && parameters.size() == (sampler ? 23 : 7), "App did not obtain the prepared host surface");
        require (data.getProperty ("errors", {}).size() == 0, "Packaged app displayed a host error");
        require (data.getProperty ("family", {}).toString().contains ("Barlow Semi Condensed"), "App is not using the licensed UI font");
        require (data.getProperty ("valueFamily", {}).toString().contains ("JetBrains Mono"), "Values are not using the licensed numeric font");
        const auto resources = data.getProperty ("resources", {});
        require (resources.isArray(), "WebKit resource report is missing");
        for (const auto& resource : *resources.getArray())
            require (resource.toString().startsWith (juce::WebBrowserComponent::getResourceProviderRoot())
                         || resource.toString() == "data:font", "External resource requested: " + resource.toString());
        require (static_cast<int> (data.getProperty ("width", {})) > 0
                     && static_cast<int> (data.getProperty ("width", {})) <= 1200
                     && static_cast<int> (data.getProperty ("scrollWidth", {}))
                         <= static_cast<int> (data.getProperty ("width", {})), "Full-size page overflows horizontally");
    }
};

template<bool Sampler, typename Probe>
class Application final : public juce::JUCEApplication
{
public:
    const juce::String getApplicationName() override { return "Dandrum Web Runtime Test"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise (const juce::String&) override
    {
        if constexpr (Sampler)
        {
            std::unique_ptr<juce::AudioProcessor> plugin (createPluginFilter());
            if (auto* host = dynamic_cast<DandrumAudioProcessor*> (plugin.get())) { plugin.release(); processor.reset (host); }
        }
        else processor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::tb303());
        if (processor == nullptr) { setApplicationReturnValue (1); quit(); return; }
        processor->setPlayConfigDetails (0, 2, 48000.0, 64);
        processor->prepareToPlay (48000.0, 64);
        if (! processor->isInstrumentLoaded()) { setApplicationReturnValue (1); quit(); return; }
        editor = std::make_unique<DandrumAudioProcessorEditor> (*processor);
        check = std::make_shared<Check> (*processor, Sampler);
        auto options = Probe::runtimeOptions (*editor)
            .withUserScript (runtimeScript)
            .withNativeFunction ("reportPackagedRuntime", [weak = std::weak_ptr<Check> (check)] (const auto& args, auto completion)
            {
                if (const auto active = weak.lock()) active->report (args, std::move (completion));
                else completion (false);
            });
        browser = std::make_unique<juce::WebBrowserComponent> (options);
        browser->setName (Sampler ? "Dandrum Sampler" : "Dandrum TB-303");
        browser->setSize (1200, 800);
        browser->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
        browser->setTopLeftPosition (30, 30);
        browser->setVisible (true);
        browser->goToURL (juce::WebBrowserComponent::getResourceProviderRoot());
        check->start (*browser, [this] { Probe::publishRuntimeUpdates (*editor, *browser); });
    }
    void shutdown() override
    {
        if (check) { setApplicationReturnValue (check->result); check->cancel(); }
        browser.reset();
        check.reset();
        editor.reset();
        processor.reset();
    }
private:
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<DandrumAudioProcessorEditor> editor;
    std::unique_ptr<juce::WebBrowserComponent> browser;
    std::shared_ptr<Check> check;
};

template<bool Sampler, typename Probe>
int main (int argc, char** argv)
{
    if (! available()) return 77;
    // Linux WebKit needs an executable JUCE application child entrypoint.
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase* { return new Application<Sampler, Probe>(); };
    std::vector<const char*> arguments (argv, argv + argc);
    arguments.push_back (nullptr);
    return juce::JUCEApplicationBase::main (argc, arguments.data());
}
}
