#include "SlintEditorParameters.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-platform.h>
#include <slint-testing.h>
#include <cmath>
#include <iostream>
#include <stdexcept>

namespace {
int checks;
void require(bool ok, const std::string& message) { ++checks; if (!ok) throw std::runtime_error(message); }
struct Fixture {
    slint::ComponentHandle<dandrum_ui::SlintEditorParameters> w = dandrum_ui::SlintEditorParameters::create();
    Fixture() { dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(w->global<dandrum_ui::ValueCodec>()); w->show(); w->window().dispatch_window_active_changed_event(true); settle(); }
    ~Fixture() { w->hide(); }
    void settle() { slint::platform::update_timers_and_animations(); slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop(); }
    void wait(int milliseconds) { slint::cbindgen_private::slint_mock_elapsed_time(milliseconds); settle(); }
    auto find(const std::string& label, slint::language::AccessibleRole role) {
        settle();
        for (auto e : slint::testing::ElementHandle::find_by_accessible_label(w, label)) if (e.accessible_role() == role && e.size().height > 0) return e;
        throw std::runtime_error("Missing control: " + label);
    }
    auto point(const std::string& label, slint::language::AccessibleRole role = slint::language::AccessibleRole::Slider) {
        auto e = find(label, role); auto p = e.absolute_position(); auto s = e.size();
        return slint::LogicalPosition{{p.x + s.width / 2, p.y + s.height * .8f}};
    }
    void clickAt(slint::LogicalPosition p, slint::PointerEventButton button = slint::PointerEventButton::Left) { w->window().dispatch_pointer_press_event(p, button); w->window().dispatch_pointer_release_event(p, button); settle(); }
    void key(slint::SharedString text) { w->window().dispatch_key_press_event(text); w->window().dispatch_key_release_event(text); settle(); }
    void type(const std::string& text) { for (auto c : text) key(slint::SharedString(std::string(1,c))); }
    void menu(const std::string& label) { clickAt(point(label), slint::PointerEventButton::Right); }
    void action(const std::string& label) { find(label, slint::language::AccessibleRole::Button).invoke_accessible_default_action(); settle(); }
    float value(int i) { return w->get_values()->row_data(i).value(); }
    bool editing(int i) { return w->get_editing()->row_data(i).value(); }
    bool focused(int i) { return w->get_focused()->row_data(i).value(); }
    void drag(slint::LogicalPosition p, float dy, bool release = true) { w->window().dispatch_pointer_press_event(p, slint::PointerEventButton::Left); w->window().dispatch_pointer_move_event(slint::LogicalPosition{{p.x, p.y + dy}}); settle(); if (release) { w->window().dispatch_pointer_release_event(slint::LogicalPosition{{p.x, p.y + dy}}, slint::PointerEventButton::Left); settle(); } }
};
void editorsAndMenus() {
    Fixture f;
    for (int i = 0; i < 2; ++i) {
        const std::string label = i == 0 ? "Amount" : "Level";
        auto initial = f.value(i);
        f.w->invoke_edit(i); f.settle(); f.type("7"); f.key(slint::platform::key_codes::Return);
        require(f.value(i) == 7 && !f.editing(i) && f.focused(i), "Accepted editor commits and restores its control");
        f.w->invoke_edit(i); f.settle(); f.type("invalid"); f.key(slint::platform::key_codes::Return);
        require(f.editing(i) && f.w->get_invalid()->row_data(i).value() && f.value(i) == 7, "Invalid text stays editable without a value change");
        f.key(slint::platform::key_codes::Escape);
        require(!f.editing(i) && f.focused(i) && f.value(i) == 7, "Escape cancels invalid text and restores focus");
        f.w->invoke_edit(i); f.settle(); f.type("9");
        f.clickAt(f.point("Destination", slint::language::AccessibleRole::TextInput)); f.type("x");
        require(!f.editing(i) && !f.focused(i) && f.value(i) == 7, "Blur abandons text without reclaiming destination focus");
        require(f.find("Destination", slint::language::AccessibleRole::TextInput).accessible_value()->ends_with("x"), "Destination receives typing after blur");
        f.menu(label); f.action("Reset " + label + " to default");
        require(f.value(i) == (i == 0 ? 35 : -6), "Standard menu reset preserves the declared default");
        f.menu(label); f.action("Type " + label + " value"); f.type("4"); f.key(slint::platform::key_codes::Return);
        require(f.value(i) == 4, "Standard menu opens the working native value editor");
        for (auto [name, id] : {std::pair{"Add route ", "modulation-add"}, {"Edit route ", "modulation-edit"}, {"Invert route ", "modulation-invert"}, {"Bypass route ", "modulation-bypass"}, {"Remove route ", "modulation-remove"}, {"Learn MIDI ", "midi-learn"}, {"Map ", "macro-map"}}) {
            f.menu(label); if (std::string(id).starts_with("modulation-")) f.action("Modulation " + label);
            f.action(std::string(name) + label + (std::string(id) == "macro-map" ? " to macro" : ""));
            require(f.w->get_action() == slint::SharedString(id) && f.w->get_target() == slint::SharedString(i == 0 ? "knob" : "field"), "Generic menu forwards the selected action to its owner");
        }
        f.clickAt(f.point(label)); f.key("m");
        require(f.w->get_action() == slint::SharedString("modulation"), "Context keyboard shortcut emits presentation action");
        f.key(slint::platform::key_codes::Menu); f.action("Reset " + label + " to default");
        f.w->set_assignment_enabled(false); f.menu(label);
        require(f.find("Modulation " + label, slint::language::AccessibleRole::Button).accessible_enabled() == false, "Assignment actions can be disabled independently of value editing");
        f.action("Reset " + label + " to default"); f.w->set_assignment_enabled(true);
        f.menu(label); f.key(slint::platform::key_codes::Tab); auto losses = f.w->get_lost(); f.w->window().dispatch_window_active_changed_event(false); f.settle();
        require(f.w->get_lost() > losses, "Menu window deactivation asks its host to clean up");
        f.w->window().dispatch_window_active_changed_event(true); f.action("Reset " + label + " to default");
        auto changed = f.w->get_changes(); f.w->invoke_write(i, initial); f.settle();
        require(f.value(i) == initial && f.w->get_changes() == changed, "Host value projection does not echo a user edit");
    }
    f.menu("Macro"); require(f.w->get_action() == slint::SharedString("macro") && f.w->get_target() == slint::SharedString("macro"), "Specialized context bypasses generic parameter menu");
    require(f.w->get_lost() > 0 && f.w->get_begins() == f.w->get_ends(), "Editor cleanup is host-independent and gestures remain balanced");
}
void parameterInputAndCancellation() {
    Fixture f;
    auto p = f.point("Amount"); f.drag(p, -20);
    require(f.value(0) == 30 && f.w->get_begins() == 1 && f.w->get_ends() == 1, "Editor drag uses 200 pixels per range and balanced lifecycle");
    f.w->window().dispatch_key_press_event(slint::platform::key_codes::Shift); f.drag(p, -20); f.w->window().dispatch_key_release_event(slint::platform::key_codes::Shift);
    require(f.value(0) == 31, "Editor Shift drag preserves ten percent sensitivity");
    f.clickAt(p, slint::PointerEventButton::Middle); require(f.value(0) == 35, "Middle click resets editor value");
    f.w->invoke_write(0, 20); f.w->window().dispatch_key_press_event(slint::platform::key_codes::Alt); f.clickAt(p); f.w->window().dispatch_key_release_event(slint::platform::key_codes::Alt);
    require(f.value(0) == 35, "Alt click resets editor value");
    f.w->window().dispatch_pointer_scroll_event(p, 0, 120); f.settle(); require(f.value(0) == 37, "Wheel changes editor by its range wheel step");
    f.w->window().dispatch_key_press_event(slint::platform::key_codes::Shift); f.w->window().dispatch_pointer_scroll_event(p, 0, -120); f.w->window().dispatch_key_release_event(slint::platform::key_codes::Shift); f.settle(); require(f.value(0) == 36, "Shift wheel uses fine range step");
    f.wait(700); f.clickAt(p); f.key(slint::platform::key_codes::UpArrow); require(f.value(0) == 41, "Focused editor supports keyboard nudging");
    f.drag(p, -30, false); require(f.value(0) == 56, "Cancellation test changes the in-flight value"); f.key(slint::platform::key_codes::Escape); f.w->window().dispatch_pointer_release_event(p, slint::PointerEventButton::Left);
    require(f.value(0) == 41 && f.w->get_cancels() == 1, "Escape restores the gesture start and emits cancellation");
    f.drag(p, -30, false); f.w->set_controls_enabled(false); f.settle(); f.w->window().dispatch_pointer_exit_event();
    require(f.value(0) == 41 && f.w->get_cancels() == 2, "Disabling an active control cancels and restores it");
    auto changes = f.w->get_changes(); f.clickAt(p); f.w->window().dispatch_pointer_scroll_event(p, 0, 120); f.key(slint::platform::key_codes::UpArrow); f.find("Amount", slint::language::AccessibleRole::Slider).invoke_accessible_increment_action(); f.settle();
    require(f.w->get_changes() == changes && f.find("Amount", slint::language::AccessibleRole::Slider).accessible_enabled() == false, "Disabled controls reject pointer, wheel, key and accessibility input");
    f.w->set_controls_enabled(true); f.w->set_controls_read_only(true); f.w->invoke_edit(0); f.drag(p, -20);
    require(!f.editing(0) && f.w->get_changes() == changes, "Read-only controls reject editing and scrubbing");
    f.w->set_controls_read_only(false); f.settle(); f.drag(p, -20, false); f.w->window().dispatch_window_active_changed_event(false); f.settle(); f.w->window().dispatch_pointer_exit_event();
    require(f.value(0) == 41 && f.w->get_cancels() == 3 && f.w->get_lost() > 0, "Window deactivation cancels control gesture and asks the host to clean up");
    f.w->window().dispatch_window_active_changed_event(true); f.wait(700); f.clickAt(p); f.clickAt(p); require(f.editing(0), "Editor double-click begins text entry"); f.key(slint::platform::key_codes::Escape);
}
void gestureDefaultsAndConfiguration() {
    Fixture f;
    auto original = f.point("Default gesture", slint::language::AccessibleRole::Groupbox);
    auto editor = f.point("Editor gesture", slint::language::AccessibleRole::Groupbox);
    f.clickAt(original); require(f.w->get_default_edits() == 1, "Original gesture single-click edits");
    f.clickAt(original); require(f.w->get_default_resets() == 1, "Original gesture double-click resets");
    f.clickAt(editor); require(f.w->get_sampler_edits() == 0, "Configured gesture single-click does not edit");
    f.clickAt(editor); require(f.w->get_sampler_edits() == 1 && f.w->get_sampler_resets() == 0, "Configured gesture double-click edits instead of resetting");
    f.w->window().dispatch_key_press_event(slint::platform::key_codes::Shift); f.drag(original, -40); f.drag(editor, -40); f.w->window().dispatch_key_release_event(slint::platform::key_codes::Shift);
    require(f.w->get_default_value() == 25 && f.w->get_sampler_value() == 22, "Default and configured fine sensitivities remain distinct");
    auto oldEnded = f.w->get_default_ended(); f.drag(original, -10, false); f.w->set_gestures_enabled(false); f.settle(); f.w->window().dispatch_pointer_exit_event();
    require(f.w->get_default_ended() == oldEnded + 1 && f.w->get_default_cancelled() == 0, "Default gesture retains its existing interruption completion contract");
    f.w->set_gestures_enabled(true); f.settle(); auto editedEnded = f.w->get_sampler_ended(); f.drag(editor, -10, false); f.w->set_gestures_enabled(false); f.settle(); f.w->window().dispatch_pointer_exit_event();
    require(f.w->get_sampler_cancelled() == 1 && f.w->get_sampler_ended() == editedEnded, "Configured gesture emits explicit cancellation on disable");
}
void hoverAndFieldLifecycle() {
    Fixture f;
    auto knob = f.point("Amount");
    f.w->window().dispatch_pointer_move_event(knob); f.settle(); f.wait(150);
    require(slint::testing::ElementHandle::find_by_accessible_label(f.w, "Edit Amount value").empty(), "Editor popup waits for its 300ms hover delay");
    f.w->window().dispatch_pointer_move_event(slint::LogicalPosition{{850, 280}}); f.settle(); f.wait(170);
    require(slint::testing::ElementHandle::find_by_accessible_label(f.w, "Edit Amount value").empty(), "Leaving early cancels the pending hover popup");
    f.w->window().dispatch_pointer_move_event(knob); f.settle(); f.wait(310);
    require(f.find("Edit Amount value", slint::language::AccessibleRole::TextInput).accessible_enabled() == false, "Hover displays a read-only value popup before editing");
    f.w->window().dispatch_pointer_move_event(f.point("Edit Amount value", slint::language::AccessibleRole::TextInput)); f.settle(); f.wait(310);
    require(!slint::testing::ElementHandle::find_by_accessible_label(f.w, "Edit Amount value").empty(), "Hovering the popup keeps it available after leaving its knob");
    f.clickAt(f.point("Edit Amount value", slint::language::AccessibleRole::TextInput));
    require(f.editing(0), "Clicking the popup value starts editing");
    f.type("14"); f.key(slint::platform::key_codes::Tab);
    require(!f.editing(0) && !f.focused(0) && f.value(0) == 20, "Tab abandons popup text without restoring source focus");
    f.clickAt(f.point("Destination", slint::language::AccessibleRole::TextInput)); f.w->window().dispatch_pointer_move_event(slint::LogicalPosition{{850, 280}}); f.settle(); f.wait(310);
    require(slint::testing::ElementHandle::find_by_accessible_label(f.w, "Edit Amount value").empty(), "Unfocused unhovered popup closes after its delay");
    auto field = f.point("Level"); f.drag(field, -20); require(f.value(1) == -5, "Field scrubbing uses the actual signed range and quantization");
    f.w->window().dispatch_key_press_event(slint::platform::key_codes::Shift); f.drag(field, -20); f.w->window().dispatch_key_release_event(slint::platform::key_codes::Shift);
    require(f.value(1) == -4, "Field fine drag uses the configured editor sensitivity");
    f.w->window().dispatch_pointer_scroll_event(field, 0, -120); f.settle(); require(f.value(1) == -5, "Field wheel updates its quantized signed value");
    f.drag(field, -20, false); f.key(slint::platform::key_codes::Escape); f.w->window().dispatch_pointer_release_event(field, slint::PointerEventButton::Left); f.settle();
    require(f.value(1) == -5 && f.w->get_cancels() == 1, "Field Escape cancels its in-flight gesture");
    f.drag(field, -20, false); f.w->window().dispatch_window_active_changed_event(false); f.settle(); f.w->window().dispatch_pointer_exit_event();
    require(f.value(1) == -5 && f.w->get_cancels() == 2, "Field deactivation cancels its in-flight gesture");
    f.w->window().dispatch_window_active_changed_event(true); f.wait(700); f.clickAt(f.point("Macro")); f.key(slint::platform::key_codes::Menu);
    require(f.w->get_action() == slint::SharedString("macro"), "Specialized context also handles the standard-menu key");
    f.key("m"); require(f.w->get_action() == slint::SharedString("macro"), "Specialized context also handles the direct context shortcut");
}
void guidePopupTiming() {
    Fixture f;
    f.w->window().dispatch_pointer_move_event(f.point("Guide")); f.settle();
    require(f.w->get_guide_popup(), "Guide hover retains immediate popup opening");
    f.w->window().dispatch_pointer_move_event(slint::LogicalPosition{{850, 280}}); f.settle();
    f.wait(200); require(f.w->get_guide_popup(), "Guide popup remains available during its hover transition delay");
    f.wait(60); require(!f.w->get_guide_popup(), "Guide popup closes after 250ms without hover or focus");
    f.w->invoke_guide_popup_action(true); f.settle(); require(f.w->get_guide_popup(), "Explicit guide popup opening remains available");
    f.w->invoke_guide_popup_action(false); f.settle(); require(!f.w->get_guide_popup(), "Explicit guide popup dismissal remains available");
}
void hostHighlightTiming() {
    Fixture f;
    require(!f.w->get_probe_host_active(), "Host highlight starts inactive");
    f.w->invoke_probe_write(25); f.settle();
    require(f.w->get_probe_host_active() && f.w->get_probe_value() == 25, "First host write immediately highlights the projected value");
    f.wait(390); require(f.w->get_probe_host_active(), "Host highlight lasts through its 400ms interval");
    f.wait(20); require(!f.w->get_probe_host_active(), "First host highlight expires after 400ms");
    f.w->invoke_probe_write(30); f.wait(300); f.w->invoke_probe_write(32); f.wait(200);
    require(f.w->get_probe_host_active(), "A later host write refreshes the full highlight interval");
    f.wait(210); require(!f.w->get_probe_host_active(), "Refreshed host highlight eventually expires");
    f.w->set_probe_automated(true); f.w->invoke_probe_write(40); f.wait(410);
    require(f.w->get_probe_host_active(), "Persistent host automation survives transient highlight expiry");
    f.w->set_probe_automated(false); f.settle();
    require(!f.w->get_probe_host_active(), "Ending persistent automation reveals the already expired highlight");
}
void nudgeHighlightTiming() {
    Fixture f;
    require(!f.w->get_probe_nudged(), "User highlight starts inactive");
    f.w->invoke_probe_set(21); f.settle();
    require(f.w->get_probe_nudged() && f.w->get_probe_value() == 21, "First value change immediately highlights its value");
    f.wait(590); require(f.w->get_probe_nudged(), "User highlight lasts through its 600ms interval");
    f.wait(20); require(!f.w->get_probe_nudged(), "First user highlight expires after 600ms");
    f.w->invoke_probe_nudge(); f.wait(500); f.w->invoke_probe_wheel(); f.wait(200);
    require(f.w->get_probe_nudged() && f.w->get_probe_value() == 28, "Keyboard and wheel increments refresh the user highlight");
    f.wait(410); require(!f.w->get_probe_nudged(), "Refreshed wheel highlight expires");
    f.w->invoke_probe_reset(); f.settle();
    require(f.w->get_probe_nudged() && f.w->get_probe_value() == 35, "Reset highlights the declared default");
    f.wait(610); require(!f.w->get_probe_nudged(), "Reset highlight expires");
    f.w->invoke_probe_reset(); f.w->invoke_probe_set(35); f.settle();
    require(!f.w->get_probe_nudged(), "Unchanged reset and value writes do not highlight");
}
void highlightCancellation() {
    Fixture f;
    f.w->invoke_probe_write(22); f.w->invoke_probe_nudge(); f.settle();
    require(f.w->get_probe_host_active() && f.w->get_probe_nudged(), "Independent host and user indications can overlap");
    f.w->set_probe_enabled(false); f.settle();
    require(!f.w->get_probe_host_active() && !f.w->get_probe_nudged(), "Disabling a control clears its transient indications");
    f.w->invoke_probe_nudge(); f.w->invoke_probe_wheel(); f.w->invoke_probe_reset(); f.w->invoke_probe_set(50); f.wait(700);
    require(!f.w->get_probe_nudged() && f.w->get_probe_value() == 27, "Disabled user actions cannot restart a cancelled highlight or change its value");
    f.w->set_probe_enabled(true); f.settle(); f.w->invoke_probe_write(30); f.w->invoke_probe_reset(); f.w->set_probe_read_only(true); f.settle();
    require(!f.w->get_probe_host_active() && !f.w->get_probe_nudged(), "Read-only transition also clears transient indications");
    f.w->set_probe_automated(true); f.w->set_probe_enabled(false); f.settle();
    require(f.w->get_probe_host_active(), "Disabling a control preserves its persistent automation state");
    f.w->set_probe_read_only(false); f.w->set_probe_enabled(true); f.settle(); f.w->invoke_probe_set(36); f.wait(610);
    require(!f.w->get_probe_nudged(), "A re-enabled control starts and expires a fresh user highlight");
}
}
int main() {
    slint::testing::init(); int failures = 0;
    for (auto test : {editorsAndMenus, parameterInputAndCancellation, gestureDefaultsAndConfiguration, hoverAndFieldLifecycle, guidePopupTiming, hostHighlightTiming, nudgeHighlightTiming, highlightCancellation}) try { test(); } catch (const std::exception& error) { ++failures; std::cerr << "FAIL: " << error.what() << '\n'; }
    std::cout << checks << " standalone editor parameter checks; " << failures << " failed groups\n";
    return failures ? 1 : 0;
}
