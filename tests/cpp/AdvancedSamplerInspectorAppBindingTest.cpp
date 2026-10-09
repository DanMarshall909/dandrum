#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/advanced-sampler/theme/AppearanceBinding.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <iostream>
#include <stdexcept>

int main(){int checks=0;try{
    using namespace dandrum_ui;
    slint::testing::init();auto window=AppWindow::create();
    auto model=std::make_shared<dandrum::sampler::Model>();
    auto binding=dandrum::sampler::bindWindowSession<Session>(window,model);
    dandrum::sampler::bind_performance_sources(window->global<PerformanceState>(),binding);
    dandrum::sampler::bind_patch_search(window->global<PatchFilter>());
    dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>());
    dandrum::advanced_sampler::ThemeSettings settings;
    dandrum::advanced_sampler::apply_theme(window->global<SamplerTheme>(),dandrum::advanced_sampler::resolve_theme(settings));
    const auto require=[&](bool ok,const std::string& reason){++checks;if(!ok)throw std::runtime_error(reason);};
    const auto settle=[&]{slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
    const auto pointer=[&](const slint::testing::ElementHandle& e){const auto p=e.absolute_position();const auto s=e.size();const slint::LogicalPosition at{{p.x+s.width/2,p.y+s.height/2}};window->window().dispatch_pointer_press_event(at,slint::PointerEventButton::Left);window->window().dispatch_pointer_release_event(at,slint::PointerEventButton::Left);settle();};
    const auto click=[&](std::string_view label,bool inspector=false){for(int scroll=0;scroll<(inspector?32:1);++scroll){settle();for(const auto& e:slint::testing::ElementHandle::find_by_accessible_label(window,label)){
        const auto p=e.absolute_position();const auto s=e.size();if(e.accessible_role()==slint::language::AccessibleRole::Button&&s.width>0&&p.y>=0&&p.y+s.height<800&&(!inspector||p.x>900)){pointer(e);return;}}
        if(inspector){window->window().dispatch_pointer_scroll_event(slint::LogicalPosition{{1100,300}},0,-200);slint::cbindgen_private::slint_mock_elapsed_time(250);slint::platform::update_timers_and_animations();}}
        throw std::runtime_error("Missing actual composed Inspector/picker button: "+std::string(label));};
    const auto destination=[&](std::string_view label){for(int n=0;n<64;++n){settle();for(const auto& e:slint::testing::ElementHandle::find_by_accessible_label(window,label)){
        const auto p=e.absolute_position();const auto s=e.size();if(e.accessible_role()==slint::language::AccessibleRole::Button&&s.width>0&&p.y>=145&&p.y+s.height<788){pointer(e);return;}}
        window->window().dispatch_pointer_scroll_event(slint::LogicalPosition{{600,300}},0,-240);slint::cbindgen_private::slint_mock_elapsed_time(250);slint::platform::update_timers_and_animations();}
        throw std::runtime_error("Missing actual scrollable Inspector destination: "+std::string(label));};
    window->window().show();window->window().set_size(slint::LogicalSize{{1200,800}});
    require(model->command("state.select","macro"),"Load native macro Inspector for composed assignment");binding->refresh();settle();
    const auto macro=model->text("selected-macro");const auto bindings=model->records("binding").size();const auto macroUndo=model->undoCount();
    click("Add destination…",true);destination("Add macro destination level");
    require(model->find("binding",macro+":level")&&model->records("binding").size()==bindings+1&&model->find("binding",macro+":level")->number("amount")==.25&&model->undoCount()==macroUndo+1,"Actual Inspector pointer action completes the composed macro picker and creates exactly its native destination binding");
    require(model->command("undo")&&!model->find("binding",macro+":level")&&model->records("binding").size()==bindings,"Undo of actual composed Inspector macro assignment restores the native binding inventory");binding->refresh();settle();
    require(model->command("state.select","voice"),"Load native module Inspector for composed modulation assignment");binding->refresh();settle();
    const auto routes=model->records("route").size();const auto routeUndo=model->undoCount();
    click("Assign modulation…",true);click("LFO 1");
    require(model->records("route").size()==routes+1&&model->records("route").back().text("source")=="lfo1"&&model->records("route").back().text("destination")=="cutoff"&&model->records("route").back().number("amount")==.25&&model->undoCount()==routeUndo+1,"Actual Inspector pointer action completes the composed modulation-source picker and creates its exact native cutoff route");
    require(model->command("undo")&&model->records("route").size()==routes,"Undo of actual composed Inspector modulation assignment restores the native route inventory");
    window->window().hide();std::cout<<"PASS: "<<checks<<" actual composed Inspector assignment checks\n";
}catch(const std::exception& e){std::cerr<<"FAIL: "<<e.what()<<'\n';return 1;}}
