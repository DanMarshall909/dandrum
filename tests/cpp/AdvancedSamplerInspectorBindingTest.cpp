#include "AdvancedSamplerInspector.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <iostream>
#include <cmath>
#include <stdexcept>

namespace {
int checks=0;
void require(bool condition,const std::string& message) { ++checks;if(!condition)throw std::runtime_error(message); }
template<class Window> auto elements(const Window& window,std::string_view label) {
    return slint::testing::ElementHandle::find_by_accessible_label(window,label);
}
template<class Window> void activate(const Window& window,std::string_view label) {
    for(const auto& element:elements(window,label))if(element.accessible_role()==slint::language::AccessibleRole::Button){
        const auto origin=element.absolute_position();const auto size=element.size();
        const slint::LogicalPosition centre{{origin.x+size.width/2,origin.y+size.height/2}};
        window->window().dispatch_pointer_press_event(centre,slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(centre,slint::PointerEventButton::Left);return;
    }
    throw std::runtime_error("Missing native inspector action: "+std::string(label));
}
template<class Window> void move(const Window& window,std::string_view label,float distance) {
    for(const auto& element:elements(window,label))if(element.accessible_role()==slint::language::AccessibleRole::Button){
        const auto origin=element.absolute_position();const auto size=element.size();
        const slint::LogicalPosition start{{origin.x+size.width/2,origin.y+size.height/2}};
        const slint::LogicalPosition finish{{start.x,start.y+distance}};
        window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);
        window->window().dispatch_pointer_move_event(finish);
        window->window().dispatch_pointer_release_event(finish,slint::PointerEventButton::Left);return;
    }
    throw std::runtime_error("Missing native operation grip: "+std::string(label));
}
template<class Window> void context(const Window& window,std::string_view label) {
    for(const auto& element:elements(window,label))if(element.accessible_role()==slint::language::AccessibleRole::Button){
        const auto p=element.absolute_position();const auto s=element.size();
        const slint::LogicalPosition position{{p.x+s.width/2,p.y+s.height/2}};
        window->window().dispatch_pointer_press_event(position,slint::PointerEventButton::Right);
        window->window().dispatch_pointer_release_event(position,slint::PointerEventButton::Right);return;
    }
    throw std::runtime_error("Missing operation context target: "+std::string(label));
}
}
int main(){try{
    slint::testing::init();auto window=dandrum_ui::AdvancedSamplerInspectorTest::create();
    auto binding=dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
    const auto& session=window->global<dandrum_ui::Session>();window->window().show();
    require(session.get_inspector_type()=="Instrument","Empty patch projects an instrument inspector");
    require(!elements(window,"Sounds: None").empty(),"Empty inspector displays the guide instrument properties in the native scene");
    require(!session.get_inspector_parameters()||session.get_inspector_parameters()->row_count()==0,"Empty selection does not dump unrelated parameter knobs");
    require(session.invoke_command("patch.load","kit","",0),"Load native fixture patch");
    require(session.get_inspector_type()=="Sample","Sample workspace projects the selected asset inspector");
    for(const auto label:{"State","Format","Channels","Used by","Length","Loop","Root"}){
        bool found=false;for(std::size_t i=0;i<session.get_inspector_rows()->row_count();++i)found|=session.get_inspector_rows()->row_data(i)->label==label;
        require(found,std::string("Selected sample property exists: ")+label);
    }
    require(!session.get_inspector_parameters()||session.get_inspector_parameters()->row_count()==0,"Sample info stays a compact property panel");
    require(session.invoke_command("sample.set","","fade-in",0.12),"Record a native non-destructive operation");
    require(session.get_inspector_has_history() && session.get_history()->row_count()>0,"Operation history appears for sample workspace");
    const auto op=session.get_history()->row_data(session.get_history()->row_count()-1).value();
    activate(window,"Edit "+std::string(op.label));
    require(session.get_inspector_type()=="Operation","Actual history row selects its operation inspector");
    for(const auto label:{"In","In curve","Out","Out curve"}) {
        bool found=false;
        for(std::size_t i=0;i<session.get_inspector_rows()->row_count();++i)found|=session.get_inspector_rows()->row_data(i)->label==label;
        require(found,std::string("Selected Fade operation has its guide-specific property: ")+label);
    }
    require(session.get_history()->row_data(session.get_history()->row_count()-1)->selected,"Selected operation updates native history state");
    activate(window,"Bypass "+std::string(op.label));
    require(!session.get_history()->row_data(session.get_history()->row_count()-1)->on,"Actual checkbox bypasses the native operation");
    require(binding->model()->number("fade-in")==0,"Bypass changes effective region processing");
    activate(window,"Bypass "+std::string(op.label));
    require(std::abs(binding->model()->number("fade-in")-0.12)<0.000001,"Re-enable restores the native operation value");
    activate(window,"Edit "+std::string(op.label));
    require(session.get_inspector_type()=="Sample","Clicking the selected operation again restores the sample inspector");
    require(session.invoke_command("sample.set","","fade-out",0.22),"Add a second real operation for reordering");
    move(window,"Move "+std::string(op.label),27);
    require(session.get_history()->row_data(1)->id==op.id,"Actual grip drag changes the native operation order");
    const auto history_size=session.get_history()->row_count();
    activate(window,"Add operation");
    require(session.get_history()->row_count()==history_size+1,"Actual Add action creates a native operation");
    context(window,"Edit "+std::string(session.get_history()->row_data(session.get_history()->row_count()-1)->label));
    slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();
    bool removed=false;
    for(const auto& element:elements(window,"Remove")){
        element.invoke_accessible_default_action();removed=true;break;
    }
    require(removed&&session.get_history()->row_count()==history_size,"Actual operation context menu Remove mutates the native stack");
    require(session.invoke_command("undo","","",0)&&session.get_history()->row_count()==history_size+1,"Context removal Undo restores the native operation stack");
    const std::string original_source(session.get_selected_source());
    activate(window,"Collapse stack to a new sample");
    require(binding->model()->text("collapse-state")=="running","Actual Collapse action starts the generation-tagged mock render");
    bool collapse_disabled=false;
    for(const auto& button:elements(window,"Collapse stack to a new sample"))if(button.accessible_role()==slint::language::AccessibleRole::Button)
        collapse_disabled=button.accessible_enabled() && !*button.accessible_enabled();
    require(collapse_disabled,"Collapse prevents duplicate jobs while the native mock render runs");
    binding->model()->tick(1);binding->refresh();
    require(std::string(session.get_selected_source())!=original_source,"Collapse selects an independent derived sample");
    require(binding->model()->find("source",original_source)!=nullptr,"Collapse preserves the original source");
    require(session.get_history()->row_count()==1 && session.get_history()->row_data(0)->label=="Source","Collapsed history has one immutable source baseline");
    require(!session.invoke_command("sample.history-bypass",session.get_history()->row_data(0)->id,"",0),"The source baseline cannot be bypassed");
    require(!session.invoke_command("sample.history-remove",session.get_history()->row_data(0)->id,"",0),"The source baseline cannot be removed");
    require(session.invoke_command("undo","","",0),"Collapsed sample remains undoable");
    require(session.get_history()->row_count()==history_size+1 && std::string(session.get_selected_source())==original_source,"One Undo restores the original source and full operation stack");
    require(session.invoke_command("page.select","overview","",0),"Open effective selection overview");
    require(session.invoke_command("selection.set","drums","",0),"Select a native group");
    require(session.get_inspector_type()=="Group","Group selection projects its inheritance inspector");
    require(!session.get_inspector_has_history(),"History stack is confined to sample/slice workspaces");
    require(session.invoke_command("inherit.override","drums","cutoff",2222),"Create a real group override");
    require(session.get_inspector_override_count()==1,"Override count follows the native selected node");
    activate(window,"Reset overrides");
    require(session.get_inspector_override_count()==0,"Actual inspector Reset restores inherited values");
    require(session.invoke_command("undo","","",0),"Reset can be undone");
    require(session.get_inspector_override_count()==1,"Undo restores the inspector override affordance");
    require(session.invoke_command("page.select","mapping","",0),"Inspect mapping zone");
    require(session.get_inspector_type()=="Zone","Mapping selection owns keys/velocity/source properties");
    std::string browse;
    window->on_browse_requested([&](slint::SharedString action){browse=action;});
    require(session.invoke_command("zone.select","snH","",0),"Select a zone bound to another sample");
    activate(window,"Replace sample…");
    require(browse=="asset.replace" && session.get_selected_source()=="snare-source","Zone replacement selects the row's asset before entering the native file boundary");
    require(session.invoke_command("page.select","voice","",0),"Inspect voice module");
    require(session.get_inspector_type()=="Module","Voice module shows only its module inspector");
    require(session.invoke_command("page.select","macros","",0),"Inspect macro");
    require(session.get_inspector_type()=="Macro","Macro value/binding/routes project to inspector");
    activate(window,"Learn MIDI");
    require(!session.get_midi_learn().empty(),"Actual inspector MIDI Learn starts native learning");
    std::string context_action,context_target;
    window->on_context_requested([&](slint::SharedString action,slint::SharedString target){context_action=action;context_target=target;});
    activate(window,"Add destination…");
    require(context_action=="macro-destinations" && context_target=="tone","Actual inspector destination action opens a native picker for the selected macro");
    require(session.invoke_command("state.select","fx","",0),"Inspect the guide Compressor processor");
    require(session.get_selected_id()=="comp" && session.get_inspector_parameters()->row_count()==6,"Selected Compressor owns all six guide controls");
    for(const auto label:{"Threshold","Ratio","Attack","Release","Makeup","Mix"}) {
        bool found=false;
        for(const auto& knob:elements(window,label))found|=knob.accessible_role()==slint::language::AccessibleRole::Slider;
        require(found,std::string("Compressor parameter is an actual native control: ")+label);
    }
    require(session.invoke_command("fx.set","comp","mix",0.25),"Set a normalized effect value");
    bool percentage=false;
    for(const auto& knob:elements(window,"Mix"))if(knob.accessible_role()==slint::language::AccessibleRole::Slider){
        percentage=true;require(knob.accessible_value() && *knob.accessible_value()=="25","Inspector displays normalized quarter mix as 25 percent");
        knob.set_accessible_value("50");
    }
    require(percentage,"Selected effect exposes its actual native parameter control");
    require(binding->model()->find("fx","comp")->number("mix")==0.5,"Typed 50 percent writes normalized 0.5 through the native gesture");
    require(session.invoke_command("page.select","mod","",0),"Inspect modulation route");
    require(session.get_inspector_type()=="Route"||session.get_inspector_type()=="Modulator","Route or modulator selection determines modulation inspector");
    std::string selection_errors;
    for(const auto& [state,type]:std::initializer_list<std::pair<const char*,const char*>>{
        {"empty","Instrument"},{"loaded","Sample"},{"sample","Sample"},{"loop","Sample"},
        {"mapping","Zone"},{"vel","Layer"},{"rr","Alternate"},{"voice","Module"},
        {"mod","Route"},{"macro","Macro"},{"routing","Sound"},{"fx","Processor"},
        {"missing","Sample"},{"unsupported","Sample"},{"loading","Sample"},{"min","Sample"},
        {"default","Zone"},{"exp","Module"},{"drop","Instrument"},{"pads","Pad"},
        {"slices","Slice"},{"transients","Slice"},{"running","Slice"},{"failed","Slice"},
        {"recon","Zone"},{"diag","Pad"},{"overview","Group"},{"freeze","Module"},{"macros","Macro"}}){
        require(session.invoke_command("state.select",state,"",0),std::string("Load native design selection ")+state);
        checks+=2;
        if(session.get_inspector_type()!=type)selection_errors+=std::string(state)+" expected "+type+", actual "+std::string(session.get_inspector_type())+"; ";
        if(session.get_inspector_rows()->row_count()==0)selection_errors+=std::string(state)+" has no native property rows; ";
    }
    require(selection_errors.empty(),"Selection-specific native inspectors: "+selection_errors);
    require(session.invoke_command("state.select","fx","",0),"Load all typed guide processors for direct Inspector descriptor proof");
    std::vector<std::string> processors;for(const auto& processor:binding->model()->records("fx"))processors.push_back(processor.id);
    std::size_t descriptor_count=0;
    for(const auto& id:processors){
        require(session.invoke_command("selection.set",slint::SharedString(id),"",0),"Select the actual processor for its Inspector controls: "+id);
        const auto projected=session.get_inspector_parameters();std::vector<dandrum_ui::ParameterInfo> descriptors;
        for(std::size_t i=0;i<projected->row_count();++i)descriptors.push_back(*projected->row_data(i));
        for(const auto& p:descriptors){if(!p.available||p.maximum<=p.minimum)continue;++descriptor_count;
            std::optional<slint::testing::ElementHandle> control;
            for(const auto& element:elements(window,p.label.data()))if(element.accessible_role()==slint::language::AccessibleRole::Slider){control=element;break;}
            require(control.has_value(),"Every typed processor descriptor owns an actual Inspector control: "+std::string(p.id));
            const auto model=binding->model();const auto original=model->parameterInfo(p.id.data())->number("value");const auto undo=model->undoCount();
            const auto step=p.step>0?p.step:.001f;
            double desired=p.minimum+std::round((p.maximum-p.minimum)*.37/step)*step;
            if(std::abs(desired-original)<.002||std::abs(desired-p.default_value)<.002)desired=p.minimum+std::round((p.maximum-p.minimum)*.63/step)*step;
            const double scale=p.unit=="%"&&p.id!="pan"?100:1;
            control->set_accessible_value(slint::SharedString(std::to_string(desired*scale)));
            require(std::abs(model->parameterInfo(p.id.data())->number("value")-desired)<.002&&model->undoCount()==undo+1,"Every processor Inspector typed commit writes its own native descriptor: "+std::string(p.id));
            const auto position=control->absolute_position();const auto size=control->size();
            const slint::LogicalPosition point{{position.x+size.width/2,position.y+size.height/2}};
            window->window().dispatch_pointer_press_event(point,slint::PointerEventButton::Middle);
            window->window().dispatch_pointer_release_event(point,slint::PointerEventButton::Middle);
            require(std::abs(model->parameterInfo(p.id.data())->number("value")-p.default_value)<.002&&model->undoCount()==undo+2,"Every processor Inspector actual reset restores its own default: "+std::string(p.id));
            require(session.invoke_command("undo","","",0)&&std::abs(model->parameterInfo(p.id.data())->number("value")-desired)<.002,"Processor Inspector reset Undo restores its typed value: "+std::string(p.id));
            require(session.invoke_command("undo","","",0)&&std::abs(model->parameterInfo(p.id.data())->number("value")-original)<.002,"Processor Inspector commit Undo restores its initial value: "+std::string(p.id));
        }
    }
    require(processors.size()==9&&descriptor_count>=25,"All nine guide processors have direct per-descriptor commit/reset coverage");
    for(const auto state:{"missing","unsupported"}){
        require(session.invoke_command("state.select",state,"",0),"Select unavailable sample Inspector");
        const auto source=session.get_selected_source();
        for(const auto& [label,expected]:std::initializer_list<std::pair<const char*,const char*>>{{"Locate file…","asset.relink"},{"Search folder…","asset.search-folder"},{"Replace with…","asset.replace"}}){
            browse.clear();activate(window,label);
            require(browse==expected&&session.get_selected_source()==source,"Actual unavailable sample Inspector action preserves its selected asset and enters the precise file boundary: "+std::string(label));
        }
    }
    require(session.invoke_command("state.select","sample","",0),"Select loaded sample Inspector replacement");
    browse.clear();activate(window,"Replace…");require(browse=="asset.replace","Actual loaded sample Inspector Replace enters the native file boundary");
    for(const auto state:{"voice","fx"}){
        require(session.invoke_command("state.select",state,"",0),"Select bypassable native module Inspector");
        const auto id=std::string(session.get_selected_id());const auto table=std::string_view(state)=="fx"?"fx":"module";
        activate(window,"Bypass");require(binding->model()->find(table,id)->number("bypassed")==1,"Actual Inspector Bypass mutates its selected native processor or voice module");
        require(session.invoke_command("undo","","",0)&&binding->model()->find(table,id)->number("bypassed")==0,"Inspector module Bypass Undo restores its selected native record");
        context_action.clear();context_target.clear();activate(window,"Assign modulation…");
        require(context_action=="modulation-add"&&context_target=="cutoff","Actual Inspector Assign modulation forwards its exact parameter to the root picker");
    }
    require(session.invoke_command("state.select","mod","",0),"Select native route Inspector");
    activate(window,"Edit source");
    require(session.get_selected_id()=="filter-env"&&session.get_inspector_type()=="Modulator","Actual route Inspector Edit source selects its real modulator Inspector");
    const auto route_count=binding->model()->records("route").size();activate(window,"Apply to group…");
    require(binding->model()->records("route").size()==route_count+1&&binding->model()->records("route").back().text("source")=="filter-env","Actual modulator Inspector Apply to group creates its native cutoff route");
    require(session.invoke_command("undo","","",0)&&binding->model()->records("route").size()==route_count,"Inspector route creation is undoable");
    require(session.invoke_command("state.select","pads","",0),"Select mapped pad Inspector");activate(window,"Open mapping");
    require(session.get_page()=="mapping","Actual mapped pad Inspector opens its native mapping workspace");
    require(session.invoke_command("state.select","slices","",0)&&session.invoke_command("sample.history-select","history-slice","",0),"Select Slice operation Inspector");
    activate(window,"Detect transients");require(session.get_analysis_state()=="running","Actual Slice operation Inspector starts native transient analysis");
    require(session.invoke_command("analysis.cancel","","",0),"Cancel Inspector analysis after workflow proof");
    require(session.invoke_command("state.select","sample","",0)&&session.invoke_command("sample.set","","fade-in",.2),"Create selected operation Inspector action fixture");
    auto history=session.get_history();const auto operation=*history->row_data(history->row_count()-1);
    require(session.invoke_command("sample.history-select",operation.id,"",0),"Select operation action rows");activate(window,"Bypass");
    require(binding->model()->find("history",operation.id.data())->number("on")==0,"Actual operation Inspector Bypass mutates its selected history operation");
    activate(window,"Remove");require(!binding->model()->find("history",operation.id.data()),"Actual operation Inspector Remove deletes its selected history operation");
    require(session.invoke_command("undo","","",0)&&binding->model()->find("history",operation.id.data()),"Operation Inspector removal remains undoable");
    require(session.invoke_command("state.select","sample","",0),"Restore ordered history for actual context menu movement");
    for(const auto& [label,direction]:std::initializer_list<std::pair<const char*,int>>{{"Move up",-1},{"Move down",1}}){
        const auto operations=session.get_history();const std::size_t index=1;const auto row=*operations->row_data(index);
        context(window,"Edit "+std::string(row.label));
        slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();
        bool moved=false;for(const auto& action:elements(window,label))if(action.accessible_role()==slint::language::AccessibleRole::Button){action.invoke_accessible_default_action();moved=true;break;}
        require(moved&&session.get_history()->row_data(index+direction)->id==row.id,"Actual history context menu move changes its native operation order: "+std::string(label));
        require(session.invoke_command("undo","","",0)&&session.get_history()->row_data(index)->id==row.id,"History context menu movement Undo restores the original operation order");
    }
    require(session.invoke_command("state.select","pads","",0)&&session.invoke_command("pad.select","","",127)&&session.get_selected_pad()==127,"Select empty native pad127 Inspector through its numeric note contract");
    const auto zones=binding->model()->records("zone").size();activate(window,"Add zone…");
    require(binding->model()->records("zone").size()==zones+1&&binding->model()->records("zone").back().number("lo")==127,"Actual empty pad Inspector Add zone creates a native mapping on the selected note");
    require(session.invoke_command("undo","","",0)&&binding->model()->records("zone").size()==zones,"Empty pad Inspector mapping creation is undoable");
    require(session.invoke_command("patch.load","empty","",0),"Inspect instrument without any source for its guide fallback");
    require(session.get_inspector_type()=="Instrument"&&!elements(window,"Sounds: None").empty()&&elements(window,"Add zone…").empty(),"Source-empty Inspector shows its guide Instrument properties without an invalid mapping action");
    window->window().hide();std::cout<<"PASS: "<<checks<<" actual native inspector checks\n";
}catch(const std::exception& error){std::cerr<<"FAIL: "<<error.what()<<'\n';return 1;}}
