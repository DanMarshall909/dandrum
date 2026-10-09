#include "AdvancedSamplerImport.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <array>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool ok, const char* text) {
        ++checks;
        if (!ok) throw std::runtime_error(text);
    };
    try {
        slint::testing::init();
        auto window = AdvancedSamplerImportTest::create();
        window->show();
        window->window().dispatch_window_active_changed_event(true);
        for (const auto& choice : std::array<std::pair<const char*, const char*>, 4>{{
                {"One per key", "sequential"}, {"By root + velocity", "root-velocity"},
                {"Stack", "stack"}, {"Round robin", "round-robin"}}}) {
            bool found = false;
            for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, choice.first)) {
                if (element.accessible_role() != slint::language::AccessibleRole::Button) continue;
                found = true;
                const auto position = element.absolute_position(); const auto size = element.size();
                const slint::LogicalPosition point{{position.x + size.width / 2, position.y + size.height / 2}};
                window->window().dispatch_pointer_press_event(point, slint::PointerEventButton::Left);
                window->window().dispatch_pointer_release_event(point, slint::PointerEventButton::Left);
                require(window->get_chosen_mode() == choice.second, "Native mapping choice emits its actual import mode");
                window->set_chosen_mode("");
                window->window().dispatch_key_press_event("\n");
                require(window->get_chosen_mode() == choice.second, "Focused import choice supports Return");
                window->window().dispatch_key_release_event("\n");
                window->window().dispatch_key_press_event("\x1b");
                require(window->get_cancelled(), "Escape cancels the native import chooser");
                window->set_cancelled(false);
            }
            require(found, choice.first);
        }
        window->hide();
        std::cout << "PASS: " << checks << " native import choice checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
