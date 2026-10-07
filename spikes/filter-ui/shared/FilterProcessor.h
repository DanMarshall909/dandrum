#pragma once
#include <juce_audio_processors/juce_audio_processors.h>
#include "RustEngineBindings.h"
#include <array>
#include <span>

namespace filter_spike {
enum Parameter { hpFrequency, hpQ, bellFrequency, bellQ, bellGain, lpFrequency, lpQ, bypass, audition, parameterCount };
inline constexpr std::array<const char*, parameterCount> parameterIds {
    "hp_frequency", "hp_q", "bell_frequency", "bell_q", "bell_gain", "lp_frequency", "lp_q", "bypass", "audition" };

class FilterProcessor final : public juce::AudioProcessor {
public:
    FilterProcessor();
    ~FilterProcessor() override;
    const juce::String getName() const override { return "Dandrum Filter Lab"; }
    void prepareToPlay(double, int) override;
    void releaseResources() override;
    void processBlock(juce::AudioBuffer<float>&, juce::MidiBuffer&) override;
    bool isBusesLayoutSupported(const BusesLayout&) const override;
    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override { return true; }
    juce::AudioProcessorParameter* getBypassParameter() const override { return parameters.getParameter("bypass"); }
    bool acceptsMidi() const override { return false; }
    bool producesMidi() const override { return false; }
    bool isMidiEffect() const override { return false; }
    double getTailLengthSeconds() const override { return 0; }
    int getNumPrograms() override { return 1; }
    int getCurrentProgram() override { return 0; }
    void setCurrentProgram(int) override {}
    const juce::String getProgramName(int) override { return {}; }
    void changeProgramName(int, const juce::String&) override {}
    void getStateInformation(juce::MemoryBlock&) override;
    void setStateInformation(const void*, int) override;
    float actual(int index) const;
    float normalized(int index) const;
    float normalize(int index, float value) const;
    void beginGesture(int index);
    void setNormalized(int index, float value);
    void endGesture(int index);
    bool consumeCapture(std::span<float> input, std::span<float> output);
    std::array<float, 4> levels() const;
    double analysisSampleRate() const { return rate.load(); }
    juce::AudioProcessorValueTreeState parameters;
private:
    static juce::AudioProcessorValueTreeState::ParameterLayout layout();
    DandrumKernelInstrument* engine = nullptr;
    juce::AudioBuffer<float> inputBuffer;
    std::array<float, 7> applied {};
    std::atomic<double> rate { 48000 };
    std::array<std::atomic<float>, 4> peaks {};
    juce::AbstractFifo fifo { 32768 };
    std::array<float, 32768> captureInput {}, captureOutput {};
    uint32_t noiseState = 0x12345678;
    double phase = 0;
};
juce::AudioProcessorEditor* makeFilterEditor(FilterProcessor&);
}
