#pragma once
#include <juce_gui_basics/juce_gui_basics.h>
#include <array>
#include <memory>

namespace trigger_ui {
// Native embedding only; macro drawing and temporary preview state live in Slint.
class PerformanceView final : public juce::Component, private juce::Timer {
public:
    PerformanceView();
    ~PerformanceView() override;
    void paint(juce::Graphics&) override;
    void resized() override;
    void mouseDown(const juce::MouseEvent&) override;
    void mouseUp(const juce::MouseEvent&) override;
    void mouseMove(const juce::MouseEvent&) override;
    void mouseDrag(const juce::MouseEvent&) override;
    void mouseExit(const juce::MouseEvent&) override;
    void mouseWheelMove(const juce::MouseEvent&, const juce::MouseWheelDetails&) override;
    bool keyPressed(const juce::KeyPress&) override;
    void modifierKeysChanged(const juce::ModifierKeys&) override;
    void focusGained(FocusChangeType) override;
    void focusLost(FocusChangeType) override;
    std::array<float, 8> macroValues() const;
private:
    void timerCallback() override;
    struct Impl;
    std::unique_ptr<Impl> impl;
};
}
