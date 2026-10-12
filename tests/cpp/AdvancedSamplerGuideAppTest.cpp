#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include <slint-testing.h>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool ok, const char* message) {
        ++checks;
        if (!ok) throw std::runtime_error(message);
    };
    try {
        slint::testing::init();
        auto window = dandrum_ui::AppWindow::create();
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
        window->on_resize_window([weak = slint::ComponentWeakHandle<dandrum_ui::AppWindow>(window)](float w, float h) {
            if (auto live = weak.lock()) (*live)->window().set_size(slint::LogicalSize{{w, h}});
        });
        const auto elements = [&](const char* label) {
            return slint::testing::ElementHandle::find_by_accessible_label(window, label);
        };
        const auto frame = [&] {
            return slint::testing::ElementHandle::find_by_element_type_name(window, "EditorFrame").at(0);
        };
        const auto activate = [&](const char* label) {
            for (auto element : elements(label)) {
                if (element.accessible_role() != slint::language::AccessibleRole::Button) continue;
                const auto at = element.absolute_position(); const auto size = element.size();
                slint::LogicalPosition centre{{at.x + size.width / 2, at.y + size.height / 2}};
                window->window().dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
                window->window().dispatch_pointer_release_event(centre, slint::PointerEventButton::Left);
                return;
            }
            throw std::runtime_error(std::string("Missing composed guide control: ") + label);
        };
        window->show();
        require(elements("Acceptance patch Expressive kit").empty(), "Ordinary editor has no guide-only controls");
        require(frame().absolute_position().y == 0 && frame().size().height == 800,
                "Ordinary editor preserves its full reference crop");
        window->global<dandrum_ui::GuidePresentation>().set_enabled(true);
        window->invoke_choose_size(1);
        require(frame().absolute_position().y > 0 && frame().size().height == 800,
                "Guide adds space above, preserving the editor's logical height");
        activate("Acceptance patch Multisampled keys");
        require(binding->model()->text("patch-id") == "keys", "Composed guide reaches the real patch command");
        const auto definition = binding->model()->definition();
        activate("20 Minimum 820×560");
        require(frame().size().width == 820 && frame().size().height == 560,
                "Composed guide resizes the editor to the selected reference dimensions");
        require(binding->model()->definition() == definition, "Guide navigation preserves the real patch definition");
        window->global<dandrum_ui::GuidePresentation>().set_enabled(false);
        window->invoke_choose_size(1);
        require(frame().absolute_position().y == 0 && frame().size().height == 800,
                "Disabling the guide restores the ordinary crop");
        window->hide();
        std::cout << "PASS: " << checks << " composed guide checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
