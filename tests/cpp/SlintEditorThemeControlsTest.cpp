#include "SlintEditorThemeControls.h"
#include "ui/slint/host/ThemeBinding.h"
#include "ui/slint/host/AppearanceBinding.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <iostream>
#include <stdexcept>

int main() {
    int checks=0;
    auto require=[&](bool ok,const char* message){++checks;if(!ok)throw std::runtime_error(message);};
    try {
        slint::testing::init();
        auto window=dandrum_ui::SlintEditorThemeControls::create();window->show();
        window->window().dispatch_window_active_changed_event(true);
        const auto settle=[] { slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop(); };
        const auto element=[&](const char* label){settle();return slint::testing::ElementHandle::find_by_accessible_label(window,label).at(0);};
        const auto position=[&](const char* label){auto item=element(label);auto p=item.absolute_position();auto s=item.size();return slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};};
        const auto click=[&](const char* label){auto p=position(label);window->window().dispatch_pointer_press_event(p,slint::PointerEventButton::Left);window->window().dispatch_pointer_release_event(p,slint::PointerEventButton::Left);settle();};
        const auto press=[&](slint::SharedString key){window->window().dispatch_key_press_event(key);settle();};
        const auto release=[&](slint::SharedString key){window->window().dispatch_key_release_event(key);settle();};
        using namespace dandrum::slint_ui;
        const auto& theme=window->global<dandrum_ui::EditorTheme>();
        const auto& appearance=window->global<dandrum_ui::EditorAppearance>();
        ThemeSettings live;
        const auto apply=[&](const std::array<std::string_view,6>& values) {
            return apply_appearance(appearance,theme,live,values[0],values[1],values[2],values[3],values[4],values[5]);
        };
        for(auto surface:surface_names)for(auto finish:finish_names)for(auto palette:mod_palette_names) {
            require(apply({surface,finish,"#123","#aBcDeF",palette,"#Ff0"}),"Shared appearance accepts every named material and modulation option");
            require(appearance.get_surface()==slint::SharedString(surface)&&appearance.get_finish()==slint::SharedString(finish)&&appearance.get_mod_palette()==slint::SharedString(palette),"Shared appearance publishes all named selections");
            require(appearance.get_accent()=="#112233"&&appearance.get_secondary()=="#ABCDEF"&&appearance.get_host()=="#FFFF00","Shared appearance canonicalizes shorthand and mixed-case colors");
            const auto parsed=parse_appearance(appearance.get_surface(),appearance.get_finish(),appearance.get_accent(),appearance.get_secondary(),appearance.get_mod_palette(),appearance.get_host());
            require(parsed.settings&&*parsed.settings==live&&parsed.error.empty(),"All six appearance fields roundtrip without sampler state");
        }
        const auto retained=live;const auto retained_accent=theme.get_accent();const auto retained_surface=theme.get_surface_panel();
        const std::array<std::string_view,6> errors={"Unknown surface","Unknown finish","Invalid accent color","Invalid secondary color","Unknown modulation palette","Invalid host color"};
        for(std::size_t field=0;field<errors.size();++field) {
            std::array<std::string_view,6> invalid={"aluminium","soft","#123456","#abcdef","standard","#5B9DFF"};invalid[field]="invalid";
            appearance.set_surface("invalid");appearance.set_finish("invalid");appearance.set_accent("invalid");appearance.set_secondary("invalid");appearance.set_mod_palette("invalid");appearance.set_host("invalid");
            require(!apply(invalid)&&live==retained&&theme.get_accent()==retained_accent&&theme.get_surface_panel()==retained_surface,"Invalid appearance selection preserves the complete last valid theme");
            require(appearance.get_error()==slint::SharedString(errors[field]),"Invalid appearance field reports its exact validation error");
            const auto restored=parse_appearance(appearance.get_surface(),appearance.get_finish(),appearance.get_accent(),appearance.get_secondary(),appearance.get_mod_palette(),appearance.get_host());
            require(restored.settings&&*restored.settings==retained,"Rejected appearance input restores all six selected values");
        }
        require(apply({"paper","flat","#000000","#ffffff","standard","#0f0"})&&appearance.get_error().empty(),"A valid appearance selection clears the previous validation error");
        publish_appearance(appearance,live);
        require(appearance.get_accent()=="#000000"&&appearance.get_secondary()=="#FFFFFF"&&appearance.get_host()=="#00FF00","Appearance publish preserves extreme colors and leading zeroes");
        require(appearance_hex(0x012abf)=="#012ABF","Color serialization preserves all six hexadecimal places");
        require(window->get_summary_font_size()==11,"Standalone editor uses the shared minimum small-text size");
        theme.set_type_small(13);require(window->get_summary_font_size()==13,"Editor small-text size is supplied by its theme");
        for(auto surface:{Surface::Aluminium,Surface::Brushed,Surface::Ember,Surface::Graphite,Surface::Midnight,Surface::Forest,Surface::Camo,Surface::Paper})for(auto finish:{Finish::Soft,Finish::Flat}) {
            ThemeSettings settings;settings.surface=surface;settings.finish=finish;
            const auto palette=resolve_theme(settings);apply_theme(theme,palette);
            require(theme.get_accent()==native_color(palette[Role::Accent]),"Shared native binder applies resolved accent");
            require(theme.get_surface_panel()==native_color(palette[Role::SurfacePanel]),"Shared native binder applies material surface");
            require(theme.get_soft()==palette.soft&&theme.get_brushed()==palette.brushed&&theme.get_light()==palette.light,"Shared native binder applies material modes");
            require(theme.get_window_lighting().size().width==1600,"Material lighting is available without sampler host code");
        }
        for(int slot:{-1,0,1,2,3,4,9}) {
            const auto expected=slot<=0?theme.get_paper_3():slot==1?theme.get_mod_a():slot==2?theme.get_mod_b():slot==3?theme.get_mod_c():theme.get_mod_d();
            require(theme.invoke_mod_color(slot)==expected,"Shared modulation roles retain every slot and fallback");
        }
        click("Editor action");require(window->get_pressed()==1,"Editor button pointer activates once");
        press(" ");require(window->get_pressed()==2,"Editor compatibility button activates on key press");
        release(" ");require(window->get_pressed()==2,"Editor button release does not activate twice");
        press(slint::platform::key_codes::Return);release(slint::platform::key_codes::Return);
        require(window->get_pressed()==3,"Editor button supports Return");
        element("Editor action").invoke_accessible_default_action();require(window->get_pressed()==4,"Editor button supports accessible activation");
        window->window().dispatch_window_active_changed_event(false);settle();
        require(window->get_deactivated()==1,"Shared input reports window deactivation to its host");
        window->window().dispatch_window_active_changed_event(true);
        click("Default action");require(window->get_released()==1,"Existing action still supports pointer activation");
        require(window->get_deactivated()==1,"Moving focus inside the window does not report deactivation");
        press(" ");require(window->get_released()==1,"Existing action retains release activation by default");
        release(" ");require(window->get_released()==2,"Existing action releases exactly once");
        click("Mode B");require(window->get_selection()=="b","Option strip forwards the selected stable ID");
        require(element("Mode B").accessible_checked()==true,"Option strip selection is exposed to accessibility");
        click("Details collapsed");require(window->get_expanded()&&window->get_rollout_height()==84,"Rollout expands the configured body");
        click("Details expanded");require(!window->get_expanded()&&window->get_rollout_height()==24,"Rollout collapses its body");
        for(auto label:{"Resize horizontal","Resize vertical"}) {
            auto p=position(label);auto end=p;end.x+=17;end.y+=23;
            window->window().dispatch_pointer_press_event(p,slint::PointerEventButton::Left);
            window->window().dispatch_pointer_move_event(end);
            window->window().dispatch_pointer_release_event(end,slint::PointerEventButton::Left);
        }
        require(window->get_resize_starts()==2&&window->get_resize_ends()==2,"Resize lifecycle balances both orientations");
        require(window->get_resize_x()==17&&window->get_resize_y()==23,"Resize reports displacement in the configured axis");
        window->set_icon_name("info");auto fallback=window->get_icon_image();window->set_icon_name("unknown");
        require(window->get_icon_image()==fallback,"Unknown icons use the retained info shape");
        window->set_icon_name("plus");require(window->get_icon_image()!=fallback,"Named icons resolve to distinct retained shapes");
        click("Second");require(window->get_choice()=="Second","Choice grid forwards the selected value");
        click("#abcdef");require(window->get_color_choice()=="#abcdef","Color choice forwards preset selection");
        element("Accent custom color").set_accessible_value("#fedcba");click("Apply Accent color");
        require(window->get_color_choice()=="#fedcba","Color choice forwards custom text for host validation");
        click("Accent custom color");element("Accent custom color").set_accessible_value("#321");
        press(slint::platform::key_codes::Return);release(slint::platform::key_codes::Return);
        require(window->get_color_choice()=="#321","Color choice accepts typed values with Return");
        window->set_controls_enabled(false);settle();auto before=window->get_pressed();
        click("Editor action");press(" ");release(" ");press(slint::platform::key_codes::Return);release(slint::platform::key_codes::Return);
        element("Editor action").invoke_accessible_default_action();
        require(window->get_pressed()==before&&element("Editor action").accessible_enabled()==false,"Disabled editor action blocks every input path");
        click("Mode A");element("Mode A").invoke_accessible_default_action();require(window->get_selection()=="b","Disabled option strip leaves selection intact");
        click("Details collapsed");require(!window->get_expanded(),"Disabled rollout cannot toggle");
        auto p=position("Resize horizontal");window->window().dispatch_pointer_press_event(p,slint::PointerEventButton::Left);p.x+=40;window->window().dispatch_pointer_move_event(p);window->window().dispatch_pointer_release_event(p,slint::PointerEventButton::Left);
        require(window->get_resize_starts()==2&&window->get_resize_x()==17,"Disabled resize leaves host layout intact");
        std::cout<<"PASS: "<<checks<<" standalone editor theme and control checks\n";
    } catch(const std::exception& error) {std::cerr<<"FAIL after "<<checks<<" checks: "<<error.what()<<'\n';return 1;}
}
