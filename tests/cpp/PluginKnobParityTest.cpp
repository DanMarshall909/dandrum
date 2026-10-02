#include "PluginProcessor.h"

#if JUCE_WEB_BROWSER
 #include "PluginEditor.h"

struct PluginEditorBridgeTestProbe
{
    static juce::WebBrowserComponent& runtimeBrowser (DandrumAudioProcessorEditor& editor)
    {
        return editor.browser;
    }
};
#endif

#include <array>
#include <bit>
#include <cmath>
#include <csignal>
#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string_view>
#include <vector>

namespace
{
// Both executables compile this schedule. Oracles are literal signed samples,
// not values calculated by the control's range mapper or by the other renderer.
struct Step
{
    const char* action;
    float normalised;
    float left;
    const char* events;
};
constexpr std::array steps {
    Step { "initial", 0.5f, 0.0f, "" },
    Step { "begin", 0.5f, 0.0f, "B" },
    Step { "drag-down", 0.25f, -0.5f, "C" },
    Step { "drag-up", 0.75f, 0.5f, "C" },
    Step { "release", 0.75f, 0.5f, "E" },
    Step { "type-zero", 0.5f, 0.0f, "BCE" },
    // Exact f32 results for gain = -1 + 2 * the rounded host value.
    Step { "key-up", 0.55f, 0.100000023841858f, "BCE" },
    Step { "fine-wheel-down", 0.54f, 0.080000042915344f, "BCE" },
    Step { "reset", 0.25f, -0.5f, "BCE" },
    Step { "cancel-entry", 0.25f, -0.5f, "" },
    Step { "automation", 0.84f, 0.679999947547913f, "C" }
};
constexpr int blockSize = 64;

void require (bool condition, const juce::String& message)
{
    if (! condition)
        throw std::runtime_error (message.toStdString());
}

#if ! JUCE_WEB_BROWSER
juce::Component* findComponent (juce::Component& parent, const juce::String& id)
{
    if (parent.getComponentID() == id)
        return &parent;
    for (auto* child : parent.getChildren())
        if (auto* found = findComponent (*child, id))
            return found;
    return nullptr;
}

juce::MouseEvent pointer (juce::Component& control, float y, int modifiers)
{
    const auto now = juce::Time::getCurrentTime();
    return { juce::Desktop::getInstance().getMainMouseSource(), { 76.0f, y },
             juce::ModifierKeys (modifiers), 1.0f, 0.0f, 0.0f, 0.0f, 0.0f,
             &control, &control, now, { 76.0f, 57.0f }, now, 1, true };
}
#else
// The original browser, native functions and observer remain intact. This
// driver calls the shipped DOM handlers and waits for C++'s render acknowledgement
// before doing anything else. It cannot batch updates across an audio boundary.
constexpr auto script = R"JS(
(() => {
  if (window.__dandrumParity) return;
  const state = window.__dandrumParity = {ack:-1, step:-1, pointer:null};
  const delay = () => new Promise(resolve => setTimeout(resolve, 10));
  const wait = async predicate => {
    const deadline = Date.now() + 15000;
    while (!predicate()) {
      if (Date.now() > deadline) throw Error('Parity action timed out');
      await delay();
    }
  };
  document.addEventListener('pointermove', event => {
    if (event.isTrusted) state.pointer = event.pointerId;
  });
  const run = async () => {
    try {
      const knob = () => document.querySelector('[data-parameter-id="fixture.level"]');
      await wait(() => knob()?.getAttribute('aria-disabled') === 'false');
      const bounds = knob().querySelector('svg').getBoundingClientRect();
      state.ready = {x:bounds.x + bounds.width/2, y:bounds.y + bounds.height/2};
      await wait(() => state.pointer !== null);
      const key = key => knob().dispatchEvent(new KeyboardEvent('keydown', {
        key, bubbles:true, cancelable:true}));
      const point = (type, y) => knob().dispatchEvent(new PointerEvent(type, {
        pointerId:state.pointer, button:0, buttons:type === 'pointerup' ? 0 : 1,
        clientY:y, bubbles:true, cancelable:true}));
      const type = async (value, finish) => {
        key('Enter');
        await wait(() => knob().querySelector('.dd-knob-input'));
        const input = knob().querySelector('.dd-knob-input');
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value);
        input.dispatchEvent(new Event('input', {bubbles:true}));
        // Let React commit the controlled input before its completion event.
        await new Promise(resolve => requestAnimationFrame(resolve));
        input.dispatchEvent(new KeyboardEvent('keydown', {key:finish, bubbles:true, cancelable:true}));
        await wait(() => !knob().querySelector('.dd-knob-input'));
      };
      for (let index = 0; index < state.actions.length; ++index) {
        await wait(() => state.ack === index - 1);
        const action = state.actions[index];
        if (action === 'begin') point('pointerdown', 200);
        else if (action === 'drag-down') point('pointermove', 250);
        else if (action === 'drag-up') point('pointermove', 150);
        else if (action === 'release') point('pointerup', 150);
        else if (action === 'type-zero') await type('0', 'Enter');
        else if (action === 'key-up') key('ArrowUp');
        else if (action === 'fine-wheel-down') knob().dispatchEvent(new WheelEvent('wheel', {
          deltaY:1, shiftKey:true, bubbles:true, cancelable:true}));
        else if (action === 'reset') knob().dispatchEvent(new MouseEvent('dblclick', {bubbles:true}));
        else if (action === 'cancel-entry') await type('0.5', 'Escape');
        else if (action !== 'initial' && action !== 'automation') throw Error('Unknown parity action');
        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        state.step = index;
      }
    } catch (error) { state.error = String(error); }
  };
  void run();
})();
)JS";
#endif

class Application final : public juce::JUCEApplication, private juce::Timer,
                          private juce::AudioProcessorListener
{
public:
    const juce::String getApplicationName() override { return "Dandrum Knob Parity Test"; }
    const juce::String getApplicationVersion() override { return "1"; }

    void initialise (const juce::String&) override
    {
        try
        {
            std::unique_ptr<juce::AudioProcessor> plugin (createPluginFilter());
            auto* sampler = dynamic_cast<DandrumAudioProcessor*> (plugin.get());
            require (sampler != nullptr, "Original sampler factory has no Dandrum processor");
            plugin.release();
            processor.reset (sampler);
            processor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
            processor->prepareToPlay (48000.0, blockSize);
            require (processor->isInstrumentLoaded()
                         && processor->demoConfiguration().instrumentId == "dandrum.advanced-drum-kit",
                     "Parity did not start with the original sampler factory");
            const auto fixture = std::filesystem::path (__FILE__).parent_path().parent_path()
                                     / "fixtures/plugin-ui-knob.yaml";
            const auto yaml = juce::File (juce::String (fixture.string())).loadFileAsString();
            require (yaml.contains ("default: 0, min: -1")
                         && replacement.getFile().replaceWithText (
                             yaml.replace ("default: 0, min: -1", "default: -0.5, min: -1"))
                         && processor->reloadInstrumentFromFile (replacement.getFile()),
                     "Signed parity fixture did not prepare");
            parameter = processor->getParameterForPublicId ("fixture.level");
            require (parameter != nullptr && parameter->getParameterIndex() == 0,
                     "Parity fixture is not bound to its declared first stable host slot");
            const auto prepared = processor->getPreparedUiDocument();
            require (prepared && prepared->parameters.size() == 1
                         && std::abs (prepared->parameters.front().normalisedDefaultValue - 0.25f) < 0.00001f,
                     "Parity fixture did not expose its distinguishable loaded reset value");
            generation = prepared->generation;
            parameter->setValueNotifyingHost (0.5f);
            processor->addListener (this);
            listening = true;
            editor.reset (processor->createEditor());
            require (editor != nullptr, "Original factory did not create its editor");
            editor->setSize (820, 560);
            editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
            editor->setTopLeftPosition (30, 30);
            editor->setVisible (true);
           #if JUCE_WEB_BROWSER
            auto* web = dynamic_cast<DandrumAudioProcessorEditor*> (editor.get());
            require (web != nullptr, "Original factory did not create its Web editor");
            browser = &PluginEditorBridgeTestProbe::runtimeBrowser (*web);
           #else
            knob = dynamic_cast<juce::Slider*> (findComponent (*editor, "primary-knob-slider"));
            require (knob != nullptr && knob->isEnabled(), "Original factory has no enabled native knob");
           #endif
            startTimer (25);
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }

    void shutdown() override
    {
        stopTimer();
        if (listening)
            processor->removeListener (this);
        editor.reset();
        processor.reset();
    }

private:
    struct Event { char kind; int slot; float value; };
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    juce::TemporaryFile replacement { ".yaml" };
    juce::AudioProcessorParameter* parameter = nullptr;
    std::uint32_t generation = 0;
    std::vector<Event> events;
    std::size_t index = 0, checkedEvents = 0;
    bool listening = false, submitted = false, finished = false;
    const double deadline = juce::Time::getMillisecondCounterHiRes() + 25000.0;
   #if JUCE_WEB_BROWSER
    juce::WebBrowserComponent* browser = nullptr;
    bool evaluating = false, pointerMoved = false;
   #else
    juce::Slider* knob = nullptr;
   #endif

    void audioProcessorChanged (juce::AudioProcessor*, const ChangeDetails&) override {}
    void audioProcessorParameterChanged (juce::AudioProcessor*, int slot, float value) override
    {
        events.push_back ({ 'C', slot, value });
    }
    void audioProcessorParameterChangeGestureBegin (juce::AudioProcessor*, int slot) override
    {
        events.push_back ({ 'B', slot, 0.0f });
    }
    void audioProcessorParameterChangeGestureEnd (juce::AudioProcessor*, int slot) override
    {
        events.push_back ({ 'E', slot, 0.0f });
    }

    bool verify (double displayed, double actual)
    {
        const auto& step = steps[index];
        const juce::String expectedEvents (step.events);
        if (events.size() < checkedEvents + static_cast<std::size_t> (expectedEvents.length()))
            return false;
        const auto context = "Parity action " + juce::String (step.action) + ": ";
        require (events.size() == checkedEvents + static_cast<std::size_t> (expectedEvents.length()),
                 context + "unexpected or duplicate host notification");
        for (int position = 0; position < expectedEvents.length(); ++position)
        {
            const auto& event = events[checkedEvents + static_cast<std::size_t> (position)];
            require (event.kind == step.events[position] && event.slot == 0,
                     context + "host gesture order or stable slot changed");
            if (event.kind == 'C')
                require (std::bit_cast<std::uint32_t> (event.value)
                             == std::bit_cast<std::uint32_t> (step.normalised),
                         context + "host change notified the wrong authoritative value");
        }
        require (processor->getParameterForPublicId ("fixture.level") == parameter
                     && parameter->getParameterIndex() == 0
                     && processor->getParameterSurfaceGeneration() == generation,
                 context + "host identity or working generation changed");
        require (std::bit_cast<std::uint32_t> (parameter->getValue())
                     == std::bit_cast<std::uint32_t> (step.normalised),
                 context + "host value differs from the schedule");
        // Ordinary timer observation is asynchronous in both renderers.
        if (! std::isfinite (displayed) || ! std::isfinite (actual)
            || std::abs (displayed - step.normalised) >= 0.00001
            || std::abs (actual - step.left) >= 0.00001)
            return false;
        juce::AudioBuffer<float> output (2, blockSize);
        for (int channel = 0; channel < 2; ++channel)
            for (int frame = 0; frame < blockSize; ++frame)
                output.setSample (channel, frame, 0.777f);
        juce::MidiBuffer midi;
        processor->processBlock (output, midi);
        for (int channel = 0; channel < 2; ++channel)
            for (int frame = 0; frame < blockSize; ++frame)
                require (std::bit_cast<std::uint32_t> (output.getSample (channel, frame))
                             == std::bit_cast<std::uint32_t> (
                                 channel == 0 ? step.left : step.left < 0.0f ? -0.0f : 0.0f),
                         context + "signed PCM differs at channel " + juce::String (channel)
                             + " frame " + juce::String (index * blockSize + static_cast<std::size_t> (frame)));
        auto report = std::make_unique<juce::DynamicObject>();
        report->setProperty ("frame", static_cast<int> (index * blockSize));
        report->setProperty ("action", step.action);
        report->setProperty ("slot", parameter->getParameterIndex());
        report->setProperty ("normalised", parameter->getValue());
        report->setProperty ("displayed", displayed);
        report->setProperty ("actual", actual);
        report->setProperty ("left", output.getSample (0, 0));
        report->setProperty ("right", output.getSample (1, 0));
        report->setProperty ("events", step.events);
        std::cout << "KNOB_PARITY " << juce::JSON::toString (juce::var (report.release()), true) << std::endl;
        checkedEvents = events.size();
        ++index;
        submitted = false;
        if (index == steps.size())
            finish (true);
        return true;
    }

    void finish (bool success, const juce::String& error = {})
    {
        if (finished)
            return;
        finished = true;
        stopTimer();
        if (! success)
            std::cerr << "Knob parity failed: " << error << '\n';
        setApplicationReturnValue (success ? 0 : 1);
        quit();
    }

    void timerCallback() override
    {
        if (finished)
            return;
        if (juce::Time::getMillisecondCounterHiRes() > deadline)
        {
            finish (false, "action " + juce::String (steps[index].action) + " did not reach authoritative state");
            return;
        }
        try
        {
           #if JUCE_WEB_BROWSER
            pollWeb();
           #else
            if (! submitted)
            {
                submitted = true;
                nativeAction (steps[index].action);
            }
            auto* readout = dynamic_cast<juce::TextButton*> (findComponent (*knob, "primary-knob-readout"));
            require (readout != nullptr, "Native knob has no actual readout");
            verify (knob->getValue(), readout->getButtonText().getDoubleValue());
           #endif
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }

   #if ! JUCE_WEB_BROWSER
    void nativeAction (std::string_view action)
    {
        if (action == "initial")
            knob->grabKeyboardFocus();
        else if (action == "begin")
            knob->mouseDown (pointer (*knob, 57.0f, juce::ModifierKeys::leftButtonModifier));
        else if (action == "drag-down" || action == "drag-up")
            knob->mouseDrag (pointer (*knob, action == "drag-down" ? 107.0f : 7.0f,
                                      juce::ModifierKeys::leftButtonModifier));
        else if (action == "release")
            knob->mouseUp (pointer (*knob, 7.0f, 0));
        else if (action == "type-zero" || action == "cancel-entry")
        {
            knob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
            auto* input = dynamic_cast<juce::TextEditor*> (findComponent (*knob, "primary-knob-value-editor"));
            require (input != nullptr && input->isShowing(), "Native parity value input did not open");
            for (const auto character : juce::String (action == "type-zero" ? "0" : "0.5"))
                input->keyPressed (juce::KeyPress (static_cast<int> (character), {}, character));
            input->keyPressed (juce::KeyPress (action == "type-zero"
                                                 ? juce::KeyPress::returnKey : juce::KeyPress::escapeKey));
            require (! input->isVisible(), "Native parity value input did not close");
        }
        else if (action == "key-up")
            knob->keyPressed (juce::KeyPress (juce::KeyPress::upKey));
        else if (action == "fine-wheel-down")
            knob->mouseWheelMove (pointer (*knob, 57.0f, juce::ModifierKeys::shiftModifier),
                                  { 0.0f, -1.0f, false, false, false });
        else if (action == "reset")
            knob->mouseDoubleClick (pointer (*knob, 57.0f, juce::ModifierKeys::leftButtonModifier));
        else if (action == "automation")
            parameter->setValueNotifyingHost (0.84f);
    }
   #else
    void pollWeb()
    {
        if (evaluating)
            return;
        if (std::string_view (steps[index].action) == "automation" && ! submitted)
        {
            submitted = true;
            parameter->setValueNotifyingHost (0.84f);
        }
        juce::Array<juce::var> actions;
        for (const auto& step : steps)
            actions.add (step.action);
        const auto observation = juce::String ("(() => { if (!document.querySelector('main')) return ''; ")
            + script + "const s=window.__dandrumParity; s.actions=" + juce::JSON::toString (actions, true)
            + "; s.ack=" + juce::String (static_cast<int> (index) - 1)
            + "; const k=document.querySelector('[data-parameter-id=\"fixture.level\"]');"
              "return JSON.stringify({step:s.step,ready:s.ready,error:s.error,"
              "displayed:Number(k?.getAttribute('aria-valuenow')),actual:Number(k?.getAttribute('aria-valuetext'))}); })()";
        evaluating = true;
        juce::Component::SafePointer<juce::AudioProcessorEditor> alive (editor.get());
        browser->evaluateJavascript (observation, [this, alive] (auto evaluation)
        {
            if (alive == nullptr || finished)
                return;
            evaluating = false;
            try
            {
                require (evaluation.getError() == nullptr, "Original browser observation failed");
                const auto* result = evaluation.getResult();
                require (result != nullptr && result->isString(), "Original browser observation returned no string");
                if (result->toString().isEmpty())
                    return;
                const auto state = juce::JSON::parse (result->toString());
                require (state.isObject(), "Original browser returned an invalid parity report");
                require (! state.hasProperty ("error"), state.getProperty ("error", {}).toString());
                const auto ready = state.getProperty ("ready", {});
                if (ready.isObject() && ! pointerMoved)
                {
                    pointerMoved = true;
                    auto mouse = juce::Desktop::getInstance().getMainMouseSource();
                    mouse.setScreenPosition (browser->localPointToGlobal (juce::Point<int> { 4, 4 }).toFloat());
                    mouse.setScreenPosition (browser->localPointToGlobal (juce::Point<int> {
                        static_cast<int> (ready.getProperty ("x", {})),
                        static_cast<int> (ready.getProperty ("y", {})) }).toFloat());
                }
                if (static_cast<int> (state.getProperty ("step", -1)) == static_cast<int> (index))
                    verify (static_cast<double> (state.getProperty ("displayed", {})),
                            static_cast<double> (state.getProperty ("actual", {})));
            }
            catch (const std::exception& error) { finish (false, error.what()); }
        });
    }
   #endif
};
}

int main (int argc, char** argv)
{
   #if JUCE_LINUX
    if (std::getenv ("DISPLAY") == nullptr)
    {
        std::cout << "SKIP: actual knob renderers require an X display\n";
        return 77;
    }
    std::signal (SIGPIPE, SIG_IGN);
   #endif
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase* { return new Application(); };
    return juce::JUCEApplicationBase::main (argc, const_cast<const char**> (argv));
}
