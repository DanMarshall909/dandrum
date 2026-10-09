#include "AdvancedSamplerHeader.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <cmath>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool value, const char* message) {
        ++checks;
        if (!value) throw std::runtime_error(message);
    };
    try {
        slint::testing::init();
        auto window = AdvancedSamplerHeaderTest::create();
        std::string command;
        window->on_command([&](slint::SharedString action) {
            command = action.data();
            if (command == "size.min") window->window().set_size(slint::LogicalSize{{820, 39}});
            if (command == "size.default") window->window().set_size(slint::LogicalSize{{1200, 45}});
            if (command == "size.expanded") window->window().set_size(slint::LogicalSize{{1600, 45}});
        });
        const auto settle = [&] {
            slint::cbindgen_private::slint_mock_elapsed_time(20);
            slint::platform::update_timers_and_animations();
        };
        const auto button = [&](const char* label) {
            for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (element.accessible_role() == slint::language::AccessibleRole::Button && element.size().width > 0)
                    return element;
            throw std::runtime_error(std::string("Missing size button: ") + label);
        };
        const auto click = [&](const char* label) {
            const auto element = button(label);
            const auto p = element.absolute_position(); const auto size = element.size();
            require(p.x >= 0 && p.y >= 0 && p.x + size.width <= window->window().size().width + .01
                    && p.y + size.height <= window->window().size().height + .01,
                    "Editor size button fits inside the actual native header");
            const slint::LogicalPosition at{{p.x + size.width / 2, p.y + size.height / 2}};
            window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Left);
            settle();
        };
        window->show(); settle();
        click("Min");
        require(command == "size.min" && window->window().size().width == 820,
                "Native Min button enters the minimum editor layout");
        click("Def");
        require(command == "size.default" && window->window().size().width == 1200,
                "Minimum header exposes a working Default button to leave minimum layout");
        click("Exp");
        require(command == "size.expanded" && window->window().size().width == 1600,
                "Native Expanded button enters expanded layout");
        click("Min");
        click("Exp");
        require(command == "size.expanded" && window->window().size().width == 1600,
                "Minimum header can enter expanded layout directly");
        window->window().set_size(slint::LogicalSize{{820, 39}}); settle();
        for (const auto& element : slint::testing::ElementHandle::find_by_element_type_name(window, "FocusScope"))
            if (element.accessible_role() == slint::language::AccessibleRole::Button && element.size().width > 0) {
                const auto p = element.absolute_position(); const auto size = element.size();
                require(p.x >= 0 && p.y >= 0 && p.x + size.width <= 820.01 && p.y + size.height <= 39.01,
                        "Every minimum header action remains inside the native window");
            }
        window->hide();
        std::cout << "PASS: " << checks << " native header checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
