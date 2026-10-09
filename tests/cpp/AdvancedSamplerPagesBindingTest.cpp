#include "AdvancedSamplerPages.h"
#include "../../ui/advanced-sampler/engine/Bindings.h"
#include "../../ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <cmath>
#include <iostream>
#include <stdexcept>

namespace {
int checks = 0;
void expect(bool condition, const char* reason)
{
    ++checks;
    if (!condition) throw std::runtime_error(reason);
}
}
int main()
{
    try {
        slint::testing::init();
        auto fixture = dandrum_ui::AdvancedSamplerPagesTest::create();
        auto model = std::make_shared<dandrum::sampler::Model>();
        model->selectState("pads");
        auto binding = dandrum::sampler::bindWindowSession<dandrum_ui::Session>(fixture, model);
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(fixture->global<dandrum_ui::ValueCodec>());
        fixture->show();
        auto element = [&](const char* label) {
            slint::platform::update_timers_and_animations();
            slint::invoke_from_event_loop([] { slint::quit_event_loop(); });
            slint::run_event_loop(slint::EventLoopMode::RunUntilQuit);
            auto elements = slint::testing::ElementHandle::find_by_accessible_label(fixture, label);
            std::vector<slint::testing::ElementHandle> controls;
            for (auto& candidate : elements) {
                const auto role = candidate.accessible_role();
                if (role == slint::language::AccessibleRole::Slider || role == slint::language::AccessibleRole::Button)
                    controls.push_back(candidate);
            }
            if (controls.size() != 1) std::cerr << "Found " << controls.size() << " controls labelled " << label << "\n";
            expect(controls.size() == 1, label);
            return controls[0];
        };
        expect(fixture->invoke_run_tests(), fixture->get_report().data());
        element("Pad 36 C2, Kick").invoke_accessible_default_action();
        expect(model->number("selected-pad") == 36 && model->heldNotes().empty(), "Pad native action selects and balances its note");
        expect(!model->records("event").empty() && model->records("event").back().text("kind") == "note-off", "Pad action reaches native note ownership");
        element("Pad bank C1").invoke_accessible_default_action();
        element("Pad 32 G♯1, empty").invoke_accessible_default_action();
        expect(model->number("selected-pad") == 32 && model->heldNotes().empty(), "Empty pad still selects without an active voice");
        element("Pad bank C2").invoke_accessible_default_action();
        element("Pad 36 C2, Kick").invoke_accessible_default_action();
        element("Show pad panel").invoke_accessible_default_action();
        auto previewEvents=model->records("event").size();
        element("Audition selected pad waveform").invoke_accessible_default_action();
        expect(model->heldNotes().empty()&&model->records("event").size()==previewEvents+2&&model->records("event").back().text("kind")=="note-off","Pad waveform audition emits native note events and balances its temporary source");
        element("Pitch ratio").set_accessible_value("1.25");
        expect(model->find("pad", "36") && std::abs(model->find("pad", "36")->number("pitch-ratio") - 1.25) < 0.001, "Pad panel native value edit updates selected pad only");
        auto undoBefore = model->undoCount();
        element("Pan").set_accessible_value("0.4");
        slint::platform::update_timers_and_animations();
        expect(model->undoCount() == undoBefore + 1, "Pad value commit adds exactly one undo step");
        model->command("undo"); binding->refresh();
        expect(std::abs(model->find("pad", "36")->number("pan")) < 0.001, "Undo restores pad pan independently");
        expect(element("Pan").accessible_value() == slint::SharedString("C"), "Native undo also refreshes the rendered parameter value");
        element("Level").set_accessible_value("-12 dB");
        expect(std::abs(model->find("pad","36")->number("gain")+12)<0.001,"Pad live level accepts an actual negative dB value");
        element("Start").set_accessible_value("0.12");
        expect(std::abs(model->find("pad","36")->number("start")-0.12)<0.001,"Pad live start updates its selected native offset at the displayed step");
        element("Pad 46 A♯2, Open Hat ×2").invoke_accessible_default_action();
        element("Variation").set_accessible_value("35%");
        expect(std::abs(model->find("pad","46")->number("variation")-0.35)<0.001,"Pad variation converts displayed percentage into normalized native value");
        element("Pad 36 C2, Kick").invoke_accessible_default_action();
        element("Mute, solo, choke collapsed").invoke_accessible_default_action();
        element("Mute").invoke_accessible_default_action();
        expect(model->find("pad","36")->number("muted")==1,"Pad mute button changes the selected native pad only");
        element("Solo").invoke_accessible_default_action();
        expect(model->find("pad","36")->number("solo")==1,"Pad solo button updates its native selection state");
        element("Pad choke group 2").invoke_accessible_default_action();
        expect(model->find("pad","36")->number("choke")==2,"Pad choke selector assigns the actual native group");
        element("Output and sends collapsed").invoke_accessible_default_action();
        element("Reverb").set_accessible_value("25%");
        expect(std::abs(model->find("pad","36")->number("send-a")-0.25)<0.001,"Pad Reverb send converts displayed percentage into its normalized native amount");
        element("Delay").set_accessible_value("40%");
        expect(std::abs(model->find("pad","36")->number("send-b")-0.4)<0.001,"Pad Delay send preserves its normalized amount independently");
        element("Pad output bus").invoke_accessible_default_action();
        element("Route pad to Drums 3/4").invoke_accessible_default_action();
        expect(model->find("pad","36")->text("output")=="drums-out","Pad output picker chooses an actual native bus ID");
        expect(model->command("route.add","velocity-mod","pad:46:gain",0.5),"Another mapped pad accepts its own modulation route");binding->refresh();
        auto padRoutes=model->records("route").size();
        auto modulationSource=element("Select LFO 1 modulator"),padPan=element("Pan");
        const auto midpoint=[](auto row){auto p=row.absolute_position();auto s=row.size();return slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};};
        auto modStart=midpoint(modulationSource),panTarget=midpoint(padPan);
        fixture->window().dispatch_pointer_press_event(modStart,slint::PointerEventButton::Left);
        fixture->window().dispatch_pointer_move_event(slint::LogicalPosition{{modStart.x+12,modStart.y}});
        fixture->window().dispatch_pointer_move_event(panTarget);
        fixture->window().dispatch_pointer_release_event(panTarget,slint::PointerEventButton::Left);
        slint::platform::update_timers_and_animations();
        expect(model->records("route").size()==padRoutes+1&&model->records("route").back().text("destination")=="pad:36:pan","Actual modulator drag creates a route to the selected pad parameter");
        element("Modulation collapsed").invoke_accessible_default_action();
        expect(slint::testing::ElementHandle::find_by_accessible_label(fixture,"Edit pad modulation lfo1 to Pan").size()==1&&slint::testing::ElementHandle::find_by_accessible_label(fixture,"Edit pad modulation velocity-mod to Level").empty(),"Pad modulation displays only routes targeting the selected note");
        element("Pad 46 A♯2, Open Hat ×2").invoke_accessible_default_action();
        expect(slint::testing::ElementHandle::find_by_accessible_label(fixture,"Edit pad modulation lfo1 to Pan").empty(),"Changing pads removes the previous pad's route rows");
        element("Edit pad modulation velocity-mod to Level").invoke_accessible_default_action();
        expect(model->text("page")=="mod"&&model->find("route",model->text("selected-route"))->text("destination")=="pad:46:gain","Pad modulation row navigates to its actual native route editor");

        model->command("page.select", "macros"); binding->refresh();
        element("Select Macro 7 macro").invoke_accessible_default_action();
        expect(model->text("selected-macro") == "macro7", "Unassigned macro card remains selectable");
        expect(element("Macro 7").accessible_enabled() == false, "Unassigned macro knob cannot emit parameter edits");
        element("Add macro destination").invoke_accessible_default_action();
        element("Assign Cutoff to Macro 7").invoke_accessible_default_action();
        expect(model->find("binding", "macro7:cutoff") && model->find("macro", "macro7")->text("destinations") == "cutoff", "Destination picker creates an actual selected-macro binding");
        expect(element("Macro 7").accessible_enabled() == true, "Assigned macro becomes a usable parameter control");
        undoBefore = model->undoCount();
        element("Range start Filter cutoff").set_accessible_value("35");
        expect(std::abs(model->find("binding", "macro7:cutoff")->number("minimum") - 0.35) < 0.001 && model->undoCount() == undoBefore + 1, "Native destination endpoint edit preserves normalized value in one undo step");
        element("Range end Filter cutoff").set_accessible_value("12");
        expect(std::abs(model->find("binding", "macro7:cutoff")->number("maximum") - 0.12) < 0.001, "Descending macro ranges remain editable");
        element("Go to Filter cutoff").invoke_accessible_default_action();
        expect(model->text("page") == "voice", "Destination navigation opens the parameter workspace");
        model->command("page.select", "macros"); binding->refresh();
        element("Select Tone macro").invoke_accessible_default_action();
        element("Tone").set_accessible_value("73");
        expect(std::abs(model->find("macro", "tone")->number("value") - 0.73) < 0.001, "Percent macro entry converts displayed 73 into normalized 0.73");
        fixture->set_actions_open(true);
        element("Reset macro value").invoke_accessible_default_action();
        expect(std::abs(model->find("macro", "tone")->number("value") - 0.56) < 0.001, "Macro context reset reaches native default");
        fixture->set_actions_open(true);
        expect(fixture->invoke_rename_macro("Brightness"), "Macro context rename accepts a meaningful name");
        expect(model->find("macro", "tone")->text("name") == "Brightness", "Macro context rename updates every shared macro view");
        fixture->set_actions_open(true);element("Rename macro").invoke_accessible_default_action();
        auto renameInputs=slint::testing::ElementHandle::find_by_accessible_label(fixture,"Macro name");
        expect(!renameInputs.empty(),"Macro rename opens its actual native text entry");
        auto renameInput=renameInputs[0];for(auto row:renameInputs)if(row.accessible_role()==slint::language::AccessibleRole::TextInput)renameInput=row;
        auto renamePosition=renameInput.absolute_position();auto renameSize=renameInput.size();
        auto renamePoint=slint::LogicalPosition{{renamePosition.x+renameSize.width/2,renamePosition.y+renameSize.height/2}};
        fixture->window().dispatch_pointer_press_event(renamePoint,slint::PointerEventButton::Left);fixture->window().dispatch_pointer_release_event(renamePoint,slint::PointerEventButton::Left);
        model->noteOn(36,100,"pc:a");binding->refresh();
        fixture->window().dispatch_window_active_changed_event(false);slint::platform::update_timers_and_animations();
        expect(model->heldNotes().empty(),"Window deactivation during macro rename balances existing native note sources");
        fixture->window().dispatch_window_active_changed_event(true);
        element("Cancel macro rename").invoke_accessible_default_action();
        expect(model->find("macro","tone")->text("name")=="Brightness","Cancelling the actual macro rename preserves its native name");
        fixture->set_actions_open(true); element("Learn macro MIDI CC").invoke_accessible_default_action();
        expect(model->text("midi-learn") == "tone", "Macro context learn waits for a native MIDI CC");
        fixture->set_actions_open(true); element("Stop macro MIDI learn").invoke_accessible_default_action();
        expect(model->text("midi-learn").empty(), "Macro context stop cancels learning");
        fixture->set_actions_open(true); element("Enable macro host automation").invoke_accessible_default_action();
        expect(model->find("macro", "tone")->number("host") == 1, "Macro context automation changes genuine native host state");
        fixture->set_actions_open(true); element("Edit macro destinations").invoke_accessible_default_action();
        expect(model->text("page") == "macros" && model->text("selected-macro") == "tone", "Macro context destination action navigates to selected macro editor");
        fixture->set_actions_open(true); element("Clear macro destinations").invoke_accessible_default_action();
        expect(model->find("macro", "tone")->text("destinations").empty(), "Macro context clear removes actual bindings");
        std::cout << fixture->get_report() << '\n';
        std::cout << "PASS: " << checks << " native sampler page checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
