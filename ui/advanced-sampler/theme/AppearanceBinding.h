#pragma once
#include "AppearanceSettings.h"
#include "ThemeBinding.h"

namespace dandrum::advanced_sampler {
template<class AppearanceGlobal>
inline void publish_appearance(const AppearanceGlobal& appearance, const ThemeSettings& settings) {
    appearance.set_surface(slint::SharedString(surface_names.at(static_cast<std::size_t>(settings.surface))));
    appearance.set_finish(slint::SharedString(finish_names.at(static_cast<std::size_t>(settings.finish))));
    appearance.set_accent(slint::SharedString(appearance_hex(settings.accent)));
    appearance.set_secondary(slint::SharedString(appearance_hex(settings.secondary)));
    appearance.set_mod_palette(slint::SharedString(mod_palette_names.at(static_cast<std::size_t>(settings.mod_palette))));
    appearance.set_host(slint::SharedString(appearance_hex(settings.host_color)));
}

template<class AppearanceGlobal, class ThemeGlobal>
inline bool apply_appearance(const AppearanceGlobal& appearance, const ThemeGlobal& theme,
        ThemeSettings& settings, std::string_view surface, std::string_view finish, std::string_view accent,
        std::string_view secondary, std::string_view modulation, std::string_view host) {
    const auto selection = parse_appearance(surface, finish, accent, secondary, modulation, host);
    if (!selection.settings) {
        publish_appearance(appearance, settings);
        appearance.set_error(slint::SharedString(selection.error));
        return false;
    }
    apply_theme(theme, resolve_theme(*selection.settings));
    settings = *selection.settings;
    publish_appearance(appearance, settings);
    appearance.set_error("");
    return true;
}
} // namespace dandrum::advanced_sampler
