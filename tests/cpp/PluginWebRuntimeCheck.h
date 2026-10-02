#pragma once

#include "PluginEditor.h"

#include <algorithm>
#include <cstdlib>
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

// Observe the original production browser without replacing its Options or
// native functions. Return ASCII JSON strings because Unicode evaluation
// results can be truncated by vendored Linux JUCE's IPC character framing.
constexpr auto runtimeScript = R"JS(
(() => {
  if (window.__dandrumPackagedRuntime) return;
  window.__dandrumPackagedRuntime = [];
  const delay = () => new Promise(resolve => setTimeout(resolve, 25));
  const waitFor = async (predicate, failure) => {
    const deadline = Date.now() + 15000;
    while (!predicate()) {
      if (Date.now() > deadline) throw Error(failure);
      await delay();
    }
  };
  const report = async (name, data) => {
    if (window.__dandrumPackagedRuntime.length >= 4) throw Error('Runtime report capacity exceeded');
    window.__dandrumPackagedRuntime.push([name, data]);
  };
  const run = async () => {
    const native = window.__JUCE__.backend.getNativeFunction;
    try {
      await waitFor(() => document.querySelector('.control-value:not(:disabled),.knob-value:not(:disabled)'),
                    'Production editor controls did not become ready');
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
      const inspectIcons = () => [...document.querySelectorAll('svg[data-dd-icon]')].map(svg => {
        const bounds = svg.getBBox(), rect = svg.getBoundingClientRect();
        return {name:svg.dataset.ddIcon, size:Number(svg.getAttribute('width')),
                viewBox:svg.getAttribute('viewBox'), hidden:svg.getAttribute('aria-hidden'),
                focusable:svg.getAttribute('focusable'), stroke:getComputedStyle(svg).stroke,
                visible:rect.width > 0 && rect.height > 0 && bounds.width > 0 && bounds.height > 0};
      });
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      await report('fonts', {
        fonts, icons: inspectIcons(), parameters: state.parameters.map(p => p.id),
        errors: [...document.querySelectorAll('[role=alert]')].map(e => e.textContent),
        family: getComputedStyle(document.querySelector('main')).fontFamily,
        valueFamily: getComputedStyle(document.querySelector('.control-value,.knob-value')).fontFamily,
        resources: performance.getEntriesByType('resource').map(r => r.name.startsWith('data:') ? 'data:font' : r.name),
        viewportWidth: innerWidth, viewportHeight: innerHeight,
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
      }, 'Production editor did not display the full-size host update');
      await report('hostValue', {});
      await waitFor(() => innerWidth === 820 && innerHeight === 560,
                    'Production editor did not resize to the compact viewport');
      // Let ResizeObserver and React commit the layout at the new viewport size.
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      await report('compact', {
        icons: inspectIcons(),
        viewportWidth: innerWidth, viewportHeight: innerHeight,
        hintClear: sampler || document.querySelector('.hint').getBoundingClientRect().top + 0.5
                             >= document.querySelector('.machine').getBoundingClientRect().bottom,
        width: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth
      });
      await waitFor(() => {
        const e = document.querySelector(selector);
        return e && Math.abs(Number(e.value ?? e.getAttribute('aria-valuenow')) - 0.63) < 0.001;
      }, 'Production editor did not display the compact host update');
      await report('compactHostValue', {});
    } catch (error) { await report('error', {message:String(error)}); }
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', run, {once:true});
  else void run();
})();
)JS";

class Check final : private juce::Timer, public std::enable_shared_from_this<Check>
{
public:
    Check (DandrumAudioProcessor& host, bool isSampler) : processor (host), sampler (isSampler) {}
    void start (juce::WebBrowserComponent& view, juce::AudioProcessorEditor& owner)
    {
        browser = &view;
        editor = &owner;
        startTimer (50);
    }
    void cancel() { finished = true; stopTimer(); }
    int result = 1;

    void report (const juce::Array<juce::var>& args)
    {
        if (finished) return;
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
                editor->setSize (820, 560);
                phase = Phase::compact;
            }
            else if (phase == Phase::compact && name == "compact")
            {
                acceptIcons (data);
                require (static_cast<int> (data.getProperty ("viewportWidth", {})) == 820
                             && static_cast<int> (data.getProperty ("viewportHeight", {})) == 560,
                         "Original editor compact viewport is not 820 by 560");
                require (static_cast<int> (data.getProperty ("width", {})) > 0
                             && static_cast<int> (data.getProperty ("width", {})) <= 820
                             && static_cast<int> (data.getProperty ("scrollWidth", {}))
                                 <= static_cast<int> (data.getProperty ("width", {})), "Compact page overflows horizontally");
                require (static_cast<bool> (data.getProperty ("hintClear", {})), "Compact panel overlaps its hint");
                compactReport = data;
                processor.getParameterForPublicId (sampler ? "drums.pitch_ratio" : "filter.cutoff")
                    ->setValueNotifyingHost (0.63f);
                phase = Phase::compactHostValue;
            }
            else if (phase == Phase::compactHostValue && name == "compactHostValue")
            {
                std::cout << (sampler ? "SAMPLER" : "TB303") << " WebKit production-editor runtime: "
                          << juce::JSON::toString (fontReport, true)
                          << " compact: " << juce::JSON::toString (compactReport, true)
                          << " host updates: 0.37, 0.63" << std::endl;
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
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }

private:
    enum class Phase { fonts, hostValue, compact, compactHostValue, preview };
    DandrumAudioProcessor& processor;
    juce::WebBrowserComponent* browser = nullptr;
    juce::AudioProcessorEditor* editor = nullptr;
    bool sampler;
    bool finished = false;
    bool evaluationPending = false;
    Phase phase = Phase::fonts;
    double deadline = juce::Time::getMillisecondCounterHiRes() + 20000.0;
    juce::var fontReport;
    juce::var compactReport;

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
        if (evaluationPending || phase == Phase::preview) return;
        evaluationPending = true;
        const auto script = juce::String ("(() => { if (!document.querySelector('main')) return ''; ")
            + runtimeScript
            + "const report = window.__dandrumPackagedRuntime.shift(); "
              "return report ? JSON.stringify(report).replace(/[\\u007f-\\uffff]/g, "
              "c => '\\\\u' + c.charCodeAt(0).toString(16).padStart(4, '0')) : ''; })()";
        browser->evaluateJavascript (script, [weak = weak_from_this()] (auto evaluation)
        {
            const auto active = weak.lock();
            if (! active || active->finished) return;
            active->evaluationPending = false;
            if (const auto* error = evaluation.getError())
                active->finish (false, "Original editor observation failed: " + error->message);
            else if (const auto* value = evaluation.getResult(); value != nullptr && value->isString())
            {
                if (value->toString().isEmpty()) return;
                const auto report = juce::JSON::parse (value->toString());
                if (const auto* arguments = report.getArray()) active->report (*arguments);
                else active->finish (false, "Invalid original editor observation report");
            }
            else active->finish (false, "Original editor observation returned no string");
        });
    }
    void acceptFonts (const juce::var& data)
    {
        acceptIcons (data);
        require (static_cast<int> (data.getProperty ("viewportWidth", {})) == 1200
                     && static_cast<int> (data.getProperty ("viewportHeight", {})) == 800,
                 "Original editor full viewport is not 1200 by 800");
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
    void acceptIcons (const juce::var& data)
    {
        const auto iconList = data.getProperty ("icons", {});
        const auto* icons = iconList.getArray();
        require (icons != nullptr, "Packaged design icons were not reported");
        const auto expected = sampler ? juce::StringArray { "keyboard", "host", "level", "alternate", "choke", "lock" }
                                      : juce::StringArray { "keyboard", "midi", "level" };
        for (const auto& name : expected)
        {
            const auto found = std::find_if (icons->begin(), icons->end(), [&name] (const juce::var& icon)
            { return icon.getProperty ("name", {}).toString() == name; });
            require (found != icons->end(), "Missing packaged design icon: " + name);
        }
        for (const auto& icon : *icons)
            require (static_cast<bool> (icon.getProperty ("visible", {}))
                         && static_cast<int> (icon.getProperty ("size", {})) >= 12
                         && icon.getProperty ("viewBox", {}).toString() == "0 0 24 24"
                         && icon.getProperty ("hidden", {}).toString() == "true"
                         && icon.getProperty ("focusable", {}).toString() == "false"
                         && icon.getProperty ("stroke", {}).toString() != "none",
                     "Invisible or inaccessible decorative icon: " + icon.getProperty ("name", {}).toString());
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
        std::unique_ptr<juce::AudioProcessor> plugin (createPluginFilter());
        if (auto* host = dynamic_cast<DandrumAudioProcessor*> (plugin.get()))
        {
            plugin.release();
            processor.reset (host);
        }
        if (processor == nullptr) { setApplicationReturnValue (1); quit(); return; }
        processor->setPlayConfigDetails (0, 2, 48000.0, 64);
        processor->prepareToPlay (48000.0, 64);
        if (! processor->isInstrumentLoaded()) { setApplicationReturnValue (1); quit(); return; }
        auto* parameter = processor->getParameterForPublicId (Sampler ? "drums.pitch_ratio" : "filter.cutoff");
        require (parameter != nullptr, "Production editor fixture has no expected parameter");
        parameter->setValueNotifyingHost (0.21f);
        editor.reset (processor->createEditor());
        auto* webEditor = dynamic_cast<DandrumAudioProcessorEditor*> (editor.get());
        require (webEditor != nullptr, "Plugin factory did not create the production WebView editor");
        check = std::make_shared<Check> (*processor, Sampler);
        editor->setName (Sampler ? "Dandrum Sampler" : "Dandrum TB-303");
        editor->setSize (1200, 800);
        editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
        editor->setTopLeftPosition (30, 30);
        editor->setVisible (true);
        check->start (Probe::runtimeBrowser (*webEditor), *editor);
    }
    void shutdown() override
    {
        if (check) { setApplicationReturnValue (check->result); check->cancel(); }
        check.reset();
        editor.reset();
        processor.reset();
    }
private:
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
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
