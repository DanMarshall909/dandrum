#include "SlintEditorZoneMap.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <vector>

int main(int argc,char** argv) {
    int checks=0;
    const auto require=[&](bool ok,const std::string& message){++checks;if(!ok)throw std::runtime_error(message);};
    try {
        const bool render=argc==2;
        if(!render)slint::testing::init();
        auto window=dandrum_ui::SlintEditorZoneMap::create();
        const auto color=[](unsigned rgb){return slint::Color::from_argb_uint8(255,rgb>>16,rgb>>8,rgb);};
        auto& theme=window->global<dandrum_ui::EditorTheme>();
        theme.set_accent(color(0xe08a4e));theme.set_accent_wash(color(0x352113));theme.set_ink_0(color(0x130f0c));
        window->show();
        const auto find=[&](std::string_view label){
            for(auto item:slint::testing::ElementHandle::find_by_accessible_label(window,label))if(item.accessible_role().has_value())return item;
            throw std::runtime_error("Missing generic zone input: "+std::string(label));
        };
        if(render){
            std::string failure;
            slint::Timer timer;timer.start(slint::TimerMode::SingleShot,std::chrono::milliseconds(150),[&]{
                try{
                    auto image=window->window().take_snapshot();require(image.has_value(),"Library-only zone map renders native pixels");
                    auto selected=image->begin()[120*image->width()+220],gap=image->begin()[300*image->width()+220];
                    require(selected.r==53&&selected.g==33&&selected.b==19,"Selected zone paints supplied accent-wash at independently known key/velocity coordinates");
                    require(gap.r==19&&gap.g==15&&gap.b==12,"Key/velocity coordinates preserve the unoccupied gap outside the supplied zone");
                    std::ofstream output(std::string(argv[1])+"/shared-zone-map.pam",std::ios::binary);
                    output<<"P7\nWIDTH "<<image->width()<<"\nHEIGHT "<<image->height()<<"\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
                    output.write(reinterpret_cast<const char*>(image->begin()),image->width()*image->height()*4);
                }catch(const std::exception& error){failure=error.what();}slint::quit_event_loop();
            });window->run();require(failure.empty(),failure);
        }else{
            const auto settle=[&]{slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
            const auto point=[&](const char* label){settle();auto item=find(label);auto p=item.absolute_position();auto s=item.size();return slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};};
            const auto drag=[&](const char* label,float dx,float dy,bool cancel=false){
                auto from=point(label);slint::LogicalPosition to{{from.x+dx,from.y+dy}};
                window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);window->window().dispatch_pointer_move_event(to);settle();
                if(cancel){window->window().dispatch_key_press_event(slint::platform::key_codes::Escape);window->window().dispatch_key_release_event(slint::platform::key_codes::Escape);settle();}
                window->window().dispatch_pointer_release_event(to,slint::PointerEventButton::Left);settle();
            };
            for(auto label: {"Move zone Known zone","Low key Known zone","High key Known zone","Velocity high Known zone","Velocity low Known zone","Resize high key and high velocity Known zone"}){
                window->invoke_reset_counts();window->set_accept_begin(false);drag(label,15,-15);
                require(window->get_begins()==1&&window->get_updates()==0&&window->get_ends()==0,"Every zone body/edge/corner respects host begin refusal");
                window->set_accept_begin(true);window->invoke_reset_counts();drag(label,15,-15);
                require(window->get_begins()==1&&window->get_updates()==1&&window->get_ends()==1&&window->get_cancels()==0,"Each native zone drag brackets one accepted transaction");
                const std::string_view name=label;
                if(name=="Move zone Known zone"||name=="Low key Known zone")require(window->get_last_value()==50,"Known15pixel key delta is exactly two semitones");
                else if(name=="High key Known zone")require(window->get_last_value()==61,"High edge uses its original59 boundary plus two semitones");
                else if(name=="Velocity high Known zone")require(window->get_last_value()==102,"Known minus15pixel velocity drag raises96 to102");
                else if(name=="Velocity low Known zone")require(window->get_last_value()==38,"Low velocity edge retains its independent32 initial boundary");
                else require(window->get_corner_note()==61&&window->get_corner_velocity()==102,"Corner reports both independently calibrated dimensions");
                window->invoke_reset_counts();drag(label,15,-15,true);
                require(window->get_cancels()==1&&window->get_ends()==0,"Escape cancellation cannot commit on the subsequent pointer release");
            }
            // A host refreshes the same model row while each gesture is still captured.
            const auto original_zone=window->get_zones()->row_data(0).value();
            auto live_zones=std::make_shared<slint::VectorModel<dandrum_ui::KeyZone>>(std::vector{original_zone});
            window->set_zones(live_zones);settle();
            for(bool high: {true,false}){
                live_zones->set_row_data(0,original_zone);settle();
                const auto label=high ? "Velocity high Known zone" : "Velocity low Known zone";
                // Each minus15px movement adds six velocity units; the second drag
                // must start from the first committed host value, not its old anchor.
                for(int repetition=0;repetition<3;++repetition){
                    const bool cancel=repetition==2;
                    const int expected=(high ? 102 : 38)+repetition*6;
                    window->invoke_reset_counts();
                    const auto from=point(label);const slint::LogicalPosition to{{from.x,from.y-15}};
                    window->window().dispatch_pointer_press_event(from,slint::PointerEventButton::Left);
                    window->window().dispatch_pointer_move_event(to);settle();
                    require(window->get_begins()==1&&window->get_updates()>=1&&window->get_last_value()==expected,
                            std::string(label)+" previews the independently known velocity before host refresh");
                    auto projected=live_zones->row_data(0).value();
                    if(high)projected.vel_hi=expected;else projected.vel_lo=expected;
                    live_zones->set_row_data(0,projected);settle();
                    if(cancel){
                        window->window().dispatch_key_press_event(slint::platform::key_codes::Escape);
                        window->window().dispatch_key_release_event(slint::platform::key_codes::Escape);settle();
                    }
                    window->window().dispatch_pointer_release_event(to,slint::PointerEventButton::Left);settle();
                    require(window->get_begins()==1&&window->get_updates()>=1&&window->get_ends()==(cancel?0:1)&&window->get_cancels()==(cancel?1:0)&&window->get_last_value()==expected,
                            std::string(label)+(cancel ? " cancels once after row refresh without committing on release" : " commits once after row refresh")+
                            "; begins="+std::to_string(window->get_begins())+", updates="+std::to_string(window->get_updates())+
                            ", commits="+std::to_string(window->get_ends())+", cancels="+std::to_string(window->get_cancels()));
                }
            }
            live_zones->set_row_data(0,original_zone);settle();
            window->set_free_edit(true);window->invoke_reset_counts();drag("Move zone Known zone",15,0);
            require(window->get_captured_free(),"The chosen free-edit policy is supplied to the consumer rather than applied to its model");
            auto center=point("Move zone Known zone");window->window().dispatch_pointer_press_event(center,slint::PointerEventButton::Right);window->window().dispatch_pointer_release_event(center,slint::PointerEventButton::Right);settle();
            require(window->get_contexts()==1&&window->get_context_zone()=="zone"&&window->get_context_note()==54&&window->get_context_split(),"Zone context callback reports caller-owned identity and actual key location");
            slint::LogicalPosition gap{{50,300}};window->window().dispatch_pointer_press_event(gap,slint::PointerEventButton::Right);window->window().dispatch_pointer_release_event(gap,slint::PointerEventButton::Right);settle();
            require(window->get_contexts()==2&&window->get_context_zone().empty()&&!window->get_context_split(),"Empty-grid context reports a gap without selecting a fabricated zone");
            auto& state=window->global<dandrum_ui::EditorDragState>();state.invoke_begin("source","sample",center.x,center.y);state.invoke_finish();settle();
            require(window->get_assignments()==1&&window->get_assigned_payload()=="sample"&&state.get_handled(),"Generic target delegates source assignment and preserves accepted drop ownership");
            window->window().dispatch_key_press_event("\x11");window->window().dispatch_pointer_scroll_event(center,0,120);window->window().dispatch_key_release_event("\x11");settle();
            require(window->get_zoom()>1&&window->get_content_width()>600,"Ctrl wheel preserves the portable zoom behavior");
            auto width=window->get_content_width();window->window().dispatch_pointer_scroll_event(center,-60,0);settle();
            require(window->get_content_x()<0&&window->get_content_width()==width,"Horizontal wheel pans without changing key scale");
            window->set_input_enabled(false);settle();window->invoke_reset_counts();drag("Move zone Known zone",15,0);
            require(window->get_begins()==0&&window->get_updates()==0,"Disabled grid rejects native gestures");
            drag("Continuous graph drag",5.5f,0);
            require(std::abs(window->get_continuous_value()-.155f)<.00001f,"The same generic drag input supports unquantized graph values when step is zero");
        }
        std::cout<<"PASS: "<<checks<<" library-only zone-map "<<(render?"pixel":"interaction")<<" checks\n";
    }catch(const std::exception& error){std::cerr<<"FAIL after "<<checks<<" checks: "<<error.what()<<'\n';return 1;}
}
