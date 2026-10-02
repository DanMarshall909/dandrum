#include "InstrumentUiMeterAggregation.h"
#include "InstrumentUiMeterCapture.h"

#include <cmath>
#include <iostream>
#include <limits>

namespace
{
bool near (double actual, double expected)
{
    return std::isfinite (actual) && std::abs (actual - expected) < 0.000001;
}
}

int main()
{
    InstrumentUiMeterCapture capture;
    capture.beginStream();
    if (! capture.clipSnapshot().valid || capture.clipSnapshot().latched[0])
        return 1;
    capture.setEnabled (true);
    const float signedLeft[] { 0.5f, -0.5f };
    const float signedRight[] { 0.25f, -0.25f };
    capture.capture (signedLeft, signedRight, 2, 3);
    InstrumentUiMeterCapture::Frame frame;
    InstrumentUiMeterAggregation aggregation;
    if (! capture.pop (frame))
        return 1;
    aggregation.append (frame);
    const auto signedResult = aggregation.snapshot();
    if (! signedResult.complete || signedResult.generation != 3
        || signedResult.observedSamples != 2 || signedResult.firstSample != 0
        || signedResult.endSample != 2
        || ! near (signedResult.peak[0], 0.5) || ! near (signedResult.rms[0], 0.5)
        || ! near (signedResult.peak[1], 0.25) || ! near (signedResult.rms[1], 0.25))
    {
        std::cerr << "signed stereo meter aggregation used incorrect channel peak or RMS\n";
        return 1;
    }

    aggregation.resetWindow();
    if (aggregation.snapshot().complete || aggregation.snapshot().observedSamples != 0
        || ! near (aggregation.snapshot().peak[0], 0.0))
    {
        std::cerr << "reset window retained meter measurements\n";
        return 1;
    }
    InstrumentUiMeterCapture::Frame loud;
    loud.generation = 3;
    loud.streamId = 1;
    loud.sequence = 1;
    loud.samplePosition = 2;
    loud.sampleCount = 2;
    loud.peak = { 1.0f, 0.0f };
    loud.energy = { 2.0, 0.0 };
    auto silence = loud;
    silence.sequence = 2;
    silence.samplePosition = 4;
    silence.sampleCount = 6;
    silence.peak = { 0.0f, 0.0f };
    silence.energy = { 0.0, 0.0 };
    aggregation.append (loud);
    aggregation.append (silence);
    const auto split = aggregation.snapshot();
    aggregation.resetWindow();
    auto combined = loud;
    combined.sampleCount = 8;
    aggregation.append (combined);
    const auto single = aggregation.snapshot();
    if (! split.complete || ! near (split.peak[0], 1.0)
        || ! near (split.rms[0], 0.5) || split.observedSamples != 8
        || ! near (single.rms[0], split.rms[0]))
    {
        std::cerr << "unequal blocks did not use sample-weighted RMS\n";
        return 1;
    }

    aggregation.resetWindow();
    silence.sequence = 4;
    aggregation.append (silence);
    if (! near (aggregation.snapshot().peak[0], 0.0)
        || ! near (aggregation.snapshot().rms[0], 0.0))
    {
        std::cerr << "silence did not produce zero meter levels\n";
        return 1;
    }
    auto afterGap = silence;
    afterGap.sequence = 6;
    afterGap.samplePosition = silence.samplePosition + silence.sampleCount + 4;
    aggregation.append (afterGap);
    aggregation.observeLostFrames (1);
    aggregation.observeLostFrames (1);
    if (aggregation.snapshot().complete)
    {
        std::cerr << "missing visual history was presented as a complete measurement\n";
        return 1;
    }
    aggregation.resetWindow();
    auto emptyFrame = silence;
    emptyFrame.sampleCount = 0;
    aggregation.append (emptyFrame);
    if (aggregation.snapshot().complete || aggregation.snapshot().observedSamples != 0)
    {
        std::cerr << "empty interval was presented as a complete measurement\n";
        return 1;
    }
    aggregation.resetWindow();
    auto positionGap = silence;
    aggregation.append (positionGap);
    ++positionGap.sequence;
    positionGap.samplePosition += positionGap.sampleCount + 1;
    aggregation.append (positionGap);
    if (aggregation.snapshot().complete)
    {
        std::cerr << "sample-position gap was presented as complete\n";
        return 1;
    }
    aggregation.resetWindow();
    auto fresh = loud;
    fresh.generation = 4;
    fresh.streamId = 2;
    fresh.sequence = 0;
    fresh.samplePosition = 0;
    aggregation.append (fresh);
    if (! aggregation.snapshot().complete || aggregation.snapshot().generation != 4
        || aggregation.snapshot().observedSamples != 2)
    {
        std::cerr << "new stream retained an older window or loss state\n";
        return 1;
    }
    auto nextStream = fresh;
    nextStream.streamId = 3;
    aggregation.append (nextStream);
    if (! aggregation.snapshot().complete || aggregation.snapshot().streamId != 3
        || aggregation.snapshot().observedSamples != 2)
    {
        std::cerr << "stream change retained an old meter window\n";
        return 1;
    }
    auto nextGeneration = fresh;
    nextGeneration.generation = 5;
    nextGeneration.streamId = 3;
    aggregation.append (nextGeneration);
    if (! aggregation.snapshot().complete || aggregation.snapshot().generation != 5
        || aggregation.snapshot().observedSamples != 2)
    {
        std::cerr << "generation change retained an old meter window\n";
        return 1;
    }

    const float zero[] { 0.0f };
    const float clipping[] { 1.0f };
    for (std::size_t n = 0; n < InstrumentUiMeterCapture::capacity; ++n)
        capture.capture (zero, zero, 1, 4);
    capture.capture (clipping, zero, 1, 4);
    const auto firstClip = capture.clipSnapshot();
    if (capture.lostFrames() != 1 || ! firstClip.valid || firstClip.generation != 4
        || ! firstClip.latched[0] || firstClip.latched[1]
        || capture.acknowledgeClip (0, 3, firstClip.ticket[0])
        || capture.acknowledgeClip (2, 4, firstClip.ticket[0])
        || capture.acknowledgeClip (0, 4, 0))
    {
        std::cerr << "clipping lost its latch when meter history overflowed\n";
        return 1;
    }
    capture.capture (clipping, zero, 1, 4);
    if (capture.acknowledgeClip (0, 4, firstClip.ticket[0])
        || ! capture.clipSnapshot().latched[0])
    {
        std::cerr << "stale acknowledgement cleared a newer clip\n";
        return 1;
    }
    const auto currentClip = capture.clipSnapshot();
    if (! capture.acknowledgeClip (0, 4, currentClip.ticket[0])
        || capture.clipSnapshot().latched[0]
        || capture.acknowledgeClip (0, 4, currentClip.ticket[0]))
    {
        std::cerr << "current clip acknowledgement failed\n";
        return 1;
    }
    for (std::size_t n = 0; n < InstrumentUiMeterCapture::capacity; ++n)
        if (! capture.pop (frame))
            return 1;
    capture.capture (clipping, zero, 1, 4);
    if (! capture.clipSnapshot().latched[0])
    {
        std::cerr << "new clip after acknowledgement was hidden\n";
        return 1;
    }
    capture.capture (zero, zero, 1, 5);
    if (capture.clipSnapshot().generation != 5 || capture.clipSnapshot().latched[0]
        || capture.acknowledgeClip (0, 4, currentClip.ticket[0]))
    {
        std::cerr << "generation reset retained or accepted an old clip\n";
        return 1;
    }
    capture.capture (clipping, zero, 1, 5);
    if (! capture.clipSnapshot().latched[0])
    {
        std::cerr << "old-generation acknowledgement erased a new-generation clip\n";
        return 1;
    }
    const float negativeClip[] { -1.25f };
    capture.capture (zero, negativeClip, 1, 5);
    const auto stereoClip = capture.clipSnapshot();
    if (! stereoClip.latched[0] || ! stereoClip.latched[1]
        || ! capture.acknowledgeClip (1, 5, stereoClip.ticket[1])
        || ! capture.clipSnapshot().latched[0] || capture.clipSnapshot().latched[1])
    {
        std::cerr << "right-channel clip acknowledgement changed left-channel state\n";
        return 1;
    }
    const float clippedBlock[] { 1.0f, -1.25f };
    const float silentBlock[] { 0.0f, 0.0f };
    capture.capture (clippedBlock, silentBlock, 2, 5);
    const auto blockClip = capture.clipSnapshot();
    if (blockClip.ticket[0] != stereoClip.ticket[0] + 1)
    {
        std::cerr << "one clipped block published " << blockClip.ticket[0] - stereoClip.ticket[0]
                  << " clip occurrences instead of one\n";
        return 1;
    }

    const float notFinite[] { std::numeric_limits<float>::quiet_NaN() };
    capture.capture (notFinite, zero, 1, 6);
    while (capture.pop (frame) && frame.generation != 6) {}
    if (frame.generation != 6 || frame.valid
        || ! std::isfinite (frame.peak[0]) || ! std::isfinite (frame.energy[0]))
    {
        std::cerr << "non-finite input entered the numeric meter stream\n";
        return 1;
    }
    aggregation.resetWindow();
    aggregation.append (frame);
    if (aggregation.snapshot().complete)
    {
        std::cerr << "invalid sample interval was presented as complete\n";
        return 1;
    }
    if (capture.clipSnapshot().latched[0] || capture.clipSnapshot().latched[1])
    {
        std::cerr << "non-finite input triggered clipping\n";
        return 1;
    }
    capture.setEnabled (false);
    capture.capture (clipping, clipping, 1, 7);
    if (capture.clipSnapshot().generation != 6 || capture.clipSnapshot().latched[0]
        || capture.clipSnapshot().latched[1])
    {
        std::cerr << "disabled visual capture scanned or latched clipping\n";
        return 1;
    }
    return 0;
}
