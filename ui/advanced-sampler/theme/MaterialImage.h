#pragma once
#include "Theme.h"
#include <string>

namespace dandrum::advanced_sampler {
inline std::string material_hex(std::uint32_t rgb) {
    constexpr char digits[] = "0123456789abcdef";
    std::string result = "#000000";
    for (unsigned i = 0; i < 6; ++i) result[6 - i] = digits[(rgb >> (i * 4)) & 15];
    return result;
}
// CSS color-mix normalizes 80% ink + 3% white and retains 83% opacity.
// SVG keeps the exact elliptical radial gradient on every native renderer.
inline std::string window_lighting_svg(const ThemePalette& palette) {
    const auto top = theme_detail::mix(palette[Role::Ink3].rgb, 0xffffff, 3.0 / 83.0);
    return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1600\" height=\"1000\" viewBox=\"0 0 1600 1000\">"
           "<defs><radialGradient id=\"light\" cx=\".5\" cy=\"0\" r=\"1\" gradientTransform=\"translate(.5 0) scale(1.3 1) translate(-.5 0)\">"
           "<stop offset=\"0%\" stop-color=\"" + material_hex(top) + "\" stop-opacity=\"0.83\"/>"
           "<stop offset=\"55%\" stop-color=\"" + material_hex(palette[Role::Ink1].rgb) + "\"/>"
           "<stop offset=\"100%\" stop-color=\"" + material_hex(palette[Role::Ink0].rgb) + "\"/>"
           "</radialGradient></defs><rect width=\"1600\" height=\"1000\" fill=\"url(#light)\"/></svg>";
}
} // namespace dandrum::advanced_sampler
