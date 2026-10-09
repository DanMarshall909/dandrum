#include "../../ui/advanced-sampler/theme/Theme.h"
#include "../../ui/advanced-sampler/theme/MaterialImage.h"

#include <cmath>
#include <cstdio>
#include <fstream>
#include <iostream>
#include <regex>
#include <sstream>
#include <stdexcept>
#include <string>

namespace {
using namespace dandrum::advanced_sampler;
int checks = 0;
void require(bool condition, const std::string& reason) {
    ++checks;
    if (!condition) throw std::runtime_error(reason);
}
std::string field(const std::string& object, const std::string& key) {
    const std::regex expression("\\\"" + key + "\\\"\\s*:\\s*\\\"([^\\\"]*)\\\"");
    std::smatch match;
    if (!std::regex_search(object, match, expression)) throw std::runtime_error("Missing fixture field " + key);
    return match[1].str();
}
template<std::size_t N> std::size_t option(const std::array<std::string_view, N>& names, const std::string& name) {
    for (std::size_t i = 0; i < names.size(); ++i) if (names[i] == name) return i;
    throw std::runtime_error("Unknown fixture option " + name);
}
Color expected_color(const std::string& text) {
    if (text.starts_with('#')) return {static_cast<std::uint32_t>(std::stoul(text.substr(1), nullptr, 16)), 1.0};
    unsigned red, green, blue; double alpha;
    require(std::sscanf(text.c_str(), "rgba(%u,%u,%u,%lf)", &red, &green, &blue, &alpha) == 4,
            "Every golden role is an explicit hex or rgba color");
    return {(red << 16) | (green << 8) | blue, alpha};
}
void compare_golden(const std::string& fixture) {
    std::size_t cursor = 0, cases = 0;
    while ((cursor = fixture.find("\"props\":", cursor)) != std::string::npos) {
        const auto props_begin = fixture.find('{', cursor);
        const auto props_end = fixture.find('}', props_begin);
        const auto roles_begin = fixture.find('{', fixture.find("\"roles\":", props_end));
        const auto roles_end = fixture.find('}', roles_begin);
        const auto props = fixture.substr(props_begin, props_end - props_begin + 1);
        const auto roles = fixture.substr(roles_begin, roles_end - roles_begin + 1);
        ThemeSettings settings;
        settings.surface = static_cast<Surface>(option(surface_names, field(props, "surface")));
        settings.finish = static_cast<Finish>(option(finish_names, field(props, "finish")));
        settings.mod_palette = static_cast<ModPalette>(option(mod_palette_names, field(props, "modPalette")));
        settings.accent = *parse_hex_rgb(field(props, "accent"));
        settings.secondary = *parse_hex_rgb(field(props, "secondary"));
        settings.host_color = *parse_hex_rgb(field(props, "hostColor"));
        const auto palette = resolve_theme(settings);
        require(palette.soft == (settings.finish == Finish::Soft), "Finish must reach the material flag");
        require(palette.light == (settings.surface == Surface::Brushed || settings.surface == Surface::Paper), "Both brushed and paper need light-theme contrast");
        require(palette.brushed == (settings.surface == Surface::Brushed), "Brushed material is independent of soft/flat finish");
        for (std::size_t i = 0; i < role_names.size(); ++i) {
            const auto expected = expected_color(field(roles, std::string(reference_role_names[i])));
            require(palette.colors[i] == expected,
                    "Golden case " + std::to_string(cases) + ", " + std::string(role_names[i]) + " differs from the unmodified v3 reference");
        }
        ++cases; cursor = roles_end + 1;
    }
    require(cases == 115, "All 115 independent reference cases must be checked");
}
}
int main(int argc, char** argv) {
    try {
        const auto fixture_path = argc > 1 ? argv[1] : "tests/fixtures/advanced-sampler-theme-reference.json";
        std::ifstream input(fixture_path);
        require(input.good(), "The independent golden reference fixture must be readable");
        std::ostringstream content; content << input.rdbuf(); compare_golden(content.str());
        require(surface_names.size() == 8 && accent_presets.size() == 7 && secondary_presets.size() == 9 &&
                mod_palette_names.size() == 4 && host_presets.size() == 4 && finish_names.size() == 2,
                "Every v3 appearance option must remain exposed");
        require(parse_hex_rgb("#abc") == 0xaabbcc && parse_hex_rgb("#aBcDEF") == 0xabcdef,
                "Custom colors accept three/six-digit hex and either letter case");
        for (const auto invalid : {"", "abc", "#ab", "#abcd", "#abcdef0", "#12g456", "# aabbcc", "#aabbcc "})
            require(!parse_hex_rgb(invalid), "Invalid custom colors must not silently become black");
        ThemeSettings custom; custom.accent = 0x123456; custom.secondary = 0xcafe80; custom.host_color = 0xff00cc;
        const auto dark = resolve_theme(custom);
        require(dark[Role::Accent].rgb == custom.accent && dark[Role::Host].rgb == custom.host_color,
                "Custom dark role colors are retained exactly");
        require(dark[Role::Pad].rgb == 0x495838 && dark[Role::Cap].rgb == 0x627746,
                "Custom secondary colors must tint pad and cap faces using reference weights");
        for (const auto surface : {Surface::Brushed, Surface::Paper}) {
            custom.surface = surface; custom.accent = custom.secondary = custom.host_color = 0xffffff;
            const auto light = resolve_theme(custom);
            require(contrast_rgb(light[Role::Accent].rgb, light[Role::SurfacePanel].rgb) >= 3.2, "Custom light accent meets role contrast");
            require(contrast_rgb(light[Role::Host].rgb, light[Role::SurfacePanel].rgb) >= 3.2, "Custom light host meets role contrast");
            require(contrast_rgb(light[Role::Secondary].rgb, light[Role::SurfacePanelHeader].rgb) >= 4.5, "Custom secondary meets caption contrast");
            for (const auto role : {Role::Ok, Role::Warn, Role::Error})
                require(contrast_rgb(light[role].rgb, light[Role::SurfacePanel].rgb) >= 4.5, "Light status colors meet status contrast");
        }
        require(contrast_rgb(0x000000, 0xffffff) == 21.0 && contrast_rgb(0xffffff, 0x000000) == 21.0,
                "Contrast agrees with the known black/white reference");
        require(contrast_rgb(0x123456, 0x123456) == 1.0, "Identical colors have contrast one");
        require(alpha_byte(Color{0, 0.45}) == 115 && alpha_byte(Color{0, 0.22}) == 56, "Native alpha packing rounds the reference scrim opacity");
        const auto lighting = window_lighting_svg(resolve_theme(ThemeSettings{}));
        require(lighting.find("scale(1.3 1)") != std::string::npos && lighting.find("offset=\"55%\"") != std::string::npos,
                "Native window lighting preserves the v3 130% by 100% ellipse and middle stop");
        require(lighting.find("stop-color=\"#2b2c2e\" stop-opacity=\"0.83\"") != std::string::npos,
                "The top light preserves normalized 80:3 color-mix weights and 83% alpha");
        require(lighting.find("stop-color=\"#161718\"") != std::string::npos && lighting.find("stop-color=\"#0f1011\"") != std::string::npos,
                "Native window lighting follows the live middle and well roles");
        ThemeSettings paper_settings; paper_settings.surface = Surface::Paper;
        require(window_lighting_svg(resolve_theme(paper_settings)) != lighting,
                "Window radial lighting must change with every live surface palette");
        for (int invalid = 0; invalid < 3; ++invalid) {
            ThemeSettings settings;
            if (invalid == 0) settings.surface = static_cast<Surface>(255);
            if (invalid == 1) settings.finish = static_cast<Finish>(255);
            if (invalid == 2) settings.mod_palette = static_cast<ModPalette>(255);
            bool rejected = false;
            try { (void)resolve_theme(settings); } catch (const std::out_of_range&) { rejected = true; }
            require(rejected, "Invalid appearance indices are rejected before changing the UI");
        }
        std::cout << "PASS: " << checks << " advanced sampler theme checks across all 115 reference cases\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
