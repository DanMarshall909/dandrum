#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-platform.h>
#include <slint-testing.h>
#include <iostream>
#include <stdexcept>

namespace {
int checks = 0;
void require(bool value, const std::string& message) {
    ++checks;
    if (!value) throw std::runtime_error(message);
}
struct Fixture {
    slint::ComponentHandle<dandrum_ui::AppWindow> window = dandrum_ui::AppWindow::create();
    std::shared_ptr<dandrum::sampler::Model> model = std::make_shared<dandrum::sampler::Model>();
    std::shared_ptr<dandrum::sampler::SessionBinding<dandrum_ui::Session>> binding;
    Fixture() {
        model->command("patch.load", "kit");
        binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window, model);
        dandrum::sampler::bind_performance_sources(window->global<dandrum_ui::PerformanceState>(), binding);
        dandrum::sampler::bind_patch_search(window->global<dandrum_ui::PatchFilter>());
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        window->global<dandrum_ui::Session>().invoke_command("pc.enabled", "", "", 1);
        window->show();
        window->window().dispatch_window_active_changed_event(true);
        settle();
    }
    ~Fixture() { window->hide(); }
    void settle() {
        slint::platform::update_timers_and_animations();
        slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop();
    }
    auto controls(const std::string& label) {
        settle(); return slint::testing::ElementHandle::find_by_accessible_label(window, label);
    }
    bool exists(const std::string& label) { return !controls(label).empty(); }
    auto control(const std::string& label) {
        auto matches = controls(label);
        for (auto i = matches.size(); i > 0; --i) {
            auto element = matches[i - 1];
            if (element.accessible_role() == slint::language::AccessibleRole::Button ||
                element.accessible_role() == slint::language::AccessibleRole::TextInput) return element;
        }
        throw std::runtime_error("Missing control: " + label);
    }
    void click(const std::string& label) {
        auto element = control(label); auto p = element.absolute_position(); auto s = element.size();
        slint::LogicalPosition point{{p.x + s.width / 2, p.y + s.height / 2}};
        window->window().dispatch_pointer_press_event(point, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(point, slint::PointerEventButton::Left); settle();
    }
    void key(slint::SharedString text) {
        window->window().dispatch_key_press_event(text); window->window().dispatch_key_release_event(text); settle();
    }
    void noteKeyIsBlocked() {
        window->window().dispatch_key_press_event("a"); settle();
        require(model->heldNotes().empty(), "Modal keyboard input never starts a PC note behind the dialog");
        window->window().dispatch_key_release_event("a"); settle();
    }
    void traverse() {
        for (int i = 0; i < 60; ++i) { key(slint::platform::key_codes::Tab); noteKeyIsBlocked(); }
        for (int i = 0; i < 60; ++i) { key(slint::platform::key_codes::Backtab); noteKeyIsBlocked(); }
    }
};
void browserInitialFocusAndRestore() {
    Fixture f;
    f.model->noteOn(36, 100, "before-modal"); f.binding->refresh();
    f.click("Browse patches");
    require(f.model->heldNotes().empty(), "Opening patch browser releases previously held notes");
    f.key("K");
    require(f.control("Search patches").accessible_value() == slint::SharedString("K"), "Browser opening focuses search immediately");
    f.traverse();
    f.key(slint::platform::key_codes::Escape);
    require(!f.exists("Search patches"), "Escape dismisses browser from any tab stop");
    f.key(" ");
    require(f.exists("Search patches"), "Browser dismissal restores its invoking header button");
}
void fileEscapeAndNestedRestore() {
    Fixture f;
    f.click("Browse patches"); f.click("Import patch definition");
    f.key("x");
    require(f.control("File path").accessible_value() == slint::SharedString("x"), "File dialog initially focuses path input");
    auto definition = f.model->definition();
    f.traverse();
    require(f.model->definition() == definition, "Tabbing through file dialog never edits the instrument");
    f.click("File path"); f.key(slint::platform::key_codes::Escape);
    require(!f.exists("File path") && f.exists("Search patches"), "Escape closes only the nested file dialog");
    f.key(" ");
    require(f.exists("File path"), "Nested file dismissal restores the browser Import button");
    f.click("Cancel");
    require(!f.exists("File path") && f.exists("Search patches"), "Cancel also returns to the parent browser");
}
void appearanceAboutAndContext() {
    Fixture f;
    for (auto opener : {"Appearance and settings", "About Slint"}) {
        f.click(opener); f.noteKeyIsBlocked(); f.traverse(); f.key(slint::platform::key_codes::Escape);
        f.key(" ");
        require(f.exists(std::string(opener) == "About Slint" ? "Close About Slint" : "Accent custom color"), "Dialog closure restores " + std::string(opener));
        f.key(slint::platform::key_codes::Escape);
    }
    f.click("Browse patches"); f.key(slint::platform::key_codes::Escape);
    f.window->invoke_context("macro", "tone"); f.settle();
    require(f.exists("Close macro actions"), "Macro context modal opens");
    f.traverse(); f.key(slint::platform::key_codes::Escape);
    require(!f.exists("Close macro actions"), "Escape dismisses context modal");
    f.key(" "); require(f.exists("Search patches"), "Context dismissal restores previous focus");
    f.key(slint::platform::key_codes::Escape);
    for (auto action : {"macro-destinations", "modulation-edit"}) {
        f.window->invoke_context(action, "cutoff"); f.settle();
        const auto marker = std::string(action) == "macro-destinations"
            ? "Add macro destination cutoff" : "Choose modulation route";
        require(f.exists(marker), "Context picker opens: " + std::string(action));
        f.traverse(); f.key(slint::platform::key_codes::Escape);
        require(!f.exists(marker), "Escape dismisses context picker: " + std::string(action));
        f.key(" "); require(f.exists("Search patches"), "Context picker restores previous focus");
        f.key(slint::platform::key_codes::Escape);
    }
}
void renderDialogRestore() {
    Fixture f;
    f.window->global<dandrum_ui::Session>().invoke_command("page.select", "", "voice", 0); f.settle();
    f.click("Sample patch…");
    require(f.exists("Cancel render dialog"), "Native render dialog opens");
    f.traverse(); f.key(slint::platform::key_codes::Escape);
    require(!f.exists("Cancel render dialog"), "Escape dismisses render dialog from its current control");
    f.key(" "); require(f.exists("Cancel render dialog"), "Render dialog returns focus to Sample patch button");
}
}
int main() {
    slint::testing::init(); int failures = 0;
    for (auto test : {browserInitialFocusAndRestore, fileEscapeAndNestedRestore, appearanceAboutAndContext, renderDialogRestore}) {
        try { test(); } catch (const std::exception& error) { ++failures; std::cerr << "FAIL: " << error.what() << '\n'; }
    }
    std::cout << checks << " modal focus checks; " << failures << " failed groups\n";
    return failures ? 1 : 0;
}
