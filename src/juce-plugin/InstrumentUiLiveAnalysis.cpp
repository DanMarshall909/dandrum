#include "InstrumentUiLiveAnalysis.h"

#include <algorithm>
#include <cmath>
#include <limits>

void InstrumentUiLiveAnalysis::reset() noexcept
{
    filled = 0; hasPrevious = false; nextGap = true;
}

bool InstrumentUiLiveAnalysis::consume (const Capture::Frame& frame, Result& output)
{
    if (frame.bus != Capture::Bus::master || frame.sampleCount == 0
        || frame.sampleCount > Capture::chunkFrames || frame.channels == 0
        || frame.channels >= (1U << Capture::channelCount) || frame.generation == 0
        || frame.sampleRateHz == 0 || static_cast<std::uint8_t> (frame.selectionId) != frame.channels
        || frame.samplePosition > std::numeric_limits<std::uint64_t>::max() - frame.sampleCount)
    { reset(); return false; }
    for (std::size_t channel = 0; channel < Capture::channelCount; ++channel)
        if ((frame.channels & (1U << channel)) != 0)
            for (std::size_t n = 0; n < frame.sampleCount; ++n)
                if (! std::isfinite (frame.pcm[channel][n])) { reset(); return false; }
    const Identity current { frame.generation, frame.sampleRateHz,
                             frame.streamId, frame.selectionId };
    if (! hasPrevious || frame.gap || current != previous
        || frame.sequence != nextSequence || frame.samplePosition != nextPosition)
        reset();
    previous = current; hasPrevious = true;
    nextSequence = frame.sequence + 1; nextPosition = frame.samplePosition + frame.sampleCount;
    if (filled == 0) windowStart = frame.samplePosition;
    bool ready = false;
    for (std::size_t n = 0; n < frame.sampleCount; ++n)
    {
        for (std::size_t channel = 0; channel < Capture::channelCount; ++channel)
            if ((frame.channels & (1U << channel)) != 0)
                samples[channel][filled] = frame.pcm[channel][n];
        if (++filled < Spectrum::fftSize) continue;
        Result result;
        result.bus = frame.bus; result.channels = frame.channels;
        result.generation = frame.generation; result.sampleRateHz = frame.sampleRateHz;
        result.streamId = frame.streamId; result.selectionId = frame.selectionId;
        result.sequence = frame.sequence; result.startFrame = windowStart;
        result.endFrame = windowStart + Spectrum::fftSize; result.gap = nextGap;
        for (std::size_t bin = 0; bin < Spectrum::binCount; ++bin)
            result.frequencyHz[bin] = static_cast<double> (bin) * frame.sampleRateHz / Spectrum::fftSize;
        for (std::size_t channel = 0; channel < Capture::channelCount; ++channel)
        {
            if ((frame.channels & (1U << channel)) == 0) continue;
            // Full size and finite selected samples have already been validated.
            spectrum.analyze (samples[channel], result.channel[channel].magnitudeDbFS);
            for (std::size_t bucket = 0; bucket < scopeBuckets; ++bucket)
            {
                constexpr auto width = Spectrum::fftSize / scopeBuckets;
                const auto first = samples[channel].begin() + bucket * width;
                const auto extrema = std::minmax_element (first, first + width);
                result.channel[channel].scope[bucket] = {
                    windowStart + bucket * width, windowStart + (bucket + 1) * width,
                    *extrema.first, *extrema.second };
            }
            std::copy (samples[channel].begin() + Spectrum::hopFrames,
                       samples[channel].end(), samples[channel].begin());
        }
        output = result; ready = true; nextGap = false;
        filled = Spectrum::fftSize - Spectrum::hopFrames;
        windowStart += Spectrum::hopFrames;
    }
    return ready;
}
