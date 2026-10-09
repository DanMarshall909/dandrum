#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto expect = [&](bool value, const std::string& message) {
        ++checks;
        if (!value) throw std::runtime_error(message);
    };
    try {
        slint::testing::init();
        auto window = dandrum_ui::AppWindow::create();
        auto model = std::make_shared<dandrum::sampler::Model>();
        model->command("patch.load", "kit");
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window, model);
        dandrum::sampler::bind_performance_sources(window->global<dandrum_ui::PerformanceState>(), binding);
        dandrum::sampler::bind_patch_search(window->global<dandrum_ui::PatchFilter>());
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        std::string copied, fileAction, filePath;
        window->on_clipboard_copy([&](slint::SharedString text) { copied = text.data(); });
        window->on_file_operation([&](slint::SharedString action, slint::SharedString path, slint::SharedString) {
            fileAction = action.data(); filePath = path.data(); return true;
        });
        window->show();
        window->window().dispatch_window_active_changed_event(true);
        window->invoke_choose_size(1);
        const auto settle = [&] {
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); });
            slint::run_event_loop();
        };
        const auto elements = [&](const std::string& label) {
            settle(); std::vector<slint::testing::ElementHandle> found;
            for (auto element : slint::testing::ElementHandle::find_by_accessible_label(window, label))
                if (element.size().width > 0 && element.size().height > 0) found.push_back(element);
            return found;
        };
        const auto element = [&](const std::string& label) {
            auto found = elements(label);
            if (found.empty()) throw std::runtime_error("Missing actual patch-browser control: " + label);
            for (auto control : found) if (control.accessible_role() == slint::language::AccessibleRole::TextInput || control.accessible_role() == slint::language::AccessibleRole::Button) return control;
            return found.back();
        };
        const auto point = [&](slint::LogicalPosition at) {
            window->window().dispatch_pointer_press_event(at, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(at, slint::PointerEventButton::Left);
            settle();
        };
        const auto centre = [](auto control) {
            auto at = control.absolute_position(); auto size = control.size();
            return slint::LogicalPosition{{at.x + size.width / 2, at.y + size.height / 2}};
        };
        const auto click = [&](const std::string& label) { point(centre(element(label))); };
        const auto key = [&](slint::SharedString text) {
            window->window().dispatch_key_press_event(text);
            window->window().dispatch_key_release_event(text); settle();
        };
        const auto edit = [&](const std::string& label, const std::string& value) {
            auto input = element(label); point(centre(input));
            expect(!elements(label).empty(), "Native editor pointer retains " + label);
            input.set_accessible_selection_offsets(0, 10000);
            key(slint::platform::key_codes::Backspace);
            for (char character : value) key(slint::SharedString(std::string(1, character)));
            expect(input.accessible_value() == slint::SharedString(value), "Actual native text entry updates " + label + "; actual=" + (input.accessible_value() ? std::string(input.accessible_value()->data()) : "<none>"));
        };
        const auto patchLabel = [&](const char* id) {
            auto patch = model->find("preset", id);
            return patch->text("name") + " · " + patch->text("category") + " · " + patch->text("contents");
        };
        const auto open = [&] { click("Browse patches"); expect(!elements("Search patches").empty(), "Header pointer opens the real patch-browser modal"); };
        const auto doubleClick = [&](const std::string& label) {
            slint::cbindgen_private::slint_mock_elapsed_time(600);
            auto at = centre(element(label));
            point(at); slint::cbindgen_private::slint_mock_elapsed_time(20); point(at);
        };

        auto undo = model->undoCount();
        open();
        edit("Search patches", "FeLt");
        expect(!elements(patchLabel("felt")).empty() && elements(patchLabel("kit")).empty(), "Modal native case-insensitive search filters actual preset rows");
        edit("Search patches", "no-such-patch");
        expect(elements(patchLabel("felt")).empty() && element("Load").accessible_enabled() == false, "Unmatched patch query disables actual Load without replacing the instrument");
        edit("Search patches", "");
        click("Keys patches");
        expect(element("Keys patches").accessible_checked() == true && !elements(patchLabel("keys")).empty() && elements(patchLabel("kit")).empty(), "Native category pointer filters actual Keys rows");
        click("All patches");
        click("Favourite Felt Upright");
        expect(model->find("preset", "felt")->number("favourite") == 1 && model->undoCount() == undo + 1, "Modal favourite pointer changes its own native preset in one undo step");
        click("Favourites patches");
        expect(!elements(patchLabel("felt")).empty() && elements(patchLabel("keys")).empty(), "Favourite category observes native favourite projection");
        click("Remove favourite Felt Upright");
        expect(model->find("preset", "felt")->number("favourite") == 0 && elements(patchLabel("felt")).empty(), "Removing favourite refreshes the active favourite category");
        click("All patches");
        click(patchLabel("keys"));
        expect(model->text("patch-id") == "kit" && element("Load").accessible_enabled() == true, "Modal row pointer selects without loading the instrument");
        click("Load");
        expect(model->text("patch-id") == "keys" && elements("Search patches").empty(), "Actual modal Load replaces the native instrument and closes browser");
        open();
        expect(element("Loaded").accessible_enabled() == false, "Current native preset exposes a disabled Loaded button");
        auto loadedGeneration = model->generation();
        click("Loaded");
        expect(model->text("patch-id") == "keys" && model->generation() == loadedGeneration && !elements("Search patches").empty(), "Actual disabled Loaded pointer preserves the instrument and open modal");
        doubleClick(patchLabel("felt"));
        expect(model->text("patch-id") == "felt" && elements("Search patches").empty(), "Actual modal row double-click loads its own native preset");
        open();
        edit("Search patches", "AmEn");
        key(slint::platform::key_codes::Return);
        expect(model->text("patch-id") == "amen" && elements("Search patches").empty(), "Actual query Return loads its matching native preset");

        open();
        click("Copy definition");
        expect(copied == model->definition() && model->text("clipboard") == copied, "Actual Copy definition emits the complete native definition to the clipboard boundary");
        click("Import patch definition");
        edit("File path", "/tmp/native-patch-import.yaml"); click("Continue");
        expect(fileAction == "patch.import" && filePath == "/tmp/native-patch-import.yaml", "Actual browser Import reaches the native file-operation boundary with its entered path");
        click("Export patch definition");
        edit("File path", "/tmp/native-patch-export.yaml"); click("Continue");
        expect(fileAction == "patch.export" && filePath == "/tmp/native-patch-export.yaml", "Actual browser Export reaches the native file-operation boundary with its entered path");
        click("Cancel");
        expect(elements("Search patches").empty(), "Actual browser Cancel closes its modal");
        open(); click("Close patch browser (Esc)");
        expect(elements("Search patches").empty(), "Actual browser close button closes its modal");
        open(); click("Search patches"); key(slint::platform::key_codes::Escape);
        expect(elements("Search patches").empty(), "Actual focused patch query Escape closes its modal");
        open(); point(slint::LogicalPosition{{50, 500}});
        expect(elements("Search patches").empty(), "Actual outside scrim pointer closes patch browser");
        open(); point(slint::LogicalPosition{{850, 380}});
        expect(!elements("Search patches").empty(), "Actual empty modal body pointer stays inside patch browser");
        click("Cancel");

        click("Perform");
        edit("Search patches", "FeLt");
        expect(!elements(patchLabel("felt")).empty() && elements(patchLabel("kit")).empty(), "Perform native search filters its compact preset rows");
        auto performPatch = model->text("patch-id"); auto performUndo = model->undoCount();
        click("Favourite Felt Upright");
        expect(model->find("preset", "felt")->number("favourite") == 1 && model->text("patch-id") == performPatch && model->undoCount() == performUndo + 1, "Perform favourite pointer updates its own native preset without loading it");
        click(patchLabel("felt"));
        expect(model->text("patch-id") == "felt", "Perform compact row pointer loads its matching native preset");
        std::cout << "PASS: " << checks << " whole-app patch-browser checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL after " << checks << " patch-browser checks: " << error.what() << '\n';
        return 1;
    }
}
