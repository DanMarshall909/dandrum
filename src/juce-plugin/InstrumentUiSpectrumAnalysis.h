#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <compare>
#include <memory>
#include <span>

namespace juce::dsp { class FFT; }

// Numeric measurement shared by prepared jobs and the live worker. Construct
// and use off audio; the FFT implementation may allocate its own workspace.
class InstrumentUiSpectrumAnalysis final
{
public:
    static constexpr std::size_t fftSize = 1024;
    static constexpr std::size_t binCount = fftSize / 2 + 1;
    static constexpr std::uint64_t hopFrames = 256;
    static constexpr float floorDbFS = -120.0f;
    enum class Window { periodicHann };
    enum class Scaling { oneSidedPeakDbFS };
    enum class ChannelPolicy { selectedChannel };
    struct Settings
    {
        Window window = Window::periodicHann;
        Scaling scaling = Scaling::oneSidedPeakDbFS;
        ChannelPolicy channelPolicy = ChannelPolicy::selectedChannel;
        std::size_t fftSize = InstrumentUiSpectrumAnalysis::fftSize;
        std::uint64_t hopFrames = InstrumentUiSpectrumAnalysis::hopFrames;
        float floorDbFS = InstrumentUiSpectrumAnalysis::floorDbFS;
        auto operator<=> (const Settings&) const = default;
    };
    using Magnitudes = std::array<float, binCount>;
    InstrumentUiSpectrumAnalysis();
    ~InstrumentUiSpectrumAnalysis();
    // A short prepared-source tail is zero padded. Invalid input is rejected.
    bool analyze (std::span<const float> samples, Magnitudes& result);
private:
    std::array<float, fftSize> window {};
    double windowSum = 0;
    std::unique_ptr<juce::dsp::FFT> fft;
};
