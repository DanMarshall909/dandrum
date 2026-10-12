#include "SlintEditorChrome.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <chrono>
#include <fstream>
#include <iostream>
#include <stdexcept>

int main(int argc,char** argv) {
    int checks=0;
    const auto require=[&](bool ok,const std::string& message){++checks;if(!ok)throw std::runtime_error(message);};
    try {
        const bool render=argc==2;if(!render)slint::testing::init();
        auto window=dandrum_ui::SlintEditorChrome::create();
        const auto color=[](unsigned rgb){return slint::Color::from_argb_uint8(255,rgb>>16,rgb>>8,rgb);};
        auto& theme=window->global<dandrum_ui::EditorTheme>();theme.set_accent(color(0xffbb22));theme.set_accent_wash(color(0x223344));theme.set_ink_3(color(0x334455));theme.set_paper_1(color(0xffffff));theme.set_ok(color(0x22cc55));theme.set_warn(color(0xee4433));
        window->show();window->window().dispatch_window_active_changed_event(true);
        if(render) {
            int phase=0;std::string failure;slint::Timer timer;timer.start(slint::TimerMode::Repeated,std::chrono::milliseconds(150),[&]{
                try {
                    auto image=window->window().take_snapshot();require(image.has_value(),"Standalone editor chrome renders native pixels");
                    std::ofstream output(std::string(argv[1])+"/shared-chrome-"+std::to_string(phase)+".pam",std::ios::binary);
                    output<<"P7\nWIDTH "<<image->width()<<"\nHEIGHT "<<image->height()<<"\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
                    output.write(reinterpret_cast<const char*>(image->begin()),image->width()*image->height()*4);
                    const auto green=[&](int x,int y){auto p=image->begin()[y*image->width()+x];return p.r==0&&p.g==255&&p.b==0;};
                    require(green(120,195)&&green(205,195)&&!green(215,195),"Linear meter paints exactly the supplied first half");
                    require(!green(345,195)&&green(380,195)&&green(420,195)&&!green(435,195),"Centered linear meter preserves the original implicit rectangle geometry");
                    const auto background=image->begin()[120*image->width()+118];
                    if(phase==0) {
                        require(!green(150,220)&&green(170,220)&&green(205,220)&&!green(215,220),"Negative bipolar meter paints only the interval below its center");
                        require(background.r==34&&background.g==51&&background.b==68,"Selected item paints the supplied semantic selection color");
                        const auto mark=image->begin()[120*image->width()+289];require(mark.r==255&&mark.g==187&&mark.b==34,"Selected item retains the original centered selection-mark geometry");
                        const auto highlighted=image->begin()[155*image->width()+115];require(highlighted.r==34&&highlighted.g==51&&highlighted.b==68,"External drop highlight uses the shared selection paint without selecting the row");
                        const auto progress=image->begin()[134*image->width()+150];require(progress.r==255&&progress.g==255&&progress.b==255,"Preview progress paints the known quarter-length interval");
                        const auto status=image->begin()[294*image->width()+113];require(status.r==34&&status.g==204&&status.b==85,"Status row paints the supplied semantic success color");
                    } else {
                        require(!green(205,220)&&green(215,220)&&green(255,220)&&!green(270,220),"Positive bipolar meter paints only the interval above its center");
                        require(background.r==51&&background.g==68&&background.b==85,"Unselected hovered item paints the supplied hover color");
                        const auto warning=image->begin()[294*image->width()+113];require(warning.r==238&&warning.g==68&&warning.b==51,"Status row paints the supplied warning color when requested");
                    }
                    if(phase++==0){window->set_selected(false);window->set_warning(true);window->set_signed_level(0.5);window->window().dispatch_pointer_move_event(slint::LogicalPosition{{125,120}});return;}
                }catch(const std::exception& error){failure=error.what();}timer.stop();slint::quit_event_loop();
            });window->run();require(failure.empty(),failure);
        } else {
            const auto settle=[] {slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
            const auto element=[&](const char* label){settle();for(auto item:slint::testing::ElementHandle::find_by_accessible_label(window,label))if(item.accessible_role().has_value()&&item.accessible_role()!=slint::language::AccessibleRole::Text)return item;throw std::runtime_error(std::string("Missing ")+label);};
            const auto click=[&](const char* label,slint::PointerEventButton button=slint::PointerEventButton::Left){auto item=element(label);auto p=item.absolute_position();auto s=item.size();slint::LogicalPosition at{{p.x+s.width/2,p.y+s.height/2}};window->window().dispatch_pointer_press_event(at,button);window->window().dispatch_pointer_release_event(at,button);settle();};
            const auto key=[&](slint::SharedString value){window->window().dispatch_key_press_event(value);window->window().dispatch_key_release_event(value);settle();};
            click("Second page");require(window->get_page()=="two","Navigation pointer emits supplied identity");
            key(" ");require(window->get_page()=="two","Navigation Space activates focused item");
            element("First page").invoke_accessible_default_action();require(window->get_page()=="one","Navigation accessibility emits supplied identity");
            key(slint::platform::key_codes::Return);require(window->get_page()=="two","Navigation Return activates the focused item after external selection");
            window->set_compact(true);click("Second page");require(window->get_page()=="two","Compact navigation retains the same identity contract");
            click("Name");element("Name").set_accessible_value("Updated");key(slint::platform::key_codes::Return);
            require(window->get_name()=="Updated"&&window->get_accepts()==1,"Themed text field binds edits and accepts Return");
            key(slint::platform::key_codes::Escape);require(window->get_cancels()==1,"Explicitly cancellable field delegates Escape");
            auto blurs=window->get_blurs();window->window().dispatch_window_active_changed_event(false);settle();require(window->get_blurs()==blurs+1,"Text field forwards focus loss when the native window deactivates");window->window().dispatch_window_active_changed_event(true);
            window->set_input_read_only(true);auto name=window->get_name();key("X");require(window->get_name()==name,"Readonly text cannot be changed with keyboard input");window->set_input_read_only(false);
            click("Search items");element("Search items").set_accessible_value("needle");settle();require(window->get_query()=="needle","Search accessible edits update the supplied text: "+std::string(window->get_query()));key(slint::platform::key_codes::Return);
            require(window->get_query()=="needle"&&window->get_accepts()==2&&window->get_blurs()>0,"Search field binds value and forwards edit, accept and focus lifecycle: query="+std::string(window->get_query())+", accepts="+std::to_string(window->get_accepts())+", blurs="+std::to_string(window->get_blurs()));
            click("Preview item");require(window->get_picks()==1,"Item row pointer activates once");
            key(" ");key(slint::platform::key_codes::Return);element("Preview item").invoke_accessible_default_action();require(window->get_picks()==4,"Item row shares keyboard and accessible activation");
            click("Preview item",slint::PointerEventButton::Right);require(window->get_contexts()==1&&window->get_picks()==4,"Item context input does not also activate");
            window->set_context_on_release(true);window->window().dispatch_pointer_press_event(slint::LogicalPosition{{150,120}},slint::PointerEventButton::Right);settle();require(window->get_contexts()==1,"Release-mode context waits while the pointer is down");window->window().dispatch_pointer_release_event(slint::LogicalPosition{{150,120}},slint::PointerEventButton::Right);settle();require(window->get_contexts()==2&&window->get_picks()==4,"Release-mode context emits once without activation");
            window->window().dispatch_window_active_changed_event(false);settle();require(window->get_window_losses()==1,"Item row reports window deactivation to its adapter");window->window().dispatch_window_active_changed_event(true);
            click("External item");require(window->get_picks()==5,"External drag input can use the same guarded row activation");
            require(window->get_linear_width()==100&&window->get_signed_x()==50&&window->get_signed_width()==50,"Meters preserve known linear and negative bipolar geometry");
            window->set_centered(true);require(window->get_linear_x()==50&&window->get_linear_width()==100,"Centered linear fill preserves half-scale width and centered origin");
            window->set_level(-1);window->set_signed_level(2);require(window->get_linear_width()==0&&window->get_signed_x()==100&&window->get_signed_width()==100,"Meters clamp out-of-range input without drawing outside tracks");
            window->set_level(2);window->set_signed_level(-2);require(window->get_linear_width()==200&&window->get_signed_x()==0&&window->get_signed_width()==100,"Meters retain full-scale endpoints in both directions");
            window->set_signed_level(0);require(window->get_signed_width()==0&&window->get_signed_x()==100,"Bipolar zero has no fill and stays centered");
            require(!slint::testing::ElementHandle::find_by_accessible_label(window,"Rate: 48 kHz").empty(),"Property row exposes its complete value and unit");
            window->set_unit("");window->set_locked(false);settle();require(!slint::testing::ElementHandle::find_by_accessible_label(window,"Rate: 48").empty(),"Property row omits absent units and optional lock decoration");
            require(!slint::testing::ElementHandle::find_by_accessible_label(window,"Prepare: Ready").empty(),"Status step exposes the supplied detail");
            click("Browse content");click("Load several");require(window->get_primary_actions()==1&&window->get_secondary_actions()==1,"Empty state delegates independent primary and secondary actions");
            window->set_input_enabled(false);click("Preview item");key(" ");key(slint::platform::key_codes::Return);element("Preview item").invoke_accessible_default_action();click("Preview item",slint::PointerEventButton::Right);
            click("External item");require(window->get_picks()==5&&window->get_contexts()==2&&!element("Preview item").accessible_enabled().value_or(true),"Disabled rows block pointer, keyboard, accessibility, context and external input");
            click("First page");element("First page").invoke_accessible_default_action();require(window->get_page()=="two","Disabled rail cannot change page");
            auto query=window->get_query();click("Search items");key("X");require(window->get_query()==query,"Disabled search keeps its supplied value");
            click("Browse content");element("Load several").invoke_accessible_default_action();require(window->get_primary_actions()==1&&window->get_secondary_actions()==1,"Disabled empty state rejects pointer and accessible actions");
        }
        std::cout<<"PASS: "<<checks<<" standalone editor chrome "<<(render?"pixel":"interaction")<<" checks\n";
    }catch(const std::exception& error){std::cerr<<"FAIL after "<<checks<<" checks: "<<error.what()<<'\n';return 1;}
}
