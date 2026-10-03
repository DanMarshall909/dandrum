#include "InstrumentUiSpectralGeometry.h"
#include <iostream>
#include <stdexcept>

namespace
{
void require (bool condition, const char* message)
{
    if (! condition) throw std::runtime_error (message);
}
bool near (double a, double b) { return std::abs (a - b) < 0.00001; }
}

int main()
{
    try
    {
        using Geometry = InstrumentUiSpectralGeometry;
        InstrumentUiDocument::Source source;
        source.id = "sine"; source.sampleRateHz = 48000; source.channelCount = 2;
        // Above JavaScript's safe integer range; subtract frames before converting.
        constexpr std::uint64_t start = 9007199254740993ULL;
        source.frameCount = start + 4096;
        source.slices = { { "hit", start + 1024, start + 2048 } };
        InstrumentUiDocument::Region region;
        region.id = "body"; region.startFrame = start; region.endFrame = start + 4096;
        region.fadeInMs = 2; region.fadeOutMs = 4;
        region.loop = InstrumentUiDocument::RegionLoop { "forward", start + 512, start + 3072, 0 };
        InstrumentUiSpectralService::Result result;
        result.sourceId = "sine"; result.regionId = "body"; result.sampleRateHz = 48000;
        result.startFrame = start; result.endFrame = start + 4096;
        for (std::size_t bin = 0; bin < result.frequencyHz.size(); ++bin)
            result.frequencyHz[bin] = bin * 46.875;
        result.columns.resize (1);
        result.columns[0].startFrame = start; result.columns[0].endFrame = start + 1024;
        result.columns[0].magnitudeDbFS.fill (-120);
        result.columns[0].magnitudeDbFS[64] = -6.020599913f;
        result.columns[0].magnitudeDbFS[63] = -12.041199826f;
        result.columns[0].magnitudeDbFS[0] = 0; // DC must not appear at 47 Hz.
        for (const auto width : { 1200, 820 })
        {
            const auto view = Geometry::fromPrepared (source, region, result, width, 180);
            require (view.has_value(), "valid prepared spectrum geometry unavailable");
            require (near (view->columnX (start + 256), width / 16.0), "source-frame column offset lost precision");
            require (near (view->frequencyY (3000), 60), "3000 Hz did not reach the source-rate log-frequency row");
            require (near (view->frequencyY (46.875), 180) && near (view->frequencyY (24000), 0), "frequency endpoints differ");
            require (near (view->frequencyY (0), 180) && near (view->frequencyY (48000), 0), "outside frequencies were not clipped");
            require (near (view->rowMagnitude (result.columns[0], 60), -6.020599913), "known sine magnitude or row aggregation differs");
            require (view->rowMagnitude (result.columns[0], 179) == -120, "DC leaked into the lowest positive frequency row");
            require (view->markers().size() == 8, "spectral overlays missing");
            const std::array expected { 0.0, double(width), width * 96.0/4096, width * 3904.0/4096,
                                       width/8.0, width*3.0/4, width/4.0, width/2.0 };
            for (std::size_t i=0; i<expected.size(); ++i)
                require (near (view->markers()[i].x, expected[i]), "spectral prepared overlay has wrong source coordinates");
            require (near (view->startSeconds(), double(start)/48000) && near (view->endSeconds(), double(start+4096)/48000), "time labels use a different rate");
        }
        require (! Geometry::fromPrepared (source, region, result, 0, 180), "zero plot width accepted");
        require (! Geometry::fromPrepared (source, region, result, 820, 0), "zero plot height accepted");
        result.channel = 2;
        require (! Geometry::fromPrepared (source, region, result, 820, 180), "invalid source channel accepted");
        result.channel = 0; result.sampleRateHz = 44100;
        require (! Geometry::fromPrepared (source, region, result, 820, 180), "host-rate result accepted as source-rate spectrum");
        result.sampleRateHz = 48000; result.regionId = "wrong";
        require (! Geometry::fromPrepared (source, region, result, 820, 180), "wrong region accepted");
        result.regionId = "body"; result.frequencyHz[64] = NAN;
        require (! Geometry::fromPrepared (source, region, result, 820, 180), "nonfinite bin accepted");
        result.frequencyHz[64] = 3000; result.columns.clear();
        require (! Geometry::fromPrepared (source, region, result, 820, 180), "empty spectrum accepted");
        std::cout << "spectral coordinates, magnitude and source overlays PASS\n";
        return 0;
    }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
