#include "ControlInteractions.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <array>
#include <iostream>
#include <stdexcept>

int main(int argc, char** argv) {
    int checks = 0;
    const auto require = [&](bool ok, const std::string& message) {
        ++checks;
        if (!ok) throw std::runtime_error(message);
    };
    try {
        slint::testing::init();
        auto window = dandrum_ui::ControlInteractions::create();
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        window->show();
        const auto settle = [&] {
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); });
            slint::run_event_loop();
        };
        const auto element = [&](const char* label) {
            settle();
            for (auto item : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (item.accessible_role().has_value()) return item;
            throw std::runtime_error(std::string("Missing control: ") + label);
        };
        const auto click = [&](const char* label) {
            auto item = element(label); auto p = item.absolute_position(); auto s = item.size();
            slint::LogicalPosition centre{{p.x + s.width / 2, p.y + s.height / 2}};
            window->window().dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(centre, slint::PointerEventButton::Left); settle();
        };
        const auto key = [&](slint::SharedString text) {
            window->window().dispatch_key_press_event(text);
            window->window().dispatch_key_release_event(text); settle();
        };
        const std::string scope = argc > 1 ? argv[1] : "all";
        if (scope == "all" || scope == "focus") {
            const auto value = [&](int i) { return i == 0 ? window->get_knob_value() : i == 1 ? window->get_slider_value() : window->get_field_value(); };
            const auto editing = [&](int i) { return i == 0 ? window->get_knob_editing() : i == 1 ? window->get_slider_editing() : window->get_field_editing(); };
            const auto focused = [&](int i) { return i == 0 ? window->get_knob_focused() : i == 1 ? window->get_slider_focused() : window->get_field_focused(); };
            for (int i = 0; i < 3; ++i) {
                const float before = value(i);
                window->invoke_edit_control(i); settle(); key("9");
                click("Destination"); key(slint::platform::key_codes::UpArrow);
                require(!editing(i) && value(i) == before && !focused(i), "Blur preserves the destination focus and cancels the uncommitted value for control " + std::to_string(i));
                key("x");
                require(std::string_view(element("Destination").accessible_value()->data()).ends_with("x"), "Typing after blur reaches the clicked destination");
                window->invoke_edit_control(i); settle(); key("9"); key(slint::platform::key_codes::Tab);
                require(!editing(i) && !focused(i) && value(i) == before, "Tab leaves the value editor without reclaiming focus");
                window->invoke_edit_control(i); settle(); key("9"); key(slint::platform::key_codes::Escape);
                require(!editing(i) && focused(i) && value(i) == before, "Escape restores the source control without changing its value");
                key(slint::platform::key_codes::UpArrow);
                require(value(i) > before, "The source receives keyboard increments after explicit cancellation");
                window->invoke_edit_control(i); settle(); key("7"); key(slint::platform::key_codes::Return);
                require(!editing(i) && focused(i) && value(i) == 7, "Enter commits and restores the source control");
            }
        }
        if (scope == "all" || scope == "accessibility") {
            for (const char* label : {"Mode", "Latch"}) {
                auto control = element(label);
                require(control.accessible_checkable() == true && control.accessible_checked() == true, "Selected button exposes a checkable checked state");
                control.invoke_accessible_default_action(); settle();
                require(element(label).accessible_checked() == false, "Accessible activation updates the announced unchecked state");
            }
            for (const char* label : {"Momentary", "Momentary icon"})
                require(element(label).accessible_checkable() != true && element(label).accessible_checked() != true, "Momentary actions do not advertise toggle behavior or a checked state");
        }
        if (scope == "all" || scope == "disabled") {
            const char* bus = "Inspect output bus Main, stereo";
            click(bus); require(window->get_bus_selections() == 1, "Enabled bus selects using pointer input");
            key(slint::platform::key_codes::Return); require(window->get_bus_selections() == 2, "Enabled bus selects using keyboard input");
            element(bus).invoke_accessible_default_action(); require(window->get_bus_selections() == 3, "Enabled bus selects using accessibility input");
            element("Acknowledge clipping on Left").invoke_accessible_default_action(); require(window->get_clip_resets() == 1, "Enabled clip reset remains functional");
            window->set_bus_enabled(false); settle();
            click(bus); require(window->get_bus_selections() == 3, "Disabled bus ignores pointer input");
            element(bus).invoke_accessible_default_action(); key(slint::platform::key_codes::Return);
            require(window->get_bus_selections() == 3 && element(bus).accessible_enabled() == false, "Disabled bus rejects keyboard/accessibility activation and announces disabled state");
            click("Acknowledge clipping on Left"); element("Acknowledge clipping on Left").invoke_accessible_default_action(); key(" "); key(slint::platform::key_codes::Return);
            require(window->get_clip_resets() == 1 && element("Acknowledge clipping on Left").accessible_enabled() == false, "Disabling a bus also disables clip acknowledgement");
        }
        std::cout << "PASS: " << checks << " native control interaction checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL after " << checks << " checks: " << error.what() << '\n'; return 1;
    }
}
