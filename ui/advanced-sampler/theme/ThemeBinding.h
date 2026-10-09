#pragma once
#include "Theme.h"
#include "MaterialImage.h"
#include <slint.h>

namespace dandrum::advanced_sampler {
inline slint::Color native_color(Color color) {
    return slint::Color::from_argb_uint8(alpha_byte(color), (color.rgb >> 16) & 255,
                                       (color.rgb >> 8) & 255, color.rgb & 255);
}
// Accept the generated global by const reference, including a temporary returned
// by window->global<SamplerTheme>(); Slint setters are const shared-handle methods.
template<class Global> void apply_theme(const Global& target, const ThemePalette& palette) {
    const auto lighting = window_lighting_svg(palette);
    target.set_window_lighting(slint::Image::load_from_data(
            std::span<const std::uint8_t>(reinterpret_cast<const std::uint8_t*>(lighting.data()), lighting.size()), "svg"));
    target.set_soft(palette.soft);
    target.set_light(palette.light);
    target.set_brushed(palette.brushed);
    target.set_accent(native_color(palette[Role::Accent]));
    target.set_accent_hi(native_color(palette[Role::AccentHi]));
    target.set_accent_lo(native_color(palette[Role::AccentLo]));
    target.set_accent_wash(native_color(palette[Role::AccentWash]));
    target.set_secondary(native_color(palette[Role::Secondary]));
    target.set_secondary_wash(native_color(palette[Role::SecondaryWash]));
    target.set_host(native_color(palette[Role::Host]));
    target.set_host_wash(native_color(palette[Role::HostWash]));
    target.set_mod_a(native_color(palette[Role::ModA]));
    target.set_mod_b(native_color(palette[Role::ModB]));
    target.set_mod_c(native_color(palette[Role::ModC]));
    target.set_mod_d(native_color(palette[Role::ModD]));
    target.set_mod_wash(native_color(palette[Role::ModWash]));
    target.set_ok(native_color(palette[Role::Ok]));
    target.set_warn(native_color(palette[Role::Warn]));
    target.set_error(native_color(palette[Role::Error]));
    target.set_ok_wash(native_color(palette[Role::OkWash]));
    target.set_warn_wash(native_color(palette[Role::WarnWash]));
    target.set_error_wash(native_color(palette[Role::ErrorWash]));
    target.set_cap(native_color(palette[Role::Cap]));
    target.set_cap_hover(native_color(palette[Role::CapHover]));
    target.set_cap_press(native_color(palette[Role::CapPress]));
    target.set_pad(native_color(palette[Role::Pad]));
    target.set_pad_hover(native_color(palette[Role::PadHover]));
    target.set_text_on_accent(native_color(palette[Role::TextOnAccent]));
    target.set_scrim(native_color(palette[Role::Scrim]));
    target.set_waveform(native_color(palette[Role::Waveform]));
    target.set_waveform_region(native_color(palette[Role::WaveformRegion]));
    target.set_surface_window(native_color(palette[Role::SurfaceWindow]));
    target.set_surface_panel(native_color(palette[Role::SurfacePanel]));
    target.set_surface_panel_header(native_color(palette[Role::SurfacePanelHeader]));
    target.set_surface_well(native_color(palette[Role::SurfaceWell]));
    target.set_surface_control(native_color(palette[Role::SurfaceControl]));
    target.set_surface_control_hover(native_color(palette[Role::SurfaceControlHover]));
    target.set_surface_control_pressed(native_color(palette[Role::SurfaceControlPressed]));
    target.set_surface_menu(native_color(palette[Role::SurfaceMenu]));
    target.set_surface_prepared(native_color(palette[Role::SurfacePrepared]));
    target.set_text_primary(native_color(palette[Role::TextPrimary]));
    target.set_text_secondary(native_color(palette[Role::TextSecondary]));
    target.set_text_tertiary(native_color(palette[Role::TextTertiary]));
    target.set_text_disabled(native_color(palette[Role::TextDisabled]));
    target.set_border_hairline(native_color(palette[Role::BorderHairline]));
    target.set_border_control(native_color(palette[Role::BorderControl]));
    target.set_border_strong(native_color(palette[Role::BorderStrong]));
    target.set_color_selected(native_color(palette[Role::ColorSelected]));
    target.set_color_action(native_color(palette[Role::ColorAction]));
    target.set_color_focus(native_color(palette[Role::ColorFocus]));
    target.set_color_value(native_color(palette[Role::ColorValue]));
    target.set_color_track(native_color(palette[Role::ColorTrack]));
    target.set_color_cursor(native_color(palette[Role::ColorCursor]));
    target.set_ink_0(native_color(palette[Role::Ink0]));
    target.set_ink_1(native_color(palette[Role::Ink1]));
    target.set_ink_2(native_color(palette[Role::Ink2]));
    target.set_ink_3(native_color(palette[Role::Ink3]));
    target.set_ink_4(native_color(palette[Role::Ink4]));
    target.set_ink_5(native_color(palette[Role::Ink5]));
    target.set_ink_6(native_color(palette[Role::Ink6]));
    target.set_line_1(native_color(palette[Role::Line1]));
    target.set_line_2(native_color(palette[Role::Line2]));
    target.set_line_3(native_color(palette[Role::Line3]));
    target.set_paper_1(native_color(palette[Role::Paper1]));
    target.set_paper_2(native_color(palette[Role::Paper2]));
    target.set_paper_3(native_color(palette[Role::Paper3]));
    target.set_paper_4(native_color(palette[Role::Paper4]));
}
} // namespace dandrum::advanced_sampler
