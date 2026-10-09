#include "AdvancedSamplerUtilityPages.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include <slint-testing.h>
#include <iostream>
#include <stdexcept>

namespace {
int checks = 0;
void require(bool condition, const std::string& message) {
    ++checks;
    if (!condition) throw std::runtime_error(message);
}
template<class Window> void activate(const Window& window, std::string_view label) {
    const auto elements = slint::testing::ElementHandle::find_by_accessible_label(window, label);
    for (const auto& element : elements) if (element.accessible_role() == slint::language::AccessibleRole::Button) {
        const auto origin = element.absolute_position();
        const auto size = element.size();
        const slint::LogicalPosition centre{{origin.x + size.width / 2, origin.y + size.height / 2}};
        window->window().dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(centre, slint::PointerEventButton::Left);
        return;
    }
    throw std::runtime_error("Missing native action: " + std::string(label));
}
}

int main() {
    try {
        slint::testing::init();
        auto window = dandrum_ui::AdvancedSamplerUtilityPagesTest::create();
        const auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
        const auto& session = window->global<dandrum_ui::Session>();
        window->window().show();
        std::string browse;
        window->on_browse_requested([&](slint::SharedString action) { browse = action; });
        require(session.invoke_command("page.select", "empty", "", 0), "Open the native empty-state workflow");
        activate(window, "Browse samples");
        require(browse == "asset.import", "Empty single import opens the native file boundary");
        activate(window, "Load multiple samples");
        require(browse == "asset.import-many", "Empty multi import reaches the distinct native multi-file boundary");

        require(session.invoke_command("patch.load", "kit", "", 0), "Load native acceptance patch");
        require(session.invoke_command("selection.set", "kick", "", 0), "Select a sound for effective properties");
        require(session.invoke_command("page.select", "overview", "", 0), "Open effective property overview");
        require(session.get_overview()->row_count() == 17, "All thirteen guide properties and four headings reach the native scene");
        for (const auto label : {"Output bus", "Voice limit", "Steal", "Choke group", "Source type", "Fade in", "Fade out",
                "Filter cutoff", "Amp envelope", "Pitch", "Articulation", "Release sample", "Take lock"}) {
            bool found = false;
            for (std::size_t i = 0; i < session.get_overview()->row_count(); ++i)
                found |= session.get_overview()->row_data(i)->label == label;
            require(found, std::string("Real effective property present: ") + label);
        }
        require(session.invoke_command("inherit.override", "drums", "cutoff", 2222), "Create inherited group override");
        const auto effective = [&] {
            for (std::size_t i = 0; i < session.get_overview()->row_count(); ++i) {
                const auto row = session.get_overview()->row_data(i).value();
                if (row.id == "cutoff") return row;
            }
            throw std::runtime_error("Filter cutoff missing from native overview");
        };
        require(effective().value == 2222 && effective().state == "inherited", "Child shows the effective group value");
        require(session.invoke_command("inherit.override", "kick", "cutoff", 3333), "Create a local child override");
        require(effective().value == 3333 && effective().can_reset, "Local override becomes resettable in the real scene");
        activate(window, "Reset Filter cutoff");
        require(effective().value == 2222 && !effective().can_reset, "Actual Reset action restores inherited group value");
        require(session.invoke_command("undo", "", "", 0), "Overview reset remains undoable");
        require(effective().value == 3333 && effective().can_reset, "Undo restores local override and Reset control");

        require(session.invoke_command("page.select", "diagnostics", "", 0), "Open native diagnostics");
        require(session.invoke_note_on(38, 112, "utility-test"), "Trace one mapped event");
        session.invoke_note_off(38, "utility-test");
        int first_hit = -1;
        for (std::size_t i = 0; i < session.get_events()->row_count(); ++i) {
            const auto event = session.get_events()->row_data(i).value();
            if (event.kind == "note-on") first_hit = event.sequence;
        }
        require(first_hit >= 0 && session.get_diagnostics()->row_count() >= 5, "Mapped event yields a native selection trace");
        require(session.invoke_note_on(127, 100, "utility-test"), "Record an unmapped event");
        session.invoke_note_off(127, "utility-test");
        bool silent = false;
        for (std::size_t i = 0; i < session.get_diagnostics()->row_count(); ++i)
            silent |= session.get_diagnostics()->row_data(i)->state == "failed";
        require(silent, "Unmapped event explains silence");
        activate(window, "Inspect event " + std::to_string(first_hit));
        require(session.get_selected_diagnostic_event() == first_hit, "Actual historical event action selects the trace");
        require(session.get_sounding_notes()->row_count() == 0, "Inspecting history does not trigger another note");
        silent = false;
        for (std::size_t i = 0; i < session.get_diagnostics()->row_count(); ++i)
            silent |= session.get_diagnostics()->row_data(i)->state == "failed";
        require(!silent, "Historical mapped hit restores its own successful trace");
        activate(window, "Simulate adapter failure");
        require(session.get_mock_failure(), "Actual failure simulator changes native adapter behavior");
        require(session.invoke_command("analysis.start", "transients", "", 0), "Start a mock analysis job");
        binding->model()->tick(1); binding->refresh();
        require(session.get_analysis_state() == "failed", "Failure simulation causes a real native job failure");
        activate(window, "Simulate adapter failure");
        require(!session.get_mock_failure(), "Failure simulation can be disabled again");
        activate(window, "Clear events");
        require(session.get_events()->row_count() == 0 && session.get_diagnostics()->row_count() == 0,
                "Clear removes native event and diagnostic histories");
        window->window().hide();
        std::cout << "PASS: " << checks << " actual native utility page checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
