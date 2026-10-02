#pragma once

#include "InstrumentUiDocument.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

// Copies source-frame display coordinates from prepared metadata. No marker
// retains a reference into an instrument that may be replaced on reload.
class InstrumentUiWaveformGeometry final
{
public:
    enum class MarkerKind
    {
        regionStart, regionEnd, fadeInEnd, fadeOutStart,
        loopStart, loopEnd, sliceStart, sliceEnd
    };

    struct Marker
    {
        MarkerKind kind;
        std::string id;
        double x = 0.0;
    };

    static std::optional<InstrumentUiWaveformGeometry> fromPrepared (
        const InstrumentUiDocument::Source& source,
        const InstrumentUiDocument::Region& region,
        double width, double height)
    {
        if (source.sampleRateHz == 0 || region.startFrame >= region.endFrame
            || region.endFrame > source.frameCount
            || ! std::isfinite (width) || width <= 0.0
            || ! std::isfinite (height) || height <= 0.0
            || ! std::isfinite (region.fadeInMs) || region.fadeInMs < 0.0
            || ! std::isfinite (region.fadeOutMs) || region.fadeOutMs < 0.0)
            return std::nullopt;
        if (region.loop && (region.loop->startFrame < region.startFrame
                            || region.loop->endFrame > region.endFrame
                            || region.loop->startFrame >= region.loop->endFrame))
            return std::nullopt;

        InstrumentUiWaveformGeometry geometry (
            region.startFrame, region.endFrame, source.sampleRateHz, width, height);
        geometry.markers_.push_back ({ MarkerKind::regionStart, {}, 0.0 });
        geometry.markers_.push_back ({ MarkerKind::regionEnd, {}, width });
        const auto frameSpan = static_cast<double> (region.endFrame - region.startFrame);
        if (region.fadeInMs > 0.0)
            geometry.markers_.push_back ({ MarkerKind::fadeInEnd, {},
                geometry.offsetX (std::min (frameSpan,
                    region.fadeInMs * source.sampleRateHz / 1000.0)) });
        if (region.fadeOutMs > 0.0)
            geometry.markers_.push_back ({ MarkerKind::fadeOutStart, {},
                geometry.offsetX (frameSpan - std::min (frameSpan,
                    region.fadeOutMs * source.sampleRateHz / 1000.0)) });
        if (region.loop)
        {
            geometry.markers_.push_back ({ MarkerKind::loopStart, {},
                                           geometry.frameX (region.loop->startFrame) });
            geometry.markers_.push_back ({ MarkerKind::loopEnd, {},
                                           geometry.frameX (region.loop->endFrame) });
        }
        for (const auto& slice : source.slices)
        {
            if (slice.startFrame > region.startFrame && slice.startFrame < region.endFrame)
                geometry.markers_.push_back ({ MarkerKind::sliceStart, slice.id,
                                               geometry.frameX (slice.startFrame) });
            if (slice.endFrame > region.startFrame && slice.endFrame < region.endFrame)
                geometry.markers_.push_back ({ MarkerKind::sliceEnd, slice.id,
                                               geometry.frameX (slice.endFrame) });
        }
        return geometry;
    }

    double frameX (std::uint64_t frame) const noexcept
    {
        return offsetX (static_cast<double> (std::clamp (frame, startFrame, endFrame)
                                             - startFrame));
    }

    std::optional<double> bucketX (std::uint64_t bucketStart,
                                   std::uint64_t bucketEnd) const noexcept
    {
        if (bucketStart < startFrame || bucketStart >= bucketEnd || bucketEnd > endFrame)
            return std::nullopt;
        return offsetX (static_cast<double> (bucketStart - startFrame)
                        + static_cast<double> (bucketEnd - bucketStart) * 0.5);
    }

    double sampleY (double signedSample) const noexcept
    {
        return std::isfinite (signedSample)
            ? (1.0 - std::clamp (signedSample, -1.0, 1.0)) * height * 0.5
            : height * 0.5;
    }

    double sourceSeconds (std::uint64_t frame) const noexcept
    {
        return static_cast<double> (frame) / sampleRateHz;
    }

    double durationSeconds() const noexcept
    {
        return static_cast<double> (endFrame - startFrame) / sampleRateHz;
    }

    const std::vector<Marker>& markers() const noexcept { return markers_; }

private:
    InstrumentUiWaveformGeometry (std::uint64_t start, std::uint64_t end,
                                  std::uint32_t rate, double viewportWidth,
                                  double viewportHeight)
        : startFrame (start), endFrame (end), sampleRateHz (rate),
          width (viewportWidth), height (viewportHeight) {}

    double offsetX (double offsetFrames) const noexcept
    {
        return offsetFrames / static_cast<double> (endFrame - startFrame) * width;
    }

    std::uint64_t startFrame;
    std::uint64_t endFrame;
    std::uint32_t sampleRateHz;
    double width;
    double height;
    std::vector<Marker> markers_;
};
