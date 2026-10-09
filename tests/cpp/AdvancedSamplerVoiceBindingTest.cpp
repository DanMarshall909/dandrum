#include "AdvancedSamplerVoice.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <cmath>
#include <iostream>
#include <map>
#include <algorithm>
#include <stdexcept>

namespace {
int checks=0;
void require(bool ok,const std::string& text){++checks;if(!ok)throw std::runtime_error(text);}
bool near(double a,double b){return std::abs(a-b)<0.002;}
template<class Window> slint::testing::ElementHandle find(const Window& window,std::string_view label){
    for(const auto& element:slint::testing::ElementHandle::find_by_accessible_label(window,label))
        if(element.accessible_role()==slint::language::AccessibleRole::Slider)return element;
    throw std::runtime_error("Missing native voice plot/control: "+std::string(label));
}
template<class Window> void activate(const Window& window,std::string_view label){
    for(const auto& element:slint::testing::ElementHandle::find_by_accessible_label(window,label))
        if(element.accessible_role()==slint::language::AccessibleRole::Button){
            const auto p=element.absolute_position();const auto s=element.size();
            const slint::LogicalPosition position{{p.x+s.width/2,p.y+s.height/2}};
            window->window().dispatch_pointer_press_event(position,slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(position,slint::PointerEventButton::Left);return;
        }
    throw std::runtime_error("Missing native voice action: "+std::string(label));
}
}
int main(){try{
    slint::testing::init();auto window=dandrum_ui::AdvancedSamplerVoiceTest::create();
    auto binding=dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
    dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
    const auto& session=window->global<dandrum_ui::Session>();const auto model=binding->model();
    require(session.invoke_command("state.select","voice","",0),"Load native voice guide state");
    window->window().show();window->window().dispatch_window_active_changed_event(true);
    activate(window,"Filter type BP");
    require(session.get_filter_mode()=="bp"&&model->text("filter-mode")=="bp","Actual filter type control changes the persisted native mode");
    require(session.invoke_command("undo","","",0)&&session.get_filter_mode()=="lp","One Undo restores the filter type");
    activate(window,"Filter type Notch");
    require(model->text("filter-mode")=="notch","Actual Notch control uses the native filter mode contract");
    require(session.invoke_command("undo","","",0)&&session.get_filter_mode()=="lp","Notch mode remains undoable");
    const auto source_gain=model->number("source-gain");const auto source_undo=model->undoCount();
    find(window,"Gain").set_accessible_value("-6");
    slint::platform::update_timers_and_animations();
    slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();
    require(model->number("source-gain")==-6&&model->undoCount()==source_undo+1,"Actual source Gain field edits its native parameter as one Undo");
    require(session.invoke_command("undo","","",0)&&near(model->number("source-gain"),source_gain),"One Undo restores source Gain");
    slint::platform::update_timers_and_animations();
    slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();
    const auto restored_gain=find(window,"Gain").accessible_value();
    require(restored_gain&&std::string_view(restored_gain->data())=="−1.5","Undo updates the visible source Gain value; actual="+(restored_gain?std::string(*restored_gain):"missing"));
    std::string context_action,context_target;
    window->on_context_action([&](slint::SharedString action,slint::SharedString target){context_action=action;context_target=target;});
    const auto gain=find(window,"Gain");const auto gp=gain.absolute_position();const auto gs=gain.size();
    const slint::LogicalPosition gain_context{{gp.x+gs.width/2,gp.y+gs.height/2}};
    window->window().dispatch_pointer_press_event(gain_context,slint::PointerEventButton::Left);
    window->window().dispatch_pointer_release_event(gain_context,slint::PointerEventButton::Left);
    window->window().dispatch_key_press_event("m");
    require(context_action=="modulation"&&context_target=="source-gain","Actual focused source Gain keyboard context action forwards the precise native parameter");
    const slint::LogicalPosition gain_finish{{gain_context.x,gain_context.y-10}};
    window->window().dispatch_pointer_press_event(gain_context,slint::PointerEventButton::Left);
    window->window().dispatch_pointer_move_event(gain_finish);
    window->window().dispatch_pointer_release_event(gain_finish,slint::PointerEventButton::Left);
    require(model->number("source-gain")>source_gain&&model->undoCount()==source_undo+1,"Actual source Gain drag records one native parameter edit; gain="+std::to_string(model->number("source-gain"))+", undo="+std::to_string(model->undoCount())+", expected="+std::to_string(source_undo+1));
    require(session.invoke_command("undo","","",0)&&near(model->number("source-gain"),source_gain),"Source Gain gesture Undo restores its starting value");
    activate(window,"Source playback Gated");
    require(model->text("play-mode")=="gated","Actual source playback strip changes native playback mode");
    require(session.invoke_command("undo","","",0)&&model->text("play-mode")=="once","Source playback mode remains undoable");
    auto filter=find(window,"Filter response");const auto fp=filter.absolute_position();const auto fs=filter.size();
    require(near(fs.height,84),"Default filter response has the guide's 84px plot height");
    const auto old_cutoff=model->number("cutoff"),old_resonance=model->number("resonance");
    const auto undo=model->undoCount();
    const slint::LogicalPosition start{{fp.x+8+(fs.width-16)*.5f,fp.y+4+(fs.height-8)*.5f}};
    const slint::LogicalPosition finish{{fp.x+8+(fs.width-16)*.75f,fp.y+4+(fs.height-8)*.2f}};
    window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);
    window->window().dispatch_pointer_move_event(finish);
    require(near(model->number("cutoff"),20*std::pow(1000,.75)),"Actual response drag uses logarithmic frequency");
    require(near(model->number("resonance"),2.0/3),"Actual vertical response drag updates resonance with cutoff");
    require(model->undoCount()==undo,"Response previews wait for pointer release before recording Undo");
    window->window().dispatch_pointer_release_event(finish,slint::PointerEventButton::Left);
    require(model->undoCount()==undo+1,"Two-dimensional response edit records exactly one Undo");
    require(session.invoke_command("undo","","",0),"Undo coupled response edit");
    require(near(model->number("cutoff"),old_cutoff)&&near(model->number("resonance"),old_resonance),"One Undo restores both filter dimensions");
    window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);
    window->window().dispatch_pointer_move_event(finish);
    window->window().dispatch_key_press_event("\x1b");
    require(near(model->number("cutoff"),old_cutoff)&&near(model->number("resonance"),old_resonance),"Escape cancels both dimensions of the native plot edit");
    window->window().dispatch_pointer_release_event(finish,slint::PointerEventButton::Left);
    require(model->undoCount()==undo,"Cancelled response edit creates no Undo");
    window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);
    window->window().dispatch_pointer_move_event(finish);
    window->window().dispatch_window_active_changed_event(false);
    require(near(model->number("cutoff"),old_cutoff)&&near(model->number("resonance"),old_resonance),"Window deactivation cancels a captured filter edit");
    window->window().dispatch_pointer_release_event(finish,slint::PointerEventButton::Left);
    window->window().dispatch_window_active_changed_event(true);
    auto envelope=find(window,"Amp envelope decay handle");
    const auto ep=envelope.absolute_position();const auto es=envelope.size();
    const slint::LogicalPosition decay_start{{ep.x+es.width/2,ep.y+es.height/2}};
    const slint::LogicalPosition decay_finish{{decay_start.x+25,decay_start.y+8}};
    const auto old_decay=model->number("amp-decay"),old_sustain=model->number("amp-sustain");
    window->window().dispatch_pointer_press_event(decay_start,slint::PointerEventButton::Left);
    window->window().dispatch_pointer_move_event(decay_finish);
    require(model->number("amp-decay")>old_decay&&model->number("amp-sustain")<old_sustain,"Actual ADSR decay handle edits both time and sustain");
    window->window().dispatch_pointer_release_event(decay_finish,slint::PointerEventButton::Left);
    require(model->undoCount()==undo+1,"ADSR paired edit records one Undo");
    require(session.invoke_command("undo","","",0)&&near(model->number("amp-decay"),old_decay)&&near(model->number("amp-sustain"),old_sustain),"One Undo restores both ADSR dimensions");
    for(const auto& [name,id]:std::initializer_list<std::pair<const char*,const char*>>{
        {"Amp envelope attack handle","amp-attack"},{"Amp envelope decay handle","amp-decay"},
        {"Amp envelope sustain handle","amp-sustain"},{"Amp envelope release handle","amp-release"},
        {"Filter envelope attack handle","filter-attack"},{"Filter envelope decay handle","filter-decay"},
        {"Filter envelope sustain handle","filter-sustain"},{"Filter envelope release handle","filter-release"}}){
        const auto handle=find(window,name);const auto hp=handle.absolute_position();const auto hs=handle.size();
        const auto previous=model->number(id);const bool sustain=std::string_view(id).ends_with("sustain"),release=std::string_view(id).ends_with("release");
        const slint::LogicalPosition from{{hp.x+hs.width/2,hp.y+hs.height/2}};
        const slint::LogicalPosition to{{from.x+(sustain?0:release?-10:10),from.y+(sustain?-5:0)}};
        window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);
        window->window().dispatch_pointer_move_event(to);
        window->window().dispatch_pointer_release_event(to,slint::PointerEventButton::Left);
        require(release ? model->number(id)<previous : model->number(id)>previous,std::string("Actual envelope point changes its native parameter: ")+name);
        require(model->undoCount()==undo+1,std::string("Envelope point groups one Undo: ")+name+"; expected "+std::to_string(undo+1)+", actual "+std::to_string(model->undoCount())+", value "+std::to_string(model->number(id)));
        require(session.invoke_command("undo","","",0)&&near(model->number(id),previous),std::string("Envelope point Undo restores native value: ")+name);
        const auto fresh=find(window,name);const auto pos=fresh.absolute_position();const auto size=fresh.size();
        const slint::LogicalPosition key_from{{pos.x+size.width/2,pos.y+size.height/2}};
        window->window().dispatch_pointer_press_event(key_from,slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(key_from,slint::PointerEventButton::Left);
        window->window().dispatch_key_press_event("\uF703");
        require(model->number(id)>previous,std::string("Focused envelope point accepts the keyboard: ")+name);
        require(session.invoke_command("undo","","",0)&&near(model->number(id),previous),std::string("Keyboard envelope point remains undoable: ")+name);
    }
    window->set_compact(true);window->window().set_size(slint::LogicalSize{{492,1200}});
    require(near(find(window,"Filter response").size().height,64),"Minimum workspace uses 64px filter response");
    window->set_compact(false);window->set_expanded(true);window->window().set_size(slint::LogicalSize{{1100,1200}});
    require(near(find(window,"Filter response").size().height,110),"Expanded workspace uses 110px filter response");
    const auto modules=model->records("module").size();
    activate(window,"Add source…");activate(window,"Noise");
    require(model->records("module").size()==modules+1&&model->records("module").back().text("name")=="Noise","Native source picker adds the chosen source module");
    const std::string noise_id=model->records("module").back().id;
    activate(window,"Noise mode Pink");
    require(model->find("module",noise_id)->text("mode")=="pink","Actual added Noise source exposes its persisted color choice");
    require(session.invoke_command("undo","","",0)&&model->find("module",noise_id)->text("mode")=="white","Noise color Undo restores White");
    find(window,"Noise Level").set_accessible_value("-9");
    require(near(model->find("module",noise_id)->number("gain"),-9),"Actual added Noise Level edits the typed native source parameter");
    require(session.invoke_command("undo","","",0)&&near(model->parameterInfo("module:"+noise_id+":gain")->number("value"),-4),"Added Noise Level Undo restores its effective guide default");
    require(session.invoke_command("undo","","",0)&&model->records("module").size()==modules,"Added source is undoable");
    for(const auto& [kind,choice,mode,label,field,value,expected]:std::initializer_list<std::tuple<const char*,const char*,const char*,const char*,const char*,const char*,double>>{
        {"Oscillator","Saw","saw","Oscillator Detune","detune","65",.65},
        {"Sampler","Gated","gated","Sampler Start","start","0.25",.25},
        {"Synth patch","Drone","drone","Synth patch Brightness","brightness","80",.8}}){
        activate(window,"Add source…");activate(window,kind);
        const std::string id=model->records("module").back().id;
        activate(window,std::string(kind)+" mode "+choice);
        require(model->find("module",id)->text("mode")==mode,std::string("Typed source choice persists: ")+kind);
        require(session.invoke_command("undo","","",0),std::string("Typed source choice remains undoable: ")+kind);
        find(window,label).set_accessible_value(value);
        require(near(model->find("module",id)->number(field),expected),std::string("Typed source units and parameter values persist: ")+label);
        require(session.invoke_command("undo","","",0),std::string("Typed source parameter remains undoable: ")+label);
        require(session.invoke_command("undo","","",0)&&model->records("module").size()==modules,std::string("Typed source addition is removed by one Undo: ")+kind);
    }
    activate(window,"Edit source");
    require(model->text("page")=="sample","Native source editor action opens the actual sample workspace");
    require(session.invoke_command("page.select","voice","",0),"Return to native Voice controls");
    activate(window,"Sample patch…");
    for(const auto& [label,value]:std::initializer_list<std::pair<const char*,const char*>>{
        {"Low note","40"},{"High note","52"},{"Step","6"},{"Takes","2"},{"Tail","3"}})
        find(window,label).set_accessible_value(value);
    activate(window,"Velocity layers 3 layers");
    require(model->number("autosampler:lo")==40&&model->number("autosampler:hi")==52&&model->number("autosampler:step")==6&&model->number("autosampler:rr")==2&&model->number("autosampler:tail")==3&&model->number("autosampler:layers")==3,"All guide autosampler controls persist in the native model");
    activate(window,"Cancel render dialog");activate(window,"Sample patch…");
    require(find(window,"Low note").accessible_value()&&*find(window,"Low note").accessible_value()=="40","Reopening the autosampler restores native configuration");
    const auto sources=model->records("source").size(),zones=model->records("zone").size();
    activate(window,"Render");
    require(model->text("freeze-state")=="running","Native Render starts a generation-tagged mock batch");
    binding->model()->tick(.4);binding->refresh();
    require(near(session.get_render_progress(),.5),"Native render progress follows the active generation job");
    bool render_disabled=false;
    for(const auto& button:slint::testing::ElementHandle::find_by_accessible_label(window,"Render"))
        if(button.accessible_role()==slint::language::AccessibleRole::Button)render_disabled=button.accessible_enabled()&&! *button.accessible_enabled();
    require(render_disabled,"The running native batch prevents duplicate Render requests");
    activate(window,"Cancel render dialog");binding->model()->tick(1);binding->refresh();
    require(model->records("source").size()==sources&&model->records("zone").size()==zones,"Cancelling the mock batch preserves source and mapping inventories");
    activate(window,"Sample patch…");activate(window,"Render");binding->model()->tick(1);binding->refresh();
    require(model->records("source").size()==sources+18&&model->records("zone").size()==zones+18,"Mock render creates exactly three notes by three layers by two takes");
    require(model->records("source").back().text("metadata").find("mock")!=std::string::npos,"Rendered source provenance explicitly labels the silent mock");
    activate(window,"Cancel render dialog");
    require(session.invoke_command("undo","","",0)&&model->records("source").size()==sources&&model->records("zone").size()==zones,"One Undo removes the whole mock render batch");
    const std::string original_source(session.get_selected_source());const auto original_kind=model->find("source",original_source)->text("kind");
    activate(window,"Freeze…");activate(window,"Freeze includes Source only");find(window,"Tail").set_accessible_value("4");
    require(model->text("freeze-mode")=="src"&&model->number("freeze-tail")==4,"Freeze scope and tail persist in the native model");
    activate(window,"Freeze");
    slint::cbindgen_private::slint_mock_elapsed_time(1);
    slint::platform::update_timers_and_animations();
    window->window().dispatch_key_press_event("\x1b");
    require(model->text("freeze-state")=="idle","Escape immediately cancels the native Freeze job");
    binding->model()->tick(1);binding->refresh();
    require(!session.get_voice_frozen()&&model->find("source",original_source)->text("kind")==original_kind,"Escape cancels an unfinished native Freeze and preserves the original source; state="+model->text("freeze-state")+", kind="+model->find("source",original_source)->text("kind")+", original="+original_kind);
    activate(window,"Freeze…");
    activate(window,"Freeze");binding->model()->tick(1);binding->refresh();activate(window,"Cancel render dialog");
    require(session.get_voice_frozen(),"Completed mock Freeze locks the actual selected voice source");
    const auto frozen_filter=find(window,"Filter response");
    require(frozen_filter.accessible_enabled()&&! *frozen_filter.accessible_enabled(),"Frozen voice disables the native filter plot");
    require(find(window,"Amp envelope attack handle").accessible_enabled()&&! *find(window,"Amp envelope attack handle").accessible_enabled(),"Frozen voice disables native ADSR handles");
    activate(window,"Unfreeze · restore original source");
    require(!session.get_voice_frozen()&&model->find("source",original_source)->text("kind")==original_kind,"Native Unfreeze restores the original source kind and editable controls");
    window->set_voice_visible(false);
    window->global<dandrum_ui::VoiceDialogState>().set_mode("auto");
    window->set_voice_visible(true);
    require(find(window,"Low note").accessible_value()&&*find(window,"Low note").accessible_value()=="40","Externally requested autosampler opens when the Voice page is created later");
    window->window().dispatch_key_press_event("\x1b");
    require(window->global<dandrum_ui::VoiceDialogState>().get_mode().empty(),"Escape closes the externally requested modal and clears shared UI state");
    require(session.invoke_command("patch.load","hybrid","",0),"Load the native Hybrid patch containing its actual Lush source");
    std::string lush_module;
    for(const auto& module:model->records("module"))if(module.text("name")=="Lush")lush_module=module.id;
    require(!lush_module.empty(),"Hybrid Voice includes its actual initial Lush Synth patch in the native chain");
    for(const auto& [label,field,expected]:std::initializer_list<std::tuple<const char*,const char*,double>>{
        {"Lush Level","gain",-6},{"Lush Pan","pan",0},{"Lush Tune","pitch",0},
        {"Lush Detune","detune",.34},{"Lush Brightness","brightness",.6},{"Lush Width","width",.7}}){
        const auto parameter=model->parameterInfo("module:"+lush_module+":"+field);
        require(parameter&&near(parameter->number("value"),expected)&&find(window,label).accessible_value().has_value(),std::string("Initial Lush guide control exposes its actual native value: ")+label);
    }
    activate(window,"Lush mode Drone");
    require(model->find("module",lush_module)->text("mode")=="drone","Initial Lush mode is a real persisted native control");
    require(session.invoke_command("undo","","",0)&&model->find("module",lush_module)->text("mode")=="notes","Initial Lush mode remains undoable");
    find(window,"Lush Width").set_accessible_value("25");
    require(near(model->parameterInfo("module:"+lush_module+":width")->number("value"),.25),"Initial Lush Width field converts displayed percent to its native parameter");
    require(session.invoke_command("undo","","",0)&&near(model->parameterInfo("module:"+lush_module+":width")->number("value"),.7),"Initial Lush Width Undo restores the actual guide value");
    bool found_lush=false;
    for(std::size_t i=0;i<session.get_sources()->row_count();++i){
        const auto source=session.get_sources()->row_data(i);
        if(source&&source->id=="lush"){window->set_context_source(*source);found_lush=true;break;}
    }
    require(found_lush&&window->get_context_source().kind=="synth-patch","SourceRow uses actual native typed Lush metadata");
    window->set_show_context_source(true);
    bool opened_source_menu=false;
    for(const auto& row:slint::testing::ElementHandle::find_by_accessible_label(window,"Preview Lush"))
        if(row.accessible_role()==slint::language::AccessibleRole::Button){
            const auto p=row.absolute_position();const auto s=row.size();
            const slint::LogicalPosition position{{p.x+s.width/2,p.y+s.height/2}};
            window->window().dispatch_pointer_press_event(position,slint::PointerEventButton::Right);
            window->window().dispatch_pointer_release_event(position,slint::PointerEventButton::Right);
            opened_source_menu=true;break;
        }
    require(opened_source_menu,"Actual Lush SourceRow accepts its pointer context gesture");
    bool activated_sample=false;
    for(const auto& action:slint::testing::ElementHandle::find_by_accessible_label(window,"Sample source Lush"))
        if(action.accessible_role()==slint::language::AccessibleRole::Button){action.invoke_accessible_default_action();activated_sample=true;break;}
    require(activated_sample,"The actual SourceRow popup exposes its native sample action");
    require(model->text("selected-source")=="lush"&&model->text("page")=="voice"&&window->global<dandrum_ui::VoiceDialogState>().get_mode()=="auto","Synth context action selects the actual source, opens Voice and shares modal state; source="+model->text("selected-source")+", page="+model->text("page")+", modal="+std::string(window->global<dandrum_ui::VoiceDialogState>().get_mode()));
    require(find(window,"Low note").accessible_value().has_value(),"Actual SourceRow context action opens native autosampler controls");
    find(window,"Tail").set_accessible_value("5");
    require(model->number("autosampler:tail")==5,"Source context autosampler controls persist native render configuration");
    window->window().dispatch_key_press_event("\x1b");
    require(window->global<dandrum_ui::VoiceDialogState>().get_mode().empty(),"Escape closes the SourceRow-requested autosampler");
    window->set_show_context_source(false);
    require(session.invoke_command("state.select","voice","",0),"Load native Voice descriptor coverage fixture");
    window->window().set_size(slint::PhysicalSize{{700, 2000}});
    window->window().dispatch_resize_event(slint::LogicalSize{{700, 2000}});
    std::string descriptor_errors;
    const auto exercise_cancel = [&](std::string label,std::string id,std::string table,std::string owner) {
        for(const bool window_loss:{false,true}){
            const auto control=find(window,label);const auto p=control.absolute_position();const auto s=control.size();
            const slint::LogicalPosition start{{p.x+s.width/2,p.y+s.height/2}},finish{{start.x,start.y-10}};
            const auto fields=model->find(table,owner)->fields;const auto value=model->parameterInfo(id)->number("value");const auto undo=model->undoCount();
            window->window().dispatch_pointer_press_event(start,slint::PointerEventButton::Left);
            window->window().dispatch_pointer_move_event(finish);
            require(!near(model->parameterInfo(id)->number("value"),value)&&model->undoCount()==undo,"Actual native Voice knob preview changes its model before cancellation: "+id);
            if(window_loss)window->window().dispatch_window_active_changed_event(false);
            else window->window().dispatch_key_press_event("\x1b");
            require(model->find(table,owner)->fields==fields&&near(model->parameterInfo(id)->number("value"),value)&&model->undoCount()==undo,"Actual Voice knob Escape/windowloss restores its exact captured record without Undo: "+id);
            window->window().dispatch_pointer_release_event(finish,slint::PointerEventButton::Left);
            if(window_loss)window->window().dispatch_window_active_changed_event(true);
            require(model->find(table,owner)->fields==fields&&model->undoCount()==undo,"Pointer release after Voice cancellation cannot commit a stale edit: "+id);
        }
    };
    exercise_cancel("Gain","source-gain","parameter","source-gain");
    const auto exercise_parameter = [&](const auto& p, std::string label, std::size_t occurrence) {
        if (!p.available || p.maximum <= p.minimum) return;
        std::vector<slint::testing::ElementHandle> knobs;
        for (const auto& control : slint::testing::ElementHandle::find_by_accessible_label(window,label))
            if (control.accessible_role()==slint::language::AccessibleRole::Slider) knobs.push_back(control);
        std::sort(knobs.begin(),knobs.end(),[](const auto& a,const auto& b){const auto x=a.absolute_position(),y=b.absolute_position();return x.y==y.y?x.x<y.x:x.y<y.y;});
        require(occurrence<knobs.size(),"Every editable Voice descriptor has its actual native control: "+std::string(p.id));
        const auto control=knobs[occurrence];const auto original=model->parameterInfo(p.id.data())->number("value");
        const auto step=p.step>0?p.step:.001f;
        double desired=p.minimum+std::round((p.maximum-p.minimum)*.37/step)*step;
        if(near(desired,original)||near(desired,p.default_value))desired=p.minimum+std::round((p.maximum-p.minimum)*.63/step)*step;
        const double scale=p.unit=="%"&&p.id!="pan"?100:1;const auto undo=model->undoCount();
        control.set_accessible_value(slint::SharedString(std::to_string(desired*scale)));
        require(near(model->parameterInfo(p.id.data())->number("value"),desired)&&model->undoCount()==undo+1,"Every Voice descriptor typed commit writes its native value as one Undo: "+std::string(p.id));
        const auto origin=control.absolute_position();const auto size=control.size();
        const slint::LogicalPosition centre{{origin.x+size.width/2,origin.y+size.height/2}};
        window->window().dispatch_pointer_press_event(centre,slint::PointerEventButton::Middle);
        window->window().dispatch_pointer_release_event(centre,slint::PointerEventButton::Middle);
        ++checks;
        if(!near(model->parameterInfo(p.id.data())->number("value"),p.default_value)||model->undoCount()!=undo+2)
            descriptor_errors+=std::string(p.id)+" expected="+std::to_string(p.default_value)+", actual="+std::to_string(model->parameterInfo(p.id.data())->number("value"))+"; ";
        require(session.invoke_command("undo","","",0)&&near(model->parameterInfo(p.id.data())->number("value"),desired),"Voice reset Undo restores its actual typed edit: "+std::string(p.id));
        require(session.invoke_command("undo","","",0)&&near(model->parameterInfo(p.id.data())->number("value"),original),"Voice commit Undo restores its initial effective value: "+std::string(p.id));
    };
    std::map<std::string,std::size_t> occurrences;std::size_t descriptors=0;
    for(const auto& group:{session.get_source_parameters(),session.get_pitch_parameters(),session.get_amp_parameters(),session.get_filter_parameters(),session.get_amp_envelope_parameters(),session.get_filter_envelope_parameters()})
        for(std::size_t i=0;i<group->row_count();++i){const auto p=*group->row_data(i);exercise_parameter(p,std::string(p.label),occurrences[std::string(p.label)]++);++descriptors;}
    require(descriptors>=20,"All six Voice parameter groups receive direct commit/reset coverage");
    for(const auto kind:{"Noise","Oscillator","Sampler","Synth patch"}){
        activate(window,"Add source…");activate(window,kind);
        const auto id=model->records("module").back().id;
        for(std::size_t i=0;i<session.get_voice_modules()->row_count();++i){const auto module=*session.get_voice_modules()->row_data(i);if(std::string(module.id)==id)
            for(std::size_t j=0;j<module.parameters->row_count();++j)exercise_parameter(*module.parameters->row_data(j),std::string(module.name)+" "+std::string(module.parameters->row_data(j)->label),0);}
        if(std::string_view(kind)=="Noise")exercise_cancel("Noise Level","module:"+id+":gain","module",id);
        require(session.invoke_command("undo","","",0)&&!model->find("module",id),"All typed source descriptor edits and resets leave source addition independently undoable");
    }
    require(session.invoke_command("patch.load","hybrid","",0),"Load initial Hybrid Lush for every native descriptor reset");
    for(std::size_t i=0;i<session.get_voice_modules()->row_count();++i){const auto module=*session.get_voice_modules()->row_data(i);if(module.id=="lush-voice")
        for(std::size_t j=0;j<module.parameters->row_count();++j)exercise_parameter(*module.parameters->row_data(j),std::string(module.name)+" "+std::string(module.parameters->row_data(j)->label),0);}
    require(session.invoke_command("state.select","voice","",0),"Restore native Voice policy and render fixture");
    activate(window,"Voice policy Mono");
    require(model->text("voice-policy")=="mono","Actual Voice policy choice persists Mono");
    require(session.invoke_command("undo","","",0)&&model->text("voice-policy")=="poly","Voice policy native Undo restores Poly");
    find(window,"Voice limit").set_accessible_value("12");
    require(model->number("voice-limit")==12,"Actual Voice limit field persists its native limit");
    require(session.invoke_command("undo","","",0),"Voice limit is independently undoable");
    activate(window,"Steal policy Quietest");
    require(model->text("voice-steal")=="quietest","Actual Steal policy choice persists Quietest");
    require(session.invoke_command("undo","","",0)&&model->text("voice-steal")=="oldest","Steal policy Undo restores Oldest");
    require(session.invoke_command("voice.same-note","","release",0),"Set a distinct Same note policy for native action proof");
    activate(window,"Retrigger");
    require(model->text("same-note")=="retrigger","Actual Retrigger action changes the native Same note policy");
    require(session.invoke_command("undo","","",0)&&model->text("same-note")=="release","Same note action remains undoable");
    for(const auto close_label:{"Close render dialog","scrim"}){
        activate(window,"Sample patch…");
        const auto close_controls=slint::testing::ElementHandle::find_by_accessible_label(window,"Close render dialog");
        require(!close_controls.empty(),"An actual modal header anchors its inner body pointer test");
        const auto header=close_controls[0].absolute_position();
        const slint::LogicalPosition body{{header.x+12,header.y+72}};
        window->window().dispatch_pointer_press_event(body,slint::PointerEventButton::Left);
        window->window().dispatch_pointer_release_event(body,slint::PointerEventButton::Left);
        require(window->global<dandrum_ui::VoiceDialogState>().get_mode()=="auto","Actual inner modal body consumes its pointer without dismissing the dialog");
        const auto count=model->records("source").size();activate(window,"Render");
        if(std::string_view(close_label)=="scrim"){
            const slint::LogicalPosition scrim{{2,2}};
            window->window().dispatch_pointer_press_event(scrim,slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(scrim,slint::PointerEventButton::Left);
        }else activate(window,close_label);
        require(window->global<dandrum_ui::VoiceDialogState>().get_mode().empty()&&model->text("freeze-state")=="idle",std::string("Actual modal dismissal cancels its running native job: ")+close_label);
        model->tick(1);binding->refresh();
        require(model->records("source").size()==count,"Dismissed native render cannot publish sources after its cancellation");
    }
    activate(window,"Freeze…");activate(window,"Freeze includes Source only");
    require(model->text("freeze-mode")=="src","Set a distinct native source-only mode before the Chain action");
    activate(window,"Freeze includes Source + voice + inserts");
    require(model->text("freeze-mode")=="chain","Actual Freeze chain scope selection persists the complete chain mode");
    activate(window,"Cancel render dialog");
    for(const auto& [label,choice]:std::initializer_list<std::pair<const char*,const char*>>{{"Poly","poly"},{"Mono","mono"},{"Legato","legato"}}){
        const auto previous=std::string(choice)=="poly"?"mono":"poly";
        require(session.invoke_command("voice.policy","",previous,0),"Set distinct native policy before actual choice");
        const auto undo=model->undoCount();activate(window,std::string("Voice policy ")+label);
        require(model->text("voice-policy")==choice&&model->undoCount()==undo+1,"Every actual Voice policy choice writes the intended native setting: "+std::string(label));
        require(session.invoke_command("undo","","",0)&&model->text("voice-policy")==previous,"Every actual Voice policy choice Undo restores its prior setting: "+std::string(label));
    }
    for(const auto& [label,choice]:std::initializer_list<std::pair<const char*,const char*>>{{"Oldest","oldest"},{"Quietest","quietest"},{"No steal","none"}}){
        const auto previous=std::string(choice)=="oldest"?"quietest":"oldest";
        require(session.invoke_command("voice.steal","",previous,0),"Set distinct native steal before actual choice");
        const auto undo=model->undoCount();activate(window,std::string("Steal policy ")+label);
        require(model->text("voice-steal")==choice&&model->undoCount()==undo+1,"Every actual Steal policy choice writes the intended native setting: "+std::string(label));
        require(session.invoke_command("undo","","",0)&&model->text("voice-steal")==previous,"Every actual Steal policy choice Undo restores its prior setting: "+std::string(label));
    }
    for(const auto limit:{1,128}){
        const auto previous=model->number("voice-limit");const auto undo=model->undoCount();find(window,"Voice limit").set_accessible_value(slint::SharedString(std::to_string(limit)));
        require(model->number("voice-limit")==limit&&model->undoCount()==undo+1,"Actual Voice limit boundary writes its native setting: "+std::to_string(limit));
        require(session.invoke_command("undo","","",0)&&model->number("voice-limit")==previous,"Actual Voice limit boundary Undo restores its native setting");
    }
    for(const auto& [label,choice]:std::initializer_list<std::pair<const char*,const char*>>{{"Once","once"},{"Gated","gated"},{"Loop","loop"}}){
        const auto previous=std::string(choice)=="once"?"gated":"once";
        require(session.invoke_command("sample.mode","",previous,0),"Set distinct playback before actual source choice");
        const auto undo=model->undoCount();activate(window,std::string("Source playback ")+label);
        require(model->text("play-mode")==choice&&model->undoCount()==undo+1,"Every actual Source playback choice writes the intended native setting: "+std::string(label));
        require(session.invoke_command("undo","","",0)&&model->text("play-mode")==previous,"Every actual Source playback choice Undo restores its prior setting: "+std::string(label));
    }
    for(const auto& [label,choice]:std::initializer_list<std::pair<const char*,const char*>>{{"LP","lp"},{"HP","hp"},{"BP","bp"},{"Notch","notch"}}){
        const auto previous=std::string(choice)=="lp"?"hp":"lp";
        require(session.invoke_command("filter.mode","",previous,0),"Set distinct filter before actual source choice");
        const auto undo=model->undoCount();activate(window,std::string("Filter type ")+label);
        require(model->text("filter-mode")==choice&&model->undoCount()==undo+1,"Every actual Voice filter choice writes the intended native mode: "+std::string(label));
        require(session.invoke_command("undo","","",0)&&model->text("filter-mode")==previous,"Every actual Voice filter choice Undo restores its prior mode: "+std::string(label));
    }
    for(const auto& [kind,choices]:std::initializer_list<std::pair<const char*,std::vector<std::pair<const char*,const char*>>>>{
        {"Noise",{{"White","white"},{"Pink","pink"}}},{"Oscillator",{{"Sine","sine"},{"Saw","saw"},{"Square","square"}}},
        {"Sampler",{{"Once","once"},{"Gated","gated"},{"Loop","loop"}}},{"Synth patch",{{"Follows notes","notes"},{"Drone","drone"}}}}){
        activate(window,"Add source…");activate(window,kind);const auto id=model->records("module").back().id;const auto name=model->find("module",id)->text("name");
        for(const auto& [label,choice]:choices){const auto previous=std::string(choice)==choices[0].second?choices[1].second:choices[0].second;
            const auto setupUndo=model->undoCount();require(session.invoke_command("module.mode",slint::SharedString(id),previous,0),"Set distinct typed mode before native source choice");
            const auto undo=model->undoCount();activate(window,name+" mode "+label);
            require(model->find("module",id)->text("mode")==choice&&model->undoCount()==undo+1,"Every typed Source mode choice persists its native record: "+name+" "+label);
            require(session.invoke_command("undo","","",0)&&model->find("module",id)->text("mode")==previous,"Every typed Source mode choice Undo restores its native record: "+name+" "+label);
            if(model->undoCount()>setupUndo)require(session.invoke_command("undo","","",0),"Remove distinct mode setup without folding source-add Undo");
        }
        require(session.invoke_command("undo","","",0)&&!model->find("module",id),"Typed source addition remains independently undoable after every mode choice");
    }
    const auto moduleCount=model->records("module").size();const auto moduleUndo=model->undoCount();activate(window,"Add voice module");
    const auto addedModule=model->records("module").back().id;
    require(model->records("module").size()==moduleCount+1&&model->find("module",addedModule)->text("detail")=="Waveshaper"&&model->undoCount()==moduleUndo+1,"Actual Voice module Plus adds its native Waveshaper as one Undo");
    std::vector<dandrum_ui::ModuleInfo> chain;for(std::size_t i=0;i<session.get_voice_modules()->row_count();++i)chain.push_back(*session.get_voice_modules()->row_data(i));
    for(const auto& module:chain){activate(window,std::string(module.name)+"  →");
        require(session.get_selected_id()==module.id&&session.get_inspector_type()=="Module","Every actual Voice chain button selects its matching native module and Inspector: "+std::string(module.id));}
    require(session.invoke_command("undo","","",0)&&model->records("module").size()==moduleCount&&!model->find("module",addedModule),"Undo of actual Voice module Plus restores the native chain inventory");
    const auto freeze_source=std::string(session.get_selected_source());const auto original_record=model->find("source",freeze_source)->fields;
    const auto freeze_undo=model->undoCount();activate(window,"Freeze…");activate(window,"Freeze");
    require(model->text("freeze-state")=="running"&&!session.get_voice_frozen()&&model->undoCount()==freeze_undo,"Actual Voice Freeze starts asynchronously without premature source mutation or Undo");
    model->tick(.25);binding->refresh();
    require(model->text("freeze-state")=="running"&&model->number("render-progress")>0&&model->number("render-progress")<1&&!session.get_voice_frozen(),"Actual Voice Freeze reports in-flight native progress before completion");
    model->tick(1);binding->refresh();activate(window,"Cancel render dialog");
    require(session.get_voice_frozen()&&model->find("source",freeze_source)->text("kind")=="frozen"&&model->undoCount()==freeze_undo+1,"Actual Voice Freeze completion changes its selected source in exactly one Undo");
    for(const auto& [label,key]:std::initializer_list<std::pair<const char*,const char*>>{{"Voice policy Mono","voice-policy"},{"Steal policy Quietest","voice-steal"},{"Retrigger","same-note"}}){
        const auto before=model->text(key);const auto controls=slint::testing::ElementHandle::find_by_accessible_label(window,label);
        bool disabled=false;for(const auto& control:controls)if(control.accessible_role()==slint::language::AccessibleRole::Button)disabled=control.accessible_enabled()&&! *control.accessible_enabled();
        activate(window,label);
        require(disabled&&model->text(key)==before&&model->undoCount()==freeze_undo+1,"Actual frozen Voice policy rejects editing and preserves its native setting/Undo: "+std::string(label));
    }
    const auto frozenLimit=find(window,"Voice limit");const auto originalLimit=model->number("voice-limit");frozenLimit.set_accessible_value("12");
    require(frozenLimit.accessible_enabled()&&! *frozenLimit.accessible_enabled()&&model->number("voice-limit")==originalLimit&&model->undoCount()==freeze_undo+1,"Actual frozen Voice limit rejects editing and preserves its native setting/Undo");
    require(session.invoke_command("undo","","",0)&&!session.get_voice_frozen()&&model->find("source",freeze_source)->fields==original_record,"One Undo of actual Voice Freeze restores the exact original source record");
    const auto restoredPolicy=model->text("voice-policy");const auto policyUndo=model->undoCount();activate(window,"Voice policy Mono");
    require(model->text("voice-policy")=="mono"&&model->undoCount()==policyUndo+1,"Actual Freeze Undo restores native Voice policy editing");
    require(session.invoke_command("undo","","",0)&&model->text("voice-policy")==restoredPolicy,"Policy editing restored after Freeze Undo remains independently undoable");
    require(descriptor_errors.empty(),"Every Voice descriptor actual reset restores its own native default: "+descriptor_errors);
    window->window().hide();std::cout<<"PASS: "<<checks<<" actual native voice plot checks\n";
}catch(const std::exception& error){std::cerr<<"FAIL: "<<error.what()<<'\n';return 1;}}
