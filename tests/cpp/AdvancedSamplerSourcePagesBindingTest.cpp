#include "AdvancedSamplerSourcePages.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <chrono>
#include <array>
#include <cmath>
#include <tuple>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool ok, const char* description) {
        ++checks;
        if (!ok) throw std::runtime_error(description);
    };
    try {
        slint::testing::init();
        auto window = AdvancedSamplerSourcePagesTest::create();
        auto binding = dandrum::sampler::bindWindowSession<Session>(window);
        dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>());
        const auto& session = window->global<Session>();
        require(session.invoke_command("patch.load", "kit", "", 0), "Load real native editor preset");
        window->show();
        const auto activate = [&](const char* label) {
            auto elements = slint::testing::ElementHandle::find_by_accessible_label(window, label);
            std::vector<slint::testing::ElementHandle> controls;
            for (const auto& element : elements)
                if (element.accessible_role() == slint::language::AccessibleRole::Button) controls.push_back(element);
            require(controls.size() == 1, label);
            const auto position = controls[0].absolute_position();
            const auto size = controls[0].size();
            const slint::LogicalPosition centre{{position.x + size.width / 2, position.y + size.height / 2}};
            window->window().dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(centre, slint::PointerEventButton::Left);
            slint::cbindgen_private::slint_mock_elapsed_time(20);
            slint::platform::update_timers_and_animations();
        };
        const auto typeField = [&](const char* label, const char* text) {
            bool typed = false;
            for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (element.accessible_role() == slint::language::AccessibleRole::Slider && element.size().width > 0) {
                    element.set_accessible_value(text); typed = true;
                }
            require(typed, label);
        };
        for (const auto& [label, id, text, expected] : std::array<std::tuple<const char*, const char*, const char*, double>, 4>{{
                {"Tune", "pitch", "7 st", 7}, {"Fine", "fine", "-9 ct", -9},
                {"Gain", "source-gain", "-4 dB", -4}, {"Pan", "pan", "R25", .5}}}) {
            const double original = binding->model()->find("parameter", id)->number("value");
            const double amplifier = binding->model()->find("parameter", "level")->number("value");
            const auto undo = binding->model()->undoCount();
            typeField(label, text);
            std::cout << "Sample " << label << ": value=" << binding->model()->find("parameter", id)->number("value")
                      << ", undo delta=" << binding->model()->undoCount() - undo << '\n';
            require(std::abs(binding->model()->find("parameter", id)->number("value") - expected) < .000001
                    && binding->model()->undoCount() == undo + 1,
                    "Sample pitch and output controls commit the requested signed value in one Undo");
            require(binding->model()->find("parameter", "level")->number("value") == amplifier,
                    "Sample output editing preserves the independent amplifier Level");
            for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (element.accessible_role() == slint::language::AccessibleRole::Slider && element.size().width > 0) {
                    const auto p = element.absolute_position(); const auto size = element.size();
                    const slint::LogicalPosition at{{p.x + size.width / 2, p.y + size.height / 2}};
                    window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Middle);
                    window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Middle);
                    break;
                }
            const auto defaultValue = binding->model()->find("parameter", id)->number("default");
            require(std::abs(binding->model()->find("parameter", id)->number("value") - defaultValue) < .000001
                    && binding->model()->undoCount() == undo + 2,
                    "Actual middle-click restores each sample knob's exact default in one Undo");
            require(session.invoke_command("undo", "", "", 0)
                    && std::abs(binding->model()->find("parameter", id)->number("value") - expected) < .000001,
                    "Undo restores the signed sample value after its middle-click reset");
            require(session.invoke_command("undo", "", "", 0)
                    && binding->model()->find("parameter", id)->number("value") == original,
                    "Undo restores each sample pitch and output control");
        }
        const auto originalRoot = session.get_root_note();
        const auto rootSource = binding->model()->text("selected-source");
        const auto rootUndo = binding->model()->undoCount();
        typeField("Root note", "65");
        require(session.get_root_note() == 65 && binding->model()->find("source", rootSource)->number("root-note") == 65
                && binding->model()->undoCount() == rootUndo + 1,
                "Typed sample Root commits its MIDI note to the selected source in one Undo");
        require(session.invoke_command("undo", "", "", 0) && session.get_root_note() == originalRoot,
                "Undo restores the sample Root note");
        for (const auto& [label, id, text, expected] : std::array<std::tuple<const char*, const char*, const char*, double>, 2>{{
                {"Fade in", "fade-in", "15 ms", .015}, {"Fade out", "fade-out", "90 ms", .09}}}) {
            const auto original = binding->model()->number(id);
            const auto undo = binding->model()->undoCount();
            typeField(label, text);
            require(std::abs(binding->model()->number(id) - expected / session.get_sample_duration()) < .000001
                    && binding->model()->find("source", rootSource)->number(id) == binding->model()->number(id)
                    && binding->model()->undoCount() == undo + 1,
                    "Sample fade fields convert milliseconds to persisted normalized source geometry in one Undo");
            require(session.invoke_command("undo", "", "", 0) && binding->model()->number(id) == original,
                    "Undo restores each typed sample fade");
        }
        const auto waveClick = [&](float height, float fraction) {
            bool clicked = false;
            for (const auto& element : slint::testing::ElementHandle::find_by_element_type_name(window, "WaveView")) {
                const auto size = element.size();
                if (std::abs(size.height - height) > .01) continue;
                const auto p = element.absolute_position();
                const slint::LogicalPosition at{{p.x + size.width * fraction, p.y + size.height / 2}};
                window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Left);
                window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Left);
                clicked = true; break;
            }
            require(clicked, "Actual source waveform is reachable for pointer input");
        };
        const auto seekUndo = binding->model()->undoCount();
        waveClick(250, .37);
        require(std::abs(session.get_playhead() - .37) < .001 && binding->model()->number("audition") == 1
                && binding->model()->undoCount() == seekUndo && binding->model()->heldNotes().empty(),
                "Actual waveform click seeks to its known source position without editing or owning a note");
        require(session.invoke_command("sample.stop", "", "", 0), "Stop the transient sample preview");
        activate("Reverse sample");
        require(binding->model()->number("reversed") == 1, "Native reverse button changes the actual sample setting");
        require(session.invoke_command("undo", "", "", 0), "Sample edit is undoable");
        require(binding->model()->number("reversed") == 0, "Undo restores forward playback");
        activate("Play mode Loop");
        require(binding->model()->text("play-mode") == "loop" && session.get_loop_enabled(), "Native mode button enables looping");
        activate("Loop type Ping pong");
        require(binding->model()->text("play-mode") == "ping-pong" && session.get_loop_enabled(),
                "Native loop type preserves looping while selecting ping pong");
        const auto loopOptions = slint::testing::ElementHandle::find_by_accessible_label(window, "Play mode Loop");
        bool loopSelected = false;
        for (const auto& option : loopOptions)
            if (option.accessible_role() == slint::language::AccessibleRole::Button)
                loopSelected = option.accessible_checked().value_or(false);
        require(loopSelected, "Outer Loop mode remains visibly selected for ping pong playback");
        const auto viewUndo = binding->model()->undoCount();
        activate("Display Spectral");
        bool spectralSelected = false;
        for (const auto& option : slint::testing::ElementHandle::find_by_accessible_label(window, "Display Spectral"))
            if (option.accessible_role() == slint::language::AccessibleRole::Button)
                spectralSelected = option.accessible_checked().value_or(false);
        require(spectralSelected && binding->model()->undoCount() == viewUndo,
                "Native Spectral display visibly selects its view without editing the sample");
        activate("Display Wave");
        bool waveSelected = false;
        for (const auto& option : slint::testing::ElementHandle::find_by_accessible_label(window, "Display Wave"))
            if (option.accessible_role() == slint::language::AccessibleRole::Button)
                waveSelected = option.accessible_checked().value_or(false);
        require(waveSelected, "Native Wave display restores the waveform view");
        require(!slint::testing::ElementHandle::find_by_accessible_label(window,
                "Loop seam · end → start, ±20 ms").empty(),
                "Looping exposes a source seam preview beside the editable loop fields");
        bool crossfadeDragged = false;
        const auto crossfadeBefore = session.get_loop_crossfade();
        const auto crossfadeUndo = binding->model()->undoCount();
        for (const auto& handle : slint::testing::ElementHandle::find_by_accessible_label(window, "Loop crossfade")) {
            if (handle.accessible_role() != slint::language::AccessibleRole::Slider) continue;
            const auto at = handle.absolute_position(); const auto size = handle.size();
            const slint::LogicalPosition from{{at.x + size.width / 2, at.y + size.height / 2}};
            const slint::LogicalPosition to{{from.x - 24, from.y}};
            window->window().dispatch_pointer_press_event(from, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_move_event(to);
            window->window().dispatch_pointer_release_event(to, slint::PointerEventButton::Left);
            crossfadeDragged = session.get_loop_crossfade() > crossfadeBefore;
        }
        require(crossfadeDragged && binding->model()->undoCount() == crossfadeUndo + 1,
                "Actual waveform crossfade handle changes the loop width in one undo entry");
        require(session.invoke_command("undo", "", "", 0)
                && std::abs(session.get_loop_crossfade() - crossfadeBefore) < .000001,
                "Undo restores the loop crossfade geometry");
        require(session.invoke_parameter_gesture("region-start", 0, 0), "Begin a native source gesture");
        require(session.invoke_parameter_gesture("region-start", 1, .2), "Update a source coordinate");
        require(session.invoke_parameter_gesture("region-start", 1, .3), "Update the same continuous gesture");
        const auto before = binding->model()->undoCount();
        require(session.invoke_parameter_gesture("region-start", 2, .3), "Commit the source gesture");
        require(binding->model()->undoCount() == before + 1, "Continuous source drag records one undo entry");
        require(session.get_region_start() > .299 && session.get_region_start() < .301, "Native model projects edited source geometry");
        const auto start = slint::testing::ElementHandle::find_by_accessible_label(window, "Start");
        bool typed = false;
        for (const auto& element : start) if (element.accessible_role() == slint::language::AccessibleRole::Slider) {
            element.set_accessible_value(slint::SharedString((std::to_string(session.get_sample_duration() * .25) + " s").c_str()));
            typed = true;
        }
        require(typed && std::abs(session.get_region_start() - .25) < .001,
                "Actual region field accepts seconds and converts to normalized source geometry");
        for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, "End"))
            if (element.accessible_role() == slint::language::AccessibleRole::Slider) {
                element.set_accessible_value(slint::SharedString((std::to_string(session.get_sample_duration() * .8) + " s").c_str()));
                const auto p = element.absolute_position(); const auto size = element.size();
                const slint::LogicalPosition at{{p.x + size.width / 2, p.y + size.height / 2}};
                window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Right);
                window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Right);
            }
        const auto endUndo = binding->model()->undoCount();
        bool resetEnd = false;
        for (const auto& option : slint::testing::ElementHandle::find_by_accessible_label(window, "Reset End to default"))
            if (option.accessible_role() == slint::language::AccessibleRole::Button) {
                option.invoke_accessible_default_action(); resetEnd = true; break;
            }
        std::cout << "End context reset: activated=" << resetEnd << ", normalized end=" << session.get_region_end()
                  << ", undo delta=" << binding->model()->undoCount() - endUndo << '\n';
        require(resetEnd && std::abs(session.get_region_end() - 1) < .000001
                && binding->model()->undoCount() == endUndo + 1,
                "Actual End context reset restores the full sample boundary in one undo entry");
        require(session.invoke_command("undo", "", "", 0) && std::abs(session.get_region_end() - .8) < .000001,
                "Undo restores the typed sample end after a context reset");
        const auto priorAnalysisRoot = session.get_root_note();
        const auto priorAnalysisFine = binding->model()->find("parameter", "fine")->number("value");
        activate("Detect pitch");
        require(session.get_analysis_state() == "running", "Native pitch analysis begins asynchronously");
        binding->model()->tick(1); binding->refresh();
        const auto pitch = binding->model()->number("analysis-root");
        const auto analysisUndo = binding->model()->undoCount();
        activate("Apply analysis");
        require(session.get_root_note() == static_cast<int>(pitch) && pitch == 60
                && binding->model()->find("parameter", "fine")->number("value") == 3
                && binding->model()->find("source", rootSource)->number("root-note") == 60
                && binding->model()->undoCount() == analysisUndo + 1,
                "Native Apply installs the measured C4 root and signed fine correction in one Undo");
        require(session.invoke_command("undo", "", "", 0) && session.get_root_note() == priorAnalysisRoot
                && binding->model()->find("parameter", "fine")->number("value") == priorAnalysisFine,
                "One Undo restores both pitch analysis changes");
        require(session.invoke_command("redo", "", "", 0) && session.get_root_note() == 60
                && binding->model()->find("parameter", "fine")->number("value") == 3,
                "Redo restores both measured pitch changes");
        window->window().dispatch_window_active_changed_event(true);
        const auto regionBefore = session.get_region_start();
        const auto undoBeforeCancel = binding->model()->undoCount();
        bool regionDragged = false;
        for (const auto& handle : slint::testing::ElementHandle::find_by_accessible_label(window, "Region start"))
            if (handle.accessible_role() == slint::language::AccessibleRole::Slider) {
                const auto p = handle.absolute_position(); const auto size = handle.size();
                const slint::LogicalPosition start{{p.x + size.width / 2, p.y + size.height / 2}};
                window->window().dispatch_pointer_press_event(start, slint::PointerEventButton::Left);
                window->window().dispatch_pointer_move_event(slint::LogicalPosition{{start.x + 40, start.y}});
                regionDragged = session.get_region_start() > regionBefore + .01;
                window->window().dispatch_window_active_changed_event(false);
                window->window().dispatch_pointer_release_event(start, slint::PointerEventButton::Left);
            }
        require(regionDragged, "Actual waveform pointer drag changes the selected region");
        require(std::abs(session.get_region_start() - regionBefore) < .000001
                && binding->model()->undoCount() == undoBeforeCancel,
                "Losing window focus cancels waveform geometry without an undo entry");
        window->set_slices(true);
        slint::cbindgen_private::slint_mock_elapsed_time(20);
        slint::platform::update_timers_and_animations();
        activate("Detect transients");
        require(session.get_analysis_state() == "running", "Actual analysis control starts the mockable native workflow");
        binding->model()->tick(1);
        binding->refresh();
        require(session.get_analysis_state() == "done" && session.get_analysis_candidates()->row_count() >= 2,
                "Completed analysis projects reviewable candidates");
        const auto candidates = session.get_analysis_candidates()->row_count();
        activate("Apply");
        std::cout << "Applied " << candidates << " candidates; slices=" << session.get_slices()->row_count()
                  << ", remaining=" << session.get_analysis_candidates()->row_count() << '\n';
        require(session.get_slices()->row_count() == candidates && session.get_analysis_candidates()->row_count() == 0,
                "Actual Apply control installs candidate regions");
        activate("Clear");
        require(session.get_slices()->row_count() == 0, "Actual Clear control removes slice regions");
        activate("Even");
        require(session.get_slices()->row_count() == 8, "Actual Even control creates eight bounded regions");
        bool countTyped = false;
        for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, "Slice count"))
            if (element.accessible_role() == slint::language::AccessibleRole::Slider) {
                element.set_accessible_value("12"); countTyped = true;
            }
        require(countTyped && session.get_analysis_count() == 12, "Native slice count changes the requested division count");
        bool sensitivityTyped = false;
        for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, "Sensitivity"))
            if (element.accessible_role() == slint::language::AccessibleRole::Slider) {
                element.set_accessible_value("75%"); sensitivityTyped = true;
            }
        require(sensitivityTyped && std::abs(session.get_analysis_sensitivity() - .75) < .001,
                "Native sensitivity accepts a percentage and projects the analysis threshold");
        activate("Even");
        require(session.get_slices()->row_count() == 12, "Native Even action uses the edited count");
        const auto sliceSelectionUndo = binding->model()->undoCount();
        waveClick(230, 2.5 / 12);
        require(session.get_selected_slice() == binding->model()->records("slice")[2].id.c_str()
                && binding->model()->undoCount() == sliceSelectionUndo,
                "Actual slice waveform click selects its containing third region without a patch edit");
        const auto last = binding->model()->records("slice").back();
        require(session.invoke_command("slice.select", last.id.c_str(), "", 0), "Select the final slice for midpoint splitting");
        activate("Split");
        const auto* split = binding->model()->find("slice", last.id);
        require(split && std::abs(split->number("end") - (last.number("start") + last.number("end")) / 2) < .000001,
                "Native Split divides the selected late region at its midpoint");
        activate("Back to Voice");
        require(session.get_page() == "voice", "Source breadcrumb returns to the native Voice workspace");
        activate("Sample source editor");
        require(session.get_page() == "sample", "Source subpage controls open the sample editor");
        activate("Slices source editor");
        require(session.get_page() == "slices", "Source subpage controls open the slicing editor");
        window->hide();
        std::cout << "PASS: " << checks << " native source page binding checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
