#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/advanced-sampler/theme/AppearanceBinding.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <array>
#include <cmath>
#include <iostream>
#include <stdexcept>

// These checks use the composed native editor at the exact reference sizes.
// They assert visible actor geometry and resulting commands; captures prove painting separately.
int main(int argc, char** argv) {
    int checks = 0;
    auto require = [&](bool condition, const std::string& message) {
        ++checks;
        if (!condition) throw std::runtime_error(message);
    };
    try {
        using namespace dandrum_ui;
        using namespace dandrum::advanced_sampler;
        slint::testing::init();
        auto window = AppWindow::create();
        // Exercise real popup window input rather than overlay-local coordinates.
        slint::cbindgen_private::slint_testing_use_native_popup(&window->window().window_handle(), true);
        auto model = std::make_shared<dandrum::sampler::Model>();
        auto binding = dandrum::sampler::bindWindowSession<Session>(window, model);
        dandrum::sampler::bind_performance_sources(window->global<PerformanceState>(), binding);
        dandrum::sampler::bind_patch_search(window->global<PatchFilter>());
        dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>());
        ThemeSettings settings;
        apply_theme(window->global<SamplerTheme>(), resolve_theme(settings));
        const auto& session = window->global<Session>();
        const auto settle = [&] {
            slint::cbindgen_private::slint_mock_elapsed_time(20);
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); });
            slint::run_event_loop();
        };
        const auto weakWindow = slint::ComponentWeakHandle<AppWindow>(window);
        window->on_resize_window([weakWindow](float width, float height) {
            if (auto alive = weakWindow.lock())
                (*alive)->window().set_size(slint::LogicalSize{{width, height}});
        });
        auto chooseSize = [&](int size) {
            window->invoke_choose_size(size);
            window->global<PerformanceState>().set_open(size != 0);
            settle();
        };
        window->show();
        auto actor = [&](std::string_view label, slint::language::AccessibleRole role) {
            settle();
            for (auto element : slint::testing::ElementHandle::find_by_accessible_label(window, label)) {
                auto position = element.absolute_position();
                auto size = element.size();
                if (element.accessible_role() == role && size.width > 0 && size.height > 0 && position.x < window->get_viewport_width() - 230)
                    return element;
            }
            throw std::runtime_error("Missing workspace actor: " + std::string(label));
        };
        auto button = [&](std::string_view label) {
            auto element = actor(label, slint::language::AccessibleRole::Button);
            auto position = element.absolute_position(); auto size = element.size();
            window->window().dispatch_pointer_press_event(slint::LogicalPosition{{position.x+size.width/2, position.y+size.height/2}}, slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(slint::LogicalPosition{{position.x+size.width/2, position.y+size.height/2}}, slint::PointerEventButton::Left);
            settle();
        };
        auto popupButton = [&](std::string_view label) {
            const auto element = actor(label, slint::language::AccessibleRole::Button);
            // The testing backend creates a separate PopupWindow. Its elements
            // expose popup-local coordinates, so deliver physical input to that
            // element's actual window adapter, as generated Slint code does.
            auto tree = window.into_dyn();
            auto labelSlice = slint::private_api::string_to_slice(label);
            slint::SharedVector<slint::cbindgen_private::ElementHandle> matches;
            slint::cbindgen_private::slint_testing_element_find_by_accessible_label(&tree, &labelSlice, &matches);
            require(matches.size() == 1, "Popup action has one unambiguous native element");
            auto item = slint::private_api::upgrade_item_weak(matches[0].item);
            require(item.has_value(), "Popup action belongs to a live native item tree");
            slint::private_api::WindowAdapterRc adapter;
            item->item_tree.vtable()->window_adapter(item->item_tree.borrow(), true,
                reinterpret_cast<slint::cbindgen_private::Option<slint::private_api::WindowAdapterRc>*>(&adapter));
            slint::Window popup(adapter);
            const auto position = element.absolute_position(); const auto size = element.size();
            const slint::LogicalPosition centre{{position.x+size.width/2, position.y+size.height/2}};
            popup.dispatch_pointer_press_event(centre, slint::PointerEventButton::Left);
            popup.dispatch_pointer_release_event(centre, slint::PointerEventButton::Left);
            settle();
        };
        constexpr std::array<std::array<float, 2>, 3> dimensions{{{{820,560}},{{1200,800}},{{1600,1000}}}};
        auto percentSummary = [&] {
            chooseSize(1);
            require(session.invoke_command("state.select", "fx", "", 0), "FX presentation state is supported"); settle();
            auto summary = actor("30 %", slint::language::AccessibleRole::Text);
            require(summary.size().height > 0, "Normalized Saturate drive .3 renders as 30 percent");
            const auto before = model->definition(); const auto count = model->undoCount();
            require(session.invoke_command("fx.set", "sat", "drive", .5), "Typed processor drive remains normalized"); settle();
            require(actor("50 %", slint::language::AccessibleRole::Text).size().height > 0 && model->undoCount()==count+1,
                    "FX summary reacts to a real normalized processor edit");
            session.invoke_command("undo", "", "", 0); settle();
            require(model->definition()==before && actor("30 %", slint::language::AccessibleRole::Text).size().height>0,
                    "FX summary Undo restores the normalized source and its percent readout");
        };
        auto envelopeHeader = [&] {
            chooseSize(2);
            require(session.invoke_command("state.select", "voice", "", 0), "Voice presentation state is supported"); settle();
            for (const auto group : {"Amp envelope", "Filter envelope"}) {
                std::string heading(group); for (auto& c : heading) c=std::toupper(static_cast<unsigned char>(c));
                const auto title=actor(heading, slint::language::AccessibleRole::Text);
                const auto first=actor(std::string("Voice envelope shape ")+group+" ADSR", slint::language::AccessibleRole::Button);
                require(title.absolute_position().x+title.size().width+4<=first.absolute_position().x,
                        "Expanded "+std::string(group)+" tabs sit to the right of their heading");
                // ElementHandle reports the text glyph box (16px), whereas
                // the segmented control is 22px high inside the 30px header.
                require(std::abs((first.absolute_position().y+first.size().height/2)-
                                 (title.absolute_position().y+title.size().height/2))<=3 && first.size().height<=24,
                        "Expanded "+std::string(group)+" tabs align with their heading within the 30px header");
            }
            const auto before=model->definition(); const auto count=model->undoCount();
            button("Voice envelope shape Amp envelope AHDSR");
            require(model->find("modulator","amp-env")->text("shape")=="ahdsr" && model->undoCount()==count+1,
                    "Repositioned envelope tabs retain the actual modulator edit");
            session.invoke_command("undo", "", "", 0); settle();
            require(model->definition()==before, "Repositioned envelope shape Undo restores the full definition");
        };
        auto signedRouteAmount = [&] {
            chooseSize(1);
            require(session.invoke_command("state.select", "mod", "", 0), "Mod amount presentation state is supported"); settle();
            auto field=actor("Amount Filter envelope to Filter cutoff", slint::language::AccessibleRole::Slider);
            require(field.accessible_value().has_value() && *field.accessible_value()=="+42",
                    "Positive normalized modulation amount exposes its signed percent readout");
            const auto before=model->definition(); const auto count=model->undoCount();
            field.set_accessible_value("-25 %"); settle();
            require(std::abs(model->find("route","filter-route")->number("amount")+.25)<.000001 && model->undoCount()==count+1,
                    "Signed modulation editor preserves the actual normalized route amount");
            session.invoke_command("undo", "", "", 0); settle();
            require(model->definition()==before && *actor("Amount Filter envelope to Filter cutoff",slint::language::AccessibleRole::Slider).accessible_value()=="+42",
                    "Signed amount Undo restores the complete route and its readout");
        };
        auto remainingPaint = [&] {
            chooseSize(2);
            require(session.invoke_command("state.select","mod","",0), "Default modulation badge fixture loads");settle();
            auto badge=actor("Default amplitude route",slint::language::AccessibleRole::Text);
            require(badge.size().width>0 && badge.size().height>0,"Actual default route has a visible bounded badge");
            const auto before=model->definition();const auto count=model->undoCount();
            require(session.invoke_command("amp.detach","","",0),"Detach actual default amplitude route");settle();
            require(slint::testing::ElementHandle::find_by_accessible_label(window,"Default amplitude route").empty(),
                    "Default badge disappears with its actual native route");
            require(model->undoCount()==count+1 && session.invoke_command("undo","","",0),"Default-route detach has one undo entry");settle();
            require(model->definition()==before && actor("Default amplitude route",slint::language::AccessibleRole::Text).size().width>0,
                    "Undo restores the true default route and badge");
            require(session.invoke_command("state.select","vel","",0),"Velocity caption fixture loads");settle();
            const auto pages=slint::testing::ElementHandle::find_by_element_type_name(window,"LayersPage");
            require(!pages.empty(), "Actual Layers page supplies the expanded workspace origin");
            bool leftAxis=false;
            for(const auto& axis:slint::testing::ElementHandle::find_by_accessible_label(window,"1"))
                leftAxis|=axis.accessible_role()==slint::language::AccessibleRole::Text && axis.size().width<=28 &&
                          std::abs(axis.absolute_position().x-pages[0].absolute_position().x-12)<=1;
            require(leftAxis,
                    "Velocity axis low endpoint is left aligned instead of centered under the plot");
            for(const auto label:{"GAIN","PAN","TUNE","OUTPUT"})
                require(actor(label,slint::language::AccessibleRole::Text).size().height>0,
                        std::string("Visible candidate column has its semantic caption: ")+label);
            require(session.invoke_command("state.select","routing","",0),"Routing decimal fixture loads");settle();
            const auto db=actor("Reverb send Drums",slint::language::AccessibleRole::Slider).accessible_value();
            require(db.has_value() && *db=="−18.4",
                    "Live routing sends use the native one-decimal dB formatter without floating-point tails");
            const auto zero=actor("Reverb send Kick",slint::language::AccessibleRole::Slider).accessible_value();
            require(zero.has_value() && *zero=="−∞", "A genuinely zero routing send displays minus infinity");
        };
        auto captionLayout = [&](std::string_view focus) {
            for (const int mode : {0,1}) {
                chooseSize(mode);
                require(session.invoke_command("state.select", focus=="--button-caption-check" ? "voice" : "vel", "",0), "Load actual caption comparison state");settle();
                if (focus=="--button-caption-check") {
                    require(actor("Edit source",slint::language::AccessibleRole::Button).size().width>=80,
                            "Edit source retains room for its full 12px caption and leading icon at minimum/default sizes");
                    auto perform=slint::testing::ElementHandle::find_by_accessible_label(window,"Perform");
                    require(perform.size()>0 && perform[0].size().width>=70,
                            "Header Perform retains room for its complete caption and leading icon");
                } else if (focus=="--layer-caption-check") {
                    const auto name=actor("NAME",slint::language::AccessibleRole::Text);
                    // The selected AXIS tab also exposes a VELOCITY text node.
                    // Match the heading row rather than the first same-label text.
                    auto velocity=name;
                    bool foundHeading=false;
                    for (const auto& text : slint::testing::ElementHandle::find_by_accessible_label(window,"VELOCITY")) {
                        if (text.accessible_role()==slint::language::AccessibleRole::Text &&
                            std::abs(text.absolute_position().y-name.absolute_position().y)<=1) {
                            velocity=text;foundHeading=true;break;
                        }
                    }
                    require(foundHeading,"Candidate table exposes a velocity heading on its name heading row");
                    const auto first=actor("1–95",slint::language::AccessibleRole::Text);
                    require(name.absolute_position().x+name.size().width+8<=velocity.absolute_position().x,
                            "Layer name and velocity headings remain separate at minimum/default sizes");
                    require(std::abs(velocity.absolute_position().x-first.absolute_position().x)<=1,
                            "Velocity heading stays over its actual candidate values in the wider table content");
                } else {
                    const auto label=actor("Hard · 96–127",slint::language::AccessibleRole::Text);
                    const auto split=actor("Velocity range low Hard",slint::language::AccessibleRole::Button);
                    require(label.absolute_position().x>=split.absolute_position().x+4,
                            "Hard-layer dark caption stays within its light velocity range at minimum/default sizes");
                }
            }
        };
        const std::string_view presentationFocus = argc>1 ? argv[1] : "";
        if (presentationFocus=="--button-caption-check" || presentationFocus=="--layer-caption-check" || presentationFocus=="--layer-label-check") {
            captionLayout(presentationFocus);window->hide();std::cout<<"PASS: "<<checks<<" caption layout checks\n";return 0;
        }
        if (presentationFocus=="--remaining-paint-check") {remainingPaint();window->hide();std::cout<<"PASS: "<<checks<<" remaining paint checks\n";return 0;}
        if (presentationFocus=="--fx-summary-check" || presentationFocus=="--envelope-header-check" || presentationFocus=="--mod-sign-check") {
        if (presentationFocus=="--fx-summary-check") percentSummary(); else if (presentationFocus=="--envelope-header-check") envelopeHeader(); else signedRouteAmount();
            window->hide(); std::cout<<"PASS: "<<checks<<" focused native presentation checks\n";return 0;
        }
        auto roundRobinLayout = [&] {
            require(session.invoke_command("state.select", "rr", "", 0), "Round-robin reference state is supported"); settle();
            auto lock = actor("Lock takes across mics and layers", slint::language::AccessibleRole::Button);
            auto repeat = actor("No immediate repeat", slint::language::AccessibleRole::Button);
            auto restart = actor("Restart cycle", slint::language::AccessibleRole::Button);
            require(lock.absolute_position().x+lock.size().width+4 <= repeat.absolute_position().x,
                    "Round-robin take-lock and repeat controls have separate physical hit regions");
            require(restart.absolute_position().y+restart.size().height+4 <= lock.absolute_position().y,
                    "Round-robin restart and take policies occupy separate rows");
            for (auto [label, field] : {std::pair{"Lock takes across mics and layers", "take-lock"}, std::pair{"No immediate repeat", "selector-no-repeat"}}) {
                const auto before = model->definition(); const auto count = model->undoCount(); const auto value = model->number(field);
                button(label);
                require(model->number(field) == (value ? 0 : 1) && model->undoCount() == count+1,
                        std::string("Physical round-robin toggle edits its own native policy: ")+field);
                session.invoke_command("undo", "", "", 0); settle();
                require(model->definition() == before, "Round-robin toggle Undo restores the complete definition");
            }
        };
        auto freezeLayout = [&] {
            require(session.invoke_command("state.select", "freeze", "", 0), "Freeze layout state is supported"); settle();
            for (auto [caption, value] : {std::pair{std::string("Source"), std::string(session.get_selected_source())},
                                         std::pair{std::string("Includes"), std::string("Source, voice and inserts")},
                                         std::pair{std::string("Stored as"), std::string("Frozen silent mock source")}}) {
                auto label = actor(caption, slint::language::AccessibleRole::Text);
                auto content = actor(value, slint::language::AccessibleRole::Text);
                require(label.absolute_position().x+label.size().width+4 <= content.absolute_position().x,
                        "Freeze "+caption+" caption and value have separate columns");
                require(std::abs(label.absolute_position().y-content.absolute_position().y)<1 && label.size().height<=20 && content.size().height<=20,
                        "Freeze "+caption+" uses a bounded aligned property row");
            }
            const auto before = model->definition(); const auto count = model->undoCount();
            popupButton("Freeze includes Source only");
            require(model->text("freeze-mode")=="src" && model->undoCount()==count+1, "Physical Freeze mode choice persists one native edit");
            session.invoke_command("undo", "", "", 0); settle();
            require(model->definition()==before, "Freeze mode Undo restores the complete definition");
            popupButton("Cancel render dialog");
        };
        // Isolated runs retain causal layout failures even when another case fails first.
        const std::string_view focused = argc>1 ? argv[1] : "";
        for (int i=0; i<3; ++i) {
            chooseSize(i);
            if (focused != "--freeze-layout-check") roundRobinLayout();
            if (focused != "--rr-layout-check") freezeLayout();
        }
        if (!focused.empty()) {
            window->hide(); std::cout << "PASS: " << checks << " isolated native layout checks\n"; return 0;
        }
        percentSummary(); envelopeHeader(); signedRouteAmount(); remainingPaint();
        for (const auto focus:{"--button-caption-check","--layer-caption-check","--layer-label-check"}) captionLayout(focus);
        // Run confirmation cancellation before geometry checks so a clipped actor
        // cannot hide this independent runtime regression.
        chooseSize(2);
        require(session.invoke_command("state.select", "freeze", "", 0), "Freeze reference state is supported"); settle();
        auto freezeDefinition = model->definition(); auto freezeUndo = model->undoCount();
        popupButton("Cancel render dialog");
        require(model->text("freeze-state") == "idle", "Cancel closes a reference Freeze confirmation without leaving stale confirmation state");
        require(model->definition() == freezeDefinition && model->undoCount() == freezeUndo, "Cancelling Freeze confirmation preserves the complete definition and Undo count");
        require(slint::testing::ElementHandle::find_by_accessible_label(window, "Freeze includes Source only").empty(), "Cancel removes the actual Freeze confirmation controls");
        for (std::size_t sizeIndex = 0; sizeIndex < dimensions.size(); ++sizeIndex) {
            auto size = dimensions[sizeIndex];
            chooseSize(static_cast<int>(sizeIndex));
            const float workspaceBottom = size[0] == 820 ? 516 : size[0] == 1200 ? 422 : 554;
            for (auto state : {"mapping", "vel", "voice", "mod", "routing", "fx"}) {
                require(session.invoke_command("state.select", state, "", 0), "Native reference state is supported");
                settle();
                if (std::string_view(state) == "mapping") {
                    auto zone = session.get_zones()->row_data(0).value();
                    auto high = actor("Velocity high " + std::string(zone.name.data()), slint::language::AccessibleRole::Button);
                    const float targetHeight = size[0] == 820 ? 180 : size[0] == 1200 ? 300 : 420;
                    // Default/expanded drawers intentionally clip the bottom of
                    // the reference grid. Its visible interior velocity edge
                    // still proves the entire scale without requiring a hidden actor.
                    bool foundInterior=false;
                    for(std::size_t i=0;i<session.get_zones()->row_count();++i) {
                        const auto interior=*session.get_zones()->row_data(i);
                        if(interior.vel_hi<=1 || interior.vel_hi>=zone.vel_hi)continue;
                        auto edge=actor("Velocity high "+std::string(interior.name.data()),slint::language::AccessibleRole::Button);
                        const auto expected=targetHeight*(zone.vel_hi-interior.vel_hi)/127;
                        require(std::abs(edge.absolute_position().y-high.absolute_position().y-expected)<3,"Mapping preserves reference velocity grid height from visible velocity boundaries");
                        foundInterior=true;break;
                    }
                    require(foundInterior,"Mapping reference contains a visible interior velocity boundary");
                } else if (std::string_view(state) == "vel") {
                    auto candidate = session.get_candidates()->row_data(0).value();
                    auto mute = actor("Mute " + std::string(candidate.name.data()), slint::language::AccessibleRole::Button);
                    require(mute.absolute_position().y < workspaceBottom, "Layer candidate row starts inside the reference workspace");
                    require(mute.size().height <= 30, "Layer candidate controls use compact rows");
                } else if (std::string_view(state) == "voice") {
                    auto cutoff = actor("Cutoff", slint::language::AccessibleRole::Slider);
                    require(cutoff.absolute_position().y+cutoff.size().height < workspaceBottom, "Filter control remains visible at every reference resolution");
                    auto source = actor("Start offset", slint::language::AccessibleRole::Slider);
                    require(source.absolute_position().x != cutoff.absolute_position().x || size[0] == 820, "Voice source and Filter use responsive horizontal cards");
                } else if (std::string_view(state) == "mod") {
                    auto route = session.get_routes()->row_data(0).value();
                    auto amount = actor("Amount " + std::string(route.source_name.data()) + " to " + std::string(route.destination_name.data()), slint::language::AccessibleRole::Slider);
                    require(amount.absolute_position().y < 160 && amount.size().height <= 28, "Modulation starts with the compact table rather than a large editor");
                } else if (std::string_view(state) == "routing") {
                    auto output = actor("Output bus Snare", slint::language::AccessibleRole::Button);
                    auto send = actor("Reverb send Snare", slint::language::AccessibleRole::Slider);
                    require(output.absolute_position().y < workspaceBottom && std::abs(output.absolute_position().y-send.absolute_position().y) < 8, "Routing output and send actors occupy the same visible row");
                } else {
                    auto effect = actor("Select effect Drums Compressor", slint::language::AccessibleRole::Button);
                    require(effect.absolute_position().y < 180, "FX starts with the group processor chain");
                    if (size[0] == 820) {
                        bool captionVisible = false;
                        for (auto caption : slint::testing::ElementHandle::find_by_accessible_label(window, "FX busses"))
                            captionVisible |= caption.size().height > 0 && caption.absolute_position().y < workspaceBottom;
                        require(captionVisible, "Minimum FX shows the bus caption before the reference-clipped Reverb row");
                    } else {
                        auto mute = actor("Mute chain Reverb", slint::language::AccessibleRole::Button);
                        require(mute.absolute_position().y < workspaceBottom, "Reverb bus controls fit at default and expanded reference resolutions");
                    }
                }
            }
        }
        chooseSize(1);
        session.invoke_command("state.select", "routing", "", 0); settle();
        auto oldDefinition = model->definition();
        auto undo = model->undoCount();
        button("Output bus Snare"); popupButton("Set Snare output Main 1/2");
        require(model->find("tree", "snare")->text("output") == "Main 1/2", "Per-row output popup changes the actual selected node");
        require(model->undoCount() == undo+1, "Per-row output change adds one undo entry");
        session.invoke_command("undo", "", "", 0); settle();
        require(model->definition() == oldDefinition, "Output popup Undo restores the entire definition");
        auto send = actor("Reverb send Snare", slint::language::AccessibleRole::Slider);
        send.set_accessible_value("-6 dB"); settle();
        require(std::abs(model->find("send", "snare:Reverb")->number("value")-std::pow(10.,-.3)) < 0.0001, "Per-row dB editor converts to the actual linear send gain");
        session.invoke_command("undo", "", "", 0); settle();
        require(model->definition() == oldDefinition, "Send Undo restores all source routing fields");
        session.invoke_command("state.select", "fx", "", 0); settle();
        oldDefinition = model->definition();
        button("Mute chain Drums");
        require(model->find("tree", "drums")->number("muted") == 1, "Compact FX mute operates on its own group chain");
        session.invoke_command("undo", "", "", 0); settle();
        require(model->definition() == oldDefinition, "Chain mute Undo restores the complete chain definition");
        chooseSize(2);
        session.invoke_command("state.select", "voice", "", 0); settle();
        oldDefinition = model->definition();
        button("Voice envelope shape Amp envelope AHDSR");
        require(model->find("modulator", "amp-env")->text("shape") == "ahdsr", "Compact envelope shape actor edits the real modulator");
        session.invoke_command("undo", "", "", 0); settle();
        require(model->definition() == oldDefinition, "Envelope shape Undo restores the complete definition");
        session.invoke_command("state.select", "mod", "", 0); settle();
        oldDefinition = model->definition();
        std::string polarityLabel;
        for (std::size_t i=0; i<session.get_routes()->row_count(); ++i) {
            const auto route = *session.get_routes()->row_data(i);
            if (route.id == "filter-route")
                polarityLabel = "Polarity " + std::string(route.source_name) + " to " + std::string(route.destination_name);
        }
        require(!polarityLabel.empty(), "Reference Filter route exposes its native actor label");
        button(polarityLabel);
        require(model->find("route", "filter-route")->text("polarity") == "bipolar", "Compact polarity actor edits its actual route");
        session.invoke_command("undo", "", "", 0); settle();
        require(model->definition() == oldDefinition, "Route polarity Undo restores the definition");
        window->hide();
        std::cout << "PASS: " << checks << " composed native visual domain checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
