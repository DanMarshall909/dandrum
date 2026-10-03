#pragma once

#include "InstrumentUiLiveCapture.h"
#include "InstrumentUiSpectrumAnalysis.h"

// Sole-worker accumulator. Both renderers receive owned numeric values; this
// object has no engine, editor, browser, image or scheduling dependency.
class InstrumentUiLiveAnalysis final
{
public:
    using Capture = InstrumentUiLiveCapture;
    using Spectrum = InstrumentUiSpectrumAnalysis;
    static constexpr std::size_t scopeBuckets = 128;
    static_assert (Capture::chunkFrames <= Spectrum::hopFrames);
    static_assert (Spectrum::fftSize % scopeBuckets == 0);
    struct Bucket
    {
        std::uint64_t startFrame = 0, endFrame = 0;
        float minimum = 0, maximum = 0;
    };
    struct Channel
    {
        std::array<Bucket, scopeBuckets> scope {};
        Spectrum::Magnitudes magnitudeDbFS {};
    };
    struct Result
    {
        Capture::Bus bus = Capture::Bus::master;
        std::uint8_t channels = 0;
        std::uint32_t generation = 0, sampleRateHz = 0;
        std::uint64_t streamId = 0, selectionId = 0, sequence = 0;
        std::uint64_t startFrame = 0, endFrame = 0;
        bool gap = false;
        Spectrum::Settings settings;
        std::array<double, Spectrum::binCount> frequencyHz {};
        std::array<Channel, Capture::channelCount> channel {};
    };
    // Each capture chunk is at most one hop, so it yields at most one window.
    // False leaves output untouched; no partial or padded live window is emitted.
    bool consume (const Capture::Frame&, Result& output);
    void reset() noexcept;
private:
    struct Identity
    {
        std::uint32_t generation = 0, sampleRateHz = 0;
        std::uint64_t streamId = 0, selectionId = 0;
        bool operator== (const Identity&) const = default;
    };
    Spectrum spectrum;
    std::array<std::array<float, Spectrum::fftSize>, Capture::channelCount> samples {};
    std::size_t filled = 0;
    std::uint64_t windowStart = 0;
    Identity previous;
    std::uint64_t nextSequence = 0, nextPosition = 0;
    bool hasPrevious = false;
    bool nextGap = true;
};
