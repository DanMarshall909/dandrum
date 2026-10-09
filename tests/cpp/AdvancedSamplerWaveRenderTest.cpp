#include "AdvancedSamplerWaveRender.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include <slint-testing.h>
#include <chrono>
#include <fstream>
#include <iostream>
#include <stdexcept>

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    int checks = 0;
    try {
        auto window = dandrum_ui::AdvancedSamplerWaveRenderTest::create();
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
        const auto& theme = window->global<dandrum_ui::SamplerTheme>();
        const auto color = [](unsigned rgb) { return slint::Color::from_argb_uint8(255, rgb >> 16, rgb >> 8, rgb); };
        theme.set_ink_0(color(0x130f0c)); theme.set_ink_5(color(0x41362c));
        theme.set_accent_lo(color(0xb0662f)); theme.set_accent(color(0xe08a4e));
        theme.set_paper_1(color(0xf2e6d3)); theme.set_waveform_region(color(0xd0d0d0));
        const auto require = [&](bool ok, const std::string& text) { ++checks; if (!ok) throw std::runtime_error(text); };
        const auto activate = [&](std::string_view label) {
            for (auto element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (element.accessible_role() == slint::language::AccessibleRole::Button) {
                    element.invoke_accessible_default_action(); return;
                }
            throw std::runtime_error("Missing native display control: " + std::string(label));
        };
        int phase = 0;
        std::string failure;
        slint::Timer timer;
        timer.start(slint::TimerMode::Repeated, std::chrono::milliseconds(150), [&] {
            try {
                const auto image = window->window().take_snapshot();
                require(image.has_value(), "Real native renderer supplies waveform and spectral pixels");
                std::ofstream output(std::string(argv[1]) + "/wave-" + std::to_string(phase) + ".pam", std::ios::binary);
                output << "P7\nWIDTH " << image->width() << "\nHEIGHT " << image->height()
                       << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
                output.write(reinterpret_cast<const char*>(image->begin()), image->width() * image->height() * 4);
                const auto pixel = [&](int x, int y) { return image->begin()[y * image->width() + x]; };
                if (phase == 0 || phase == 4 || phase == 7) {
                    const auto above = pixel(85, 80), negative = pixel(85, 155);
                    require(above.r == 19 && above.g == 15 && above.b == 12,
                            "Signed maximum .25 leaves its known upper waveform region empty");
                    require(negative.r == 208 && negative.g == 208 && negative.b == 208,
                            "Signed minimum -1 fills its known lower waveform region");
                } else if (phase == 5) {
                    const auto first = pixel(85, 155), last = pixel(535, 155);
                    require(first.r == 19 && first.g == 15 && first.b == 12,
                            "Reversed waveform starts with the known short negative tail instead of the original -1 peak");
                    require(last.r == 208 && last.g == 208 && last.b == 208,
                            "Reversed waveform places the known signed -1 peak at the last source column");
                } else if (phase == 6) {
                    // Stay above the native display switch at y187..205.
                    const auto last = pixel(535, 185);
                    require(last.r == 0 && last.g == 255 && last.b == 0,
                            "Reversed spectral preview places the independently calibrated original low-frequency peak at the end");
                } else if (phase == 3) {
                    // Source column 2 is centered at .625. In [.5,1] it maps
                    // to .25 of the 600px plot, plus the fixture's 10px inset.
                    const auto silence = pixel(160, 190);
                    require(silence.r == 19 && silence.g == 15 && silence.b == 12,
                            "Spectral viewport shows the selected silent source column instead of the full source");
                    bool croppedRulerStartsAtHalf = false;
                    for (const auto& tick : slint::testing::ElementHandle::find_by_accessible_label(window, "0.5s"))
                        croppedRulerStartsAtHalf |= tick.absolute_position().x < 40;
                    if (!croppedRulerStartsAtHalf)
                        for (const auto& tick : slint::testing::ElementHandle::find_by_accessible_label(window, "0.5s"))
                            std::cerr << "0.5s ruler tick at " << tick.absolute_position().x << '\n';
                    require(croppedRulerStartsAtHalf,
                            "Cropped waveform ruler starts at its actual source time instead of zero");
                } else {
                    const auto low = pixel(85, 190);
                    require(phase == 1 ? low.r == 242 && low.g == 230 && low.b == 211
                                       : low.r == 0 && low.g == 255 && low.b == 0,
                            "Spectral low-frequency pixel displays the independently calibrated preview and reacts to palette changes");
                }
                if (++phase == 8) { timer.stop(); slint::quit_event_loop(); }
                else if (phase == 1) activate("Display Spectral");
                else if (phase == 2) theme.set_paper_1(color(0x00ff00));
                else if (phase == 3) window->set_view_start(.5);
                else if (phase == 4) { window->set_view_start(0); activate("Display Wave"); }
                else if (phase == 5) window->set_reversed(true);
                else if (phase == 6) activate("Display Spectral");
                else { window->set_reversed(false); activate("Display Wave"); }
            } catch (const std::exception& error) { failure = error.what(); timer.stop(); slint::quit_event_loop(); }
        });
        window->run();
        if (!failure.empty()) throw std::runtime_error(failure);
        require(phase == 8, "Both display modes, reversal, viewport crop and live palette invalidation are rendered");
        std::cout << "PASS: " << checks << " actual native waveform renderer checks\n";
    } catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
