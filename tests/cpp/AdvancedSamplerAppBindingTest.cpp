#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/engine/FileOperations.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/advanced-sampler/theme/AppearanceBinding.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <array>
#include <algorithm>
#include <cmath>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool ok, const std::string& description) {
        ++checks;
        if (!ok) throw std::runtime_error(description);
    };
    try {
        using namespace dandrum_ui;
        using namespace dandrum::advanced_sampler;
        slint::testing::init();
        auto window = AppWindow::create();
        auto model = std::make_shared<dandrum::sampler::Model>();
        auto binding = dandrum::sampler::bindWindowSession<Session>(window, model);
        dandrum::sampler::bind_file_operations(window, binding);
        dandrum::sampler::bind_performance_sources(window->global<PerformanceState>(), binding);
        dandrum::sampler::bind_patch_search(window->global<PatchFilter>());
        dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>());
        ThemeSettings settings;
        apply_theme(window->global<SamplerTheme>(), resolve_theme(settings));
        publish_appearance(window->global<AppearanceState>(), settings);
        const slint::ComponentWeakHandle<AppWindow> weak(window);
        window->on_resize_window([weak](float width, float height) {
            if (const auto live = weak.lock()) (*live)->window().set_size(slint::LogicalSize{{width, height}});
        });
        window->on_appearance_change([&](slint::SharedString s, slint::SharedString f, slint::SharedString a,
                slint::SharedString c, slint::SharedString m, slint::SharedString h) {
            apply_appearance(window->global<AppearanceState>(), window->global<SamplerTheme>(), settings,
                s.data(), f.data(), a.data(), c.data(), m.data(), h.data());
        });
        const auto settle = [&] {
            slint::cbindgen_private::slint_mock_elapsed_time(20);
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); });
            slint::run_event_loop();
        };
        const auto control = [&](std::string_view label) {
            const auto found = slint::testing::ElementHandle::find_by_accessible_label(window, label);
            for (const auto& element : found)
                if (element.accessible_role() == slint::language::AccessibleRole::Button && element.size().width > 0)
                    return element;
            for (const auto& element : slint::testing::ElementHandle::find_by_element_type_name(window, "FocusScope")) {
                const auto label = element.accessible_label();
                if (label && element.accessible_role() == slint::language::AccessibleRole::Button) {
                    const auto p = element.absolute_position(); const auto extent = element.size();
                    std::cerr << label->data() << " at " << p.x << ',' << p.y << " size " << extent.width << ',' << extent.height << '\n';
                }
            }
            throw std::runtime_error("Missing native button: " + std::string(label));
        };
        const auto click = [&](std::string_view label) {
            const auto element = control(label);
            element.invoke_accessible_default_action();
            settle();
        };
        const auto clickDestination = [&](const std::string& label) {
            // The native catalog is scrollable: exercise the actual wheel and
            // pointer instead of expecting an offscreen row in accessibility.
            for (int scroll = 0; scroll < 64; ++scroll) {
                for (const auto& element : slint::testing::ElementHandle::find_by_accessible_label(window, label)) {
                    const auto at = element.absolute_position();
                    const auto size = element.size();
                    if (element.accessible_role() != slint::language::AccessibleRole::Button
                            || size.width <= 0 || at.y < 145 || at.y + size.height > window->window().size().height - 12)
                        continue;
                    const slint::LogicalPosition centre{{at.x + size.width / 2, at.y + size.height / 2}};
                    window->window().dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
                    window->window().dispatch_pointer_release_event(centre, slint::PointerEventButton::Left);
                    settle();
                    return;
                }
                window->window().dispatch_pointer_scroll_event(slint::LogicalPosition{{600, 300}}, 0, -240);
                slint::cbindgen_private::slint_mock_elapsed_time(250);
                slint::platform::update_timers_and_animations();
                settle();
            }
            throw std::runtime_error("Missing scrollable native destination: " + label);
        };
        window->show();
        window->invoke_choose_size(1);
        settle();
        require(model->text("patch-id") == "empty" && model->records("source").empty(), "New app opens an empty instrument");
        require(control("Browse samples").size().width > 0, "Empty workflow exposes its actual file boundary");
        slint::DataTransfer singleDrop;
        singleDrop.set_file_paths({std::filesystem::path("/tmp/Felt C4.wav")});
        window->invoke_receive_files(window->invoke_file_paths(singleDrop)); settle();
        require(model->records("source").size() == 1 && model->records("source").front().text("path") == "/tmp/Felt C4.wav",
                "Native file-transfer boundary imports one dropped sample and preserves spaces");
        click("Undo");
        require(model->records("source").empty(), "One Undo reverses a single-file drop");
        slint::DataTransfer multiDrop;
        multiDrop.set_file_paths({std::filesystem::path("felt_C3_p.wav"), std::filesystem::path("felt_C3_f.wav")});
        window->invoke_receive_files(window->invoke_file_paths(multiDrop)); settle();
        require(model->records("source").empty(), "Multi-file drop waits for the mapping choice before importing");
        click("One per key");
        bool pathsEntered = false;
        for (const auto& input : slint::testing::ElementHandle::find_by_accessible_label(window, "File paths")) {
            input.set_accessible_value("felt_C3_p.wav\nfelt_C3_f.wav");
            pathsEntered = true;
        }
        require(pathsEntered, "Actual multi-import file entry accepts one path per line");
        click("Continue");
        require(model->records("source").size() == 2 && model->records("zone").size() == 2
                && model->records("zone").front().number("root-note") == 48
                && model->records("zone").back().number("root-note") == 49,
                "Composed file dialog applies its selected mapping mode through the production bridge");
        click("Undo");
        require(model->records("source").empty() && model->records("zone").empty(),
                "One header Undo removes the composed multi-file import");
        window->invoke_receive_files(window->invoke_file_paths(multiDrop)); settle();
        click("Cancel");
        require(model->records("source").empty() && model->undoCount() == 0,
                "Cancelling the native multi-file chooser preserves the empty patch and its history");
        for (const auto& state : dandrum::sampler::Model::stateIds()) {
            require(model->selectState(state), "Load operational fixture " + state);
            binding->refresh();
            settle();
            require(std::string(window->global<Session>().get_page().data()) == model->text("page"), "Actual app receives fixture " + state);
        }
        require(model->selectState("sample"), "Restore sample workflow"); binding->refresh(); settle();
        click("Reverse sample");
        require(model->number("reversed") == 1, "Composed sample control edits the native model");
        click("Undo");
        require(model->number("reversed") == 0, "App header undo restores source state");
        click("Redo");
        require(model->number("reversed") == 1, "App header redo restores the edited source");
        const auto route = std::find_if(model->records("route").begin(), model->records("route").end(),
                [](const auto& candidate) { return candidate.text("destination") == "cutoff"; });
        require(route != model->records("route").end(), "Source fixture includes a cutoff route");
        const auto targetRoute = *route;
        require(model->command("route.add", model->records("modulator").front().id, "level", .7),
                "Create an unrelated level route for context isolation");
        const auto unrelated = model->records("route").back();
        require(model->command("route.select", unrelated.id), "Select the unrelated route independently");
        binding->refresh(); settle();
        std::string routeLabel;
        const auto projectedRoutes = window->global<Session>().get_routes();
        for (std::size_t i = 0; i < projectedRoutes->row_count(); ++i)
            if (const auto row = projectedRoutes->row_data(i); row && row->id == targetRoute.id.c_str())
                routeLabel = std::string(row->source_name.data()) + " → " + row->destination_name.data();
        require(!routeLabel.empty(), "Native route picker has descriptive source and destination names");
        window->invoke_context("modulation-invert", "cutoff"); settle();
        require(model->find("route", unrelated.id)->number("amount") == unrelated.number("amount"),
                "Opening a parameter context preserves the unrelated selected route");
        click("Invert " + routeLabel);
        require(model->find("route", targetRoute.id)->number("amount") == -targetRoute.number("amount")
                && model->find("route", unrelated.id)->number("amount") == unrelated.number("amount"),
                "Actual route picker inverts only the route attached to its parameter");
        click("Undo");
        require(model->find("route", targetRoute.id)->number("amount") == targetRoute.number("amount"),
                "Parameter context inversion is one reversible edit");
        window->invoke_context("modulation-bypass", "cutoff"); settle();
        click("Bypass " + routeLabel);
        require(model->find("route", targetRoute.id)->number("on") != targetRoute.number("on")
                && model->find("route", unrelated.id)->number("on") == unrelated.number("on"),
                "Parameter context bypass targets its chosen route");
        click("Undo");
        window->invoke_context("modulation-remove", "cutoff"); settle();
        click("Remove " + routeLabel);
        require(!model->find("route", targetRoute.id) && model->find("route", unrelated.id),
                "Parameter context remove preserves unrelated routes");
        click("Undo");
        require(model->find("route", targetRoute.id), "Undo restores the removed modulation route");
        window->invoke_context("modulation-edit", "cutoff"); settle();
        click("Edit " + routeLabel);
        require(model->text("selected-route") == targetRoute.id && model->text("page") == "mod",
                "Parameter context edit selects the actual route before opening its editor");
        require(model->command("page.select", "", "sample"), "Return to source workflow after route editing");
        binding->refresh(); settle();
        require(model->command("fx.add", "drums", "Compressor"), "Create a typed processor destination for native pickers");
        const auto typedTarget = "fx:" + model->records("fx").back().id + ":threshold";
        binding->refresh(); settle();
        window->invoke_context("macro-destinations", "tone"); settle();
        clickDestination("Add macro destination " + typedTarget);
        require(model->find("binding", "tone:" + typedTarget),
                "Actual macro picker includes processor destinations beyond the global parameter list");
        click("Undo");
        require(!model->find("binding", "tone:" + typedTarget), "Header undo reverses the typed macro binding");
        window->invoke_context("modulator-destination", "lfo1"); settle();
        clickDestination("Add modulation destination " + typedTarget);
        require(model->records("route").back().text("source") == "lfo1"
                && model->records("route").back().text("destination") == typedTarget,
                "Actual modulator destination picker creates the requested typed route");
        click("Undo");
        window->window().dispatch_window_active_changed_event(true);
        const auto pc = control("Computer keyboard notes");
        const auto at = pc.absolute_position(); const auto extent = pc.size();
        const slint::LogicalPosition centre{{at.x + extent.width / 2, at.y + extent.height / 2}};
        window->window().dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(centre, slint::PointerEventButton::Left);
        settle();
        require(model->number("pc-keys") == 1, "Actual PC toggle enables native keyboard notes");
        window->window().dispatch_key_press_event("a"); settle();
        require(!model->heldNotes().empty(), "Actual PC press creates a held note");
        window->window().dispatch_window_active_changed_event(false); settle();
        require(model->heldNotes().empty(), "Window focus loss releases held PC-key notes");
        window->window().dispatch_window_active_changed_event(true);
        window->window().dispatch_key_release_event("a");
        click("Computer keyboard notes");
        for (const auto& entry : std::array<std::pair<const char*, const char*>, 12>{{
                {"Pads", "pads"}, {"Macros", "macros"}, {"Sample", "sample"}, {"Slices", "slices"},
                {"Mapping", "mapping"}, {"Layers", "layers"}, {"Voice", "voice"}, {"Mod", "mod"},
                {"Routing", "routing"}, {"FX", "fx"}, {"Overview", "overview"}, {"Events", "diagnostics"}}}) {
            if (std::string_view(entry.second) == "mod" || std::string_view(entry.second) == "routing"
                    || std::string_view(entry.second) == "fx" || std::string_view(entry.second) == "overview"
                    || std::string_view(entry.second) == "diagnostics") {
                window->window().dispatch_pointer_scroll_event(slint::LogicalPosition{{30, 200}}, 0, -240);
                slint::cbindgen_private::slint_mock_elapsed_time(250);
                slint::platform::update_timers_and_animations();
                settle();
            }
            click(entry.first);
            require(model->text("page") == entry.second, std::string("Composed rail opens ") + entry.first);
        }
        click("Perform"); require(model->text("page") == "perform", "Header opens shared performance workspace");
        click("Return to editor"); require(model->text("page") == "diagnostics", "Return restores prior workspace");
        click("Appearance and settings");
        require(window->global<AppearanceState>().get_opened(), "Header opens native appearance controls");
        window->global<AppearanceState>().set_opened(false); settle();
        click("Why did this play?");
        require(window->get_diagnostics_open(), "Header opens docked historical diagnostics");
        require(model->text("page") == "diagnostics", "Diagnostic dock preserves the active workspace");
        click("Why did this play?");
        require(!window->get_diagnostics_open(), "Header closes historical diagnostics");
        click("Event log");
        require(window->get_eventlog_open(), "Header opens docked adapter event log");
        click("Mark selected file missing");
        require(model->text("asset-state") == "missing", "Native event controls expose the missing-file workflow");
        click("Mark selected file unsupported");
        require(model->text("asset-state") == "unsupported", "Native event controls expose an unsupported source");
        click("Restore selected file");
        require(model->text("asset-state") == "loaded", "Native restore action returns the selected source to loaded");
        click("Toggle host automation");
        require(model->number("host-automation") == 1, "Native host input mock is independently observable");
        click("Toggle host automation");
        require(model->number("host-automation") == 0, "Native host mock can stop without changing the patch");
        click("Fail next analysis");
        require(model->number("mock-failure") == 1, "Native failure toggle prepares an asynchronous failure");
        click("Fail next analysis");
        require(model->number("mock-failure") == 0, "Native failure injection can be turned off");
        click("Clear adapter events");
        require(model->records("event").empty(), "Native event clear removes the adapter log");
        click("Event log");
        require(!window->get_eventlog_open(), "Header closes the adapter event log");
        click("About Slint");
        require(control("Close About Slint").size().width > 0, "Required Slint disclosure is reachable");
        click("Close About Slint");
        for (const int size : {0, 1, 2, 0, 2}) {
            const auto current = window->window().size();
            const auto element = control(size == 0 ? "Min" : size == 1 ? (current.width < 1000 ? "Def" : "Default") : "Exp");
            const auto at = element.absolute_position(); const auto extent = element.size();
            require(at.x >= 0 && at.x + extent.width <= current.width + .01,
                    "Composed header size actions remain inside the native window");
            const slint::LogicalPosition centre{{at.x + extent.width / 2, at.y + extent.height / 2}};
            window->window().dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(centre, slint::PointerEventButton::Left);
            settle();
            const auto actual = window->window().size();
            require(actual.width == (size == 0 ? 820u : size == 1 ? 1200u : 1600u)
                    && actual.height == (size == 0 ? 560u : size == 1 ? 800u : 1000u),
                    "Native window uses each requested editor size");
        }
        window->hide();
        std::cout << "PASS: " << checks << " composed native application checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
