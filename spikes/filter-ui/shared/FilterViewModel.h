#pragma once
#include "FilterProcessor.h"
#include <juce_dsp/juce_dsp.h>
#include <cmath>

namespace filter_spike {
inline constexpr int plotBins = 256, historyColumns = 96, fftSize = 2048;
inline constexpr float minimumDb = -36, maximumDb = 18;
struct Node { float x = 0, y = 0; };
struct VisualFrame {
    std::array<float, plotBins> inputDb {}, outputDb {}, responseDb {};
    std::array<float, plotBins * historyColumns> historyDb {};
    std::array<float, 4> meters {};
    std::array<Node, 3> nodes {};
    int historyHead = 0;
    uint64_t sequence = 0;
};

// One editor owns one model and consumes its processor's visual FIFO.
class FilterViewModel {
public:
    explicit FilterViewModel(FilterProcessor&);
    ~FilterViewModel();
    bool update();
    const VisualFrame& frame() const { return visual; }
    int selectedBand() const { return selected; }
    void selectBand(int band);
    int parameterForKnob(int knob) const;
    float knobValue(int knob) const;
    juce::String knobText(int knob) const;
    juce::String knobLabel(int knob) const;
    void beginKnob(int knob);
    void setKnob(int knob, float normalized);
    void endKnob();
    void beginNode(int band);
    void dragNode(float x, float y);
    void endNode();
    void toggle(int parameter);
    bool enabled(int parameter) const { return processor.actual(parameter) >= .5f; }
    FilterProcessor& processor;
private:
    void updateResponse();
    void updateSpectrum();
    VisualFrame visual;
    int selected = 1, activeKnob = -1, activeNode = -1;
    int activeKnobParameter = -1;
    std::array<float, 7> previous {};
    std::array<float, fftSize> input {}, output {}, window {};
    std::array<float, fftSize * 2> transform {};
    juce::dsp::FFT fft { 11 };
    double previousRate = 0;
    DandrumKernelInstrument* responseEngine = nullptr;
    juce::dsp::FFT responseFft { 13 };
    std::array<float, 8192> impulse {}, responseLeft {}, responseRight {};
    std::array<float, 16384> responseTransform {};
};
inline float frequencyAt(float x) { return 20.0f * std::pow(1000.0f, x); }
inline float frequencyX(float hz) { return std::log(hz / 20.0f) / std::log(1000.0f); }
inline float responseY(float db) { return juce::jlimit(0.0f, 1.0f, (maximumDb - db) / (maximumDb - minimumDb)); }
}
