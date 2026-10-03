#pragma once

#include "InstrumentUiSpectralService.h"
#include "InstrumentUiWaveformGeometry.h"

// Numeric presentation coordinates only. The renderer owns its image and colours.
class InstrumentUiSpectralGeometry
{
public:
    static std::optional<InstrumentUiSpectralGeometry> fromPrepared (
        const InstrumentUiDocument::Source& source, const InstrumentUiDocument::Region& region,
        const InstrumentUiSpectralService::Result& result, int width, int height)
    {
        const auto coordinates = InstrumentUiWaveformGeometry::fromPrepared (source, region, width, height);
        if (! coordinates || result.sourceId != source.id || result.regionId != region.id
            || result.sampleRateHz != source.sampleRateHz || result.channel >= source.channelCount
            || result.startFrame != region.startFrame || result.endFrame != region.endFrame
            || result.columns.empty() || ! std::isfinite (result.settings.floorDbFS)
            || result.settings.floorDbFS >= 0)
            return std::nullopt;
        for (std::size_t bin = 0; bin < result.frequencyHz.size(); ++bin)
            if (! std::isfinite (result.frequencyHz[bin]) || result.frequencyHz[bin] < 0
                || (bin > 0 && result.frequencyHz[bin] <= result.frequencyHz[bin - 1]))
                return std::nullopt;
        return InstrumentUiSpectralGeometry (*coordinates, result, height);
    }

    double columnX (std::uint64_t frame) const { return coordinates.frameX (frame); }
    double frequencyY (double hz) const
    {
        return ! std::isfinite (hz) || hz <= minimumHz ? static_cast<double> (rows.size())
            : (1.0 - std::log (std::min (hz, maximumHz) / minimumHz) / logRange) * rows.size();
    }
    float rowMagnitude (const InstrumentUiSpectralService::Column& column, int row) const
    {
        const auto [first, last] = rows[static_cast<std::size_t> (
            std::clamp (row, 0, static_cast<int> (rows.size()) - 1))];
        // Max aggregation preserves narrow bands when several FFT bins share a
        // pixel. This changes presentation only, never the measured spectrum.
        auto level = floorDbFS;
        for (auto bin = first; bin <= last; ++bin)
            level = std::max (level, column.magnitudeDbFS[bin]);
        return level;
    }
    double startSeconds() const { return coordinates.sourceSeconds (startFrame); }
    double endSeconds() const { return coordinates.sourceSeconds (endFrame); }
    const std::vector<InstrumentUiWaveformGeometry::Marker>& markers() const { return coordinates.markers(); }
private:
    InstrumentUiSpectralGeometry (InstrumentUiWaveformGeometry prepared,
                                  const InstrumentUiSpectralService::Result& result, int height)
        : coordinates (std::move (prepared)), startFrame (result.startFrame), endFrame (result.endFrame),
          minimumHz (result.frequencyHz[1]), maximumHz (result.frequencyHz.back()),
          logRange (std::log (maximumHz / minimumHz)), floorDbFS (result.settings.floorDbFS)
    {
        const auto begin = result.frequencyHz.begin() + 1; // DC has no log coordinate.
        for (int row = 0; row < height; ++row)
        {
            const auto low = minimumHz * std::exp (logRange * (1.0 - (row + 1.0) / height));
            const auto high = minimumHz * std::exp (logRange * (1.0 - double (row) / height));
            const auto first = std::lower_bound (begin, result.frequencyHz.end(), low);
            const auto last = std::upper_bound (begin, result.frequencyHz.end(), high);
            const auto nearest = static_cast<std::size_t> (std::clamp (
                std::lround (std::sqrt (low * high) / minimumHz), 1L,
                static_cast<long> (result.frequencyHz.size() - 1)));
            rows.emplace_back (first < last ? static_cast<std::size_t> (first - result.frequencyHz.begin()) : nearest,
                               first < last ? static_cast<std::size_t> (last - result.frequencyHz.begin() - 1) : nearest);
        }
    }
    InstrumentUiWaveformGeometry coordinates;
    std::uint64_t startFrame, endFrame;
    double minimumHz, maximumHz, logRange;
    float floorDbFS;
    std::vector<std::pair<std::size_t, std::size_t>> rows;
};
