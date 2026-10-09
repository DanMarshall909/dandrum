#pragma once
#include <slint.h>
#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace dandrum::advanced_sampler {
// Deterministic silent preview from the imported guide, not an FFT or audio analysis.
inline constexpr unsigned spectral_rows = 64;
inline constexpr unsigned spectral_max_columns = 4096;
struct SpectralPixels { unsigned width{}, height{}; std::vector<std::uint8_t> rgba; };
using SpectralPalette = std::array<std::uint32_t, 5>;
namespace spectral_detail {
inline float amplitude(float value) noexcept { return std::isfinite(value) ? std::clamp(value, 0.f, 1.f) : 0.f; }
inline double bound(double value, double fallback) noexcept { return std::isfinite(value) ? std::clamp(value, 0., 1.) : fallback; }
inline std::size_t source_index(std::size_t x, std::size_t columns, std::size_t input_size) noexcept {
    return columns <= 1 ? 0 : static_cast<std::size_t>(static_cast<long double>(x) * (input_size - 1) / (columns - 1));
}
inline std::vector<float> bounded(std::span<const float> input) {
    const auto columns = std::min<std::size_t>(input.size(), spectral_max_columns);
    std::vector<float> result(columns);
    for (std::size_t x = 0; x < columns; ++x) result[x] = amplitude(input[source_index(x, columns, input.size())]);
    return result;
}
inline std::string_view profile(std::string_view value) noexcept {
    return value == "snare" || value == "snare-soft" || value == "hat-closed" || value == "hat-open" || value == "break" ? value : "kick";
}
inline double energy(std::string_view kind, double f, double t, double hit_random) {
    if (kind == "snare") return .55 * std::exp(-std::pow((f - .12) * 9, 2)) + .5 * std::exp(-f * 1.4) * std::exp(-t * 4);
    if (kind == "snare-soft") return .5 * std::exp(-std::pow((f - .12) * 9, 2)) + .35 * std::exp(-f * 2) * std::exp(-t * 5);
    if (kind == "hat-closed") return std::pow(f, 1.4) * .9 + .05;
    if (kind == "hat-open") return std::pow(f, 1.2) * .85 + .06;
    if (kind == "break") return hit_random < .5 ? std::exp(-f * 8) : .6 * std::exp(-f * 8) + .6 * std::pow(f, 1.3);
    return std::exp(-f * 9) * (1 + .6 * std::exp(-t * 40)) + .25 * std::exp(-t * 60) * std::exp(-f * 2);
}
}
inline SpectralPixels spectral_pixels(std::span<const float> input, std::string_view kind, bool reversed,
                                     double region_start, double region_end, const SpectralPalette& palette) {
    const auto amplitudes = spectral_detail::bounded(input);
    if (amplitudes.empty()) return {};
    SpectralPixels result{static_cast<unsigned>(amplitudes.size()), spectral_rows, {}};
    result.rgba.resize(result.width * spectral_rows * 4);
    region_start = spectral_detail::bound(region_start, 0); region_end = spectral_detail::bound(region_end, 1);
    kind = spectral_detail::profile(kind);
    std::uint32_t seed = 99;
    const auto random = [&] { seed = seed * 1664525u + 1013904223u; return double(seed) / 4294967296.; };
    for (unsigned x = 0; x < result.width; ++x) {
        const auto i = reversed ? result.width - 1 - x : x;
        const double t = double(i) / result.width, hit_random = random();
        const bool in_region = double(x) / result.width >= region_start && double(x) / result.width <= region_end;
        for (unsigned y = 0; y < spectral_rows; ++y) {
            const double f = 1 - double(y) / (spectral_rows - 1);
            const double e = amplitudes[i] * spectral_detail::energy(kind, f, t, hit_random) * (.75 + .5 * random());
            const double db = std::max(0., 1 + std::log10(std::max(1e-4, e)) / 2.2);
            const double ramp = std::clamp(in_region ? db : db * .45, 0., 1.) * 4;
            const auto a = static_cast<unsigned>(ramp), b = std::min(4u, a + 1);
            const auto at = (y * result.width + x) * 4;
            for (unsigned channel = 0; channel < 3; ++channel) {
                const auto shift = 16 - channel * 8;
                const double low = (palette[a] >> shift) & 255, high = (palette[b] >> shift) & 255;
                result.rgba[at + channel] = static_cast<std::uint8_t>(std::lround(low + (high - low) * (ramp - a)));
            }
            result.rgba[at + 3] = 255;
        }
    }
    return result;
}

// One UI-thread cache entry: <=4096 amplitude values and <=1 MiB of image pixels.
// Playhead changes are deliberately absent from this request.
class WaveDisplayCache {
    struct Key {
        std::vector<float> amplitudes; std::string profile; bool reversed;
        double start, end; SpectralPalette palette;
        bool operator==(const Key&) const = default;
    };
    std::optional<Key> key_;
    slint::Image image_;
public:
    slint::Image image(std::span<const float> amplitudes, std::string_view profile, bool reversed,
                       double start, double end, const SpectralPalette& palette) {
        Key key{spectral_detail::bounded(amplitudes), std::string(spectral_detail::profile(profile)), reversed,
                spectral_detail::bound(start, 0), spectral_detail::bound(end, 1), palette};
        if (key_ && *key_ == key) return image_;
        const auto pixels = spectral_pixels(key.amplitudes, key.profile, key.reversed, key.start, key.end, key.palette);
        if (!pixels.width) image_ = {};
        else {
            slint::SharedPixelBuffer<slint::Rgba8Pixel> buffer(pixels.width, pixels.height);
            for (std::size_t i = 0; i < pixels.rgba.size() / 4; ++i)
                buffer.begin()[i] = {pixels.rgba[i*4],pixels.rgba[i*4+1],pixels.rgba[i*4+2],pixels.rgba[i*4+3]};
            image_ = slint::Image(buffer);
        }
        key_ = std::move(key);
        return image_;
    }
    template<class Model>
    slint::Image image_for_model(const std::shared_ptr<Model>& model, bool reversed, float start, float end,
                                slint::Color ink0, slint::Color ink5, slint::Color accent_lo,
                                slint::Color accent, slint::Color paper1, std::string_view profile = "kick") {
        const auto size = model ? model->row_count() : 0;
        const auto columns = std::min<std::size_t>(size, spectral_max_columns);
        std::vector<float> amplitudes(columns);
        for (std::size_t x = 0; x < columns; ++x) {
            const auto bucket = model->row_data(spectral_detail::source_index(x, columns, size));
            if (bucket) amplitudes[x] = std::max(std::abs(bucket->minimum), std::abs(bucket->maximum));
        }
        const auto rgb = [](slint::Color c) { return std::uint32_t(c.red()) << 16 | std::uint32_t(c.green()) << 8 | c.blue(); };
        return image(amplitudes,profile,reversed,start,end,{rgb(ink0),rgb(ink5),rgb(accent_lo),rgb(accent),rgb(paper1)});
    }
};
} // namespace dandrum::advanced_sampler
