#pragma once
#include "VisualStyle.h"

namespace filter_spike::jive_ui {
class ResponseGraph final : public juce::Component {
public:
    explicit ResponseGraph(Model source) : model(std::move(source)) {
        setMouseCursor(juce::MouseCursor::CrosshairCursor);
        setName("JIVE response and live spectrum");
    }
    ~ResponseGraph() override { finishDrag(); }
    // Read the same geometry used for paint and hit testing; useful for spike
    // interaction checks without injecting state into the rendered model.
    juce::Point<float> displayedNodePosition(int band) const { return nodePoint(band); }
    uint64_t displayedSequence() const { return model->frame().sequence; }
    void paint(juce::Graphics& g) override {
        g.setColour(panel());
        g.fillRoundedRectangle(getLocalBounds().toFloat(), 12.0f);
        auto area = plot();
        g.setColour(juce::Colour(0xff293446));
        for (float hz : { 20.f, 50.f, 100.f, 200.f, 500.f, 1000.f, 2000.f, 5000.f, 10000.f, 20000.f }) {
            float x = area.getX() + frequencyX(hz) * area.getWidth();
            g.drawVerticalLine(juce::roundToInt(x), area.getY(), area.getBottom());
            label(g, hz >= 1000 ? juce::String(hz / 1000, 0) + "k" : juce::String(hz, 0),
                  { juce::roundToInt(x) - 19, juce::roundToInt(area.getBottom()) + 7, 38, 18 }, juce::Justification::centred);
        }
        for (float db : { -36.f, -24.f, -12.f, 0.f, 12.f, 18.f }) {
            float y = area.getY() + responseY(db) * area.getHeight();
            g.drawHorizontalLine(juce::roundToInt(y), area.getX(), area.getRight());
            label(g, juce::String(db, 0), { 5, juce::roundToInt(y) - 8, 32, 16 }, juce::Justification::centredRight);
        }
        const auto& frame = model->frame();
        drawTrace(g, frame.inputDb, area, false, muted().withAlpha(.48f), 1.1f);
        drawTrace(g, frame.outputDb, area, false, bandColour(0).withAlpha(.7f), 1.4f);
        drawTrace(g, frame.responseDb, area, true, juce::Colour(0xffeef4ff), 2.3f);
        for (int i = 0; i < 3; ++i) {
            auto point = nodePoint(i);
            g.setColour(bandColour(i).withAlpha(.15f));
            g.fillEllipse(point.x - 15, point.y - 15, 30, 30);
            g.setColour(bandColour(i));
            g.fillEllipse(point.x - 7, point.y - 7, 14, 14);
            if (i == model->selectedBand()) {
                g.setColour(juce::Colours::white);
                g.drawEllipse(point.x - 10, point.y - 10, 20, 20, 1.4f);
            }
            label(g, names[static_cast<size_t>(i)],
                  { juce::roundToInt(point.x) - 25, juce::roundToInt(point.y) - 31, 50, 17 }, juce::Justification::centred);
        }
        label(g, "INPUT / OUTPUT FFT     /     FILTER RESPONSE", { 48, 8, 500, 20 });
    }
    void mouseDown(const juce::MouseEvent& event) override {
        finishDrag();
        float distance = 28.0f;
        int nearest = -1;
        for (int i = 0; i < 3; ++i) {
            auto candidate = nodePoint(i).getDistanceFrom(event.position);
            if (candidate < distance) { distance = candidate; nearest = i; }
        }
        if (nearest >= 0) { dragging = true; model->beginNode(nearest); repaint(); }
    }
    void mouseDrag(const juce::MouseEvent& event) override {
        if (!dragging) return;
        const auto area = plot();
        model->dragNode(juce::jlimit(0.f, 1.f, (event.position.x - area.getX()) / area.getWidth()),
                        juce::jlimit(0.f, 1.f, (event.position.y - area.getY()) / area.getHeight()));
        repaint();
    }
    void mouseUp(const juce::MouseEvent&) override { finishDrag(); }
private:
    juce::Rectangle<float> plot() const { return getLocalBounds().toFloat().withTrimmedLeft(48).withTrimmedRight(24).withTrimmedTop(42).withTrimmedBottom(38); }
    juce::Point<float> nodePoint(int band) const {
        const auto area = plot();
        const auto node = model->frame().nodes[static_cast<size_t>(band)];
        return { area.getX() + node.x * area.getWidth(), area.getY() + node.y * area.getHeight() };
    }
    static void drawTrace(juce::Graphics& g, const std::array<float, plotBins>& values,
                          juce::Rectangle<float> area, bool response, juce::Colour colour, float width) {
        juce::Path trace;
        for (int i = 0; i < plotBins; ++i) {
            const auto db = values[static_cast<size_t>(i)];
            const auto y = response ? responseY(db) : juce::jlimit(0.f, 1.f, -db / 90.f);
            const auto xPixel = area.getX() + i * area.getWidth() / (plotBins - 1);
            const auto yPixel = area.getY() + y * area.getHeight();
            if (i == 0) trace.startNewSubPath(xPixel, yPixel); else trace.lineTo(xPixel, yPixel);
        }
        g.setColour(colour); g.strokePath(trace, juce::PathStrokeType(width));
    }
    void finishDrag() { if (dragging) { model->endNode(); dragging = false; } }
    Model model;
    bool dragging = false;
    static constexpr std::array<const char*, 3> names { "HP", "BELL", "LP" };
};
}
