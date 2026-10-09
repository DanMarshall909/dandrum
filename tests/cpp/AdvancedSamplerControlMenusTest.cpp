#include "AdvancedSamplerControlMenus.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <cmath>
#include <iostream>
#include <stdexcept>

int main() {
    int checks=0;
    const auto expect=[&](bool ok,const char* text){++checks;if(!ok)throw std::runtime_error(text);};
    try {
        slint::testing::init();
        auto window=dandrum_ui::AdvancedSamplerControlMenusTest::create();
        auto model=std::make_shared<dandrum::sampler::Model>();
        auto binding=dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window,model);
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        window->show();
        const auto settle=[&]{slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
        const auto find=[&](const char* label,slint::language::AccessibleRole role){settle();for(auto e:slint::testing::ElementHandle::find_by_accessible_label(window,label))if(e.accessible_role()==role&&e.size().height>0)return e;throw std::runtime_error(label);};
        const auto center=[](const auto& e){auto p=e.absolute_position();auto s=e.size();return slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};};
        const auto menu=[&](const char* label){auto p=center(find(label,slint::language::AccessibleRole::Slider));window->window().dispatch_pointer_press_event(p,slint::PointerEventButton::Right);window->window().dispatch_pointer_release_event(p,slint::PointerEventButton::Right);settle();};
        const auto click=[&](const char* label){find(label,slint::language::AccessibleRole::Button).invoke_accessible_default_action();settle();};
        const auto type=[&](const char* label,const char* text){menu(label);auto action=std::string("Type ")+label+" value";click(action.c_str());find((std::string("Edit ")+label+" value").c_str(),slint::language::AccessibleRole::TextInput);for(auto c:std::string(text)){slint::SharedString key(std::string(1,c).c_str());window->window().dispatch_key_press_event(key);window->window().dispatch_key_release_event(key);}window->window().dispatch_key_press_event("\n");window->window().dispatch_key_release_event("\n");settle();};
        model->command("parameter.set","cutoff","",900);binding->refresh();auto undo=model->undoCount();
        menu("Cutoff");click("Reset Cutoff to default");expect(model->find("parameter","cutoff")->number("value")==model->find("parameter","cutoff")->number("default")&&model->undoCount()==undo+1,"Actual knob menu Reset changes its native parameter in one undoable edit");
        type("Cutoff","1200");expect(std::abs(model->find("parameter","cutoff")->number("value")-1200)<0.01,"Actual knob menu Type commits the typed Hz value to its native parameter");
        model->command("parameter.set","level","",-12);binding->refresh();undo=model->undoCount();
        menu("Level");click("Reset Level to default");expect(model->find("parameter","level")->number("value")==model->find("parameter","level")->number("default")&&model->undoCount()==undo+1,"Actual numeric menu Reset changes its native parameter in one undoable edit");
        type("Level","-9");expect(std::abs(model->find("parameter","level")->number("value")+9)<0.01,"Actual numeric menu Type commits signed dB to its native parameter");
        for(auto [text,action]:std::vector<std::pair<const char*,const char*>>{{"Add route Cutoff","modulation-add"},{"Edit route Cutoff","modulation-edit"},{"Invert route Cutoff","modulation-invert"},{"Bypass route Cutoff","modulation-bypass"},{"Remove route Cutoff","modulation-remove"},{"Learn MIDI Cutoff","midi-learn"},{"Map Cutoff to macro","macro-map"}}){menu("Cutoff");if(std::string(action).starts_with("modulation-"))click("Modulation Cutoff");click(text);expect(window->get_action()==slint::SharedString(action)&&window->get_target()==slint::SharedString("cutoff"),"Actual knob context activation forwards the requested action and owning parameter");}
        menu("Level");click("Modulation Level");click("Add route Level");expect(window->get_action()==slint::SharedString("modulation-add")&&window->get_target()==slint::SharedString("level"),"Actual numeric context activation forwards its own parameter target");
        for(auto label:{"Cutoff","Level"}){
            menu(label);click((std::string("Type ")+label+" value").c_str());
            find((std::string("Edit ")+label+" value").c_str(),slint::language::AccessibleRole::TextInput);
            window->global<dandrum_ui::Session>().invoke_command("pc.enabled","","",1);
            window->global<dandrum_ui::Session>().invoke_pc_key("a",true,false,false);
            window->global<dandrum_ui::DragState>().invoke_begin("source","kick-source",10,10);
            expect(!model->heldNotes().empty(),"Typed parameter editor shares an existing native PC note before window loss");
            window->window().dispatch_window_active_changed_event(false);settle();
            expect(model->heldNotes().empty()&&!window->global<dandrum_ui::DragState>().get_active(),"Window loss while typing a parameter releases native notes and cancels internal drag");
            window->window().dispatch_window_active_changed_event(true);
            window->window().dispatch_key_press_event("\x1b");window->window().dispatch_key_release_event("\x1b");settle();
        }
        auto macro=find("Macro 7",slint::language::AccessibleRole::Slider);auto macroAt=center(macro);
        expect(macro.accessible_enabled()==false,"Unassigned performance macro value remains disabled");
        auto macroUndo=model->undoCount();auto macroValue=model->find("macro","macro7")->number("value");
        window->window().dispatch_pointer_press_event(macroAt,slint::PointerEventButton::Right);window->window().dispatch_pointer_release_event(macroAt,slint::PointerEventButton::Right);settle();
        click("Learn macro MIDI CC");expect(model->text("midi-learn")=="macro7"&&model->undoCount()==macroUndo&&model->find("macro","macro7")->number("value")==macroValue,"Actual disabled macro value's external context action starts native learn without a parameter edit");
        window->window().dispatch_pointer_press_event(macroAt,slint::PointerEventButton::Right);window->window().dispatch_pointer_release_event(macroAt,slint::PointerEventButton::Right);settle();
        click("Stop macro MIDI learn");expect(model->text("midi-learn").empty()&&macro.accessible_enabled()==false,"Unassigned macro context stops native learn and remains inert");
        expect(model->command("patch.load","hybrid"),"Native fractional cancellation fixture loads its actual Lush module");binding->refresh();settle();
        for(const auto* label:{"Lush Detune field","Lush Detune knob"})for(const bool windowLoss:{false,true}){
            auto control=find(label,slint::language::AccessibleRole::Slider);auto start=center(control);slint::LogicalPosition end{{start.x,start.y-30}};
            const auto original=model->find("module","lush-voice")->fields;const auto cancelUndo=model->undoCount();
            window->window().dispatch_window_active_changed_event(true);
            window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);window->window().dispatch_pointer_move_event(end);settle();
            expect(model->parameterInfo("module:lush-voice:detune")->number("value")>.34,"Actual fractional field or knob drag changes the native typed parameter");
            if(windowLoss)window->window().dispatch_window_active_changed_event(false);
            else {window->window().dispatch_key_press_event("\x1b");window->window().dispatch_key_release_event("\x1b");}
            settle();window->window().dispatch_pointer_release_event(end,slint::PointerEventButton::Left);settle();
            expect(model->find("module","lush-voice")->fields==original&&model->undoCount()==cancelUndo,windowLoss?"Window loss cancels exact typed native record with no Undo":"Escape cancels exact typed native record with no Undo");
        }
        std::cout<<"PASS: "<<checks<<" native parameter menu checks\n";
        return 0;
    }catch(const std::exception& e){std::cerr<<"FAIL: "<<e.what()<<'\n';return 1;}
}
