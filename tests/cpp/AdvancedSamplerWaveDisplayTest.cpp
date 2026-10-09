#include "ui/advanced-sampler/render/WaveDisplay.h"
#include "tests/fixtures/advanced_sampler_spectral_golden.h"
#include <iostream>
#include <limits>
#include <stdexcept>

namespace {
int checks = 0;
void expect(bool result, const char* message) { ++checks; if (!result) throw std::runtime_error(message); }
std::uint32_t hash(const dandrum::advanced_sampler::SpectralPixels& pixels) {
    std::uint32_t result = 2166136261u;
    for (const auto value : pixels.rgba) result = (result ^ value) * 16777619u;
    return result;
}
std::uint32_t pixel(const dandrum::advanced_sampler::SpectralPixels& pixels, unsigned x, unsigned y) {
    const auto at = (y * pixels.width + x) * 4;
    return std::uint32_t(pixels.rgba[at]) << 24 | std::uint32_t(pixels.rgba[at+1]) << 16 |
           std::uint32_t(pixels.rgba[at+2]) << 8 | pixels.rgba[at+3];
}
}
int main() { try {
    using namespace dandrum::advanced_sampler;
    using dandrum::spectral_test::palette;
    const std::array<float, 4> amplitudes{1, .5f, 0, .25f};
    for (const auto& oracle : dandrum::spectral_test::cases) {
        const auto pixels = spectral_pixels(amplitudes, oracle.profile, oracle.reversed, oracle.start, oracle.end, palette);
        expect(pixels.width == 4 && pixels.height == 64, "The silent reference preview has one column per bounded amplitude and exactly 64 rows");
        expect(hash(pixels) == oracle.hash, "Every RGBA pixel matches the independent JavaScript RNG99/profile/region/reversal oracle");
        for (const auto& known : oracle.pixels)
            expect(pixel(pixels, known.x, known.y) == known.rgba, "Known signed source amplitudes produce the exact reference spectral RGB and opaque alpha");
    }
    expect(hash(spectral_pixels(amplitudes, "unknown", false, 0, 1, palette)) == dandrum::spectral_test::cases[0].hash, "Unknown mock profiles use the reference kick fallback");
    const std::array<float, 4> invalid{std::numeric_limits<float>::quiet_NaN(), -1, std::numeric_limits<float>::infinity(), 0};
    const auto silent = spectral_pixels(invalid, "kick", false, 0, 1, palette);
    for (unsigned x = 0; x < 4; ++x) expect(pixel(silent, x, 31) == 0x130f0cff, "Invalid or negative amplitudes safely produce the darkest reference pixel");
    const auto empty = spectral_pixels({}, "kick", false, 0, 1, palette);
    expect(empty.width == 0 && empty.height == 0 && empty.rgba.empty(), "Empty prepared waveform has no fabricated spectral pixels");
    const std::array<float,1> overloaded{2};
    const auto single = spectral_pixels(overloaded,"kick",false,0,1,palette);
    expect(single.width==1&&single.height==64&&pixel(single,0,63)==0xf2e6d3ff,"Single-column amplitudes clamp safely to the known opaque paper endpoint");
    std::vector<float> huge(8192, .5f); huge.front() = 1; huge.back() = 0;
    const auto bounded = spectral_pixels(huge, "kick", false, 0, 1, palette);
    expect(bounded.width == 4096 && bounded.height == 64 && bounded.rgba.size() == 4096 * 64 * 4, "Large previews remain bounded to one MiB of opaque RGBA pixels");
    expect(pixel(bounded, 4095, 31) == 0x130f0cff, "Bounded sampling retains the last source amplitude");
    expect(hash(spectral_pixels(amplitudes, "kick", false, -1, 2, palette)) == dandrum::spectral_test::cases[0].hash, "Finite region bounds clamp to the visible waveform");
    expect(hash(spectral_pixels(amplitudes, "kick", false, std::numeric_limits<double>::quiet_NaN(), std::numeric_limits<double>::infinity(), palette)) == dandrum::spectral_test::cases[0].hash, "Nonfinite region bounds use the complete visible region");
    std::cout << "PASS: " << checks << " exact spectral pixel checks\n";
} catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; } }
