#include "AppWindow.h"
#include "engine/Bindings.h"
#include "engine/FileOperations.h"
#include "performance/Bindings.h"
#include "theme/AppearanceBinding.h"
#include "../slint/host/ValueCodec.h"
#include <iostream>
#include <fstream>
#include <chrono>
#include <algorithm>
#include <memory>
#include <string>

int main(int argc, char** argv) {
    using namespace dandrum_ui;
    using namespace dandrum::advanced_sampler;
    auto model = std::make_shared<dandrum::sampler::Model>();
    std::string state;
    std::string screenshot;
    ThemeSettings initial_theme;
    int size = 1;
    for (int i = 1; i < argc; ++i) {
        const std::string argument = argv[i];
        if (argument == "--state" && i + 1 < argc) state = argv[++i];
        else if (argument == "--screenshot" && i + 1 < argc) screenshot = argv[++i];
        else if (argument == "--theme" && i + 1 < argc) {
            const std::string value = argv[++i];
            auto found = std::find(surface_names.begin(), surface_names.end(), value);
            if (found == surface_names.end()) { std::cerr << "Unknown surface: " << value << '\n'; return 2; }
            initial_theme.surface = static_cast<Surface>(found - surface_names.begin());
        }
        else if (argument == "--finish" && i + 1 < argc) {
            const std::string value = argv[++i];
            if (value != "soft" && value != "flat") { std::cerr << "Finish must be soft or flat\n"; return 2; }
            initial_theme.finish = value == "soft" ? Finish::Soft : Finish::Flat;
        }
        else if (argument == "--size" && i + 1 < argc) {
            const std::string value = argv[++i];
            if (value == "min") size = 0;
            else if (value == "default") size = 1;
            else if (value == "expanded") size = 2;
            else { std::cerr << "Size must be min, default or expanded\n"; return 2; }
        } else {
            std::cerr << "Usage: dandrum-advanced-sampler [--state STATE] [--size min|default|expanded]\n";
            return 2;
        }
    }
    if (!state.empty() && !model->selectState(state)) {
        std::cerr << "Unknown sampler state: " << state << '\n';
        return 2;
    }
    auto window = AppWindow::create();
    auto binding = dandrum::sampler::bindWindowSession<Session>(window, model);
    dandrum::sampler::bind_patch_search(window->global<PatchFilter>());
    dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>());
    auto settings = std::make_shared<ThemeSettings>(initial_theme);
    apply_theme(window->global<SamplerTheme>(), resolve_theme(*settings));
    publish_appearance(window->global<AppearanceState>(), *settings);
    const slint::ComponentWeakHandle<AppWindow> weak(window);
    window->on_resize_window([weak](float width, float height) {
        if (const auto window = weak.lock()) (*window)->window().set_size(slint::LogicalSize{{width, height}});
    });
    window->on_appearance_change([weak, settings](slint::SharedString surface, slint::SharedString finish,
            slint::SharedString accent, slint::SharedString secondary, slint::SharedString modulation, slint::SharedString host) {
        if (const auto window = weak.lock()) apply_appearance((*window)->global<AppearanceState>(),
                (*window)->global<SamplerTheme>(), *settings, surface.data(), finish.data(), accent.data(),
                secondary.data(), modulation.data(), host.data());
    });
    dandrum::sampler::bind_file_operations(window, binding);
    dandrum::sampler::bind_performance_sources(window->global<PerformanceState>(), binding);
    window->invoke_choose_size(size);
    if (state == "perf" && size == 1) window->window().set_size(slint::LogicalSize{{860, 610}});
    std::cout << model->text("status") << '\n';
    slint::Timer capture;
    bool captured = screenshot.empty();
    if (!screenshot.empty()) capture.start(slint::TimerMode::SingleShot, std::chrono::milliseconds(400), [&] {
        if (const auto image = window->window().take_snapshot()) {
            std::ofstream output(screenshot, std::ios::binary);
            output << "P7\nWIDTH " << image->width() << "\nHEIGHT " << image->height()
                   << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
            output.write(reinterpret_cast<const char*>(image->begin()), image->width() * image->height() * 4);
            captured = output.good();
        }
        slint::quit_event_loop();
    });
    window->run();
    model->releaseNotes();
    return captured ? 0 : 1;
}
