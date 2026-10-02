#pragma once

#include "InstrumentUiMeterCapture.h"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>

// A value-only, off-audio consumer. A renderer receives its snapshot, never
// the SPSC queue or a pointer into a processor buffer.
class InstrumentUiMeterAggregation final
{
public:
    struct Snapshot
    {
        std::uint32_t generation = 0;
        std::uint64_t streamId = 0;
        std::uint64_t firstSample = 0;
        std::uint64_t endSample = 0;
        std::uint64_t observedSamples = 0;
        std::array<double, InstrumentUiMeterCapture::channelCount> peak {};
        std::array<double, InstrumentUiMeterCapture::channelCount> rms {};
        bool complete = false;
    };

    void resetWindow() noexcept
    {
        started = false;
        complete = true;
        observedSamples = 0;
        peak = {};
        energy = {};
    }

    void append (const InstrumentUiMeterCapture::Frame& frame) noexcept
    {
        if (! started || frame.generation != generation || frame.streamId != streamId)
        {
            resetWindow();
            started = true;
            generation = frame.generation;
            streamId = frame.streamId;
            firstSample = frame.samplePosition;
        }
        else if (frame.sequence != nextSequence || frame.samplePosition != endSample)
            complete = false;

        nextSequence = frame.sequence + 1;
        endSample = frame.samplePosition + frame.sampleCount;
        if (! frame.valid)
        {
            complete = false;
            return;
        }

        observedSamples += frame.sampleCount;
        for (std::size_t channel = 0; channel < peak.size(); ++channel)
        {
            peak[channel] = std::max (peak[channel], static_cast<double> (frame.peak[channel]));
            energy[channel] += frame.energy[channel];
        }
    }

    void observeLostFrames (std::uint64_t total) noexcept
    {
        if (total > previousLostFrames)
            complete = false;
        previousLostFrames = total;
    }

    Snapshot snapshot() const noexcept
    {
        Snapshot result;
        result.generation = generation;
        result.streamId = streamId;
        result.firstSample = firstSample;
        result.endSample = endSample;
        result.observedSamples = observedSamples;
        result.peak = peak;
        result.complete = started && complete && observedSamples > 0;
        if (observedSamples > 0)
            for (std::size_t channel = 0; channel < result.rms.size(); ++channel)
                result.rms[channel] = std::sqrt (energy[channel] / static_cast<double> (observedSamples));
        return result;
    }

private:
    bool started = false;
    bool complete = true;
    std::uint32_t generation = 0;
    std::uint64_t streamId = 0;
    std::uint64_t firstSample = 0;
    std::uint64_t endSample = 0;
    std::uint64_t nextSequence = 0;
    std::uint64_t observedSamples = 0;
    std::uint64_t previousLostFrames = 0;
    std::array<double, InstrumentUiMeterCapture::channelCount> peak {};
    std::array<double, InstrumentUiMeterCapture::channelCount> energy {};
};
