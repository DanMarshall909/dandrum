#include "AdvancedSamplerPatchRow.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool ok, const char* message) { ++checks; if (!ok) throw std::runtime_error(message); };
    try {
        slint::testing::init(); auto window = dandrum_ui::AdvancedSamplerPatchRowTest::create(); window->show();
        const auto star = [&] {
            const auto label = window->get_favourite() ? "Remove favourite Felt Upright" : "Favourite Felt Upright";
            return slint::testing::ElementHandle::find_by_accessible_label(window, label).at(0);
        };
        const auto key = [&](slint::SharedString text) { window->window().dispatch_key_press_event(text); window->window().dispatch_key_release_event(text); };
        for (bool compact : {false, true}) {
            window->set_compact(compact);
            const auto before = window->get_favourites();
            const auto selections = window->get_selections(), loads = window->get_loads();
            star().invoke_accessible_default_action();
            require(window->get_favourite() && window->get_favourites() == before + 1, "Accessible favourite activation updates its preset");
            require(star().accessible_checkable() == true && star().accessible_checked() == true, "Favourite announces its checked state");
            auto p = star().absolute_position(); auto s = star().size(); slint::LogicalPosition at{{p.x+s.width/2,p.y+s.height/2}};
            window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Left);
            require(!window->get_favourite(), "Favourite retains pointer activation");
            key(" "); require(window->get_favourite(), "Space toggles the focused favourite");
            key(slint::platform::key_codes::Return); require(!window->get_favourite(), "Return toggles the focused favourite");
            require(window->get_loads() == loads && window->get_selections() == selections, "Favourite input never activates its containing patch row");
            auto row = slint::testing::ElementHandle::find_by_accessible_label(window, "Felt Upright · Keys · Samples").at(0);
            p = row.absolute_position(); s = row.size(); at = slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};
            window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Left);
            const auto selected = window->get_selections(), loaded = window->get_loads();
            key(slint::platform::key_codes::Tab); key(" ");
            require(window->get_favourite() && window->get_loads() == loaded && window->get_selections() == selected, "Tab reaches the favourite independently of patch selection");
            star().invoke_accessible_default_action();
        }
        std::cout << "PASS: " << checks << " native patch favourite checks\n";
    } catch (const std::exception& error) { std::cerr << "FAIL after " << checks << " checks: " << error.what() << '\n'; return 1; }
}
