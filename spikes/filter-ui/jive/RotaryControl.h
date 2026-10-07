#pragma once
#include "VisualStyle.h"

namespace filter_spike::jive_ui {
class RotaryLook final : public juce::LookAndFeel_V4 {
public:
    explicit RotaryLook(Model source) : model(std::move(source)) {}
    void drawRotarySlider(juce::Graphics& g, int x, int y, int width, int height,
                           float position, float start, float end, juce::Slider&) override {
        auto bounds = juce::Rectangle<float>(static_cast<float>(x), static_cast<float>(y), static_cast<float>(width), static_cast<float>(height)).reduced(12.f);
        const auto radius = std::min(bounds.getWidth(), bounds.getHeight()) / 2.f;
        const auto centre = bounds.getCentre();
        juce::Path track, arc;
        track.addCentredArc(centre.x, centre.y, radius, radius, 0.f, start, end, true);
        arc.addCentredArc(centre.x, centre.y, radius, radius, 0.f, start, start + position * (end - start), true);
        g.setColour(juce::Colour(0xff2a3547)); g.strokePath(track, juce::PathStrokeType(4.f));
        g.setColour(bandColour(model->selectedBand())); g.strokePath(arc, juce::PathStrokeType(4.f));
        g.setColour(juce::Colour(0xff222c3b)); g.fillEllipse(bounds.reduced(7.f));
        auto angle = start + position * (end - start) - juce::MathConstants<float>::halfPi;
        auto point = centre + juce::Point<float>(std::cos(angle), std::sin(angle)) * (radius - 13.f);
        g.setColour(juce::Colour(0xffeef4ff)); g.drawLine({ centre, point }, 2.4f);
    }
private:
    Model model;
};

class RotaryControl final : public juce::Slider {
public:
    RotaryControl(Model source, int control) : model(std::move(source)), index(control), look(model) {
        setLookAndFeel(&look);
        setName("JIVE rotary " + juce::String(index));
        onDragStart = [this] { finish(); model->beginKnob(index); active = true; };
        onValueChange = [this] {
            if (active) model->setKnob(index, static_cast<float>(getValue()));
            else { model->beginKnob(index); model->setKnob(index, static_cast<float>(getValue())); model->endKnob(); }
        };
        onDragEnd = [this] { finish(); };
    }
    ~RotaryControl() override { finish(); setLookAndFeel(nullptr); }
    void refresh() {
        setValue(model->knobValue(index), juce::dontSendNotification);
        setEnabled(index != 2 || model->selectedBand() == 1);
        repaint();
    }
private:
    void finish() { if (active) { model->endKnob(); active = false; } }
    Model model;
    int index;
    bool active = false;
    RotaryLook look;
};
}
