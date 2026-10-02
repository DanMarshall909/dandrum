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
      await waitFor(() => document.querySelector('.dd-knob[aria-disabled="false"]'),
                    'Production editor controls did not become ready');
      const sampler = !!document.querySelector('.sampler');
      const control = document.querySelector(sampler
        ? '[role=slider][aria-label="drums.pitch_ratio"]'
        : '[role=slider][aria-label="CUT OFF FREQ"]');
      if (!control?.classList.contains('dd-knob'))
        throw Error('Supplied design-system knob did not replace the production control');
      if (!control.querySelector('[data-dd-knob-part=cap]')
          || !control.querySelector('[data-dd-knob-part=value-arc]')
          || control.querySelector('.knob-indicator,input[type=range]'))
        throw Error('Production knob does not have the pointer-free cap and value arc');
      control.focus();
      await waitFor(() => control.querySelector('[data-dd-knob-value]'),
                    'Focused knob did not open its editable value popup');
      const initialActual = Number(control.querySelector('[data-dd-knob-value]').textContent);
      if (Math.abs(initialActual - (sampler ? 1.77875 : 0.2048)) > 0.00001)
        throw Error('Knob popup does not display the loaded actual parameter range');
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
        valueFamily: getComputedStyle(control.querySelector('[data-dd-knob-value]')).fontFamily,
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
      const selector = sampler ? '[role=slider][aria-label="drums.pitch_ratio"]'
                               : '[role=slider][aria-label="CUT OFF FREQ"]';
      await waitFor(() => {
        const e = document.querySelector(selector);
        return e && Math.abs(Number(e.value ?? e.getAttribute('aria-valuenow')) - 0.37) < 0.001;
      }, 'Production editor did not display the full-size host update');
      await report('hostValue', {});
      await waitFor(() => innerWidth === 820 && innerHeight === 560,
                    'Production editor did not resize to the compact viewport');
      // Observe ResizeObserver/React completion instead of assuming a frame count.
      await waitFor(() => sampler ||
        Math.abs(document.querySelector('.machine').getBoundingClientRect().width
          - document.querySelector('.machine-frame').getBoundingClientRect().width) < 3,
        'Production panel did not finish scaling to the compact viewport');
      await report('compact', {
        icons: inspectIcons(),
        viewportWidth: innerWidth, viewportHeight: innerHeight,
        hintClear: sampler || document.querySelector('.hint').getBoundingClientRect().top + 0.5
                             >= document.querySelector('.machine').getBoundingClientRect().bottom,
        width: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        overflow: [...document.querySelectorAll('body *')].map(e => {
          const r = e.getBoundingClientRect();
          return {tag:e.tagName,classes:e.className?.baseVal ?? e.className,left:r.left,right:r.right};
        }).filter(e => e.right > innerWidth + 0.5 || e.left < -0.5).slice(0,20)
      });
      await waitFor(() => {
        const e = document.querySelector(selector);
        return e && Math.abs(Number(e.value ?? e.getAttribute('aria-valuenow')) - 0.63) < 0.001;
      }, 'Production editor did not display the compact host update');
      await report('compactHostValue', {});
      // Exercise the shipped React handlers, not native command shortcuts.
      const knob = () => document.querySelector(selector);
      const publicId = sampler ? 'drums.pitch_ratio' : 'filter.cutoff';
      const defaultValue = sampler ? 1 / 9 : (0.4 - 0.02) / (0.9 - 0.02);
      let expectedGestures = 0;
      const assertHostValue = async (expected, name, writes = true) => {
        if (writes) ++expectedGestures;
        const deadline = Date.now() + 5000;
        while (true) {
          const host = await native('getParameterState')();
          const value = host.parameters.find(p => p.id === publicId)?.value;
          if (Math.abs(value - expected) < 0.000001
              && Math.abs(Number(knob().getAttribute('aria-valuenow')) - expected) < 0.000001
              && window.__dandrumKnobGestures?.ends === expectedGestures) return;
          if (Date.now() > deadline) throw Error('Knob host binding failed: ' + name);
          await delay();
        }
      };
      const key = (key, shiftKey = false) => knob().dispatchEvent(
        new KeyboardEvent('keydown', {key, shiftKey, bubbles:true, cancelable:true}));
      const wheel = (deltaY, shiftKey = false) => {
        const event = new WheelEvent('wheel', {deltaY, shiftKey, bubbles:true, cancelable:true});
        knob().dispatchEvent(event);
        if (!event.defaultPrevented) throw Error('Knob wheel did not prevent page scrolling');
      };
      knob().focus();
      key('ArrowUp'); await assertHostValue(0.68, 'coarse keyboard');
      key('ArrowLeft', true); await assertHostValue(0.67, 'fine keyboard');
      key('Home'); await assertHostValue(0, 'minimum');
      key('End'); await assertHostValue(1, 'maximum');
      key('Delete'); await assertHostValue(defaultValue, 'loaded default');
      key('Backspace'); await assertHostValue(defaultValue, 'Backspace default');
      wheel(-1); await assertHostValue(defaultValue + 0.02, 'wheel');
      wheel(1, true); await assertHostValue(defaultValue + 0.01, 'fine wheel');
      wheel(0);
      knob().dispatchEvent(new MouseEvent('dblclick', {bubbles:true}));
      await assertHostValue(defaultValue, 'double click default');
      const type = async (text, finish = 'Enter') => {
        key('Enter');
        await waitFor(() => knob().querySelector('.dd-knob-input'), 'Actual value input did not open');
        if (text !== null) {
          const input = knob().querySelector('.dd-knob-input');
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, text);
          input.dispatchEvent(new Event('input', {bubbles:true}));
        }
        knob().querySelector('.dd-knob-input').dispatchEvent(
          new KeyboardEvent('keydown', {key:finish, bubbles:true, cancelable:true}));
        await waitFor(() => !knob().querySelector('.dd-knob-input'), 'Actual value input did not close');
      };
      await type(sampler ? '4.0625' : '0.46'); await assertHostValue(0.5, 'typed actual value');
      await type('invalid'); await assertHostValue(0.5, 'invalid entry restores host value', false);
      await type(sampler ? '7' : '0.8', 'Escape'); await assertHostValue(0.5, 'Escape cancels entry', false);
      await type(null); await assertHostValue(0.5, 'unchanged readout preserves precise value', false);
      key('Enter');
      await waitFor(() => knob().querySelector('.dd-knob-input'), 'Blur value input did not open');
      const input = knob().querySelector('.dd-knob-input');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, sampler ? '6.03125' : '0.68');
      input.dispatchEvent(new Event('input', {bubbles:true}));
      const other = [...document.querySelectorAll('.dd-knob')].find(e => e !== knob());
      other.focus();
      await assertHostValue(0.75, 'blur commits actual value');
      if (document.activeElement !== other) throw Error('Actual value blur trapped keyboard focus');
      if (!sampler) {
        await report('interactions', {value:0.75, gestures:11});
        return;
      }
      // The sampler is the owning real-host overlap regression for the shared
      // component; the TB-303 run covers its distinct command/range bindings.
      let pointer = null;
      document.addEventListener('pointermove', event => { if (event.isTrusted) pointer = event.pointerId; }, {once:true});
      const bounds = knob().querySelector('svg').getBoundingClientRect();
      await report('pointerReady', {x:bounds.x + bounds.width / 2, y:bounds.y + bounds.height / 2});
      await waitFor(() => pointer !== null, 'Native mouse movement did not reach the original browser');
      // This synthetic hold tests React/host reconciliation. Actual capture and
      // mouse-button input are separately exercised by the CDP component lane.
      knob().dispatchEvent(new PointerEvent('pointerdown', {pointerId:pointer, button:0, bubbles:true, cancelable:true}));
      await waitFor(() => window.__dandrumKnobGestures?.begins === 12,
                    'Held knob did not begin its host gesture');
      let echoedHost = false;
      const hostEcho = state => { if (Math.abs(state.parameters.find(p => p.id === publicId)?.value - 0.84) < 0.000001) echoedHost = true; };
      window.__JUCE__.backend.addEventListener('parameterStateChanged', hostEcho);
      await report('pointerHeld', {});
      await waitFor(() => echoedHost, 'Automation update did not reach the held knob');
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      knob().dispatchEvent(new PointerEvent('pointerup', {pointerId:pointer, button:0, bubbles:true}));
      await assertHostValue(0.84, 'release reconciles automation received during hold');
      window.__JUCE__.backend.removeEventListener?.('parameterStateChanged', hostEcho);
      await report('interactions', {value:0.84, gestures:12});
    } catch (error) { await report('error', {message:String(error)}); }
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', run, {once:true});
  else void run();
})();
)JS";

class Check final : private juce::Timer, private juce::AudioProcessorListener, public std::enable_shared_from_this<Check>
{
public:
    Check (DandrumAudioProcessor& host, bool isSampler) : processor (host), sampler (isSampler)
    {
        parameter = processor.getParameterForPublicId (sampler ? "drums.pitch_ratio" : "filter.cutoff");
        processor.addListener (this);
    }
    ~Check() override { processor.removeListener (this); }
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
                std::cout << "Compact layout report: " << juce::JSON::toString (data, true) << std::endl;
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
                phase = Phase::interactions;
            }
            else if (phase == Phase::interactions && name == "pointerReady")
            {
                const auto location = browser->localPointToGlobal (juce::Point<int> (
                    static_cast<int> (data.getProperty ("x", {})),
                    static_cast<int> (data.getProperty ("y", {}))));
                auto mouse = juce::Desktop::getInstance().getMainMouseSource();
                mouse.setScreenPosition (browser->localPointToGlobal (juce::Point<int> { 4, 4 }).toFloat());
                mouse.setScreenPosition (location.toFloat());
            }
            else if (phase == Phase::interactions && name == "pointerHeld")
            {
                parameter->setValueNotifyingHost (0.84f);
            }
            else if (phase == Phase::interactions && name == "interactions")
            {
                require (parameter == processor.getParameterForPublicId (sampler ? "drums.pitch_ratio" : "filter.cutoff"),
                         "React knob changed the stable host parameter object");
                const auto expectedGestures = sampler ? 12 : 11;
                require (beginCount == expectedGestures && endCount == expectedGestures && ! gestureOpen && validGestureSlots,
                         "React knob interactions did not preserve balanced gestures on their host slot: "
                             + juce::String (beginCount) + "/" + juce::String (endCount));
                require (std::abs (parameter->getValue() - (sampler ? 0.84f : 0.75f)) < 0.000001f,
                         "React actual-value entry did not reach the host parameter");
                std::cout << (sampler ? "SAMPLER" : "TB303") << " WebKit production-editor runtime: "
                          << juce::JSON::toString (fontReport, true)
                          << " compact: " << juce::JSON::toString (compactReport, true)
                          << " host updates: 0.37, 0.63; balanced gestures: " << beginCount
                          << "; final value: " << parameter->getValue() << std::endl;
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
    enum class Phase { fonts, hostValue, compact, compactHostValue, interactions, preview };
    DandrumAudioProcessor& processor;
    juce::WebBrowserComponent* browser = nullptr;
    juce::AudioProcessorEditor* editor = nullptr;
    bool sampler;
    bool finished = false;
    bool evaluationPending = false;
    Phase phase = Phase::fonts;
    double deadline = juce::Time::getMillisecondCounterHiRes() + 35000.0;
    juce::var fontReport;
    juce::var compactReport;
    juce::AudioProcessorParameter* parameter = nullptr;
    int beginCount = 0, endCount = 0;
    bool gestureOpen = false, validGestureSlots = true;

    void audioProcessorParameterChanged (juce::AudioProcessor*, int, float) override {}
    void audioProcessorChanged (juce::AudioProcessor*, const ChangeDetails&) override {}
    void audioProcessorParameterChangeGestureBegin (juce::AudioProcessor*, int index) override
    {
        validGestureSlots = validGestureSlots && ! gestureOpen && index == parameter->getParameterIndex();
        gestureOpen = true;
        ++beginCount;
    }
    void audioProcessorParameterChangeGestureEnd (juce::AudioProcessor*, int index) override
    {
        validGestureSlots = validGestureSlots && gestureOpen && index == parameter->getParameterIndex();
        gestureOpen = false;
        ++endCount;
    }

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
            + "window.__dandrumKnobGestures = {begins:" + juce::String (beginCount)
            + ",ends:" + juce::String (endCount) + "}; "
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
