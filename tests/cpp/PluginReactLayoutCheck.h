#pragma once

// Real production TB-303 assets in the original native browser at every
// supported width. No page fixture or CSS override participates in this check.
namespace reactLayoutCheck
{
template <typename Probe>
class App final : public juce::JUCEApplication, private juce::Timer
{
public:
    const juce::String getApplicationName() override { return "Dandrum React layout check"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise (const juce::String&) override
    {
        processor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::tb303());
        editor = std::make_unique<DandrumAudioProcessorEditor> (*processor);
        editor->setSize (widths[0], 860);
        editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
        editor->setVisible (true);
        deadline = juce::Time::getMillisecondCounterHiRes() + 30000.0;
        startTimer (40);
    }
    void shutdown() override { stopTimer(); editor.reset(); processor.reset(); }
private:
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<DandrumAudioProcessorEditor> editor;
    std::array<int, 4> widths { 760, 820, 1180, 1500 };
    std::size_t index = 0;
    bool pending = false;
    double deadline = 0;
    void timerCallback() override
    {
        if (pending) return;
        if (juce::Time::getMillisecondCounterHiRes() > deadline)
        { std::cerr << "React layout timed out\n"; setApplicationReturnValue (1); quit(); return; }
        pending = true;
        const auto script = juce::String (R"JS(
(() => {
  const expected = )JS") + juce::String (widths[index]) + R"JS(;
  const panel = document.querySelector('.machine');
  const frame = document.querySelector('.machine-frame');
  const knobs = [...document.querySelectorAll('[data-parameter-id]')];
  if (innerWidth !== expected || knobs.length !== 7 || knobs.some(k => k.getAttribute('aria-disabled') !== 'false')
      || document.fonts.status !== 'loaded') return JSON.stringify({waiting:true});
  const controls = [...knobs, ...document.querySelectorAll('.key')];
  const fit = controls.length === 27 && controls.every(k => {
    const r = k.getBoundingClientRect(); return r.left >= 0 && r.right <= innerWidth && r.width >= 12;
  });
  const labels = [...panel.querySelectorAll('button, span, strong')].filter(e => e.textContent.trim());
  return JSON.stringify({width:innerWidth, fit, unscaled:getComputedStyle(panel).transform === 'none',
    stableHeight:frame.style.height !== '' && Math.abs(frame.getBoundingClientRect().height - panel.getBoundingClientRect().height) < 1,
    noOverflow:document.documentElement.scrollWidth <= innerWidth,
    readable:labels.every(e => parseFloat(getComputedStyle(e).fontSize) >= 11)});
})()
)JS";
        Probe::runtimeBrowser (*editor).evaluateJavascript (script, [this] (auto reply)
        {
            pending = false;
            const auto* value = reply.getResult();
            if (value == nullptr || !value->isString()) return;
            const auto result = juce::JSON::parse (value->toString());
            if (static_cast<bool> (result["waiting"])) return;
            if (!static_cast<bool> (result["fit"]) || !static_cast<bool> (result["unscaled"])
                || !static_cast<bool> (result["stableHeight"])
                || !static_cast<bool> (result["noOverflow"]) || !static_cast<bool> (result["readable"]))
            {
                std::cerr << "React layout failed: " << juce::JSON::toString (result, true) << '\n';
                setApplicationReturnValue (1); quit(); return;
            }
            std::cout << "REACT_LAYOUT " << juce::JSON::toString (result, true) << std::endl;
            if (++index == widths.size()) { setApplicationReturnValue (0); quit(); }
            else editor->setSize (widths[index], 860);
        });
    }
};
template <typename Probe>
int main (int argc, char** argv)
{
    if (!packagedWebRuntime::available()) return 77;
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase* { return new App<Probe>(); };
    return juce::JUCEApplicationBase::main (argc, const_cast<const char**> (argv));
}
}
