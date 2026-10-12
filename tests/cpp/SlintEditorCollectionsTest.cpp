#include "SlintEditorCollections.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>

int main(int argc,char** argv) {
    int checks=0;
    const auto require=[&](bool ok,const std::string& message){++checks;if(!ok)throw std::runtime_error(message);};
    try {
        const bool render=argc==2;if(!render)slint::testing::init();
        auto window=dandrum_ui::SlintEditorCollections::create();
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(window->global<dandrum_ui::ValueCodec>());
        std::vector<dandrum_ui::PadInfo> data(128);
        for(int i=0;i<128;++i){data[i].note=i;data[i].choke_group=-1;data[i].active_alternate=-1;}
        data[60].mapped=true;data[60].name="Independent pad";
        window->set_pads(std::make_shared<slint::VectorModel<dandrum_ui::PadInfo>>(data));
        window->global<dandrum_ui::EditorTheme>().set_paper_2(slint::Color::from_argb_uint8(255,192,192,192));
        window->show();
        if(render){
            std::string failure;slint::Timer timer;timer.start(slint::TimerMode::SingleShot,std::chrono::milliseconds(150),[&]{
                try{auto image=window->window().take_snapshot();require(image.has_value(),"Library-only collections render native pixels");
                    auto key=image->begin()[60*image->width()+30],body=image->begin()[234*image->width()+24];
                    require(key.r==192&&key.g==192&&key.b==192,"Fitted key collection uses the supplied mapped white-key color");
                    require(body.r==35&&body.g==69&&body.b==103,"Pad panel places consumer-provided content at the preserved body inset");
                    std::ofstream out(std::string(argv[1])+"/shared-collections.pam",std::ios::binary);out<<"P7\nWIDTH "<<image->width()<<"\nHEIGHT "<<image->height()<<"\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";out.write(reinterpret_cast<const char*>(image->begin()),image->width()*image->height()*4);
                }catch(const std::exception& error){failure=error.what();}slint::quit_event_loop();
            });window->run();require(failure.empty(),failure);
        }else{
            const auto settle=[&]{slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
            const auto element=[&](const char* label){settle();for(auto e:slint::testing::ElementHandle::find_by_accessible_label(window,label))if(e.accessible_role().has_value())return e;throw std::runtime_error(std::string("Missing reusable collection input: ")+label);};
            const auto point=[&](const char* label){auto e=element(label);auto p=e.absolute_position();auto s=e.size();return slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};};
            const auto click=[&](const char* label){auto p=point(label);window->window().dispatch_pointer_press_event(p,slint::PointerEventButton::Left);window->window().dispatch_pointer_release_event(p,slint::PointerEventButton::Left);settle();};
            const auto key=[&](slint::SharedString text){window->window().dispatch_key_press_event(text);window->window().dispatch_key_release_event(text);settle();};
            require(window->get_actual_key_width()==48&&window->get_last_key_width()==48,"Fitted mapped keyboard honors its48pixel maximum and publishes actual width");
            window->invoke_zoom_keys(-1);settle();require(window->get_key_width()==44&&window->get_actual_key_width()==44,"Keyboard zoom uses caller-owned width state");
            window->invoke_fit_keys();settle();require(window->get_key_width()==0&&window->get_actual_key_width()==48,"Fit resets width state and recomputes from mapped endpoints");
            auto from=point("Key 60 C4");window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);settle();
            require(window->get_presses()==1&&window->get_releases()==0,"Keyboard collection forwards an owned note press");
            window->set_generation("two");settle();require(window->get_releases()==1,"Collection generation invalidation releases its captured key once");
            window->window().dispatch_pointer_release_event(from,slint::PointerEventButton::Left);settle();require(window->get_releases()==1,"Late release cannot duplicate the invalidated input");
            const auto fitted=window->get_actual_pad_size();window->invoke_zoom_pads(1);settle();
            require(window->get_pad_size()==fitted+8&&window->get_pad_start()==60,"Pad zoom publishes its size and captures mapped start");
            window->invoke_fit_pads();settle();require(window->get_pad_size()==0&&window->get_pad_start()==-1&&window->get_actual_pad_size()==fitted,"Pad fit restores caller state and the computed grid");
            click("Pad 60 C4, Independent pad");require(window->get_presses()==2&&window->get_releases()==2&&window->get_selections()==1,"Mapped pad collection forwards balanced note input and selection");
            click("Pad 61 C♯4, empty");require(window->get_presses()==2&&window->get_selections()==2,"Empty pad collection selects without emitting a note");
            click("Range start Independent range");key(slint::platform::key_codes::Return);key("7");key("0");key(slint::platform::key_codes::Return);
            require(std::abs(window->get_lower()-.7f)<.001&&window->get_range_begins()==window->get_range_ends(),"Binding range editor converts its percentage edit into balanced caller-owned range callbacks");
            window->set_lower(.3f);settle();key(slint::platform::key_codes::UpArrow);
            require(std::abs(window->get_lower()-.35f)<.001,"External range projection remains authoritative after a local edit replaces the value binding");
            click("Go to Independent range");require(window->get_openings()==1,"Binding destination navigation remains a callback");
            auto pad=window->get_selected_pad();pad.mapped=false;window->set_selected_pad(pad);settle();
            require(!slint::testing::ElementHandle::find_by_accessible_label(window,"Independent empty pad").empty(),"Unmapped pad panel uses supplied empty-state copy");
        }
        std::cout<<"PASS: "<<checks<<" library-only collection "<<(render?"pixel":"interaction")<<" checks\n";
    }catch(const std::exception& error){std::cerr<<"FAIL after "<<checks<<" checks: "<<error.what()<<'\n';return 1;}
}
