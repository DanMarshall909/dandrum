#pragma once

#include <algorithm>
#include <cmath>
#include <locale>
#include <numbers>
#include <sstream>
#include <string>
#include <string_view>

namespace dandrum::slint_ui {
namespace velocity_blend {
inline constexpr float minimum_velocity = 1;
inline constexpr float maximum_velocity = 127;
inline constexpr float minimum_fade_width = 1;
inline constexpr int sample_count = 101;
inline constexpr int segment_count = sample_count - 1;
inline constexpr int coordinate_extent = 100;
inline constexpr std::string_view no_crossfade = "none";
inline constexpr std::string_view linear = "linear";
} // namespace velocity_blend

// UI-thread preview geometry, independent of engine state or audio processing.
inline double velocity_blend_gain(double t, float low, float high, float width, std::string_view curve) {
    using namespace velocity_blend;
    if (curve == no_crossfade) return t*maximum_velocity >= low-minimum_velocity && t*maximum_velocity <= high ? 1. : 0.;
    const double f = std::clamp((low <= minimum_velocity ? high+minimum_velocity-t*maximum_velocity : t*maximum_velocity-low+minimum_velocity)/std::max(minimum_fade_width,width),0.,1.);
    return curve == linear ? f : std::sin(f*std::numbers::pi/2);
}

inline std::string velocity_blend_path(float low, float high, float width, std::string_view curve, bool closed) {
    using namespace velocity_blend;
    const auto gain = [&](int i) { return velocity_blend_gain(double(i)/segment_count,low,high,width,curve); };
    std::ostringstream path;
    path.imbue(std::locale::classic());
    if (closed) path << "M 0 " << coordinate_extent << " L 0 " << (1-gain(0))*coordinate_extent;
    else path << "M 0 " << (1-gain(0))*coordinate_extent;
    for (int i=1; i<sample_count; ++i) path << " L " << i << ' ' << (1-gain(i))*coordinate_extent;
    if (closed) path << " L " << coordinate_extent << ' ' << coordinate_extent << " Z";
    return path.str();
}
} // namespace dandrum::slint_ui
