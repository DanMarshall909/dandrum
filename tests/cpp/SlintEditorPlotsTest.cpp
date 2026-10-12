#include "SlintEditorPlots.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <array>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>

int main(int argc, char** argv) {
    int checks = 0;
    const auto require = [&](bool ok, const std::string& message) { ++checks; if (!ok) throw std::runtime_error(message); };
    try {
        const bool render = argc == 2;
        if (!render) slint::testing::init();
        auto window = dandrum_ui::SlintEditorPlots::create();
        window->show();
        const auto find = [&](std::string_view label) {
            for (auto item : slint::testing::ElementHandle::find_by_accessible_label(window,label))
                if (item.accessible_role() == slint::language::AccessibleRole::Slider) return item;
            throw std::runtime_error("Missing independent plot: " + std::string(label));
        };
        if (render) {
            int phase = 0;
            std::string failure;
            const auto bright = [](const auto& image,int x,int y) {
                if (x < 0 || y < 0 || x >= static_cast<int>(image.width()) || y >= static_cast<int>(image.height())) return false;
                auto p = image.begin()[y*image.width()+x];
                return p.r > 145 && p.g > 145 && p.b > 135 && std::abs(int(p.r)-int(p.g)) < 35;
            };
            const auto near_line = [&](const auto& image,float x,float y) {
                for (int dy=-2;dy<=2;++dy) for(int dx=-1;dx<=1;++dx)
                    if (bright(image,std::lround(x)+dx,std::lround(y)+dy)) return true;
                return false;
            };
            slint::Timer timer;
            timer.start(slint::TimerMode::Repeated,std::chrono::milliseconds(150),[&] {
                try {
                    auto snapshot=window->window().take_snapshot(); require(snapshot.has_value(),"Library-only graph consumer produces native renderer pixels");
                    std::ofstream output(std::string(argv[1])+"/shared-plots-"+std::to_string(phase)+".pam",std::ios::binary);
                    output << "P7\nWIDTH " << snapshot->width() << "\nHEIGHT " << snapshot->height() << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
                    output.write(reinterpret_cast<const char*>(snapshot->begin()),snapshot->width()*snapshot->height()*4);
                    const float low_x=10+8+(600-16)*.2f,low_y=10+4+(150-8)*.22f;
                    if(phase==0){
                        int count=0; for(int y=27;y<143;++y) count+=bright(*snapshot,std::lround(low_x),y);
                        require(count<=5&&near_line(*snapshot,low_x,low_y),"LP renders exactly one response through the independently known low-frequency zero dB position");
                        auto a=find("Independent envelope sustain handle").absolute_position(),b=find("Independent envelope release handle").absolute_position();
                        require(near_line(*snapshot,(a.x+b.x)/2+5.5f,(a.y+b.y)/2+5.5f),"Envelope release segment joins the supplied sustain/release positions");
                    }else if(phase==3){
                        const float cutoff_x=10+8+(600-16)*std::log(window->get_cutoff()/20)/std::log(1000);
                        require(!near_line(*snapshot,cutoff_x,low_y),"Notch response suppresses its supplied cutoff frequency");
                    }else require(!near_line(*snapshot,low_x,low_y),"HP/BP change the independently known low-frequency LP response");
                    if(++phase==4){timer.stop();slint::quit_event_loop();}
                    else window->set_mode(phase==1?"hp":phase==2?"bp":"notch");
                }catch(const std::exception& error){failure=error.what();timer.stop();slint::quit_event_loop();}
            });
            window->run();require(failure.empty(),failure);
        } else {
            const auto settle=[&]{slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
            const auto key=[&](slint::SharedString text){window->window().dispatch_key_press_event(text);window->window().dispatch_key_release_event(text);settle();};
            const auto filter_point=[](float x,float y){return slint::LogicalPosition{{18+584*x,14+142*y}};};
            auto from=filter_point(.5,.5),to=filter_point(.75,.2);
            window->set_accept_begin(false);window->invoke_reset_counts();
            window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);window->window().dispatch_pointer_move_event(to);window->window().dispatch_pointer_release_event(to,slint::PointerEventButton::Left);settle();
            require(window->get_begins()==1&&window->get_updates()==0&&window->get_related_updates()==0&&window->get_ends()==0&&!window->get_filter_dragging(),"Rejected begin emits no primary/related preview or ending");
            window->set_accept_begin(true);window->invoke_reset_counts();
            window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);window->window().dispatch_pointer_move_event(to);settle();
            require(std::abs(window->get_cutoff()-20*std::pow(1000,.75))<.01&&std::abs(window->get_resonance()-2.f/3)<.001,"Native filter drag retains known logarithmic cutoff and coupled resonance mapping");
            require(window->get_begins()==1&&window->get_updates()==2&&window->get_related_updates()==2&&window->get_ends()==0,"Filter previews bracket one accepted host transaction");
            key(slint::platform::key_codes::Escape);
            require(window->get_cancels()==1&&!window->get_filter_dragging()&&window->get_cutoff()==2400&&std::abs(window->get_resonance()-.22)<.001,"Escape lets the consumer roll both dimensions back");
            window->window().dispatch_pointer_release_event(to,slint::PointerEventButton::Left);settle();
            require(window->get_ends()==0,"Pointer release after cancellation cannot commit");
            window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);window->window().dispatch_pointer_move_event(to);window->window().dispatch_pointer_release_event(to,slint::PointerEventButton::Left);settle();
            require(window->get_ends()==1&&!window->get_filter_dragging(),"Pointer release commits exactly one coupled transaction");
            window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);window->set_plot_enabled(false);settle();
            require(window->get_cancels()==2&&!window->get_filter_dragging(),"Disabling a captured filter cancels its accepted transaction");
            window->window().dispatch_pointer_release_event(from,slint::PointerEventButton::Left);window->set_plot_enabled(true);settle();
            const std::array<float,4> saved{15,180,.6f,500};
            for(int index=0;index<4;++index){
                window->invoke_reset_counts();window->set_accept_begin(false);window->invoke_envelope_start(index);window->invoke_envelope_update(index,.4,.2);window->invoke_envelope_finish(index,false);
                require(window->get_begins()==1&&window->get_updates()==0&&window->get_ends()==0&&!window->get_envelope_dragging(),"Each envelope handle respects host begin refusal");
                window->set_accept_begin(true);window->invoke_envelope_start(index);window->invoke_envelope_update(index,index==3?.9:.4,.2);
                require(window->get_updates()==1&&window->get_last_index()==index&&window->get_envelope_dragging(),"Each envelope handle reports its supplied parameter index");
                const float expected=index==0?240:index==1?(.4f-.025f)*2111:index==2?.8f:(.9f-220.f/300)*4500;
                require(std::abs(window->get_last_value()-expected)<.02,"Envelope point uses independently calibrated time/sustain mapping");
                if(index==1)require(window->get_related_updates()==1&&std::abs(window->get_related_value()-.8f)<.001,"Decay handle reports its paired sustain preview separately");
                window->invoke_envelope_finish(index,true);settle();
                require(window->get_cancels()==1&&!window->get_envelope_dragging()&&std::abs(window->get_envelope_parameters()->row_data(index)->value-saved[index])<.001,"Cancelled handle lets its consumer restore supplied parameter data");
                window->invoke_envelope_start(index);window->invoke_envelope_finish(index,false);
                require(window->get_cancels()==2&&window->get_ends()==0,"A click without modification cancels without a commit");
            }
            window->invoke_reset_counts();window->invoke_envelope_start(0);window->invoke_envelope_update(0,.3,0);window->invoke_envelope_finish(0,false);
            require(window->get_ends()==1&&!window->get_envelope_dragging(),"Modified envelope commits once");
            window->invoke_envelope_start(0);window->set_plot_enabled(false);settle();
            require(window->get_cancels()==1&&!window->get_envelope_dragging(),"Disabled envelope cancels its captured gesture");
            window->invoke_reset_counts();window->invoke_envelope_start(0);window->invoke_envelope_update(0,.3,0);window->invoke_filter_update(.5,.5);
            require(window->get_begins()==0&&window->get_updates()==0&&window->get_related_updates()==0,"Disabled plots reject imperative entry as well as pointer input");
            window->set_plot_enabled(true);window->set_envelope_parameters(std::make_shared<slint::VectorModel<dandrum_ui::PlotParameter>>());settle();window->invoke_reset_counts();window->invoke_envelope_start(0);
            require(window->get_begins()==0&&!window->get_envelope_dragging(),"Incomplete envelope input is inert and never indexes nonexistent parameters");
        }
        std::cout<<"PASS: "<<checks<<" library-only graph "<<(render?"pixel":"interaction")<<" checks\n";
    }catch(const std::exception& error){std::cerr<<"FAIL after "<<checks<<" checks: "<<error.what()<<'\n';return 1;}
}
