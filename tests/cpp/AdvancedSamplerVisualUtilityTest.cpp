#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/advanced-sampler/theme/ThemeBinding.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <algorithm>
#include <array>
#include <cmath>
#include <chrono>
#include <fstream>
#include <filesystem>
#include <limits>
#include <iostream>
#include <stdexcept>
#include <vector>

namespace {
// Native snapshots prove that material paint reaches the composed drawer and keys.
int render_materials(const std::filesystem::path& evidence) {
    using namespace dandrum_ui;
    using namespace dandrum::advanced_sampler;
    auto window = AppWindow::create();
    auto binding = dandrum::sampler::bindWindowSession<Session>(window);
    dandrum::sampler::bind_performance_sources(window->global<PerformanceState>(), binding);
    dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>());
    const auto& session = window->global<Session>();
    if (!session.invoke_command("state.select", "sample", "", 0)) throw std::runtime_error("Load actual material comparison sample");
    std::filesystem::create_directories(evidence);
    int phase = 0, checks = 0;
    std::string failure;
    ThemeSettings settings;
    apply_theme(window->global<SamplerTheme>(), resolve_theme(settings));
    const auto require = [&](bool ok, const std::string& why) { ++checks; if (!ok) throw std::runtime_error(why); };
    const auto actual = [&](std::string_view type) {
        const auto found = slint::testing::ElementHandle::find_by_element_type_name(window, type);
        for (const auto& e : found) if (e.size().width > 0 && e.size().height > 0 && (type != "WaveView" || e.size().height > 80)) return e;
        throw std::runtime_error("Missing actual material target: " + std::string(type));
    };
    slint::Timer timer;
    timer.start(slint::TimerMode::Repeated, std::chrono::milliseconds(150), [&] {
        try {
            const auto image = window->window().take_snapshot();
            require(image.has_value() && image->width() == 1200 && image->height() == 800, "Material proof uses actual scale1 native1200x800 pixels");
            const auto pixel = [&](float x, float y) {
                const int xx = std::clamp(int(std::lround(x)), 0, int(image->width()) - 1);
                const int yy = std::clamp(int(std::lround(y)), 0, int(image->height()) - 1);
                return image->begin()[yy * image->width() + xx];
            };
            if (phase>=16) {
                const auto targets=slint::testing::ElementHandle::find_by_accessible_label(window,"Browse samples");
                require(!targets.empty(), "Empty invitation exposes its actual primary browse action");
                const auto p=targets[0].absolute_position();const auto size=targets[0].size();
                const auto near=pixel(p.x-3,p.y+size.height/2),far=pixel(p.x-13,p.y+size.height/2);
                require(phase==16 ? near.r>far.r+5 && near.g>far.g+3
                                  : std::abs(int(near.r)-int(far.r))<=2 && std::abs(int(near.g)-int(far.g))<=2,
                        phase==16 ? "Soft primary invitation paints the guide's accent glow around its actual action"
                                  : "Flat primary invitation removes the Soft accent glow");
                if (++phase==18) {timer.stop();slint::quit_event_loop();}
                else {settings.finish=Finish::Flat;apply_theme(window->global<SamplerTheme>(),resolve_theme(settings));}
                return;
            }
            const auto wave = actual("WaveView"); const auto p = wave.absolute_position(); const auto size = wave.size();
            const auto top = pixel(p.x + size.width * .5f, p.y + size.height * .16f);
            const auto bottom = pixel(p.x + size.width * .5f, p.y + size.height * .83f);
            const auto palette = resolve_theme(settings);
            if (!palette.soft) {
                const auto cap = palette[Role::Ink5].rgb;
                for (const auto label : {"Default", "Sample source editor", "Play mode Once", "Sound"}) {
                    bool checked = false;
                    for (const auto& e : slint::testing::ElementHandle::find_by_accessible_label(window, label)) {
                        if (e.accessible_role() != slint::language::AccessibleRole::Button || e.size().width <= 0) continue;
                        const auto origin = e.absolute_position();
                        const auto color = pixel(origin.x + 4, origin.y + 4);
                        require(std::abs(int(color.r) - int((cap >> 16) & 255)) <= 2
                                && std::abs(int(color.g) - int((cap >> 8) & 255)) <= 2
                                && std::abs(int(color.b) - int(cap & 255)) <= 2,
                            std::string(label) + " selected segment uses the reference neutral ink5 face instead of accent wash");
                        checked = true; break;
                    }
                    require(checked, std::string("Actual selected segment remains accessible: ") + label);
                }
                const auto accent = palette[Role::Accent].rgb;
                bool accent_action = false;
                for (const auto& e : slint::testing::ElementHandle::find_by_accessible_label(window, "Show or hide pads")) {
                    if (e.accessible_role() != slint::language::AccessibleRole::Button || e.size().width <= 0) continue;
                    const auto origin = e.absolute_position();
                    const auto color = pixel(origin.x + 8, origin.y);
                    require(std::abs(int(color.r) - int((accent >> 16) & 255)) <= 8
                            && std::abs(int(color.g) - int((accent >> 8) & 255)) <= 8
                            && std::abs(int(color.b) - int(accent & 255)) <= 8,
                        "Ordinary selected Play toggle retains its actual accent border");
                    accent_action = true; break;
                }
                require(accent_action, "Actual selected Play toggle remains accessible");
            }
            if (palette.brushed || palette.soft) {
                require(bottom.g > top.g, "Soft/Brushed actual waveform well has the reference dark-top/light-bottom shading");
                if (palette.light) require(int(bottom.g) - int(top.g) >= 8, "Light well retains visible inset depth instead of a uniform fill");
            } else {
                require(std::abs(int(bottom.g) - int(top.g)) <= 2, "Non-Brushed Flat waveform well removes the soft gradient");
            }
            const auto range = [&](const slint::testing::ElementHandle& e, float xx, float yy) {
                const auto origin = e.absolute_position(); const auto extent = e.size();
                int low = 255, high = 0;
                for (int i = 0; i < 12; ++i) {
                    const auto color = pixel(origin.x + xx * extent.width, origin.y + yy * extent.height + i);
                    low = std::min(low, int(color.g)); high = std::max(high, int(color.g));
                }
                return high - low;
            };
            const auto macros = actual("MacroGrid");
            if (palette.brushed) {
                require(range(macros, .015f, .5f) >= 6, "Brushed grain reaches the actual Play macro background in both finishes");
                bool key = false;
                for (const auto& e : slint::testing::ElementHandle::find_by_element_type_name(window, "KeyCell")) {
                    const auto label = e.accessible_label(); const auto position = e.absolute_position(); const auto extent = e.size();
                    if (label && std::string_view(label->data()).starts_with("Key 61 ") && extent.width > 0 && position.x >= 0 && position.x + extent.width < 1200) {
                        require(range(e, .5f, .35f) >= 6, "Brushed grain reaches an actual mapped short key cap"); key = true; break;
                    }
                }
                require(key, "A genuine visible mapped C-sharp4 short key was sampled");
            }
            const std::string name = std::string(surface_names[static_cast<std::size_t>(settings.surface)]) + "-" + (settings.finish == Finish::Soft ? "soft" : "flat");
            std::ofstream output(evidence / (name + ".pam"), std::ios::binary);
            output << "P7\nWIDTH " << image->width() << "\nHEIGHT " << image->height() << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
            output.write(reinterpret_cast<const char*>(image->begin()), image->width() * image->height() * 4);
            if (++phase == 16) { settings=ThemeSettings{};apply_theme(window->global<SamplerTheme>(),resolve_theme(settings));session.invoke_command("state.select","empty","",0); }
            else { settings.surface = static_cast<Surface>(phase / 2); settings.finish = static_cast<Finish>(phase % 2); apply_theme(window->global<SamplerTheme>(), resolve_theme(settings)); }
        } catch (const std::exception& e) { failure = e.what(); timer.stop(); slint::quit_event_loop(); }
    });
    window->window().set_size(slint::LogicalSize{{1200,800}}); window->run();
    if (!failure.empty()) throw std::runtime_error(failure);
    std::cout << "PASS: " << checks << " actual composed material pixel checks across16 finishes\n";
    return 0;
}
}

// These bounds are measured on the composed native window, not on a duplicate layout.
int main(int argc, char** argv) {
    int checks = 0;
    try {
        if (argc == 2) return render_materials(argv[1]);
        if (argc == 3 && std::string_view(argv[1]) == "--render") return render_materials(argv[2]);
        using namespace dandrum_ui;
        slint::testing::init();
        auto window = AppWindow::create();
        auto binding = dandrum::sampler::bindWindowSession<Session>(window);
        auto model = binding->model();
        const auto& session = window->global<Session>();
        dandrum::sampler::bind_performance_sources(window->global<PerformanceState>(), binding);
        dandrum::sampler::bind_patch_search(window->global<PatchFilter>());
        dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>());
        dandrum::advanced_sampler::apply_theme(window->global<SamplerTheme>(), dandrum::advanced_sampler::resolve_theme({}));
        const auto require = [&](bool ok, const std::string& why) { ++checks; if (!ok) throw std::runtime_error(why); };
        const auto settle = [&] {
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop();
        };
        const auto click = [&](std::string_view label) {
            for (const auto& e : slint::testing::ElementHandle::find_by_accessible_label(window, label)) {
                const auto p = e.absolute_position(); const auto size = e.size();
                if (e.accessible_role() != slint::language::AccessibleRole::Button || size.width <= 0
                    || p.y < 0 || p.y + size.height > window->window().size().height) continue;
                const slint::LogicalPosition at{{p.x + size.width / 2, p.y + size.height / 2}};
                window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Left);
                window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Left); settle(); return;
            }
            throw std::runtime_error("Missing visible actual utility action: " + std::string(label));
        };
        window->show();
        for (const auto size : std::array{slint::LogicalSize{{820,560}}, slint::LogicalSize{{1200,800}}, slint::LogicalSize{{1600,1000}}}) {
            window->window().set_size(size);
            require(session.invoke_command("state.select", "diag", "", 0), "Open composed guide Diagnostics"); settle();
            const auto docks = slint::testing::ElementHandle::find_by_element_type_name(window, "DiagnosticsPage");
            require(docks.size() == 1, "Exactly one guide historical diagnostic dock is composed");
            const auto p = docks[0].absolute_position(); const auto extent = docks[0].size();
            auto found = slint::testing::ElementHandle::find_by_element_type_name(window, "TraceHeading");
            std::vector<slint::testing::ElementHandle> headings(found.begin(), found.end());
            std::sort(headings.begin(), headings.end(), [](const auto& a, const auto& b) { return a.absolute_position().x < b.absolute_position().x; });
            require(headings.size() == 2, "Historical dock retains its two guide column headings");
            const auto left = headings.front().absolute_position(); const auto right = headings.back().absolute_position();
            require(std::abs(left.x - (p.x + 12)) < 2 && std::abs(left.y - p.y) < 2,
                "WHY DID THIS PLAY remains at the left/top of its history column at every guide resolution");
            require(std::abs(right.x - (p.x + extent.width - 255 + 13)) < 2 && std::abs(right.y - (p.y + 9)) < 2,
                "IF SILENT remains above its right-column reasons at every guide resolution");
            require(left.x + headings.front().size().width <= p.x + 241 && right.x >= p.x + extent.width - 255,
                "Neither caption crosses the trace column or another column");
            require(session.invoke_command("event.clear", "", "", 0), "Start historical selection from actual empty events");
            require(session.invoke_note_on(38, 112, "visual-utility"), "Record one actual native mapped hit");
            session.invoke_note_off(38, "visual-utility"); settle();
            require(session.get_hits()->row_count() == 1, "One hit creates one historical selectable row");
            const auto sequence = session.get_hits()->row_data(0)->sequence;
            const auto hits = slint::testing::ElementHandle::find_by_accessible_label(window, "Inspect event " + std::to_string(sequence));
            require(!hits.empty() && hits[0].absolute_position().x < p.x + 240
                    && hits[0].absolute_position().y >= left.y + headings.front().size().height,
                "Actual historical event stays below its heading in the left history column");
            click("Inspect event " + std::to_string(sequence));
            require(session.get_selected_diagnostic_event() == sequence && session.get_sounding_notes()->row_count() == 0,
                "The repositioned historical action selects its trace without replaying a note");
        }
        window->window().set_size(slint::LogicalSize{{1200,800}});
        require(session.invoke_command("state.select", "overview", "", 0), "Open native inherited property guide state"); settle();
        const auto row = [&](std::string_view id) {
            for (std::size_t i = 0; i < session.get_overview()->row_count(); ++i) {
                auto value = session.get_overview()->row_data(i).value(); if (std::string_view(value.id.data()) == id) return value;
            }
            throw std::runtime_error("Missing native effective property: " + std::string(id));
        };
        require(row("fade-in").value_text == "2 ms" && row("fade-out").value_text == "380 ms",
            "Overview projects the advertised sample fade durations in milliseconds");
        require(row("cutoff").value_text == "2.40 kHz", "Overview uses the guide cutoff precision and unit");
        require(row("output").state == "local" && row("output").can_reset && row("fade-out").state == "mixed",
            "Guide inherited overview presents its real Local/Reset and Mixed rows");
        const auto overview = slint::testing::ElementHandle::find_by_element_type_name(window, "OverviewPage");
        const auto columns = slint::testing::ElementHandle::find_by_element_type_name(window, "ColumnHeading");
        require(overview.size() == 1 && columns.size() == 4, "Actual Overview retains its four property column headings");
        const auto overview_origin = overview[0].absolute_position();
        require(std::all_of(columns.begin(), columns.end(), [&](const auto& e) {
                return std::abs(e.absolute_position().y - (overview_origin.y + 44)) < .5f
                    && std::abs(e.size().height - 27) < .5f;
            }), "Overview reserves the reference header padding and border before its column headings");
        const auto output_values = slint::testing::ElementHandle::find_by_accessible_label(window, "Output bus: Drums 3/4");
        require(output_values.size() == 1 && std::abs(output_values[0].absolute_position().y - (overview_origin.y + 94)) < .5f,
            "The first effective value keeps the reference 43px header,27px columns and23px section heading");
        const auto before = model->definition(); const auto undo = model->undoCount();
        click("Reset Output bus");
        require(row("output").state == "inherited" && !row("output").can_reset && model->undoCount() == undo + 1,
            "Actual guide Reset preserves one native edit and restores inheritance");
        require(session.invoke_command("undo", "", "", 0) && model->definition() == before,
            "Undo restores the exact native property definition after the repositioned Reset action");
        window->hide(); std::cout << "PASS: " << checks << " composed native visual utility checks\n";
    } catch (const std::exception& e) { std::cerr << "FAIL: " << e.what() << '\n'; return 1; }
}
