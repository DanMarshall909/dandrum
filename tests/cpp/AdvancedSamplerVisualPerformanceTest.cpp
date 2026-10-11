#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <array>
#include <cmath>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

int main() {
    int checks = 0;
    try {
        slint::testing::init();
        auto window = dandrum_ui::AppWindow::create();
        auto model = std::make_shared<dandrum::sampler::Model>();
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window, model);
        dandrum::sampler::bind_performance_sources(window->global<dandrum_ui::PerformanceState>(), binding);
        dandrum::sampler::bind_patch_search(window->global<dandrum_ui::PatchFilter>());
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        const slint::ComponentWeakHandle<dandrum_ui::AppWindow> weak(window);
        window->on_resize_window([weak](float width, float height) {
            if (const auto live = weak.lock())
                (*live)->window().set_size(slint::LogicalSize{{width, height}});
        });
        window->show();
        const auto settle = [&] {
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); });
            slint::run_event_loop();
        };
        const auto require = [&](bool ok, const std::string& message) {
            ++checks;
            if (!ok) throw std::runtime_error(message);
        };
        const auto bounds = [](auto row) {
            auto p = row.absolute_position(); auto s = row.size();
            return std::array<float, 4>{p.x, p.y, s.width, s.height};
        };
        const auto type = [&](const char* name) {
            settle();
            auto rows = slint::testing::ElementHandle::find_by_element_type_name(window, name);
            require(!rows.empty(), std::string("Actual composed component exists: ") + name);
            return rows[0];
        };
        const auto within = [&](const char* label, auto parent) {
            auto area = bounds(parent);
            for (auto row : slint::testing::ElementHandle::find_by_accessible_label(window, label)) {
                auto b = bounds(row);
                if (b[0] >= area[0] - 1 && b[0] < area[0] + area[2] && b[1] >= area[1] - 1 && b[1] < area[1] + area[3]) return row;
            }
            throw std::runtime_error(std::string("Missing composed control in its page: ") + label);
        };
        for (auto size : {std::array<unsigned, 2>{820, 560}, {860, 610}, {1200, 800}, {1600, 1000}}) {
            const auto dimensions = std::to_string(size[0]) + "x" + std::to_string(size[1]);
            // Use the maintained application's size action: Min closes Play.
            // Each case starts with its reference drawer state independently.
            window->global<dandrum_ui::PerformanceState>().set_open(size[0] >= 1000);
            window->invoke_choose_size(size[0] < 1000 ? 0 : size[0] >= 1500 ? 2 : 1);
            window->window().set_size(slint::LogicalSize{{float(size[0]), float(size[1])}});
            require(model->selectState("macros"), "Macros guide state loads at " + dimensions);
            binding->refresh(); settle();
            auto page = type("MacrosPage"); auto pageBounds = bounds(page);
            auto tone = within("Tone", page); auto toneBounds = bounds(tone);
            require(toneBounds[1] < pageBounds[1] + 90, "Macro first row remains near its panel header at " + dimensions);
            auto last = within("Select Macro 8 macro", page); auto lastBounds = bounds(last);
            auto destinations = within("Add macro destination", page); auto destinationBounds = bounds(destinations);
            require(lastBounds[1] + lastBounds[3] <= destinationBounds[1], "All eight Macro cards and assignment captions precede Destinations without occlusion at " + dimensions);
            require(destinationBounds[1] < pageBounds[1] + 340, "Macro Destinations follows a compact 300px panel at " + dimensions);
            if (size[0] >= 1000) {
                auto drawer = type("PlayDrawer");
                auto white = within("Key 24 C1", drawer); auto black = within("Key 25 C♯1", drawer);
                require(std::abs(bounds(white)[1] - bounds(black)[1]) <= 1, "Drawer black and white keys share the same top edge at " + dimensions);
                auto keys = type("PlayKeys");
                const auto expected = size[0] >= 1500 ? 697.f : 589.f;
                require(std::abs(bounds(keys)[0] - expected) <= 2, "Drawer piano horizontal split matches reference at " + dimensions);
            }
            require(model->selectState("empty"), "Empty guide state loads at " + dimensions);
            binding->refresh(); settle();
            auto empty = type("EmptyPage"); auto browse = within("Browse samples", empty);
            require(bounds(browse)[1] < bounds(empty)[1] + bounds(empty)[3], "Empty Browse remains inside its invitation canvas at " + dimensions);
            if (size[0] >= 1500) {
                auto map = type("ReadOnlyKeyMap");
                auto overviewKey = within("Key 24 C1", map);
                const auto b = bounds(overviewKey);
                const slint::LogicalPosition position{{b[0] + b[2] / 2, b[1] + b[3] - 4}};
                const auto definition = model->definition();
                const auto events = model->records("event").size();
                const auto undo = model->undoCount();
                window->window().dispatch_pointer_press_event(position, slint::PointerEventButton::Left);
                window->window().dispatch_pointer_release_event(position, slint::PointerEventButton::Left);
                settle();
                require(model->definition() == definition && model->undoCount() == undo
                            && model->records("event").size() == events && model->heldNotes().empty(),
                        "Actual expanded read-only overview key pointer creates no edits, diagnostics or owned notes");
            }
            if (size[0] == 820) {
                require(model->selectState("slices"), "Native slice-list fixture loads");
                binding->refresh(); settle();
                const auto rows = window->global<dandrum_ui::Session>().get_slice_list();
                require(rows && rows->row_count() > 0, "Compact slice list projects actual native persisted rows");
                const auto row = *rows->row_data(0);
                require(model->command("asset.select", "kick-source"), "Slice preview starts from a different actual selected source");
                binding->refresh(); settle();
                const auto label = std::string("Preview slice ") + row.name.data();
                auto action = within(label.c_str(), type("SlicesPage"));
                const auto b = bounds(action);
                const slint::LogicalPosition position{{b[0] + b[2] / 2, b[1] + b[3] / 2}};
                const auto undo = model->undoCount();
                window->window().dispatch_pointer_press_event(position, slint::PointerEventButton::Left);
                window->window().dispatch_pointer_release_event(position, slint::PointerEventButton::Left);
                settle();
                require(model->text("selected-source") == row.source.data()
                            && model->text("selected-slice") == row.id.data()
                            && model->heldNotes().empty() && model->undoCount() == undo,
                        "Actual compact Slice-list preview pointer selects its persisted source and slice without MIDI ownership or Undo");
            }
            require(model->selectState("perf"), "Perform guide state loads at " + dimensions);
            binding->refresh(); settle();
            auto perform = type("PerformView"); auto octave = within("Octave down (Z)", perform);
            const auto performBounds = bounds(perform);
            require(std::abs(performBounds[1] + performBounds[3] - size[1]) <= 1,
                    "Perform surface reaches the actual window bottom without an editor-footer gap at " + dimensions);
            require(bounds(octave)[1] < bounds(perform)[1] + 32, "Perform octave toolbar stays at the top rather than the center at " + dimensions);
        }
        // State feedback must remain visible before scrolling past the waveform.
        // Cached guide peaks deliberately remain present in unavailable states.
        window->global<dandrum_ui::PerformanceState>().set_open(true);
        window->invoke_choose_size(1);
        std::vector<std::string> feedbackFailures;
        for (const std::string state : {"missing", "unsupported", "loading", "running", "failed"}) {
            try {
                require(model->selectState(state), "Operational feedback state loads: " + state);
                binding->refresh(); settle();
                const bool analysis = state == "running" || state == "failed";
                auto page = type(analysis ? "SlicesPage" : "SamplePage");
                const auto area = bounds(page);
                const auto textContaining = [&](const std::string& needle) {
                    for (const auto& text : slint::testing::ElementHandle::find_by_element_type_name(window, "Text")) {
                        const auto label = text.accessible_label();
                        const auto b = bounds(text);
                        if (label && std::string_view(label->data()).find(needle) != std::string_view::npos
                                && b[0] >= area[0] && b[0] < area[0] + area[2] && b[1] >= area[1])
                            return text;
                    }
                    throw std::runtime_error("Missing actual source-page feedback text: " + needle);
                };
                const auto visiblyInside = [&](auto control) {
                    const auto b = bounds(control);
                    return b[2] > 0 && b[3] > 0 && b[0] + b[2] / 2 < area[0] + area[2]
                        && b[1] >= area[1] && b[1] + b[3] / 2 < area[1] + area[3];
                };
                if (state == "unsupported") {
                    require(window->global<dandrum_ui::Session>().get_waveform()->row_count() > 0,
                            "Rejected unsupported replacement retains the previous source's native cached peaks");
                    require(!slint::testing::ElementHandle::find_by_accessible_label(window,"Sample could not be loaded").empty(),
                            "Rejected unsupported replacement keeps a clear visible failure banner");
                    require(slint::testing::ElementHandle::find_by_accessible_label(window,"Unsupported sample format").empty(),
                            "Rejected unsupported replacement keeps the previous source view instead of a blank unavailable waveform");
                } else if (!analysis) {
                    const std::string needle = state == "missing" ? "Sample unavailable" : "62%";
                    auto feedback = textContaining(needle);
                    require(visiblyInside(feedback), "Actual " + state + " waveform feedback is visible in its viewport despite retained cached peaks");
                    bool insideMainWave = false;
                    const auto f = bounds(feedback);
                    for (const auto& wave : slint::testing::ElementHandle::find_by_element_type_name(window, "WaveView")) {
                        const auto b = bounds(wave);
                        insideMainWave |= b[3] >= 100 && f[0] >= b[0] && f[0] + f[2] / 2 <= b[0] + b[2]
                            && f[1] >= b[1] && f[1] + f[3] / 2 <= b[1] + b[3];
                    }
                    require(insideMainWave, "Actual " + state + " feedback occupies the main waveform rather than only a global alert");
                } else {
                    auto feedback = textContaining(state == "running" ? "48%" : "failed");
                    float waveBottom = -1;
                    for (const auto& wave : slint::testing::ElementHandle::find_by_element_type_name(window, "WaveView")) {
                        const auto b = bounds(wave);
                        if (b[3] >= 100 && b[0] >= area[0] && b[0] < area[0] + area[2])
                            waveBottom = b[1] + b[3];
                    }
                    // The reference places the following Slice list below this
                    // viewport. Its clipped title is absent from Slint's testing
                    // tree; assert the visible banner follows the waveform and
                    // the visible Transients strip that occupies the next row.
                    const auto strip = bounds(textContaining("TRANSIENTS"));
                    const auto stripBottom = strip[1] + strip[3];
                    require(waveBottom >= area[1] && visiblyInside(feedback)
                                && strip[1] >= waveBottom && bounds(feedback)[1] >= stripBottom
                                && bounds(feedback)[1] <= stripBottom + 24,
                            "Actual " + state + " transient feedback is visible after the waveform and Transients strip without scrolling");
                    for (const char* label : state == "failed" ? std::vector<const char*>{"Retry", "Cancel"} : std::vector<const char*>{"Cancel"}) {
                        auto action = within(label, page);
                        require(visiblyInside(action) && bounds(action)[1] + bounds(action)[3] <= area[1] + area[3]
                                    && bounds(action)[1] >= stripBottom && bounds(action)[1] <= stripBottom + 24,
                                "Actual " + state + " " + label + " action remains visible directly after the Transients strip without scrolling");
                    }
                }
            } catch (const std::exception& error) {
                feedbackFailures.push_back(state + ": " + error.what());
            }
        }
        for (const auto& failure : feedbackFailures) std::cerr << "FEEDBACK RED: " << failure << '\n';
        require(feedbackFailures.empty(), "All five operational source feedback states are visible in the native default viewport");
        window->window().hide();
        std::cout << "PASS: " << checks << " composed visual performance geometry checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
