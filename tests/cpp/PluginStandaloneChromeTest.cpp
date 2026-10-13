#include "PluginEditor.h"
#include "PluginTriggerWebRuntimeTest.h"
#include <juce_audio_utils/juce_audio_utils.h>

#include <juce_audio_plugin_client/Standalone/juce_StandaloneFilterWindow.h>

#include <algorithm>
#include <array>
#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <vector>

struct PluginStandaloneChromeTestProbe
{
    static auto browserOptions (DandrumAudioProcessorEditor& editor) { return editor.createBrowserOptions(); }
    static void refresh (DandrumAudioProcessorEditor& editor) { editor.timerCallback(); }
};

namespace
{
void require (bool condition, const char* message)
{
    if (! condition) throw std::runtime_error (message);
}

class HostWindow final : public juce::DocumentWindow
{
public:
    HostWindow() : DocumentWindow ("Host", juce::Colours::black, closeButton) {}
    void closeButtonPressed() override {}
};

juce::var requestTitle (DandrumAudioProcessorEditor& editor, const juce::Array<juce::var>& arguments)
{
    const auto browserOptions = PluginStandaloneChromeTestProbe::browserOptions (editor);
    const auto& functions = browserOptions.getNativeFunctions();
    const auto found = functions.find (juce::Identifier ("setTriggerWindowTitle"));
    require (found != functions.end(), "full Trigger sampler must register setTriggerWindowTitle");
    juce::var result;
    bool completed = false;
    found->second (arguments, [&] (juce::var reply) { result = std::move (reply); completed = true; });
    require (completed, "Trigger title request did not complete");
    return result;
}

void checkTitleRequests (DandrumAudioProcessorEditor& editor, juce::StandaloneFilterWindow& window)
{
    const auto patchName = juce::String::fromUTF8 ("Möbius \"Kit\" 🥁");
    const auto title = "trigger - " + patchName;
    const auto reply = requestTitle (editor, { patchName });
    require (reply.getProperty ("status", {}).toString() == "accepted"
             && reply.getProperty ("title", {}).toString() == title,
             "Trigger title request must acknowledge the exact Unicode patch name");
    require (editor.getName() == title && window.getName() == title,
             "Trigger title request must update the native standalone window");
    for (const auto& arguments : std::vector<juce::Array<juce::var>> {
             {}, { 42 }, { true }, { juce::var{} }, { "" }, { " \t " }, { "one", "two" } })
    {
        const auto rejected = requestTitle (editor, arguments);
        require (rejected.isString() && rejected.toString().isNotEmpty(),
                 "malformed Trigger title request must return an error");
        require (editor.getName() == title && window.getName() == title,
                 "malformed Trigger title request must preserve the accepted title");
    }
}

juce::MenuBarModel& options (juce::StandaloneFilterWindow& window)
{
    require (window.isUsingNativeTitleBar(), "sampler standalone must use a system-managed title bar");
    auto* bar = dynamic_cast<juce::MenuBarComponent*> (window.getMenuBarComponent());
    require (bar != nullptr && bar->getHeight() >= 20, "native title bar must retain visible Options access");
    auto* model = bar->getModel();
    require (model != nullptr && model->getMenuBarNames() == juce::StringArray { "Options" },
             "standalone Options menu is missing");
    const auto menu = model->getMenuForIndex (0, "Options");
    std::array<bool, 4> found {};
    for (juce::PopupMenu::MenuItemIterator items (menu); items.next();)
    {
        const auto& item = items.getItem();
        if (item.itemID >= 1 && item.itemID <= 4) found[static_cast<std::size_t> (item.itemID - 1)] = true;
    }
    require (std::all_of (found.begin(), found.end(), [] (bool value) { return value; }),
             "Options must preserve audio settings and save/load/reset actions");
    require (model->getMenuForIndex (1, "Unknown").getNumItems() == 0,
             "unknown standalone menu index must be empty");
    model->menuItemSelected (0, 0);
    model->menuItemSelected (99, 0);
    return *model;
}

void checkPresets (juce::StandaloneFilterWindow& window)
{
    auto* editor = window.getAudioProcessor()->getActiveEditor();
    require (editor != nullptr, "standalone editor is unavailable");
    for (const auto [width, height] : std::array<std::pair<int, int>, 4> {{ {820, 560}, {1200, 800}, {1600, 1000}, {900, 126} }})
    {
        editor->setSize (width, height);
        const auto area = window.getLocalArea (editor, editor->getLocalBounds());
        require (area.getWidth() == width && area.getHeight() == height,
                 "native standalone chrome changed an editor size preset");
        require (window.getLocalBounds().contains (area), "standalone window clips the editor preset");
        require (area.getY() >= window.getMenuBarComponent()->getBottom(), "Options overlaps the editor");
        require (window.getHeight() >= height + window.getMenuBarComponent()->getHeight(),
                 "standalone window must include Options above the editor");
    }
}

class StandaloneChromeApplication final : public juce::JUCEApplication,
                                         private juce::Timer
{
public:
    const juce::String getApplicationName() override { return "Sampler standalone chrome check"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise (const juce::String&) override
    {
        try
        {
            // Chrome and editor lifetime checks do not need a hardware audio stream.
            juce::XmlElement noDevice ("DEVICESETUP");
            noDevice.setAttribute ("audioInputDeviceName", "");
            noDevice.setAttribute ("audioOutputDeviceName", "");
            settings.setValue ("audioSetup", &noDevice);
            std::cerr << "CHECK: creating standalone window without an audio device\n";
            window = std::make_unique<juce::StandaloneFilterWindow> (
                "Sampler chrome check", juce::Colours::black, &settings, false);
            require (window->getDeviceManager().getCurrentAudioDevice() == nullptr,
                     "chrome fixture must not open a hardware audio stream");
            auto& editor = *static_cast<DandrumAudioProcessorEditor*> (
                window->getAudioProcessor()->getActiveEditor());
            require (! window->isUsingNativeTitleBar(), "early title check must precede chrome installation");
            requestTitle (editor, { "Early patch" });
            require (editor.getName() == "trigger - Early patch" && window->getName() == "Sampler chrome check",
                     "early Trigger title must be remembered before standalone chrome attaches");
            PluginStandaloneChromeTestProbe::refresh (editor);
            require (window->getName() == "trigger - Early patch",
                     "standalone chrome must adopt the title received before attachment");
            window->setVisible (true);
            deadline = juce::Time::getMillisecondCounterHiRes() + 5000.0;
            startTimer (20);
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }

    void shutdown() override { stopTimer(); window.reset(); }

private:
    enum class Phase { startup, invalidIndex, reset };

    bool chromeSettled()
    {
        auto* bar = dynamic_cast<juce::MenuBarComponent*> (window->getMenuBarComponent());
        if (! window->isUsingNativeTitleBar() || bar == nullptr || bar->getModel() == nullptr)
        {
            chromeReadyAt = 0;
            return false;
        }
        const auto now = juce::Time::getMillisecondCounterHiRes();
        if (chromeReadyAt <= 0) chromeReadyAt = now;
        // Allow the native window manager to publish the replacement frame geometry.
        return now - chromeReadyAt >= 150.0;
    }

    void finish (bool passed, const char* message)
    {
        stopTimer();
        (passed ? std::cout : std::cerr) << (passed ? "PASS: " : "FAIL: ") << message << '\n';
        setApplicationReturnValue (passed ? 0 : 1);
        quit();
    }

    void timerCallback() override
    {
        try
        {
            require (juce::Time::getMillisecondCounterHiRes() < deadline,
                     "standalone chrome or Options Reset did not finish");
            if (phase == Phase::startup)
            {
                if (! chromeSettled()) return;
                auto* editor = window->getAudioProcessor()->getActiveEditor();
                require (editor != nullptr && editor->getWidth() == 1180 && editor->getHeight() == 860,
                         "standalone editor must initially open at 1180x860");
                auto& menu = options (*window);
                checkTitleRequests (*static_cast<DandrumAudioProcessorEditor*> (editor), *window);
                checkPresets (*window);
                std::cerr << "CHECK: native chrome, initial size and editor presets verified\n";

                // DAW-owned windows retain their existing host chrome.
                DandrumAudioProcessor hosted (InstrumentDemoConfiguration::sampler());
                std::unique_ptr<juce::AudioProcessorEditor> hostedEditor (hosted.createEditorIfNeeded());
                HostWindow host;
                host.setContentNonOwned (hostedEditor.get(), true);
                requestTitle (*static_cast<DandrumAudioProcessorEditor*> (hostedEditor.get()), { "Hosted patch" });
                require (! host.isUsingNativeTitleBar() && host.getMenuBarComponent() == nullptr,
                         "sampler editor must not replace a DAW host's chrome");
                require (host.getName() == "Host" && hostedEditor->getName() == "trigger - Hosted patch",
                         "Trigger title request must preserve the DAW-owned window title");
                host.clearContentComponent();
                std::cerr << "CHECK: DAW chrome retained\n";

                DandrumAudioProcessor other (InstrumentDemoConfiguration::tb303());
                std::unique_ptr<juce::AudioProcessorEditor> otherEditor (other.createEditorIfNeeded());
                require (PluginStandaloneChromeTestProbe::browserOptions (
                             *static_cast<DandrumAudioProcessorEditor*> (otherEditor.get()))
                             .getNativeFunctions().count (juce::Identifier ("setTriggerWindowTitle")) == 0,
                         "other instruments must not register the Trigger window title command");

                retiredEditor = editor;
                menu.menuItemSelected (4, 1);
                phase = Phase::invalidIndex;
                return;
            }
            if (phase == Phase::invalidIndex)
            {
                require (retiredEditor != nullptr, "unknown menu index must not reset the editor");
                std::cerr << "CHECK: invalid menu index retained editor; requesting Reset\n";
                phase = Phase::reset;
                chromeReadyAt = 0;
                deadline = juce::Time::getMillisecondCounterHiRes() + 5000.0;
                options (*window).menuItemSelected (4, 0);
                return;
            }
            if (retiredEditor != nullptr || ! chromeSettled()) return;
            auto* editor = window->getAudioProcessor()->getActiveEditor();
            require (editor != nullptr && editor->getWidth() == 1180 && editor->getHeight() == 860,
                     "Options Reset must restore the 1180x860 editor size");
            options (*window);
            checkTitleRequests (*static_cast<DandrumAudioProcessorEditor*> (editor), *window);
            checkPresets (*window);
            require (window->getDeviceManager().getCurrentAudioDevice() == nullptr,
                     "Options Reset must retain the fixture's audio device choice");
            finish (true, "standalone native title bar retains Options and presets across Reset");
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }

    juce::PropertySet settings;
    std::unique_ptr<juce::StandaloneFilterWindow> window;
    juce::Component::SafePointer<juce::AudioProcessorEditor> retiredEditor;
    Phase phase = Phase::startup;
    double chromeReadyAt = 0;
    double deadline = 0;
};
}

int main (int argc, char** argv)
{
    if (argc > 1 && juce::String (argv[1]) == "--juce-gtkwebkitfork-child")
        return runTriggerWebRuntime (argc, argv);
#if JUCE_LINUX
    if (std::getenv ("DISPLAY") == nullptr)
    {
        std::cout << "SKIP: standalone window checks require an X display\n";
        return 77;
    }
#endif
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase* {
        return new StandaloneChromeApplication;
    };
    std::vector<const char*> arguments (argv, argv + argc);
    arguments.push_back (nullptr);
    return juce::JUCEApplicationBase::main (argc, arguments.data());
}
