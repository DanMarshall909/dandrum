#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <algorithm>
#include <cmath>
#include <iostream>
#include <stdexcept>

int main() {
    int checks=0;
    const auto expect=[&](bool ok,const char* message){++checks;if(!ok)throw std::runtime_error(message);};
    try {
        slint::testing::init();
        auto window=dandrum_ui::AppWindow::create();
        auto model=std::make_shared<dandrum::sampler::Model>(); model->command("patch.load","kit");
        auto binding=dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window,model);
        auto& performance=window->global<dandrum_ui::PerformanceState>();
        dandrum::sampler::bind_performance_sources(performance,binding);
        dandrum::sampler::bind_patch_search(window->global<dandrum_ui::PatchFilter>());
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        window->show();
        const auto settle=[&]{slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
        const auto element=[&](const char* label){settle();for(auto row:slint::testing::ElementHandle::find_by_accessible_label(window,label))if(row.accessible_role()==slint::language::AccessibleRole::Button||row.accessible_role()==slint::language::AccessibleRole::Slider)return row;throw std::runtime_error(std::string("Missing app control: ")+label);};
        const auto click=[&](const char* label){element(label).invoke_accessible_default_action();settle();};
        const auto center=[](auto row){auto p=row.absolute_position();auto s=row.size();return slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};};
        const auto pointerClick=[&](const char* label){auto point=center(element(label));window->window().dispatch_pointer_press_event(point,slint::PointerEventButton::Left);window->window().dispatch_pointer_release_event(point,slint::PointerEventButton::Left);settle();};
        auto undo=model->undoCount();
        click("Show or hide macros"); expect(!performance.get_show_macros()&&performance.get_open(),"App section control hides macros and keeps drawer open");
        click("Show or hide macros"); expect(performance.get_show_macros(),"App section control restores macros");
        model->noteOn(36,100,"test-pad"); binding->refresh();
        click("Show or hide pads"); expect(!performance.get_show_pads()&&model->heldNotes().empty(),"Hiding pads releases held native sources");
        click("Show or hide pads");
        click("Show or hide keyboard"); expect(!performance.get_show_keys(),"Keyboard section can collapse independently");
        click("Show or hide keyboard"); expect(model->undoCount()==undo,"Performance layout controls preserve editing history");
        auto unassignedMacro=element("Macro 7");expect(unassignedMacro.accessible_enabled()==false,"Unassigned Perform macro retains an inert value control");
        auto unassignedPoint=center(unassignedMacro);auto unassignedValue=model->find("macro","macro7")->number("value");auto unassignedUndo=model->undoCount();
        window->window().dispatch_pointer_press_event(unassignedPoint,slint::PointerEventButton::Right);window->window().dispatch_pointer_release_event(unassignedPoint,slint::PointerEventButton::Right);settle();
        click("Learn macro MIDI CC");expect(model->text("midi-learn")=="macro7"&&model->find("macro","macro7")->number("value")==unassignedValue&&model->undoCount()==unassignedUndo,"Actual unassigned Perform macro right-click opens native MIDI learn without editing its value");
        window->window().dispatch_pointer_press_event(unassignedPoint,slint::PointerEventButton::Right);window->window().dispatch_pointer_release_event(unassignedPoint,slint::PointerEventButton::Right);settle();
        click("Stop macro MIDI learn");expect(model->text("midi-learn").empty()&&element("Macro 7").accessible_enabled()==false,"Unassigned Perform macro context cancels native learn and remains inert");
        auto tone=element("Tone");auto tonePoint=center(tone);auto originalTone=model->find("macro","tone")->number("value");
        window->window().dispatch_pointer_press_event(tonePoint,slint::PointerEventButton::Left);
        window->window().dispatch_pointer_move_event(slint::LogicalPosition{{tonePoint.x,tonePoint.y-18}});settle();
        window->window().dispatch_pointer_release_event(slint::LogicalPosition{{tonePoint.x,tonePoint.y-18}},slint::PointerEventButton::Left);settle();
        expect(model->find("macro","tone")->number("value")>originalTone&&model->undoCount()==undo+1,"Actual drawer macro pointer drag changes its native value in one undo step");
        model->command("undo");binding->refresh();settle();
        expect(std::abs(model->find("macro","tone")->number("value")-originalTone)<0.001,"Undo restores the dragged macro value in the shared app");
        window->window().dispatch_pointer_press_event(tonePoint,slint::PointerEventButton::Right);
        window->window().dispatch_pointer_release_event(tonePoint,slint::PointerEventButton::Right);settle();
        click("Learn macro MIDI CC");
        expect(model->text("midi-learn")=="tone","Actual macro right-click opens the macro-aware actions and starts its native MIDI learn");
        window->window().dispatch_pointer_press_event(tonePoint,slint::PointerEventButton::Right);
        window->window().dispatch_pointer_release_event(tonePoint,slint::PointerEventButton::Right);settle();
        click("Stop macro MIDI learn");
        expect(model->text("midi-learn").empty(),"Actual macro context cancels MIDI learn without changing performance layout");
        click("Macros");click("Select Tone macro");
        pointerClick("Add destination…");
        pointerClick("Add macro destination level");
        expect(model->find("binding","tone:level")&&model->find("binding","tone:level")->number("amount")==0.25,"Composed macro Inspector destination action opens the real app parameter picker and creates its native binding");
        model->command("undo");binding->refresh();settle();
        expect(!model->find("binding","tone:level"),"Undo restores the composed Inspector macro assignment");
        click("Larger pads"); expect(performance.get_pad_size()>0,"Actual app pad zoom leaves Fit mode");
        click("Fit mapped pads"); expect(performance.get_pad_size()==0&&performance.get_pad_start()==-1,"Actual app pad Fit restores mapped range");
        click("Wider keys"); expect(performance.get_key_width()>0,"Actual app keyboard zoom leaves Fit mode");
        click("Fit mapped keys"); expect(performance.get_key_width()==0,"Actual app keyboard Fit restores mapped range");
        pointerClick("Computer keyboard notes"); expect(model->number("pc-keys")==1&&performance.get_pc_enabled(),"App PC toggle updates canonical native and shared state");
        window->window().dispatch_key_press_event("a"); settle();
        expect(model->heldNotes()==std::vector<int>{36},"Real app keyboard input creates the octave-base note");
        window->window().dispatch_key_press_repeat_event("a");settle();
        expect(model->heldNotes()==std::vector<int>{36},"Repeated PC input preserves one held source");
        click("Octave up (X)"); expect(model->heldNotes().empty()&&model->number("octave-base")==48&&performance.get_base_note()==48,"App octave arrow releases held notes and updates the native base");
        window->window().dispatch_key_release_event("a");settle();expect(model->heldNotes().empty(),"Late PC release cannot recreate a shifted note");
        window->window().dispatch_key_press_event("a");settle();expect(model->heldNotes()==std::vector<int>{48},"The next PC press uses the shifted octave");
        click("Computer keyboard notes");expect(model->heldNotes().empty()&&model->number("pc-keys")==0,"App PC disable balances its native note");
        window->window().dispatch_key_release_event("a");
        model->command("macro.set","tone","",0.73);binding->refresh();settle();
        auto toneResetUndo=model->undoCount();auto toneResetAt=center(element("Tone"));
        window->window().dispatch_pointer_press_event(toneResetAt,slint::PointerEventButton::Middle);window->window().dispatch_pointer_release_event(toneResetAt,slint::PointerEventButton::Middle);settle();
        expect(std::abs(model->find("macro","tone")->number("value")-model->find("macro","tone")->number("default"))<0.00001&&model->undoCount()==toneResetUndo+1,"Actual macro middle-click resets its native default in one undo step");
        click("Pads");
        // The Pads editor and persistent drawer both expose this mapped note.
        // Exercise the drawer instance, which remains fully visible below the editor.
        auto pad=element("Pad 36 C2, Kick");
        for(auto candidate:slint::testing::ElementHandle::find_by_accessible_label(window,"Pad 36 C2, Kick")) {
            auto p=candidate.absolute_position();auto size=candidate.size();
            if(candidate.accessible_role()==slint::language::AccessibleRole::Button&&p.y+size.height<=window->window().size().height&&p.y>pad.absolute_position().y)pad=candidate;
        }
        auto at=center(pad);
        window->window().dispatch_pointer_press_event(at,slint::PointerEventButton::Left);settle();
        expect(model->heldNotes()==std::vector<int>{36},"Native pointer down on app pad holds its own source");
        window->window().dispatch_pointer_release_event(at,slint::PointerEventButton::Left);settle();expect(model->heldNotes().empty(),"Native app pad pointer release balances ownership");
        const auto resizeHandle=[&](bool vertical,bool last){
            settle();std::vector<slint::testing::ElementHandle> handles;
            for(auto row:slint::testing::ElementHandle::find_by_element_type_name(window,"ResizeHandle")){
                auto size=row.size();if(vertical?size.width>100&&size.height<10:size.width<10&&size.height>100)handles.push_back(row);
            }
            std::sort(handles.begin(),handles.end(),[](auto a,auto b){return a.absolute_position().x<b.absolute_position().x;});
            if(handles.empty())throw std::runtime_error("Missing actual drawer resize handle");
            return last?handles.back():handles.front();
        };
        const auto doubleClick=[&](auto handle){
            slint::cbindgen_private::slint_mock_elapsed_time(600);
            auto at=center(handle);
            for(int clickIndex=0;clickIndex<2;++clickIndex){window->window().dispatch_pointer_press_event(at,slint::PointerEventButton::Left);window->window().dispatch_pointer_release_event(at,slint::PointerEventButton::Left);settle();slint::cbindgen_private::slint_mock_elapsed_time(20);}
        };
        const auto drag=[&](auto handle,float dx,float dy){
            auto start=center(handle);
            window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);
            window->window().dispatch_pointer_move_event(slint::LogicalPosition{{start.x+dx/2,start.y+dy/2}});settle();
            window->window().dispatch_pointer_move_event(slint::LogicalPosition{{start.x+dx,start.y+dy}});settle();
            window->window().dispatch_pointer_release_event(slint::LogicalPosition{{start.x+dx,start.y+dy}},slint::PointerEventButton::Left);settle();
        };
        undo=model->undoCount();
        drag(resizeHandle(false,false),45,0);
        expect(std::abs(performance.get_macro_width()-175)<0.01,"Actual macro split follows absolute pointer distance through relayout");
        drag(resizeHandle(false,true),25,0);
        expect(std::abs(performance.get_pad_width()-411)<0.01,"Actual pad and keyboard split preserves the requested pointer distance");
        drag(resizeHandle(true,false),0,-40);
        expect(std::abs(performance.get_body_height()-324)<0.01,"Actual drawer resize follows pointer through changing drawer position");
        model->noteOn(36,90,"resize-held");binding->refresh();
        drag(resizeHandle(true,false),0,500);
        expect(!performance.get_open()&&model->heldNotes().empty(),"Dragging the actual drawer closed releases every held native source");
        click("Show or hide pads");click("Show or hide pads");
        expect(performance.get_open()&&model->undoCount()==undo,"Reopening the drawer preserves performance layout and editing history");
        performance.set_macro_width(190);performance.set_pad_width(420);performance.set_body_height(320);settle();
        auto resetUndo=model->undoCount();
        doubleClick(resizeHandle(false,false));expect(performance.get_macro_width()==0,"Actual macro split double-click restores automatic width");
        doubleClick(resizeHandle(false,true));expect(performance.get_pad_width()==0,"Actual pad-keyboard split double-click restores automatic width");
        doubleClick(resizeHandle(true,false));expect(performance.get_body_height()==0&&performance.get_open()&&model->undoCount()==resetUndo,"Actual drawer double-click restores standard open height without editing history");
        auto zoom=performance.get_key_width();click("Perform");expect(model->text("page")=="perform"&&performance.get_key_width()==zoom,"Perform view preserves shared keyboard geometry");
        model->noteOn(36,90,"perform-held");binding->refresh();click("Return to editor");expect(model->text("page")=="pads"&&model->heldNotes().empty(),"Return to editor restores page and releases performance sources");
        click("Computer keyboard notes");
        window->invoke_show_file("asset.import");settle();window->window().dispatch_key_press_event("a");settle();
        expect(model->heldNotes().empty(),"Focused native text entry prevents PC-key notes");
        window->window().dispatch_key_release_event("a");
        const auto oldBase=model->number("octave-base"),oldPc=model->number("pc-keys");
        binding.reset(); model->noteOn(36,100,"expired-session");
        performance.invoke_release_notes(); performance.invoke_octave_changed(72); performance.invoke_pc_changed(false);
        expect(model->heldNotes()==std::vector<int>{36}&&model->number("octave-base")==oldBase&&model->number("pc-keys")==oldPc,"Expired native session makes every retained performance callback a safe no-op");
        model->releaseNotes();
        std::cout<<"PASS: "<<checks<<" whole-app performance input checks\n";return 0;
    } catch(const std::exception& error){std::cerr<<"FAIL: "<<error.what()<<'\n';return 1;}
}
