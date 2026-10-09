#pragma once

// A direct native port of the preserved v3 theme() role and contrast rules.
// This module is independent of Slint and the audio engine.
#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string_view>

namespace dandrum::advanced_sampler {
enum class Surface { Aluminium, Brushed, Ember, Graphite, Midnight, Forest, Camo, Paper };
enum class Finish { Soft, Flat };
enum class ModPalette { Standard, Warm, Cool, Mono };
struct ThemeSettings {
    Surface surface = Surface::Aluminium;
    Finish finish = Finish::Soft;
    std::uint32_t accent = 0xe8c47a;
    std::uint32_t secondary = 0x8c8f95;
    ModPalette mod_palette = ModPalette::Standard;
    std::uint32_t host_color = 0x5b9dff;
    bool operator==(const ThemeSettings&) const = default;
};
struct Color {
    std::uint32_t rgb = 0;
    double alpha = 1.0;
    bool operator==(const Color&) const = default;
};
enum class Role {
    Accent,
    AccentHi,
    AccentLo,
    AccentWash,
    Secondary,
    SecondaryWash,
    Host,
    HostWash,
    ModA,
    ModB,
    ModC,
    ModD,
    ModWash,
    Ok,
    Warn,
    Error,
    OkWash,
    WarnWash,
    ErrorWash,
    Cap,
    CapHover,
    CapPress,
    Pad,
    PadHover,
    TextOnAccent,
    Scrim,
    Waveform,
    WaveformRegion,
    SurfaceWindow,
    SurfacePanel,
    SurfacePanelHeader,
    SurfaceWell,
    SurfaceControl,
    SurfaceControlHover,
    SurfaceControlPressed,
    SurfaceMenu,
    SurfacePrepared,
    TextPrimary,
    TextSecondary,
    TextTertiary,
    TextDisabled,
    BorderHairline,
    BorderControl,
    BorderStrong,
    ColorSelected,
    ColorAction,
    ColorFocus,
    ColorValue,
    ColorTrack,
    ColorCursor,
    Ink0,
    Ink1,
    Ink2,
    Ink3,
    Ink4,
    Ink5,
    Ink6,
    Line1,
    Line2,
    Line3,
    Paper1,
    Paper2,
    Paper3,
    Paper4,
    Count
};
inline constexpr auto role_count = static_cast<std::size_t>(Role::Count);
inline constexpr std::array<std::string_view, role_count> role_names = {
    "accent",
    "accent-hi",
    "accent-lo",
    "accent-wash",
    "secondary",
    "secondary-wash",
    "host",
    "host-wash",
    "mod-a",
    "mod-b",
    "mod-c",
    "mod-d",
    "mod-wash",
    "ok",
    "warn",
    "error",
    "ok-wash",
    "warn-wash",
    "error-wash",
    "cap",
    "cap-hover",
    "cap-press",
    "pad",
    "pad-hover",
    "text-on-accent",
    "scrim",
    "waveform",
    "waveform-region",
    "surface-window",
    "surface-panel",
    "surface-panel-header",
    "surface-well",
    "surface-control",
    "surface-control-hover",
    "surface-control-pressed",
    "surface-menu",
    "surface-prepared",
    "text-primary",
    "text-secondary",
    "text-tertiary",
    "text-disabled",
    "border-hairline",
    "border-control",
    "border-strong",
    "color-selected",
    "color-action",
    "color-focus",
    "color-value",
    "color-track",
    "color-cursor",
    "ink-0",
    "ink-1",
    "ink-2",
    "ink-3",
    "ink-4",
    "ink-5",
    "ink-6",
    "line-1",
    "line-2",
    "line-3",
    "paper-1",
    "paper-2",
    "paper-3",
    "paper-4",
};
inline constexpr std::array<std::string_view, role_count> reference_role_names = {
    "--dd-vermilion",
    "--dd-vermilion-hi",
    "--dd-vermilion-lo",
    "--dd-vermilion-wash",
    "--dd-secondary",
    "--dd-secondary-wash",
    "--dd-host",
    "--dd-host-wash",
    "--dd-mod-a",
    "--dd-mod-b",
    "--dd-mod-c",
    "--dd-mod-d",
    "--dd-mod-wash",
    "--dd-ok",
    "--dd-warn",
    "--dd-error",
    "--dd-ok-wash",
    "--dd-warn-wash",
    "--dd-error-wash",
    "--dd-cap",
    "--dd-cap-hover",
    "--dd-cap-press",
    "--dd-pad",
    "--dd-pad-hover",
    "--text-on-accent",
    "--dd-scrim",
    "--color-waveform",
    "--color-waveform-region",
    "--surface-window",
    "--surface-panel",
    "--surface-panel-header",
    "--surface-well",
    "--surface-control",
    "--surface-control-hover",
    "--surface-control-pressed",
    "--surface-menu",
    "--surface-prepared",
    "--text-primary",
    "--text-secondary",
    "--text-tertiary",
    "--text-disabled",
    "--border-hairline",
    "--border-control",
    "--border-strong",
    "--color-selected",
    "--color-action",
    "--color-focus",
    "--color-value",
    "--color-track",
    "--color-cursor",
    "--dd-ink-0",
    "--dd-ink-1",
    "--dd-ink-2",
    "--dd-ink-3",
    "--dd-ink-4",
    "--dd-ink-5",
    "--dd-ink-6",
    "--dd-line-1",
    "--dd-line-2",
    "--dd-line-3",
    "--dd-paper-1",
    "--dd-paper-2",
    "--dd-paper-3",
    "--dd-paper-4",
};
inline constexpr std::array<std::string_view, 8> surface_names = {"aluminium", "brushed", "ember", "graphite", "midnight", "forest", "camo", "paper"};
inline constexpr std::array<std::string_view, 2> finish_names = {"soft", "flat"};
inline constexpr std::array<std::string_view, 4> mod_palette_names = {"standard", "warm", "cool", "mono"};
inline constexpr std::array<std::uint32_t, 7> accent_presets = {0xe8c47a, 0xe08a4e, 0xe0574e, 0xd8b24a, 0x5bb38a, 0x5b9dff, 0xc77dff};
inline constexpr std::array<std::uint32_t, 9> secondary_presets = {0x8c8f95, 0xa07a58, 0x7a5c45, 0xb5653f, 0x8e8a5a, 0x6f7f8c, 0x8a6a86, 0x5f8072, 0x9a8f7e};
inline constexpr std::array<std::uint32_t, 4> host_presets = {0x5b9dff, 0x4fd1c5, 0xb58cff, 0xffffff};
struct ThemePalette {
    std::array<Color, role_count> colors{};
    bool soft = true;
    bool light = false;
    bool brushed = false;
    const Color& operator[](Role role) const { return colors.at(static_cast<std::size_t>(role)); }
};
namespace theme_detail {
struct Ramp { std::array<std::uint32_t, 7> ink; std::array<std::uint32_t, 3> line; std::array<std::uint32_t, 4> paper; };
inline constexpr std::array<Ramp, 8> surfaces = {{
    {{0x0f1011, 0x161718, 0x1c1d1f, 0x232426, 0x2b2c2f, 0x36373a, 0x45464a}, {0x2a2b2e, 0x3a3b3f, 0x535459}, {0xede9e1, 0xc3beb4, 0x99948b, 0x67635d}},
    {{0x9fa3a8, 0xb9bcc0, 0xcbced1, 0xbfc2c6, 0xb1b4b8, 0xa2a5aa, 0x8e9196}, {0xa0a3a7, 0x83868b, 0x62656a}, {0x121314, 0x2b2d30, 0x45484c, 0x6a6d72}},
    {{0x130f0c, 0x1a1511, 0x211b16, 0x2a231d, 0x342b23, 0x41362c, 0x524437}, {0x2f271f, 0x42372c, 0x5f4f40}, {0xf2e6d3, 0xcbb9a0, 0xa8957d, 0x706252}},
    {{0x0e0f11, 0x141619, 0x1b1d21, 0x23262b, 0x2c3036, 0x383d44, 0x474d56}, {0x272a2f, 0x383c43, 0x505660}, {0xeceef1, 0xbfc4cc, 0x9097a2, 0x626874}},
    {{0x090c14, 0x0f1320, 0x151a2a, 0x1c2236, 0x252c44, 0x313a57, 0x414c6e}, {0x1f2639, 0x2f3850, 0x475170}, {0xe8ecf8, 0xb8c0da, 0x8b94b3, 0x5c6483}},
    {{0x0d100e, 0x131714, 0x1a1e1b, 0x212622, 0x2a302b, 0x353c36, 0x444c45}, {0x242a25, 0x343b35, 0x4d564f}, {0xe9ede6, 0xbec6bb, 0x959f92, 0x677064}},
    {{0x11120c, 0x181a11, 0x1f2216, 0x272a1c, 0x313523, 0x3d422b, 0x4c5236}, {0x2a2d1f, 0x3b3f2b, 0x565b40}, {0xece9d8, 0xc4c0a4, 0x9c987c, 0x6e6b54}},
    {{0xd9d2c5, 0xe7e1d5, 0xf1ede4, 0xe4ded1, 0xd6cfc0, 0xc8c0ae, 0xb6ad98}, {0xc9c0ae, 0xa39985, 0x7a705e}, {0x14110d, 0x2f2a22, 0x4a443a, 0x6e6758}}
}};
inline constexpr std::array<std::array<std::uint32_t, 4>, 4> modulations = {{
    {0x3fd0c9, 0xa98bff, 0x8edb5a, 0xff85be},
    {0xf2c14e, 0xf07b5a, 0xe6557a, 0xc7e26b},
    {0x5bc8f5, 0x7b8cff, 0x4fd6a5, 0xb58cff},
    {0xffffff, 0xcfcfcf, 0xa0a0a0, 0x7c7c7c}
}};
inline std::uint32_t mix(std::uint32_t a, std::uint32_t b, double weight) {
    std::uint32_t result = 0;
    for (const int shift : {16, 8, 0}) {
        const auto low = static_cast<double>((a >> shift) & 255);
        const auto high = static_cast<double>((b >> shift) & 255);
        const auto channel = std::clamp(std::lround(low + (high - low) * weight), 0L, 255L);
        result |= static_cast<std::uint32_t>(channel) << shift;
    }
    return result;
}
inline double luminance(std::uint32_t rgb) {
    const auto linear = [](std::uint32_t channel) {
        const double value = channel / 255.0;
        return value <= 0.03928 ? value / 12.92 : std::pow((value + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * linear((rgb >> 16) & 255) + 0.7152 * linear((rgb >> 8) & 255) + 0.0722 * linear(rgb & 255);
}
} // namespace theme_detail
inline double contrast_rgb(std::uint32_t a, std::uint32_t b) {
    const auto x = theme_detail::luminance(a), y = theme_detail::luminance(b);
    return (std::max(x, y) + 0.05) / (std::min(x, y) + 0.05);
}
inline std::uint8_t alpha_byte(Color color) {
    return static_cast<std::uint8_t>(std::lround(std::clamp(color.alpha, 0.0, 1.0) * 255.0));
}
inline std::optional<std::uint32_t> parse_hex_rgb(std::string_view text) {
    if ((text.size() != 4 && text.size() != 7) || text.front() != '#') return std::nullopt;
    std::uint32_t value = 0;
    for (const char c : text.substr(1)) {
        int digit = -1;
        if (c >= '0' && c <= '9') digit = c - '0';
        else if (c >= 'a' && c <= 'f') digit = c - 'a' + 10;
        else if (c >= 'A' && c <= 'F') digit = c - 'A' + 10;
        if (digit < 0) return std::nullopt;
        value = (value << 4) | static_cast<std::uint32_t>(digit);
    }
    if (text.size() == 4) {
        value = ((value & 0xf00) << 12) | ((value & 0xf00) << 8) |
                ((value & 0x0f0) << 8) | ((value & 0x0f0) << 4) |
                ((value & 0x00f) << 4) | (value & 0x00f);
    }
    return value;
}
inline ThemePalette resolve_theme(const ThemeSettings& settings) {
    using namespace theme_detail;
    const auto& ramp = surfaces.at(static_cast<std::size_t>(settings.surface));
    (void)finish_names.at(static_cast<std::size_t>(settings.finish));
    auto mods = modulations.at(static_cast<std::size_t>(settings.mod_palette));
    ThemePalette theme;
    theme.soft = settings.finish == Finish::Soft;
    theme.light = settings.surface == Surface::Brushed || settings.surface == Surface::Paper;
    theme.brushed = settings.surface == Surface::Brushed;
    const auto base = ramp.ink[2];
    const auto role_ink = [&](std::uint32_t input, double target) {
        auto color = input & 0xffffff;
        if (!theme.light) return color;
        for (int step = 1; contrast_rgb(color, base) < target && step <= 20; ++step)
            color = mix(input, 0x000000, step * 0.05);
        return color;
    };
    if (theme.light && settings.mod_palette == ModPalette::Mono) mods = {0x14110d, 0x3a342b, 0x5a5347, 0x7a7262};
    else for (auto& color : mods) color = role_ink(color, 3.2);
    const auto accent = role_ink(settings.accent, 3.2);
    const auto host = role_ink(settings.host_color, 3.2);
    auto secondary_text = settings.secondary & 0xffffff;
    for (int step = 1; contrast_rgb(secondary_text, ramp.ink[3]) < 4.5 && step <= 20; ++step)
        secondary_text = mix(settings.secondary, theme.light ? 0x000000 : 0xffffff, step * 0.05);
    const auto on_accent = contrast_rgb(0xffffff, accent) >= contrast_rgb(0x1e1209, accent) ? 0xffffff : 0x1e1209;
    const auto ok = theme.light ? role_ink(0x5fd38a, 4.5) : 0x5fd38a;
    const auto warn = theme.light ? role_ink(0xe9c15a, 4.5) : 0xe9c15a;
    const auto error = theme.light ? role_ink(0xf0545e, 4.5) : 0xf0545e;
    const auto set = [&](Role role, std::uint32_t rgb, double alpha = 1.0) {
        theme.colors[static_cast<std::size_t>(role)] = {rgb, alpha};
    };
    set(Role::Accent, accent); set(Role::AccentHi, mix(accent, theme.light ? 0 : 0xffffff, 0.22));
    set(Role::AccentLo, mix(accent, 0, 0.25)); set(Role::AccentWash, mix(base, accent, theme.light ? 0.2 : 0.16));
    set(Role::Secondary, secondary_text); set(Role::SecondaryWash, mix(base, settings.secondary, theme.light ? 0.18 : 0.14));
    set(Role::Host, host); set(Role::HostWash, mix(base, host, theme.light ? 0.16 : 0.14));
    set(Role::ModA, mods[0]); set(Role::ModB, mods[1]); set(Role::ModC, mods[2]); set(Role::ModD, mods[3]);
    set(Role::ModWash, mix(base, mods[0], theme.light ? 0.16 : 0.12));
    set(Role::Ok, ok); set(Role::Warn, warn); set(Role::Error, error);
    set(Role::OkWash, mix(base, ok, theme.light ? 0.14 : 0.1));
    set(Role::WarnWash, mix(base, warn, theme.light ? 0.16 : 0.1));
    set(Role::ErrorWash, mix(base, error, theme.light ? 0.14 : 0.1));
    set(Role::Cap, mix(base, settings.secondary, theme.light ? 0.42 : 0.4));
    set(Role::CapHover, mix(base, settings.secondary, theme.light ? 0.32 : 0.5));
    set(Role::CapPress, mix(base, settings.secondary, theme.light ? 0.55 : 0.28));
    set(Role::Pad, mix(base, settings.secondary, 0.26)); set(Role::PadHover, mix(base, settings.secondary, theme.light ? 0.18 : 0.34));
    set(Role::TextOnAccent, theme.light ? on_accent : 0x1e1209);
    set(Role::Scrim, theme.light ? 0x3c3226 : 0, theme.light ? 0.22 : 0.45);
    set(Role::Waveform, theme.light ? 0x8a806d : mix(ramp.paper[2], ramp.ink[2], 0.1));
    set(Role::WaveformRegion, theme.light ? 0x2f2a22 : mix(ramp.paper[0], ramp.paper[1], 0.3));
    set(Role::SurfaceWindow, ramp.ink[1]); set(Role::SurfacePanel, ramp.ink[2]); set(Role::SurfacePanelHeader, ramp.ink[3]);
    set(Role::SurfaceWell, ramp.ink[0]); set(Role::SurfaceControl, ramp.ink[4]); set(Role::SurfaceControlHover, ramp.ink[5]);
    set(Role::SurfaceControlPressed, ramp.ink[6]); set(Role::SurfaceMenu, ramp.ink[3]); set(Role::SurfacePrepared, ramp.ink[2]);
    set(Role::TextPrimary, ramp.paper[0]); set(Role::TextSecondary, ramp.paper[1]); set(Role::TextTertiary, ramp.paper[2]); set(Role::TextDisabled, ramp.paper[3]);
    set(Role::BorderHairline, ramp.line[0]); set(Role::BorderControl, ramp.line[1]); set(Role::BorderStrong, ramp.line[2]);
    set(Role::ColorSelected, accent); set(Role::ColorAction, accent); set(Role::ColorFocus, ramp.paper[0]);
    set(Role::ColorValue, theme.soft && !theme.light ? mix(accent, 0xffffff, 0.12) : ramp.paper[0]);
    set(Role::ColorTrack, ramp.ink[6]); set(Role::ColorCursor, ramp.paper[0]);
    for (std::size_t i = 0; i < ramp.ink.size(); ++i) set(static_cast<Role>(static_cast<std::size_t>(Role::Ink0) + i), ramp.ink[i]);
    for (std::size_t i = 0; i < ramp.line.size(); ++i) set(static_cast<Role>(static_cast<std::size_t>(Role::Line1) + i), ramp.line[i]);
    for (std::size_t i = 0; i < ramp.paper.size(); ++i) set(static_cast<Role>(static_cast<std::size_t>(Role::Paper1) + i), ramp.paper[i]);
    return theme;
}
} // namespace dandrum::advanced_sampler
