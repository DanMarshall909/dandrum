#pragma once

#include "InstrumentUiMeterAggregation.h"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <cstdint>

// Message-thread display values shared by native and Web renderers. Measured
// levels attack immediately and decay by elapsed time during quieter blocks.
class InstrumentUiMeterDisplay final
{
public:
    static constexpr double decayMilliseconds = 300.0;

    struct Snapshot
    {
        std::uint32_t generation = 0;
        std::uint64_t streamId = 0;
        std::array<double, InstrumentUiMeterCapture::channelCount> peak {};
        std::array<double, InstrumentUiMeterCapture::channelCount> rms {};
        std::array<bool, InstrumentUiMeterCapture::channelCount> clipped {};
        bool complete = false;
        bool valid = false;
    };

    void reset() noexcept
    {
        current = {};
        lastTimeMilliseconds = 0.0;
    }

    bool ingest (const InstrumentUiMeterAggregation::Snapshot& meter,
                 const InstrumentUiMeterCapture::ClipSnapshot& clip,
                 double nowMilliseconds) noexcept
    {
        if (! clip.valid || clip.generation != meter.generation
            || ! std::isfinite (nowMilliseconds) || nowMilliseconds < 0.0)
            return false;
        for (std::size_t channel = 0; channel < current.peak.size(); ++channel)
            if (! std::isfinite (meter.peak[channel]) || ! std::isfinite (meter.rms[channel])
                || meter.peak[channel] < 0.0 || meter.rms[channel] < 0.0)
                return false;

        const bool newIdentity = ! current.valid || current.generation != meter.generation
                                 || current.streamId != meter.streamId
                                 || nowMilliseconds < lastTimeMilliseconds;
        const double decay = newIdentity ? 0.0
            : std::exp (-(nowMilliseconds - lastTimeMilliseconds) / decayMilliseconds);
        for (std::size_t channel = 0; channel < current.peak.size(); ++channel)
        {
            current.peak[channel] = std::max (std::min (1.0, meter.peak[channel]),
                                              current.peak[channel] * decay);
            current.rms[channel] = std::max (std::min (1.0, meter.rms[channel]),
                                             current.rms[channel] * decay);
            current.clipped[channel] = clip.latched[channel];
        }
        current.generation = meter.generation;
        current.streamId = meter.streamId;
        current.complete = meter.complete;
        current.valid = true;
        lastTimeMilliseconds = nowMilliseconds;
        return true;
    }

    Snapshot snapshot() const noexcept { return current; }

private:
    Snapshot current;
    double lastTimeMilliseconds = 0.0;
};
