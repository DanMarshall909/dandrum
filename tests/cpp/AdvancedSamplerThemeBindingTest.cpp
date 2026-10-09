#include "AdvancedSamplerTheme.h"
#include "slint-testing.h"
#include "slint-platform.h"
#include "../../ui/advanced-sampler/theme/ThemeBinding.h"
#include "../../ui/advanced-sampler/theme/AppearanceBinding.h"

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
        slint::testing::init();
        const auto window = dandrum_ui::AdvancedSamplerThemeTest::create();
        const auto& global = window->global<dandrum_ui::SamplerTheme>();
        ThemeSettings settings;
        const auto verify = [&] {
            const auto palette = resolve_theme(settings);
            apply_theme(global, palette);
            require(global.get_window_lighting().size().width == 1600 && global.get_window_lighting().size().height == 1000,
                    "Native theme updates must decode the dynamic window lighting image");
            require(window->get_soft() == palette.soft && window->get_light() == palette.light && window->get_brushed() == palette.brushed,
                    "Native material flags must update the real generated global");
            const auto colors = window->get_colors();
            require(colors->row_count() == role_count, "Every semantic role must reach the generated interface");
            for (std::size_t i = 0; i < role_count; ++i) {
                const auto color = colors->row_data(i).value();
                const auto expected = palette.colors[i];
                require(color.red() == ((expected.rgb >> 16) & 255) && color.green() == ((expected.rgb >> 8) & 255) &&
                        color.blue() == (expected.rgb & 255) && color.alpha() == alpha_byte(expected),
                        "A native global setter omitted or mis-bound a semantic color");
            }
        };
        for (int surface = 0; surface < 8; ++surface) for (int finish = 0; finish < 2; ++finish) for (int palette = 0; palette < 4; ++palette) {
            settings.surface = static_cast<Surface>(surface); settings.finish = static_cast<Finish>(finish);
            settings.mod_palette = static_cast<ModPalette>(palette); verify();
        }
        for (const auto accent : accent_presets) { settings.accent = accent; verify(); }
        for (const auto secondary : secondary_presets) { settings.secondary = secondary; verify(); }
        for (const auto host : host_presets) { settings.host_color = host; verify(); }
        settings.accent = 0x123456; settings.secondary = 0xcafe80; settings.host_color = 0xff00cc; verify();
        const auto& appearance = window->global<dandrum_ui::AppearanceState>();
        require(appearance.get_surface() == "aluminium" && appearance.get_finish() == "soft" &&
                appearance.get_accent() == "#E8C47A" && appearance.get_secondary() == "#8C8F95" &&
                appearance.get_mod_palette() == "standard" && appearance.get_host() == "#5B9DFF",
                "Appearance defaults match the v3 design controls");
        const auto choose = [&](const char* field, std::string_view value, int slot) {
            const auto before = window->get_appearance_events();
            window->invoke_choose_theme(field, slint::SharedString(value));
            require(window->get_appearance_events() == before + 1, "Every appearance selection emits one native change");
            require(window->get_appearance_values()->row_data(slot).value() == slint::SharedString(value),
                    "The selected theme field reaches the six-field native callback");
        };
        for (const auto value : surface_names) choose("surface", value, 0);
        for (const auto value : finish_names) choose("finish", value, 1);
        for (const auto value : mod_palette_names) choose("mod-palette", value, 4);
        for (const auto value : accent_presets) choose("accent", material_hex(value), 2);
        for (const auto value : secondary_presets) choose("secondary", material_hex(value), 3);
        for (const auto value : host_presets) choose("host", material_hex(value), 5);
        choose("accent", "#123456", 2); choose("secondary", "#cafe80", 3); choose("host", "#ff00cc", 5);
        const auto event_count = window->get_appearance_events();
        window->invoke_choose_theme("unknown", "ignored");
        require(window->get_appearance_events() == event_count, "Unknown appearance fields cannot emit a change");
        ThemeSettings live_settings;
        publish_appearance(appearance, live_settings);
        apply_theme(global, resolve_theme(live_settings));
        window->on_appearance_changed([&](const slint::SharedString& surface, const slint::SharedString& finish,
                const slint::SharedString& accent, const slint::SharedString& secondary,
                const slint::SharedString& modulation, const slint::SharedString& host) {
            apply_appearance(appearance, global, live_settings, surface, finish, accent, secondary, modulation, host);
        });
        window->invoke_choose_theme("surface", "paper");
        require(live_settings.surface == Surface::Paper && global.get_light(), "A real appearance event changes native material and selected state");
        window->invoke_choose_theme("accent", "#AbC");
        require(live_settings.accent == 0xaabbcc && appearance.get_accent() == "#AABBCC", "Custom input reaches the real theme and canonical selection");
        const auto retained_settings = live_settings;
        const auto retained_accent = global.get_accent();
        window->invoke_choose_theme("accent", "invalid");
        require(live_settings == retained_settings && global.get_accent() == retained_accent,
                "Invalid custom input cannot partially change the live palette");
        require(appearance.get_accent() == "#AABBCC" && appearance.get_error() == "Invalid accent color",
                "Rejected input restores the selected color and reports the failed field");
        window->invoke_choose_theme("accent", "#E08A4E");
        require(appearance.get_accent() == "#E08A4E" && appearance.get_error().empty(),
                "A valid retry clears error and preserves preset button selection");
        appearance.set_opened(true);
        window->window().set_size(slint::PhysicalSize{{520, 1000}});
        window->window().dispatch_resize_event(slint::LogicalSize{{520, 1000}});
        window->window().show();
        const auto activate = [&](std::string_view label) {
            const auto buttons = slint::testing::ElementHandle::find_by_accessible_label(window, label);
            require(!buttons.empty(), "The actual appearance control is present in the native scene");
            const auto before = window->get_appearance_events();
            buttons[0].invoke_accessible_default_action();
            require(window->get_appearance_events() == before + 1, "The actual appearance button dispatches one event");
        };
        for (std::size_t i = 0; i < surface_names.size(); ++i) {
            activate(surface_names[i]);
            require(static_cast<std::size_t>(live_settings.surface) == i, "The surface button changes the native selection");
        }
        for (std::size_t i = 0; i < finish_names.size(); ++i) {
            activate(finish_names[i]);
            require(static_cast<std::size_t>(live_settings.finish) == i, "The finish button changes native material flags");
        }
        for (std::size_t i = 0; i < mod_palette_names.size(); ++i) {
            activate(mod_palette_names[i]);
            require(static_cast<std::size_t>(live_settings.mod_palette) == i, "The modulation button updates the native palette");
        }
        for (const auto field : {"Accent", "Secondary", "Host automation"}) {
            const auto presets = std::string_view(field) == "Accent" ? std::vector<std::uint32_t>(accent_presets.begin(), accent_presets.end()) :
                    std::string_view(field) == "Secondary" ? std::vector<std::uint32_t>(secondary_presets.begin(), secondary_presets.end()) :
                    std::vector<std::uint32_t>(host_presets.begin(), host_presets.end());
            auto anchor = slint::testing::ElementHandle::find_by_accessible_label(window, std::string(field) + " custom color");
            if (anchor.empty()) {
                const auto visible = slint::testing::ElementHandle::find_by_accessible_label(window, "Secondary custom color");
                require(!visible.empty(), "A visible field anchors preset scrolling");
                const auto p = visible[0].absolute_position();
                window->window().dispatch_pointer_scroll_event(slint::LogicalPosition{{p.x + 6, p.y + 6}}, 0, -700);
                slint::cbindgen_private::slint_mock_elapsed_time(1000); slint::platform::update_timers_and_animations();
                anchor = slint::testing::ElementHandle::find_by_accessible_label(window, std::string(field) + " custom color");
            }
            require(!anchor.empty(), "Preset family has a visible native section anchor");
            for (const auto preset : presets) {
                auto preset_label = material_hex(preset);
                for (auto& character : preset_label) character = static_cast<char>(std::toupper(static_cast<unsigned char>(character)));
                const auto before = window->get_appearance_events(); const auto retained = live_settings; bool selected = false;
                for (const auto& button : slint::testing::ElementHandle::find_by_accessible_label(window, preset_label)) {
                    const auto y = button.absolute_position().y, anchor_y = anchor[0].absolute_position().y;
                    if (button.accessible_role() == slint::language::AccessibleRole::Button && y < anchor_y && y > anchor_y - 100) {
                        button.invoke_accessible_default_action(); selected = true; break;
                    }
                }
                const auto value = std::string_view(field) == "Accent" ? live_settings.accent : std::string_view(field) == "Secondary" ? live_settings.secondary : live_settings.host_color;
                const bool untouched = live_settings.surface == retained.surface && live_settings.finish == retained.finish && live_settings.mod_palette == retained.mod_palette &&
                    (std::string_view(field) == "Accent" || live_settings.accent == retained.accent) &&
                    (std::string_view(field) == "Secondary" || live_settings.secondary == retained.secondary) &&
                    (std::string_view(field) == "Host automation" || live_settings.host_color == retained.host_color);
                require(selected && value == preset && untouched && window->get_appearance_events() == before + 1,
                        (std::string("Actual native preset swatch changes only its intended field: ") + field + " " + material_hex(preset)).c_str());
            }
        }
        for (const auto label : {"Accent", "Secondary", "Host automation"}) {
            auto input = slint::testing::ElementHandle::find_by_accessible_label(window, std::string(label) + " custom color");
            if (input.empty()) {
                window->window().dispatch_pointer_scroll_event(slint::LogicalPosition{{260, 500}}, 0, std::string_view(label) == "Host automation" ? -700 : 700);
                slint::cbindgen_private::slint_mock_elapsed_time(1000);
                slint::platform::update_timers_and_animations();
                input = slint::testing::ElementHandle::find_by_accessible_label(window, std::string(label) + " custom color");
            }
            require(!input.empty(), (std::string("Native custom input present: ") + label).c_str());
            input[0].set_accessible_value("#123ABC");
            activate(std::string("Apply ") + label + " color");
            const auto value = std::string_view(label) == "Accent" ? live_settings.accent :
                    std::string_view(label) == "Secondary" ? live_settings.secondary : live_settings.host_color;
            require(value == 0x123abc, "Applying native text input changes the intended custom palette field");
        }
        const auto close = slint::testing::ElementHandle::find_by_accessible_label(window, "Close");
        require(!close.empty(), "Appearance has an accessible close action");
        close[0].invoke_accessible_default_action();
        require(!appearance.get_opened(), "The native close button dismisses the appearance editor");
        appearance.set_opened(true);
        const slint::LogicalPosition inside{{30, 255}};
        window->window().dispatch_pointer_press_event(inside, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(inside, slint::PointerEventButton::Left);
        require(appearance.get_opened(), "Clicking inside the native Appearance panel keeps it open");
        const slint::LogicalPosition scrim{{2, 2}};
        window->window().dispatch_pointer_press_event(scrim, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(scrim, slint::PointerEventButton::Left);
        require(!appearance.get_opened(), "Clicking the actual Appearance scrim dismisses it");
        appearance.set_opened(true);
        const auto focus_input = slint::testing::ElementHandle::find_by_accessible_label(window, "Host automation custom color");
        require(!focus_input.empty(), "Appearance Escape test starts with a visible actual input");
        const auto p = focus_input[0].absolute_position();
        const slint::LogicalPosition focus{{p.x + 6, p.y + 6}};
        window->window().dispatch_pointer_press_event(focus, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(focus, slint::PointerEventButton::Left);
        window->window().dispatch_key_press_event("\x1b");
        require(!appearance.get_opened(), "Escape from a focused native Appearance input dismisses its editor");
        window->window().hide();
        std::cout << "PASS: " << checks << " actual generated advanced sampler theme binding checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
