#include "SlintEditorDisplays.h"
#include "ui/slint/host/SpectralPreview.h"
#include "tests/fixtures/advanced_sampler_spectral_golden.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>

int main(int argc, char** argv) {
    int checks = 0;
    const auto require = [&](bool ok, const std::string& message) {
        ++checks; if (!ok) throw std::runtime_error(message);
    };
    try {
        const bool render = argc == 2;
        if (!render) slint::testing::init();
        auto window = dandrum_ui::SlintEditorDisplays::create();
        auto& theme = window->global<dandrum_ui::EditorTheme>();
        const auto color = [](unsigned rgb) { return slint::Color::from_argb_uint8(255, rgb >> 16, rgb >> 8, rgb); };
        theme.set_soft(false); theme.set_ink_0(color(0x130f0c));
        theme.set_waveform_region(color(0xd0d0d0)); theme.set_waveform(color(0xd0d0d0));
        theme.set_accent(color(0xe08a4e)); theme.set_paper_2(color(0xc0c0c0));
        const auto report=window->invoke_run_contracts();
        require(std::string_view(report).starts_with("PASS:"), std::string(report));
        checks += window->get_checks();
        const std::array<float, 4> amplitudes{1,.5f,0,.25f};
        for (const auto& oracle : dandrum::spectral_test::cases) {
            const auto pixels = dandrum::slint_ui::spectral_pixels(amplitudes, oracle.profile, oracle.reversed, oracle.start, oracle.end, dandrum::spectral_test::palette);
            std::uint32_t hash = 2166136261u;
            for (auto value : pixels.rgba) hash = (hash ^ value) * 16777619u;
            require(hash == oracle.hash, "Shared spectral preview retains every independent guide oracle pixel");
        }
        window->show();
        if (render) {
            int phase = 0;
            std::string failure;
            slint::Timer timer;
            timer.start(slint::TimerMode::Repeated, std::chrono::milliseconds(150), [&] {
                try {
                    const auto snapshot = window->window().take_snapshot();
                    require(snapshot.has_value(), "The independent library consumer produces native pixels");
                    const auto pixel = [&](int x, int y) { return snapshot->begin()[y * snapshot->width() + x]; };
                    std::ofstream output(std::string(argv[1]) + "/shared-displays-" + std::to_string(phase) + ".pam", std::ios::binary);
                    output << "P7\nWIDTH " << snapshot->width() << "\nHEIGHT " << snapshot->height() << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
                    output.write(reinterpret_cast<const char*>(snapshot->begin()), snapshot->width()*snapshot->height()*4);
                    if (phase == 0) {
                        auto above = pixel(85,170), negative = pixel(85,250), mini = pixel(190,75);
                        require(above.r == 19 && above.g == 15 && above.b == 12, "Signed maximum .25 leaves the known upper waveform area empty");
                        require(negative.r == 208 && negative.g == 208 && negative.b == 208, "Signed minimum -1 fills the known lower waveform area");
                        require(mini.r == 208 && mini.g == 208 && mini.b == 208, "Shared pad waveform preserves signed negative extrema");
                        const auto cursor = pixel(255,75);
                        require(cursor.r == 224 && cursor.g == 138 && cursor.b == 78, "Pad waveform playhead paints the supplied accent role");
                        window->set_reversed(true);
                    } else if (phase == 1) {
                        const auto first = pixel(85,250), last = pixel(535,250);
                        require(first.r == 19 && first.g == 15 && first.b == 12 && last.r == 208 && last.g == 208, "Shared waveform reversal moves the independently known -1 peak to the final column");
                        window->set_wave_state("missing");
                    } else {
                        const auto empty = pixel(85,250);
                        require(empty.r == 19 && empty.g == 15 && empty.b == 12, "Missing source retains feedback without painting cached data");
                        require(!slint::testing::ElementHandle::find_by_accessible_label(window,"Sample unavailable · Relink or replace").empty(), "Missing source presents its actionable library feedback");
                        timer.stop(); slint::quit_event_loop(); return;
                    }
                    ++phase;
                } catch (const std::exception& error) { failure = error.what(); timer.stop(); slint::quit_event_loop(); }
            });
            window->run();
            require(failure.empty(), failure);
        } else {
            const auto settle = [&] {
                slint::platform::update_timers_and_animations();
                slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop();
            };
            const auto element = [&](const char* label) {
                settle();
                for (auto item : slint::testing::ElementHandle::find_by_accessible_label(window,label))
                    if (item.accessible_role().has_value()) return item;
                throw std::runtime_error(std::string("Missing shared display: ") + label);
            };
            const auto point = [&](const char* label, float fraction = .5f) {
                auto item = element(label); auto p = item.absolute_position(); auto s = item.size();
                return slint::LogicalPosition{{p.x+s.width*.5f,p.y+s.height*fraction}};
            };
            const auto click = [&](const char* label) {
                auto p = point(label); window->window().dispatch_pointer_press_event(p,slint::PointerEventButton::Left);
                window->window().dispatch_pointer_release_event(p,slint::PointerEventButton::Left); settle();
            };
            const auto key = [&](slint::SharedString text) { window->window().dispatch_key_press_event(text); window->window().dispatch_key_release_event(text); settle(); };
            for (bool pad : {false,true}) {
                const char* label = pad ? "Pad 60 C4, Known pad" : "Key 60 C4";
                const auto press = [&] { if (pad) window->invoke_press_pad(); else window->invoke_press_key(); settle(); };
                for (int change = 0; change < 4; ++change) {
                    window->set_input_enabled(true); window->set_input_visible(true); window->set_note(60); window->set_generation("one"); window->set_source("original"); settle();
                    press(); const int before = window->get_releases();
                    if (change == 0) window->set_note(61);
                    if (change == 1) window->set_generation("two");
                    if (change == 2) window->set_input_enabled(false);
                    if (change == 3) window->set_input_visible(false);
                    settle();
                    require(window->get_releases() == before+1 && window->get_last_release() == 60 && window->get_released_source() == "original", "Remapping, generation, disable and hide release each originally captured input exactly once");
                }
                window->set_input_enabled(true); window->set_input_visible(true); window->set_note(60); settle();
                const int before = window->get_presses();
                click(label); require(window->get_presses() == before+1 && window->get_releases() == window->get_presses(), "Native pointer press and release stay balanced");
                key(" "); key(slint::platform::key_codes::Return);
                require(window->get_presses() == before+3 && window->get_releases() == window->get_presses(), "Native Space and Enter hold/release through the reusable component");
                element(label).invoke_accessible_default_action(); settle();
                require(window->get_presses() == before+4 && window->get_releases() == window->get_presses(), "Accessibility activation is balanced");
                press(); window->set_source("new source"); settle(); window->invoke_release_inputs(); settle();
                require(window->get_released_source() == "original", "Changing display source leaves the held input's release ownership intact");
            }
            auto start = point("Standalone waveform handle");
            window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);
            window->window().dispatch_pointer_move_event(slint::LogicalPosition{{start.x+25,start.y}}); settle();
            require(window->get_gesture_phase() == 1 && std::abs(window->get_gesture_value()-.5f)<.001f, "Reusable waveform handle reports normalized native drag updates");
            key(slint::platform::key_codes::Escape);
            require(window->get_gesture_phase() == 3 && std::abs(window->get_gesture_value()-.25f)<.001f, "Escape cancels to the captured normalized position");
            window->window().dispatch_pointer_release_event(start,slint::PointerEventButton::Left); settle();
            key(slint::platform::key_codes::RightArrow);
            require(window->get_gesture_phase() == 2 && std::abs(window->get_handle_position()-.251f)<.001f, "Keyboard changes are bracketed and end through the generic handle callback");
            click("Audition selected pad waveform"); element("Audition selected pad waveform").invoke_accessible_default_action(); settle();
            require(window->get_auditions() == 2, "Pad waveform audition is a caller-owned pointer/accessibility callback");
            window->set_wave_zoom(2); settle();
            require(std::abs(window->get_visible_end()-.5f)<.001f, "Waveform zoom publishes the visible normalized range");
            click("Display Wave"); const auto before=window->get_deactivations();
            window->window().dispatch_window_active_changed_event(false); settle();
            require(window->get_deactivations()==before+1, "Waveform display controls forward window deactivation to their consumer's note owner");
            window->window().dispatch_window_active_changed_event(true); settle();
        }
        std::cout << "PASS: " << checks << " library-only display " << (render ? "pixel" : "interaction") << " checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL after " << checks << " checks: " << error.what() << '\n'; return 1;
    }
}
