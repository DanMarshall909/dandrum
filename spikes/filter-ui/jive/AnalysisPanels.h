#pragma once
#include "VisualStyle.h"

namespace filter_spike::jive_ui {
class Spectrogram final : public juce::Component {
public:
    explicit Spectrogram(Model source) : model(std::move(source)) { setName("JIVE scrolling output spectrogram"); }
    void paint(juce::Graphics& g) override {
        g.setColour(panel()); g.fillRoundedRectangle(getLocalBounds().toFloat(), 12.f);
        label(g, "OUTPUT HISTORY     /     OLDEST -> NEWEST", { 18, 8, 650, 20 });
        const auto& frame = model->frame();
        if (sequence != frame.sequence) {
            juce::Image::BitmapData pixels(image, juce::Image::BitmapData::writeOnly);
            for (int x = 0; x < historyColumns; ++x) {
                auto column = (frame.historyHead + x) % historyColumns;
                for (int bin = 0; bin < plotBins; ++bin) {
                    auto strength = juce::jlimit(0.f, 1.f, (frame.historyDb[static_cast<size_t>(column * plotBins + bin)] + 90.f) / 90.f);
                    auto colour = juce::Colour(0xff101a2c).interpolatedWith(juce::Colour(0xff37bea8), strength * strength);
                    if (strength > .7f) colour = colour.interpolatedWith(juce::Colour(0xffffc675), (strength - .7f) / .3f);
                    pixels.setPixelColour(x, plotBins - 1 - bin, colour);
                }
            }
            sequence = frame.sequence;
        }
        g.drawImage(image, getLocalBounds().toFloat().withTrimmedLeft(18).withTrimmedRight(18).withTrimmedTop(36).withTrimmedBottom(24));
        label(g, "20 kHz ^", { getWidth() - 118, 8, 100, 20 }, juce::Justification::centredRight);
        label(g, "20 Hz", { getWidth() - 108, getHeight() - 23, 90, 18 }, juce::Justification::centredRight);
    }
private:
    Model model;
    juce::Image image { juce::Image::RGB, historyColumns, plotBins, true };
    uint64_t sequence = std::numeric_limits<uint64_t>::max();
};

class Meters final : public juce::Component {
public:
    explicit Meters(Model source) : model(std::move(source)) { setName("JIVE stereo input and output meters"); }
    void paint(juce::Graphics& g) override {
        g.setColour(panel()); g.fillRoundedRectangle(getLocalBounds().toFloat(), 12.f);
        label(g, "IN / OUT", { 8, 8, getWidth() - 16, 20 }, juce::Justification::centred);
        const float barWidth = (getWidth() - 36.f) / 4.f;
        const auto height = getHeight() - 72.f;
        for (int i = 0; i < 4; ++i) {
            auto observed = model->frame().meters[static_cast<size_t>(i)];
            envelope[static_cast<size_t>(i)] = std::max(observed, envelope[static_cast<size_t>(i)] * .89f);
            const auto db = juce::Decibels::gainToDecibels(envelope[static_cast<size_t>(i)], -60.f);
            const auto amount = juce::jlimit(0.f, 1.f, (db + 60.f) / 60.f);
            juce::Rectangle<float> bar(10.f + i * (barWidth + 5.f), 36.f, barWidth, height);
            g.setColour(juce::Colour(0xff293446)); g.fillRoundedRectangle(bar, 3.f);
            g.setColour(db >= -.5f ? juce::Colour(0xffff6978) : bandColour(i < 2 ? 0 : 1));
            g.fillRoundedRectangle(bar.withTrimmedTop(height * (1.f - amount)), 3.f);
            label(g, i % 2 == 0 ? "L" : "R", { juce::roundToInt(bar.getX()), getHeight() - 30, juce::roundToInt(barWidth), 20 }, juce::Justification::centred);
        }
    }
private:
    Model model;
    std::array<float, 4> envelope {};
};
}
