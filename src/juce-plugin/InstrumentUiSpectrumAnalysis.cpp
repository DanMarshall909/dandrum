#include "InstrumentUiSpectrumAnalysis.h"

#include <cmath>
#include <numbers>
#include <numeric>
#include <juce_dsp/juce_dsp.h>

InstrumentUiSpectrumAnalysis::InstrumentUiSpectrumAnalysis()
    : fft (std::make_unique<juce::dsp::FFT> (10))
{
    for (std::size_t n = 0; n < fftSize; ++n)
        window[n] = static_cast<float> (0.5 * (1.0 - std::cos (
            2.0 * std::numbers::pi * static_cast<double> (n) / fftSize)));
    windowSum = std::accumulate (window.begin(), window.end(), 0.0);
}
InstrumentUiSpectrumAnalysis::~InstrumentUiSpectrumAnalysis() = default;
bool InstrumentUiSpectrumAnalysis::analyze (std::span<const float> samples, Magnitudes& result)
{
    if (samples.empty() || samples.size() > fftSize) return false;
    double scale = 1.0;
    for (const auto sample : samples)
    {
        if (! std::isfinite (sample)) return false;
        scale = std::max (scale, std::abs (static_cast<double> (sample)));
    }
    // Bound float FFT arithmetic even for finite overrange PCM. Restoring its
    // scale in double keeps the declared peak dBFS convention unchanged.
    std::array<float, fftSize * 2> data {};
    for (std::size_t n = 0; n < samples.size(); ++n)
        data[n] = static_cast<float> (samples[n] / scale * window[n]);
    fft->performFrequencyOnlyForwardTransform (data.data(), true);
    for (std::size_t bin = 0; bin < binCount; ++bin)
    {
        const auto amplitude = data[bin] * (bin == 0 || bin == fftSize / 2 ? 1.0 : 2.0) / windowSum * scale;
        result[bin] = amplitude > 0.0
            ? static_cast<float> (std::max<double> (floorDbFS, 20.0 * std::log10 (amplitude)))
            : floorDbFS;
    }
    return true;
}
