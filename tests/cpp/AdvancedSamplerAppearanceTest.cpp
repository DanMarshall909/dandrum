#include "../../ui/advanced-sampler/theme/AppearanceSettings.h"

#include <iostream>
#include <stdexcept>

namespace {
using namespace dandrum::advanced_sampler;
int checks = 0;
void require(bool condition, const char* message) {
    ++checks;
    if (!condition) throw std::runtime_error(message);
}
}

int main() {
    try {
        for (std::size_t surface = 0; surface < surface_names.size(); ++surface)
            for (std::size_t finish = 0; finish < finish_names.size(); ++finish)
                for (std::size_t modulation = 0; modulation < mod_palette_names.size(); ++modulation) {
                    const auto selection = parse_appearance(surface_names[surface], finish_names[finish],
                            "#aBc", "#cAFE80", mod_palette_names[modulation], "#F0c");
                    require(selection.settings.has_value() && selection.error.empty(), "Every guide option accepts custom colors");
                    const auto settings = *selection.settings;
                    require(static_cast<std::size_t>(settings.surface) == surface &&
                            static_cast<std::size_t>(settings.finish) == finish &&
                            static_cast<std::size_t>(settings.mod_palette) == modulation,
                            "Appearance names reach their exact native settings");
                    require(settings.accent == 0xaabbcc && settings.secondary == 0xcafe80 && settings.host_color == 0xff00cc,
                            "Three/six-digit colors are expanded without losing values");
                }
        for (const auto value : accent_presets) require(appearance_hex(value).front() == '#', "Every accent has a canonical UI value");
        require(appearance_hex(0xabcdef) == "#ABCDEF" && appearance_hex(0x00000f) == "#00000F",
                "Canonical values retain preset selection and zero padding");
        const std::array<std::array<std::string_view, 6>, 6> invalid{{
            {"unknown", "soft", "#abc", "#abc", "standard", "#abc"},
            {"aluminium", "unknown", "#abc", "#abc", "standard", "#abc"},
            {"aluminium", "soft", "invalid", "#abc", "standard", "#abc"},
            {"aluminium", "soft", "#abc", "invalid", "standard", "#abc"},
            {"aluminium", "soft", "#abc", "#abc", "unknown", "#abc"},
            {"aluminium", "soft", "#abc", "#abc", "standard", "invalid"},
        }};
        const std::array<std::string_view, 6> errors{{"Unknown surface", "Unknown finish", "Invalid accent color",
                "Invalid secondary color", "Unknown modulation palette", "Invalid host color"}};
        for (std::size_t i = 0; i < invalid.size(); ++i) {
            const auto& fields = invalid[i];
            const auto selection = parse_appearance(fields[0], fields[1], fields[2], fields[3], fields[4], fields[5]);
            require(!selection.settings && selection.error == errors[i], "Invalid settings identify the rejected field atomically");
        }
        std::cout << "PASS: " << checks << " advanced sampler appearance settings checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
