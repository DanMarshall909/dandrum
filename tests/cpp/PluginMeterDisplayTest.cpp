#include "InstrumentUiMeterDisplay.h"

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
    InstrumentUiMeterDisplay display;
    if (display.snapshot().valid)
        return 1;

    InstrumentUiMeterAggregation::Snapshot meter;
    meter.generation = 3;
    meter.streamId = 1;
    meter.peak = { 0.5, 0.25 };
    meter.rms = { 0.5, 0.25 };
    meter.complete = true;
    InstrumentUiMeterCapture::ClipSnapshot clip;
    clip.generation = 3;
    clip.valid = true;
    if (! display.ingest (meter, clip, 1000.0))
        return 1;
    const auto first = display.snapshot();
    if (! first.valid || ! first.complete || first.generation != 3 || first.streamId != 1
        || ! near (first.peak[0], 0.5) || ! near (first.rms[0], 0.5)
        || ! near (first.peak[1], 0.25) || ! near (first.rms[1], 0.25))
    {
        std::cerr << "display did not preserve signed stereo measurement levels\n";
        return 1;
    }

    meter.peak = {};
    meter.rms = {};
    if (! display.ingest (meter, clip, 1300.0))
        return 1;
    const auto faded = display.snapshot();
    if (! near (faded.peak[0], 0.5 * std::exp (-1.0))
        || ! near (faded.rms[1], 0.25 * std::exp (-1.0)))
    {
        std::cerr << "meter level decay did not use elapsed time\n";
        return 1;
    }

    meter.peak = { 0.8, 1.25 };
    meter.rms = { 0.4, 0.7 };
    clip.latched = { false, true };
    if (! display.ingest (meter, clip, 1600.0))
        return 1;
    const auto raised = display.snapshot();
    if (! near (raised.peak[0], 0.8) || ! near (raised.rms[0], 0.4)
        || ! near (raised.peak[1], 1.0) || ! raised.clipped[1] || raised.clipped[0])
    {
        std::cerr << "new level or independent clip state was rendered incorrectly\n";
        return 1;
    }

    meter.peak = {};
    meter.rms = {};
    meter.complete = false;
    if (! display.ingest (meter, clip, 1900.0))
        return 1;
    const auto incomplete = display.snapshot();
    if (incomplete.complete || ! incomplete.clipped[1]
        || ! near (incomplete.peak[0], 0.8 * std::exp (-1.0)))
    {
        std::cerr << "dropped history or latched clipping disappeared during decay\n";
        return 1;
    }
    clip.latched[1] = false;
    if (! display.ingest (meter, clip, 2200.0) || display.snapshot().clipped[1])
    {
        std::cerr << "clip acknowledgement was not reflected in the display\n";
        return 1;
    }

    meter.streamId = 2;
    if (! display.ingest (meter, clip, 2300.0)
        || ! near (display.snapshot().peak[0], 0.0))
    {
        std::cerr << "new stream retained an old level\n";
        return 1;
    }
    meter.generation = 4;
    meter.peak[0] = 0.25;
    meter.rms[0] = 0.125;
    clip.generation = 4;
    if (! display.ingest (meter, clip, 2400.0)
        || display.snapshot().generation != 4
        || ! near (display.snapshot().peak[0], 0.25))
    {
        std::cerr << "new generation retained the old display state\n";
        return 1;
    }

    const auto beforeInvalid = display.snapshot();
    clip.valid = false;
    if (display.ingest (meter, clip, 2500.0))
        return 1;
    clip.valid = true;
    clip.generation = 3;
    if (display.ingest (meter, clip, 2500.0))
        return 1;
    clip.generation = 4;
    if (display.ingest (meter, clip, std::numeric_limits<double>::quiet_NaN()))
        return 1;
    if (display.ingest (meter, clip, -1.0))
        return 1;
    meter.peak[0] = std::numeric_limits<double>::infinity();
    if (display.ingest (meter, clip, 2500.0)
        || ! near (display.snapshot().peak[0], beforeInvalid.peak[0]))
    {
        std::cerr << "invalid telemetry changed the display model\n";
        return 1;
    }
    meter.peak[0] = -0.1;
    if (display.ingest (meter, clip, 2500.0))
        return 1;
    meter.peak[0] = 0.0;
    meter.rms[0] = std::numeric_limits<double>::quiet_NaN();
    if (display.ingest (meter, clip, 2500.0))
        return 1;
    meter.rms[0] = -0.1;
    if (display.ingest (meter, clip, 2500.0))
        return 1;
    meter.rms[0] = 0.0;
    if (! display.ingest (meter, clip, 2000.0)
        || ! near (display.snapshot().peak[0], 0.0))
    {
        std::cerr << "clock reset retained an old peak\n";
        return 1;
    }
    display.reset();
    if (display.snapshot().valid || ! near (display.snapshot().peak[0], 0.0))
    {
        std::cerr << "hidden meter retained its displayed level\n";
        return 1;
    }
    return 0;
}
