#include "AdvancedSamplerTheme.h"
#include "../../ui/advanced-sampler/theme/AppearanceBinding.h"

#include <filesystem>
#include <fstream>
#include <iostream>

// Run this executable on the same private display and renderer as the app.
// Images come from the compiled native component after all native palette setters.
int main(int argc, char** argv) {
    if (argc != 2) { std::cerr << "Usage: advanced-sampler-theme-capture OUTPUT_DIRECTORY\n"; return 2; }
    using namespace dandrum::advanced_sampler;
    const std::filesystem::path destination = argv[1];
    std::filesystem::create_directories(destination);
    const auto window = dandrum_ui::AdvancedSamplerThemeTest::create();
    const auto& theme = window->global<dandrum_ui::SamplerTheme>();
    const auto& appearance = window->global<dandrum_ui::AppearanceState>();
    ThemeSettings settings;
    int capture = 0;
    slint::Timer timer;
    const auto prepare = [&] {
        settings.surface = static_cast<Surface>(capture / 2);
        settings.finish = static_cast<Finish>(capture % 2);
        if (capture >= 16) {
            settings.surface = Surface::Aluminium;
            settings.finish = Finish::Soft;
            settings.accent = 0x123456; settings.secondary = 0xcafe80; settings.host_color = 0xff00cc;
        }
        apply_theme(theme, resolve_theme(settings));
        publish_appearance(appearance, settings);
        appearance.set_opened(capture >= 17);
        window->window().set_size(slint::PhysicalSize{{520, capture >= 17 ? 800u : 300u}});
        if (capture == 18) window->window().dispatch_pointer_scroll_event(slint::LogicalPosition{{260, 600}}, 0, -700);
    };
    prepare();
    window->window().show();
    timer.start(slint::TimerMode::Repeated, std::chrono::milliseconds(150), [&] {
        const auto image = window->window().take_snapshot();
        if (!image) { std::cerr << "Native renderer does not support screenshots\n"; slint::quit_event_loop(); return; }
        const auto name = capture < 16 ? std::string(surface_names[capture / 2]) + "-" + std::string(finish_names[capture % 2]) :
                capture == 16 ? "custom-colors" : capture == 17 ? "appearance" : "appearance-scrolled";
        std::ofstream output(destination / (name + ".pam"), std::ios::binary);
        output << "P7\nWIDTH " << image->width() << "\nHEIGHT " << image->height()
               << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
        output.write(reinterpret_cast<const char*>(image->begin()), image->width() * image->height() * 4);
        std::cout << "Captured " << name << '\n';
        ++capture;
        if (capture == 19) { timer.stop(); window->window().hide(); slint::quit_event_loop(); }
        else prepare();
    });
    slint::run_event_loop();
    return capture == 19 ? 0 : 1;
}
