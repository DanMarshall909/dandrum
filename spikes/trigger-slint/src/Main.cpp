#include "PerformanceView.h"

namespace trigger_ui {
class PreviewApplication final : public juce::JUCEApplication {
public:
    const juce::String getApplicationName() override { return juce::String::fromUTF8("Trigger · Slint performance"); }
    const juce::String getApplicationVersion() override { return "0.1.0"; }
    void initialise(const juce::String&) override { window = std::make_unique<PreviewWindow>(); }
    void shutdown() override { window.reset(); }
private:
    class PreviewWindow final : public juce::DocumentWindow {
    public:
        PreviewWindow() : DocumentWindow(juce::String::fromUTF8("Trigger · Slint performance · silent preview"),
            juce::Colour(0xff211b16), DocumentWindow::closeButton) {
            setUsingNativeTitleBar(true);
            // Realize the native peer before constructing the software renderer.
            setSize(900, 90); addToDesktop();
            setContentOwned(new PerformanceView(), true);
            setResizable(true, false);
            setResizeLimits(720, 90, 1600, 200);
            centreWithSize(getWidth(), getHeight());
            setVisible(true);
        }
        void closeButtonPressed() override { JUCEApplication::getInstance()->systemRequestedQuit(); }
    };
    std::unique_ptr<PreviewWindow> window;
};
}
START_JUCE_APPLICATION(trigger_ui::PreviewApplication)
