#include "SlintEditorDialogs.h"
#include <slint-platform.h>
#include <slint-testing.h>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool value, const char* message) {
        ++checks; if (!value) throw std::runtime_error(message);
    };
    try {
        slint::testing::init();
        auto window = dandrum_ui::SlintEditorDialogs::create();
        window->show();
        const auto settle = [&] {
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop();
        };
        const auto key = [&](slint::SharedString value) {
            window->window().dispatch_key_press_event(value);
            window->window().dispatch_key_release_event(value); settle();
        };
        const auto point = [&](const char* label) {
            settle(); auto items = slint::testing::ElementHandle::find_by_accessible_label(window, label);
            if (items.empty()) throw std::runtime_error(std::string("Missing: ") + label);
            auto item = items[items.size() - 1]; auto p = item.absolute_position(); auto size = item.size();
            return slint::LogicalPosition{{p.x + size.width / 2, p.y + size.height / 2}};
        };
        const auto at = [&](slint::LogicalPosition p, slint::PointerEventButton button) {
            window->window().dispatch_pointer_press_event(p, button);
            window->window().dispatch_pointer_release_event(p, button); settle();
        };
        const auto click = [&](const char* label) { at(point(label), slint::PointerEventButton::Left); };
        click("Dialog opener"); key("a");
        require(window->get_text() == "a", "Caller-supplied initial focus receives text");
        for (int i = 0; i < 12; ++i) key("\t");
        key("b"); require(window->get_text() == "b", "Tab cycles inside the modal");
        for (int i = 0; i < 3; ++i) {
            window->window().dispatch_key_press_event(slint::SharedString("\x10"));
            key("\t"); window->window().dispatch_key_release_event(slint::SharedString("\x10"));
        }
        key("c"); require(window->get_text() == "c", "Reverse Tab stays inside the modal");
        at(slint::LogicalPosition{{120, 240}}, slint::PointerEventButton::Left);
        require(window->get_dismissals() == 0, "Blank dialog content does not dismiss the modal");
        click("Open child dialog"); key("\x1b");
        require(window->get_child_dismissals() == 1 && window->get_dismissals() == 0,
                "Escape dismisses only the active nested modal");
        key(" "); key("\x1b");
        require(window->get_child_dismissals() == 2, "Nested close restores its opener focus");
        key("\x1b"); require(window->get_dismissals() == 1, "Escape reaches the parent after child close");
        key(" "); key("\x1b");
        require(window->get_dismissals() == 2, "Closing restores the original opener");
        window->set_dismiss_on_background(false); click("Dialog opener");
        at(slint::LogicalPosition{{18, 65}}, slint::PointerEventButton::Left);
        require(window->get_dismissals() == 2 && window->get_background_clicks() == 0,
                "A guarded backdrop neither dismisses nor activates the application behind it");
        click("Close shared dialog"); window->set_dismiss_on_background(true); click("Dialog opener");
        at(slint::LogicalPosition{{18, 65}}, slint::PointerEventButton::Left);
        require(window->get_dismissals() == 3, "An enabled backdrop requests dismissal");
        settle(); click("Shared drag source");
        require(window->get_source_clicks() == 1, "Click without drag activates the source");
        at(point("Shared drag source"), slint::PointerEventButton::Right);
        require(window->get_source_contexts() == 1, "Right click exposes the source context callback");
        const auto& state = window->global<dandrum_ui::EditorDragState>();
        const auto drag = [&](const char* kind) {
            window->set_drag_kind(kind);
            auto source = point("Shared drag source");
            window->window().dispatch_pointer_press_event(source, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_move_event(slint::LogicalPosition{{source.x + 3, source.y}}); settle();
            require(!state.get_active(), "Movement below the drag threshold stays a click");
            window->window().dispatch_pointer_move_event(slint::LogicalPosition{{290, 280}}); settle();
            require(state.get_active(), "A real drag exposes its payload through shared state");
            window->window().dispatch_pointer_release_event(slint::LogicalPosition{{290, 280}}, slint::PointerEventButton::Left); settle();
        };
        drag("item"); require(window->get_drops() == 1 && window->get_dropped_id() == "source-a" && state.get_handled(), "A matching target receives the exact payload once");
        drag("alternate"); require(window->get_drops() == 2, "An additional caller-supplied kind is accepted");
        drag("other"); require(window->get_drops() == 2 && !state.get_handled(), "Incompatible kinds stay unhandled");
        window->set_target_enabled(false); drag("item"); require(window->get_drops() == 2, "Disabled targets emit no drop");
        window->set_target_enabled(true); window->set_target_visible(false); drag("item"); require(window->get_drops() == 2, "Hidden targets emit no drop");
        window->set_target_visible(true); window->set_accept_drop(false); drag("item"); require(window->get_drops() == 3 && !state.get_handled(), "Rejected drops remain available to another target");
        state.invoke_finish(); settle(); require(window->get_drops() == 3, "Finishing an inactive drag emits no duplicate");
        state.invoke_begin("item", "external", 290, 280); settle(); require(window->get_drag_over(), "External sources expose compatible drag-over state");
        state.invoke_move(450, 330); settle(); require(!window->get_drag_over(), "Moving outside the target clears drag-over");
        state.invoke_cancel(); settle(); require(!state.get_active() && state.get_id().size() == 0, "Cancellation clears active payload ownership");
        auto source = point("Shared drag source");
        window->window().dispatch_window_active_changed_event(true); settle();
        window->window().dispatch_pointer_press_event(source, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_move_event(slint::LogicalPosition{{290, 280}}); settle();
        require(state.get_active(), "The source owns an active drag before native deactivation");
        window->window().dispatch_window_active_changed_event(false); settle();
        require(!state.get_active() && state.get_id().size() == 0,
                "Native window deactivation cancels the active source payload");
        window->window().dispatch_window_active_changed_event(true); settle();
        const auto dropsBeforeLoss = window->get_drops();
        window->window().dispatch_pointer_move_event(slint::LogicalPosition{{290, 280}});
        window->window().dispatch_pointer_release_event(slint::LogicalPosition{{290, 280}}, slint::PointerEventButton::Left); settle();
        require(!state.get_active() && window->get_drops() == dropsBeforeLoss,
                "Returning to the window cannot complete a cancelled drag");
        const auto clicksBeforeLoss = window->get_source_clicks();
        window->window().dispatch_pointer_press_event(source, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_move_event(slint::LogicalPosition{{source.x + 3, source.y}}); settle();
        window->window().dispatch_window_active_changed_event(false); settle();
        window->window().dispatch_window_active_changed_event(true); settle();
        window->window().dispatch_pointer_move_event(slint::LogicalPosition{{290, 280}}); settle();
        require(!state.get_active(), "A press interrupted below the threshold cannot arm a later drag");
        window->window().dispatch_pointer_release_event(slint::LogicalPosition{{290, 280}}, slint::PointerEventButton::Left); settle();
        require(window->get_source_clicks() == clicksBeforeLoss && window->get_drops() == dropsBeforeLoss,
                "An interrupted source press cannot activate or drop after reactivation");
        click("Shared drag source");
        require(window->get_source_clicks() == clicksBeforeLoss + 1,
                "A fresh source press still activates after window deactivation");
        window->window().dispatch_pointer_press_event(source, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_move_event(slint::LogicalPosition{{290, 280}}); settle();
        window->set_drag_enabled(false); settle();
        require(!state.get_active(), "Disabling the source cancels its active drag");
        window->window().dispatch_pointer_release_event(slint::LogicalPosition{{290, 280}}, slint::PointerEventButton::Left); settle();
        const auto clicks = window->get_source_clicks(); click("Shared drag source");
        require(window->get_source_clicks() == clicks, "Disabled sources cannot activate");
        window->hide(); std::cout << "PASS: " << checks << " standalone modal and drag checks\n";
        return 0;
    } catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
