#include "InstrumentUiLiveAnalysis.h"

#include <algorithm>
#include <cmath>
#include <iostream>
#include <limits>
#include <numbers>
#include <stdexcept>
#include <string_view>

namespace
{
using Analysis = InstrumentUiLiveAnalysis;
using Capture = InstrumentUiLiveCapture;
using Spectrum = InstrumentUiSpectrumAnalysis;
void require (bool value, const char* message, std::string_view scenario = {})
{ if (! value) throw std::runtime_error (scenario.empty() ? message : std::string (scenario) + ": " + message); }
void near (double actual, double expected, double tolerance, const char* message, std::string_view scenario = {})
{ require (std::isfinite (actual) && std::abs (actual - expected) <= tolerance, message, scenario); }
std::array<float, 1024> sine (unsigned bin, double amplitude)
{
    std::array<float, 1024> result;
    for (std::size_t n = 0; n < result.size(); ++n)
        result[n] = static_cast<float> (amplitude * std::sin (
            2 * std::numbers::pi * bin * static_cast<double> (n) / result.size()));
    return result;
}
std::size_t dominant (const Spectrum::Magnitudes& data)
{ return static_cast<std::size_t> (std::max_element (data.begin(), data.end()) - data.begin()); }

void contiguousMeasurement()
{
    auto left = sine (64, 0.5), right = sine (96, 0.25);
    Capture capture;
    Analysis analysis;
    Analysis::Result result;
    Capture::Frame frame;
    capture.beginStream();
    capture.capture (nullptr, nullptr, 13, 7, 48000);
    require (capture.setChannels (3), "Stereo analysis selection was rejected");
    capture.capture (left.data(), right.data(), 1024, 7, 48000);
    for (unsigned n = 0; n < 4; ++n)
    {
        require (capture.pop (frame), "Live analysis fixture lost PCM");
        const auto ready = analysis.consume (frame, result);
        require (ready == (n == 3), "Live analysis did not wait for a complete contiguous window");
    }
    require (result.bus == Capture::Bus::master && result.channels == 3
        && result.generation == 7 && result.sampleRateHz == 48000
        && result.streamId == frame.streamId && result.selectionId == capture.selectionId()
        && result.sequence == 3 && result.startFrame == 13 && result.endFrame == 1037
        && result.gap && result.settings.fftSize == 1024 && result.settings.hopFrames == 256
        && result.settings.floorDbFS == -120.0f
        && result.settings.window == Spectrum::Window::periodicHann
        && result.settings.scaling == Spectrum::Scaling::oneSidedPeakDbFS
        && result.settings.channelPolicy == Spectrum::ChannelPolicy::selectedChannel,
        "Live measurement lost its actual rate, identities, bounds or declared settings");
    for (unsigned k = 0; k < 513; ++k)
        near (result.frequencyHz[k], k * 46.875, 0, "Live frequency coordinates used an incorrect rate");
    require (dominant (result.channel[0].magnitudeDbFS) == 64
        && dominant (result.channel[1].magnitudeDbFS) == 96, "Live stereo bands lost channel identity");
    near (result.channel[0].magnitudeDbFS[64], -6.020599913, 0.0002,
          "Live half-amplitude sine has incorrect peak dBFS scaling");
    near (result.channel[0].magnitudeDbFS[63], -12.041199826, 0.0002,
          "Live periodic Hann window has incorrect adjacent band");
    near (result.channel[1].magnitudeDbFS[96], -12.041199826, 0.0002,
          "Live quarter-amplitude right channel has incorrect scaling");
    near (result.channel[0].scope[0].minimum, 0, 0, "Live scope lost signed minimum");
    near (result.channel[0].scope[0].maximum, 0.5, 0, "Live scope lost positive extremum");
    near (result.channel[0].scope[1].minimum, -0.5, 0, "Live scope made negative samples positive");
    near (result.channel[0].scope[1].maximum, 0, 0.000001, "Live scope lost bucket maximum");
    for (std::size_t b = 0; b < Analysis::scopeBuckets; ++b)
        require (result.channel[0].scope[b].startFrame == 13 + b * 8
            && result.channel[0].scope[b].endFrame == 21 + b * 8,
            "Live scope buckets lost sample-frame bounds");

    const auto owned = result;
    left.fill (0.875f);
    capture.capture (right.data(), right.data(), 256, 7, 48000);
    require (capture.pop (frame) && analysis.consume (frame, result)
        && result.startFrame == 269 && result.endFrame == 1293 && result.sequence == 4 && ! result.gap,
        "Live analysis did not retain the declared overlapping 256-frame hop");
    near (owned.channel[0].scope[1].minimum, -0.5, 0,
          "A retained live result changed after input or accumulator reuse");
}

void numericMeasurement()
{
    Spectrum analysis;
    Spectrum::Magnitudes result;
    std::array<float, 1024> pcm {};
    require (analysis.analyze (pcm, result), "Finite silent measurement was rejected");
    for (const auto value : result) require (value == -120.0f, "Live silence has nonfinite or incorrect floor");
    pcm.fill (0.25f);
    require (analysis.analyze (pcm, result), "Finite DC measurement was rejected");
    near (result[0], -12.041199826, 0.0001, "Shared DC endpoint was doubled as an interior bin");
    for (unsigned n = 0; n < 1024; ++n) pcm[n] = n % 2 == 0 ? 0.25f : -0.25f;
    require (analysis.analyze (pcm, result), "Finite Nyquist measurement was rejected");
    near (result[512], -12.041199826, 0.0001, "Shared Nyquist endpoint was doubled");
    pcm.fill (0.25f);
    require (analysis.analyze (std::span<const float> (pcm.data(), 256), result), "Short prepared tail was rejected");
    near (result[0], -32.921937327, 0.0001, "Shared prepared tail lost full-window zero-padding scaling");
    require (! analysis.analyze ({}, result), "Empty spectrum input was accepted");
    std::array<float, 1025> oversized {};
    require (! analysis.analyze (oversized, result), "Oversized spectrum input was accepted");
    pcm[31] = std::numeric_limits<float>::quiet_NaN();
    require (! analysis.analyze (pcm, result), "Nonfinite spectral input was accepted");
}

void largeFiniteMeasurement()
{
    Spectrum analysis;
    Spectrum::Magnitudes result;
    std::array<float, 1024> pcm;
    pcm.fill (std::numeric_limits<float>::max());
    require (analysis.analyze (pcm, result), "Large finite PCM was rejected");
    near (result[0], 770.636788839, 0.001, "Large finite DC overflowed spectral measurement");
    for (auto value : result)
        require (std::isfinite (value) && value >= -120, "Finite PCM produced nonfinite spectral data");
    for (unsigned n = 0; n < 1024; ++n) pcm[n] = n % 2 == 0
        ? std::numeric_limits<float>::max() : -std::numeric_limits<float>::max();
    require (analysis.analyze (pcm, result), "Large finite Nyquist input was rejected");
    near (result[512], 770.636788839, 0.001, "Large finite Nyquist overflowed spectral measurement");
    pcm = sine (64, std::numeric_limits<float>::max() * 0.5);
    require (analysis.analyze (pcm, result), "Large finite sine was rejected");
    near (result[64], 764.616188926, 0.001, "Large finite sine lost its declared dBFS scaling");
}

void singleChannelAndInvalidPcm()
{
    Analysis analysis;
    Capture capture;
    Capture::Frame frame;
    Analysis::Result result;
    std::array<float, 1024> pcm {};
    pcm[19] = std::numeric_limits<float>::infinity();
    capture.setChannels (2);
    capture.beginStream();
    capture.capture (nullptr, pcm.data(), 1024, 1, 44100);
    while (capture.pop (frame))
        require (! analysis.consume (frame, result), "Nonfinite live PCM emitted a numeric window");
    pcm.fill (0);
    pcm[0] = -0.75f; pcm[1] = 0.5f;
    capture.capture (nullptr, pcm.data(), 1024, 1, 44100);
    unsigned windows = 0;
    while (capture.pop (frame)) if (analysis.consume (frame, result))
    {
        if (++windows == 1)
            require (result.startFrame == 256 && result.endFrame == 1280 && result.gap
                && result.channel[1].scope[96].minimum == -0.75f
                && result.channel[1].scope[96].maximum == 0.5f,
                "Recovery did not begin at the first complete valid interval after bad PCM");
    }
    require (windows == 4 && result.channels == 2 && ! result.gap
        && result.startFrame == 1024 && result.endFrame == 2048,
        "Finite live input did not recover a full signed selected-channel window");
    require (result.channel[1].scope[0].minimum == -0.75f
        && result.channel[1].scope[0].maximum == 0.5f
        && result.channel[0].scope[0].minimum == 0 && result.channel[0].scope[0].maximum == 0,
        "Right-only live scope lost transients or leaked an unselected channel");
    near (result.frequencyHz[64], 2756.25, 0, "Live measurement did not use 44.1 kHz coordinates");
}

Capture::Frame toneFrame (const std::array<float, 1024>& tone, unsigned chunk,
                          std::uint64_t origin, std::uint64_t sequence)
{
    Capture::Frame frame;
    frame.channels = 1; frame.generation = 7; frame.sampleRateHz = 48000;
    frame.streamId = 5; frame.selectionId = 257; frame.sequence = sequence;
    frame.samplePosition = origin; frame.sampleCount = 256;
    std::copy_n (tone.begin() + chunk * 256, 256, frame.pcm[0].begin());
    std::copy_n (tone.begin() + chunk * 256, 256, frame.pcm[1].begin());
    return frame;
}
void discontinuities()
{
    const auto before = sine (64, 0.5), after = sine (96, 0.25);
    constexpr std::string_view names[] = { "gap", "sequence", "position", "generation",
        "stream", "selection", "rate", "channel", "reset" };
    for (unsigned change = 0; change < 9; ++change)
    {
        Analysis analysis;
        Analysis::Result result;
        for (unsigned n = 0; n < 2; ++n)
            require (! analysis.consume (toneFrame (before, n, 100 + n * 256, n), result),
                     "Partial pre-gap window was emitted", names[change]);
        if (change == 8) analysis.reset();
        const auto origin = change == 2 ? 1612U : 612U;
        for (unsigned n = 0; n < 5; ++n)
        {
            auto frame = toneFrame (after, n % 4, origin + n * 256, 2 + n + (change == 1 ? 10 : 0));
            frame.gap = change == 0 && n == 0;
            if (change == 3) ++frame.generation;
            if (change == 4) ++frame.streamId;
            if (change == 5) frame.selectionId += 256;
            if (change == 6) frame.sampleRateHz = 96000;
            if (change == 7) { frame.channels = 2; frame.selectionId = 258; }
            const auto ready = analysis.consume (frame, result);
            require (ready == (n >= 3), "Live discontinuity joined nonadjacent partial windows", names[change]);
            if (! ready) continue;
            const auto channel = change == 7 ? 1U : 0U;
            require (result.startFrame == origin + (n - 3) * 256
                && result.endFrame == origin + (n + 1) * 256
                && result.gap == (n == 3) && result.generation == frame.generation
                && result.streamId == frame.streamId && result.selectionId == frame.selectionId,
                "Restarted live window retained stale coordinates or identities", names[change]);
            require (dominant (result.channel[channel].magnitudeDbFS) == 96,
                     "Restarted live spectrum retained the pre-gap tone", names[change]);
            near (result.channel[channel].magnitudeDbFS[96], -12.041199826, 0.0002,
                  "Restarted live spectrum has a mixed-window amplitude", names[change]);
            require (result.channel[channel].magnitudeDbFS[64] <= -115,
                     "Restarted live spectrum contains the discarded pre-gap band", names[change]);
            near (result.frequencyHz[96], change == 6 ? 9000 : 4500, 0,
                  "Restarted live spectrum retained an old sample rate", names[change]);
        }
    }
}

void malformedFrames()
{
    const auto before = sine (64, 0.5), after = sine (96, 0.25);
    constexpr std::string_view names[] = { "empty", "oversized", "disabled", "unsupported-channel",
        "zero-rate", "zero-generation", "time-overflow", "nonfinite", "unsupported-bus", "selection-mask" };
    for (unsigned fault = 0; fault < 10; ++fault)
    {
        Analysis analysis;
        Analysis::Result result;
        for (unsigned n = 0; n < 2; ++n)
            analysis.consume (toneFrame (before, n, n * 256, n), result);
        auto invalid = toneFrame (before, 2, 512, 2);
        if (fault == 0) invalid.sampleCount = 0;
        if (fault == 1) invalid.sampleCount = 257;
        if (fault == 2) invalid.channels = 0;
        if (fault == 3) invalid.channels = 4;
        if (fault == 4) invalid.sampleRateHz = 0;
        if (fault == 5) invalid.generation = 0;
        if (fault == 6) invalid.samplePosition = std::numeric_limits<std::uint64_t>::max() - 2;
        if (fault == 7) invalid.pcm[0][127] = std::numeric_limits<float>::quiet_NaN();
        if (fault == 8) invalid.bus = static_cast<Capture::Bus> (1);
        if (fault == 9) invalid.selectionId = 258;
        require (! analysis.consume (invalid, result), "Malformed live input emitted a window", names[fault]);
        for (unsigned n = 0; n < 4; ++n)
        {
            auto frame = toneFrame (after, n, 768 + n * 256, 3 + n);
            // Invalid PCM in an unselected channel does not spoil selected audio.
            frame.pcm[1][0] = std::numeric_limits<float>::infinity();
            require (analysis.consume (frame, result) == (n == 3),
                     "Invalid live input contaminated recovery or reset unselected data", names[fault]);
        }
        require (result.startFrame == 768 && result.endFrame == 1792 && result.gap
            && dominant (result.channel[0].magnitudeDbFS) == 96,
            "Malformed-frame recovery retained an old partial window", names[fault]);
        near (result.channel[0].magnitudeDbFS[96], -12.041199826, 0.0002,
              "Malformed-frame recovery produced incorrect amplitude", names[fault]);
    }
}
void unevenChunks()
{
    const auto tone = sine (64, 0.5);
    constexpr unsigned sizes[] = { 256, 256, 256, 200, 100, 212 };
    Analysis analysis;
    Analysis::Result result;
    std::uint64_t position = 17;
    for (unsigned block = 0; block < 6; ++block)
    {
        auto frame = toneFrame (tone, 0, position, block);
        frame.sampleCount = sizes[block];
        for (unsigned n = 0; n < sizes[block]; ++n) frame.pcm[0][n] = tone[(position - 17 + n) % 1024];
        const auto ready = analysis.consume (frame, result);
        require (ready == (block >= 4), "Unequal chunks changed live window admission");
        if (ready)
        {
            require (result.startFrame == (block == 4 ? 17 : 273)
                && result.endFrame == (block == 4 ? 1041 : 1297)
                && result.sequence == block && result.gap == (block == 4),
                "A window ending inside a capture chunk lost its continuation");
            near (result.channel[0].magnitudeDbFS[64], -6.020599913, 0.0002,
                  "Unequal live chunks changed known spectral amplitude");
        }
        position += sizes[block];
    }
}
}
int main()
{
    try { contiguousMeasurement(); numericMeasurement(); singleChannelAndInvalidPcm(); discontinuities(); malformedFrames(); unevenChunks(); largeFiniteMeasurement(); }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
    std::cout << "LIVE_ANALYSIS signed scope and declared spectral measurement PASS\n";
}
