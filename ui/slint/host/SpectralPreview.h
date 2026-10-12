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

namespace dandrum::slint_ui {
// Deterministic silent preview from the imported guide, not an FFT or audio analysis.
inline constexpr unsigned spectral_rows = 64;
inline constexpr unsigned spectral_max_columns = 4096;
inline constexpr unsigned spectral_rgba_channels = 4;
inline constexpr unsigned spectral_rgb_channels = 3;
inline constexpr unsigned spectral_palette_stops = 5;
inline constexpr unsigned spectral_channel_bits = 8;
inline constexpr unsigned spectral_byte_max = 255;
inline constexpr std::uint32_t spectral_rng_seed = 99;
inline constexpr std::uint32_t spectral_rng_multiplier = 1664525u;
inline constexpr std::uint32_t spectral_rng_increment = 1013904223u;
inline constexpr double spectral_rng_denominator = 4294967296.;
inline constexpr double spectral_noise_base = .75;
inline constexpr double spectral_noise_span = .5;
inline constexpr double spectral_energy_floor = 1e-4;
inline constexpr double spectral_log_range = 2.2;
inline constexpr double spectral_outside_region_gain = .45;
namespace spectral_profiles {
inline constexpr std::string_view kick = "kick", snare = "snare", soft_snare = "snare-soft";
inline constexpr std::string_view closed_hat = "hat-closed", open_hat = "hat-open", drum_break = "break";
// Coefficients describe the deterministic design-guide picture, not measured DSP.
inline constexpr double snare_center = .12, snare_width = 9, snare_bell = .55;
inline constexpr double snare_noise = .5, snare_frequency_decay = 1.4, snare_time_decay = 4;
inline constexpr double soft_snare_bell = .5, soft_snare_noise = .35;
inline constexpr double soft_snare_frequency_decay = 2, soft_snare_time_decay = 5;
inline constexpr double closed_hat_power = 1.4, closed_hat_level = .9, closed_hat_floor = .05;
inline constexpr double open_hat_power = 1.2, open_hat_level = .85, open_hat_floor = .06;
inline constexpr double break_choice = .5, break_decay = 8, break_mixture = .6, break_power = 1.3;
inline constexpr double kick_decay = 9, kick_attack = .6, kick_attack_decay = 40;
inline constexpr double kick_transient = .25, kick_transient_decay = 60, kick_transient_frequency_decay = 2;
}
struct SpectralPixels { unsigned width{}, height{}; std::vector<std::uint8_t> rgba; };
using SpectralPalette = std::array<std::uint32_t, spectral_palette_stops>;
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
    return value == spectral_profiles::snare || value == spectral_profiles::soft_snare || value == spectral_profiles::closed_hat || value == spectral_profiles::open_hat || value == spectral_profiles::drum_break ? value : spectral_profiles::kick;
}
inline double energy(std::string_view kind, double f, double t, double hit_random) {
    using namespace spectral_profiles;
    if (kind == snare) return snare_bell * std::exp(-std::pow((f - snare_center) * snare_width, 2)) + snare_noise * std::exp(-f * snare_frequency_decay) * std::exp(-t * snare_time_decay);
    if (kind == soft_snare) return soft_snare_bell * std::exp(-std::pow((f - snare_center) * snare_width, 2)) + soft_snare_noise * std::exp(-f * soft_snare_frequency_decay) * std::exp(-t * soft_snare_time_decay);
    if (kind == closed_hat) return std::pow(f, closed_hat_power) * closed_hat_level + closed_hat_floor;
    if (kind == open_hat) return std::pow(f, open_hat_power) * open_hat_level + open_hat_floor;
    if (kind == drum_break) return hit_random < break_choice ? std::exp(-f * break_decay) : break_mixture * std::exp(-f * break_decay) + break_mixture * std::pow(f, break_power);
    return std::exp(-f * kick_decay) * (1 + kick_attack * std::exp(-t * kick_attack_decay)) + kick_transient * std::exp(-t * kick_transient_decay) * std::exp(-f * kick_transient_frequency_decay);
}
}
inline SpectralPixels spectral_pixels(std::span<const float> input, std::string_view kind, bool reversed,
                                     double region_start, double region_end, const SpectralPalette& palette) {
    const auto amplitudes = spectral_detail::bounded(input);
    if (amplitudes.empty()) return {};
    SpectralPixels result{static_cast<unsigned>(amplitudes.size()), spectral_rows, {}};
    result.rgba.resize(result.width * spectral_rows * spectral_rgba_channels);
    region_start = spectral_detail::bound(region_start, 0); region_end = spectral_detail::bound(region_end, 1);
    kind = spectral_detail::profile(kind);
    std::uint32_t seed = spectral_rng_seed;
    const auto random = [&] { seed = seed * spectral_rng_multiplier + spectral_rng_increment; return double(seed) / spectral_rng_denominator; };
    for (unsigned x = 0; x < result.width; ++x) {
        const auto i = reversed ? result.width - 1 - x : x;
        const double t = double(i) / result.width, hit_random = random();
        const bool in_region = double(x) / result.width >= region_start && double(x) / result.width <= region_end;
        for (unsigned y = 0; y < spectral_rows; ++y) {
            const double f = 1 - double(y) / (spectral_rows - 1);
            const double e = amplitudes[i] * spectral_detail::energy(kind, f, t, hit_random) * (spectral_noise_base + spectral_noise_span * random());
            const double db = std::max(0., 1 + std::log10(std::max(spectral_energy_floor, e)) / spectral_log_range);
            const double ramp = std::clamp(in_region ? db : db * spectral_outside_region_gain, 0., 1.) * (spectral_palette_stops - 1);
            const auto a = static_cast<unsigned>(ramp), b = std::min(spectral_palette_stops - 1, a + 1);
            const auto at = (y * result.width + x) * spectral_rgba_channels;
            for (unsigned channel = 0; channel < spectral_rgb_channels; ++channel) {
                const auto shift = (spectral_rgb_channels - 1 - channel) * spectral_channel_bits;
                const double low = (palette[a] >> shift) & spectral_byte_max, high = (palette[b] >> shift) & spectral_byte_max;
                result.rgba[at + channel] = static_cast<std::uint8_t>(std::lround(low + (high - low) * (ramp - a)));
            }
            result.rgba[at + spectral_rgb_channels] = spectral_byte_max;
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
            for (std::size_t i = 0; i < pixels.rgba.size() / spectral_rgba_channels; ++i)
                buffer.begin()[i] = {pixels.rgba[i*spectral_rgba_channels],pixels.rgba[i*spectral_rgba_channels+1],pixels.rgba[i*spectral_rgba_channels+2],pixels.rgba[i*spectral_rgba_channels+spectral_rgb_channels]};
            image_ = slint::Image(buffer);
        }
        key_ = std::move(key);
        return image_;
    }
    template<class Model>
    slint::Image image_for_model(const std::shared_ptr<Model>& model, bool reversed, float start, float end,
                                slint::Color ink0, slint::Color ink5, slint::Color accent_lo,
                                slint::Color accent, slint::Color paper1, std::string_view profile = spectral_profiles::kick) {
        const auto size = model ? model->row_count() : 0;
        const auto columns = std::min<std::size_t>(size, spectral_max_columns);
        std::vector<float> amplitudes(columns);
        for (std::size_t x = 0; x < columns; ++x) {
            const auto bucket = model->row_data(spectral_detail::source_index(x, columns, size));
            if (bucket) amplitudes[x] = std::max(std::abs(bucket->minimum), std::abs(bucket->maximum));
        }
        const auto rgb = [](slint::Color c) { return std::uint32_t(c.red()) << ((spectral_rgb_channels - 1) * spectral_channel_bits) | std::uint32_t(c.green()) << spectral_channel_bits | c.blue(); };
        return image(amplitudes,profile,reversed,start,end,{rgb(ink0),rgb(ink5),rgb(accent_lo),rgb(accent),rgb(paper1)});
    }
};
} // namespace dandrum::slint_ui
