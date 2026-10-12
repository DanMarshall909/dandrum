#pragma once
#include "../../slint/host/Theme.h"

namespace dandrum::advanced_sampler {
using dandrum::slint_ui::Surface;
using dandrum::slint_ui::Finish;
using dandrum::slint_ui::ModPalette;
using dandrum::slint_ui::ThemeSettings;
using dandrum::slint_ui::Color;
using dandrum::slint_ui::Role;
using dandrum::slint_ui::role_count;
using dandrum::slint_ui::role_names;
using dandrum::slint_ui::reference_role_names;
using dandrum::slint_ui::surface_names;
using dandrum::slint_ui::finish_names;
using dandrum::slint_ui::mod_palette_names;
using dandrum::slint_ui::accent_presets;
using dandrum::slint_ui::secondary_presets;
using dandrum::slint_ui::host_presets;
using dandrum::slint_ui::ThemePalette;
using dandrum::slint_ui::contrast_rgb;
using dandrum::slint_ui::alpha_byte;
using dandrum::slint_ui::parse_hex_rgb;
using dandrum::slint_ui::resolve_theme;
namespace theme_detail = dandrum::slint_ui::theme_detail;
} // namespace dandrum::advanced_sampler
