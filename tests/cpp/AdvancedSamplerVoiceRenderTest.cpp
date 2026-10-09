#include "AdvancedSamplerVoice.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/theme/ThemeBinding.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>

namespace {
int checks = 0;
void require(bool result, const std::string& message) { ++checks; if (!result) throw std::runtime_error(message); }
template<class Window> auto slider(const Window& window, std::string_view label) {
    for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
        if (element.accessible_role() == slint::language::AccessibleRole::Slider) return element;
    throw std::runtime_error("Missing native rendered plot: " + std::string(label));
}
bool bright(const slint::SharedPixelBuffer<slint::Rgba8Pixel>& image, int x, int y) {
    if (x < 0 || y < 0 || x >= static_cast<int>(image.width()) || y >= static_cast<int>(image.height())) return false;
    const auto& pixel = image.begin()[y * image.width() + x];
    return pixel.r > 145 && pixel.g > 145 && pixel.b > 135 && std::abs(int(pixel.r) - int(pixel.g)) < 35;
}
bool nearLine(const slint::SharedPixelBuffer<slint::Rgba8Pixel>& image, float x, float y) {
    for (int dy = -2; dy <= 2; ++dy) for (int dx = -1; dx <= 1; ++dx)
        if (bright(image, std::lround(x) + dx, std::lround(y) + dy)) return true;
    return false;
}
}

// This uses the actual native renderer, with or without a private virtual display.
// The native testing backend intentionally cannot prove Path rendering.
int main(int argc, char** argv) {
    if (argc != 2) { std::cerr << "Usage: sampler-voice-render-test EVIDENCE_DIRECTORY\n"; return 2; }
    try {
        auto window = dandrum_ui::AdvancedSamplerVoiceTest::create();
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
        const auto& session = window->global<dandrum_ui::Session>();
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        dandrum::advanced_sampler::apply_theme(window->global<dandrum_ui::SamplerTheme>(),
            dandrum::advanced_sampler::resolve_theme({}));
        require(session.invoke_command("state.select", "voice", "", 0), "Load the actual native Voice state");
        int phase = 0;
        std::string failure;
        slint::Timer capture;
        capture.start(slint::TimerMode::Repeated, std::chrono::milliseconds(150), [&] {
            try {
                const auto image = window->window().take_snapshot();
                require(image.has_value(), "The native renderer supplies a real plot snapshot");
                const auto name = phase == 0 ? "lp" : phase == 1 ? "hp" : phase == 2 ? "bp" : "notch";
                std::ofstream output(std::string(argv[1]) + "/voice-" + name + ".pam", std::ios::binary);
                output << "P7\nWIDTH " << image->width() << "\nHEIGHT " << image->height()
                       << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
                output.write(reinterpret_cast<const char*>(image->begin()), image->width() * image->height() * 4);
                const auto plot = slider(window, "Filter response");
                const auto p = plot.absolute_position(); const auto s = plot.size();
                const float low_x = p.x + 8 + (s.width - 16) * .2f;
                const float low_y = p.y + 4 + (s.height - 8) * .22f;
                if (phase == 0) {
                    int stroke_pixels = 0;
                    for (int y = std::lround(p.y + 17); y < std::lround(p.y + s.height - 17); ++y)
                        stroke_pixels += bright(*image, std::lround(low_x), y);
                    require(stroke_pixels <= 5, "LP draws one response curve, not independently stretched Path segments; bright pixels=" + std::to_string(stroke_pixels));
                    require(nearLine(*image, low_x, low_y), "LP response passes through its known low-frequency 0 dB position");
                    const auto sustain = slider(window, "Filter envelope sustain handle");
                    const auto release = slider(window, "Filter envelope release handle");
                    const auto a = sustain.absolute_position(); const auto b = release.absolute_position();
                    require(nearLine(*image, (a.x + b.x) / 2 + 5.5f, (a.y + b.y) / 2 + 5.5f),
                        "The rendered Filter release segment joins its actual sustain and release handles");
                } else if (phase == 3) {
                    const float cutoff_x = p.x + 8 + (s.width - 16) * std::log(binding->model()->number("cutoff") / 20) / std::log(1000);
                    require(!nearLine(*image, cutoff_x, low_y), "Notch visibly suppresses its cutoff frequency instead of retaining the LP response");
                } else {
                    require(!nearLine(*image, low_x, low_y), std::string(name) + " visibly changes the low-frequency response from LP");
                }
                if (++phase == 4) { capture.stop(); slint::quit_event_loop(); }
                else require(session.invoke_command("filter.mode", "", phase == 1 ? "hp" : phase == 2 ? "bp" : "notch", 0), "Set the next actual native filter mode");
            } catch (const std::exception& error) { failure = error.what(); capture.stop(); slint::quit_event_loop(); }
        });
        window->run();
        if (!failure.empty()) throw std::runtime_error(failure);
        require(phase == 4, "Every native response mode is rendered and checked");
        std::cout << "PASS: " << checks << " actual native Voice renderer checks\n";
    } catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
