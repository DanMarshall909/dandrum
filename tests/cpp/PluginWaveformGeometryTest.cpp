#include "InstrumentUiWaveformGeometry.h"

#include <cmath>
#include <iostream>
#include <optional>
#include <string>

namespace
{
using Geometry = InstrumentUiWaveformGeometry;

bool near (double actual, double expected)
{
    return std::abs (actual - expected) < 0.00001;
}

std::optional<double> markerX (const Geometry& geometry, Geometry::MarkerKind kind,
                               const std::string& id = {})
{
    for (const auto& marker : geometry.markers())
        if (marker.kind == kind && marker.id == id)
            return marker.x;
    return std::nullopt;
}
}

int main()
{
    InstrumentUiDocument::Source source;
    source.id = "break";
    source.sampleRateHz = 48000;
    source.channelCount = 2;
    source.frameCount = 24000;
    source.slices = { { "before", 0, 1000 }, { "kick", 1000, 6000 },
                      { "hat", 6000, 9000 }, { "after", 13000, 16000 } };
    InstrumentUiDocument::Region region;
    region.id = "body";
    region.startFrame = 1000;
    region.endFrame = 13000;
    region.fadeInMs = 2.0;
    region.fadeOutMs = 3.0;
    region.loop = InstrumentUiDocument::RegionLoop { "forward", 2000, 12000, 1.0 };

    const auto full = Geometry::fromPrepared (source, region, 1200.0, 100.0);
    const auto compact = Geometry::fromPrepared (source, region, 820.0, 100.0);
    if (! full || ! compact || ! near (full->durationSeconds(), 0.25)
        || ! near (full->frameX (1000), 0.0)
        || ! near (full->frameX (13000), 1200.0)
        || ! near (full->frameX (0), 0.0)
        || ! near (full->frameX (24000), 1200.0)
        || ! near (full->sourceSeconds (12000), 0.25)
        || ! near (full->sampleY (-0.75), 87.5)
        || ! near (full->sampleY (0.5), 25.0)
        || ! near (full->sampleY (2.0), 0.0)
        || ! near (full->sampleY (-2.0), 100.0)
        || ! near (full->sampleY (NAN), 50.0)
        || ! full->bucketX (1000, 2000) || ! near (*full->bucketX (1000, 2000), 50.0)
        || full->bucketX (0, 2000) || full->bucketX (1000, 1000)
        || full->bucketX (12000, 14000))
    {
        std::cerr << "waveform geometry lost source-rate, signed or region-bucket coordinates\n";
        return 1;
    }
    if (! markerX (*full, Geometry::MarkerKind::regionStart)
        || ! near (*markerX (*full, Geometry::MarkerKind::regionStart), 0.0)
        || ! near (*markerX (*full, Geometry::MarkerKind::regionEnd), 1200.0)
        || ! near (*markerX (*full, Geometry::MarkerKind::fadeInEnd), 9.6)
        || ! near (*markerX (*full, Geometry::MarkerKind::fadeOutStart), 1185.6)
        || ! near (*markerX (*full, Geometry::MarkerKind::loopStart), 100.0)
        || ! near (*markerX (*full, Geometry::MarkerKind::loopEnd), 1100.0)
        || ! near (*markerX (*full, Geometry::MarkerKind::sliceEnd, "kick"), 500.0)
        || ! near (*markerX (*full, Geometry::MarkerKind::sliceStart, "hat"), 500.0)
        || ! near (*markerX (*full, Geometry::MarkerKind::sliceEnd, "hat"), 800.0)
        || markerX (*full, Geometry::MarkerKind::sliceStart, "before")
        || markerX (*full, Geometry::MarkerKind::sliceEnd, "after")
        || ! near (*markerX (*compact, Geometry::MarkerKind::fadeInEnd), 6.56)
        || ! near (*markerX (*compact, Geometry::MarkerKind::loopStart), 68.3333333333)
        || ! near (*markerX (*compact, Geometry::MarkerKind::sliceEnd, "kick"), 341.666666667))
    {
        std::cerr << "prepared markers did not retain full/compact source-frame alignment\n";
        return 1;
    }

    if (Geometry::fromPrepared (source, region, 0.0, 100.0)
        || Geometry::fromPrepared (source, region, NAN, 100.0))
    {
        std::cerr << "invalid viewport was accepted\n";
        return 1;
    }
    source.sampleRateHz = 0;
    if (Geometry::fromPrepared (source, region, 1200.0, 100.0))
        return 1;
    source.sampleRateHz = 48000;
    region.endFrame = region.startFrame;
    if (Geometry::fromPrepared (source, region, 1200.0, 100.0))
        return 1;
    region.endFrame = source.frameCount + 1;
    if (Geometry::fromPrepared (source, region, 1200.0, 100.0))
        return 1;
    region.endFrame = 13000;
    region.loop->startFrame = 0;
    if (Geometry::fromPrepared (source, region, 1200.0, 100.0))
        return 1;
    region.loop.reset();
    region.fadeInMs = 0.0;
    region.fadeOutMs = 0.0;
    source.slices.clear();
    const auto plain = Geometry::fromPrepared (source, region, 1200.0, 100.0);
    if (! plain || plain->markers().size() != 2)
    {
        std::cerr << "unprepared overlays were invented for a plain region\n";
        return 1;
    }
    return 0;
}
