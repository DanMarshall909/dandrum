#pragma once
#include "Theme.h"
#include <string>

namespace dandrum::advanced_sampler {
struct AppearanceSelection {
    std::optional<ThemeSettings> settings;
    std::string_view error;
};

template<class Enum, std::size_t N>
inline std::optional<Enum> appearance_option(const std::array<std::string_view, N>& names, std::string_view value) {
    for (std::size_t i = 0; i < N; ++i) if (names[i] == value) return static_cast<Enum>(i);
    return std::nullopt;
}

// Validate the complete selection before changing any live palette or setting.
inline AppearanceSelection parse_appearance(std::string_view surface, std::string_view finish,
        std::string_view accent, std::string_view secondary, std::string_view modulation, std::string_view host) {
    const auto surface_value = appearance_option<Surface>(surface_names, surface);
    if (!surface_value) return {std::nullopt, "Unknown surface"};
    const auto finish_value = appearance_option<Finish>(finish_names, finish);
    if (!finish_value) return {std::nullopt, "Unknown finish"};
    const auto accent_value = parse_hex_rgb(accent);
    if (!accent_value) return {std::nullopt, "Invalid accent color"};
    const auto secondary_value = parse_hex_rgb(secondary);
    if (!secondary_value) return {std::nullopt, "Invalid secondary color"};
    const auto modulation_value = appearance_option<ModPalette>(mod_palette_names, modulation);
    if (!modulation_value) return {std::nullopt, "Unknown modulation palette"};
    const auto host_value = parse_hex_rgb(host);
    if (!host_value) return {std::nullopt, "Invalid host color"};
    return {ThemeSettings{*surface_value, *finish_value, *accent_value, *secondary_value, *modulation_value, *host_value}, {}};
}

inline std::string appearance_hex(std::uint32_t rgb) {
    constexpr char digits[] = "0123456789ABCDEF";
    std::string value = "#000000";
    for (unsigned i = 0; i < 6; ++i) value[6 - i] = digits[(rgb >> (i * 4)) & 15];
    return value;
}
} // namespace dandrum::advanced_sampler
