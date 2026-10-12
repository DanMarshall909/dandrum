#include "SlintEditorModulationLayers.h"
#include "ui/slint/host/VelocityBlend.h"
#include <slint-platform.h>
#include <slint-testing.h>
#include <array>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iostream>
#include <stdexcept>

namespace {
int checks;
void require(bool ok, const std::string& message) { ++checks; if (!ok) throw std::runtime_error(message); }
void near(float actual, float expected, const std::string& message) { require(std::abs(actual-expected)<.0001f, message+": "+std::to_string(actual)); }
struct Fixture {
    slint::ComponentHandle<dandrum_ui::SlintEditorModulationLayers> w = dandrum_ui::SlintEditorModulationLayers::create();
    Fixture() { w->show(); w->window().dispatch_window_active_changed_event(true); settle(); }
    ~Fixture() { w->hide(); }
    void settle() { slint::platform::update_timers_and_animations(); slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop(); }
    void key(slint::SharedString text) { w->window().dispatch_key_press_event(text); w->window().dispatch_key_release_event(text); settle(); }
    void drag(float x,float y,float delta,bool release=true) {
        w->window().dispatch_pointer_press_event(slint::LogicalPosition{{x,y}},slint::PointerEventButton::Left);
        w->window().dispatch_pointer_move_event(slint::LogicalPosition{{x+delta,y}}); settle();
        if(release) { w->window().dispatch_pointer_release_event(slint::LogicalPosition{{x+delta,y}},slint::PointerEventButton::Left); settle(); }
    }
    void release() { w->window().dispatch_pointer_release_event(slint::LogicalPosition{{600,100}},slint::PointerEventButton::Left); settle(); }
};
void modulationMath() {
    Fixture f;
    for(auto shape:{"1","triangle","tri"}) { near(f.w->invoke_waveform(0,shape),-1,"Triangle starts signed negative"); near(f.w->invoke_waveform(.5,shape),1,"Triangle midpoint is positive"); }
    for(auto shape:{"2","saw"}) { near(f.w->invoke_waveform(.25,shape),.5,"Saw quarter-cycle"); near(f.w->invoke_waveform(.75,shape),-.5,"Saw retains negative half"); }
    for(auto shape:{"3","square","sq"}) { near(f.w->invoke_waveform(.49,shape),1,"Square positive half"); near(f.w->invoke_waveform(.5,shape),-1,"Square midpoint changes sign"); }
    near(f.w->invoke_waveform(.25,"sine"),1,"Sine quarter-cycle"); near(f.w->invoke_waveform(.75,"0"),-1,"Numeric sine alias");
    near(f.w->invoke_waveform(1.25,"other"),1,"Unknown shape retains periodic sine fallback");
    for(int i=0;i<8;++i) near(f.w->invoke_waveform(i*.25f,"sh"),std::array{.6f,-.3f,.9f,-.8f,.2f,-.5f,.75f,-.1f}[i],"Sample-and-hold fixed preview levels");
    near(f.w->invoke_performance(.25,0,1,"lin",false),.25,"Linear performance input");
    near(f.w->invoke_performance(.25,0,1,"exp",false),.0625,"Squared performance input");
    near(f.w->invoke_performance(.25,0,1,"log",false),.5,"Square-root performance input");
    near(f.w->invoke_performance(.25,0,1,"s",false),.15625,"Smoothstep performance input");
    near(f.w->invoke_performance(.25,0,1,"s",true),.84375,"Inversion follows shaping");
    near(f.w->invoke_performance(-1,.2,.8,"lin",false),0,"Input clamps below range"); near(f.w->invoke_performance(2,.2,.8,"lin",false),1,"Input clamps above range");
    near(f.w->invoke_performance(.5,.5,.5,"lin",false),0,"Degenerate input range is finite");
    near(f.w->invoke_random_value(.05,"step"),.3,"Random step keeps its initial level");
    near(f.w->invoke_random_value(.05,"smooth"),-.2,"Random smoothing interpolates signed levels");
    near(f.w->invoke_random_value(1,"smooth"),.25,"Random preview preserves legacy terminal sample");
    for(int i=0;i<10;++i) near(f.w->invoke_random_value((i+.25f)/10,"step"),std::array{.3f,-.7f,.55f,.1f,-.4f,.85f,-.2f,.6f,-.9f,.25f}[i],"All deterministic random steps preserve signed data");
    for(auto [t,v]:std::array<std::pair<float,float>,6>{{{0,0},{.0625,.5},{.1875,.75},{.5,.5},{.9375,.25},{1,0}}}) near(f.w->invoke_envelope(t,100,100,.5,100),v,"Envelope stage geometry");
    near(f.w->invoke_envelope(0,0,0,.5,0),0,"Zero duration stages remain finite");
    near(f.w->invoke_envelope(2,100,100,.5,100),0,"Envelope release clips to zero after its duration");
    near(f.w->invoke_modulation_y(.125),.05,"Positive LFO maps above center"); near(f.w->invoke_modulation_y(.375),.95,"Negative LFO maps below center");
    f.w->set_phase(.5); f.w->set_depth(.5); near(f.w->invoke_modulation_y(.125),.725,"Plot applies independent phase and depth"); f.w->set_phase(0); f.w->set_depth(1);
    f.w->set_kind("random"); near(f.w->invoke_modulation_y(.05),.365,"Random plot retains signed offset");
    f.w->set_kind("envelope"); near(f.w->invoke_modulation_y(.0625),.5,"Envelope maps into padded vertical span");
    f.w->set_kind("performance"); near(f.w->invoke_modulation_y(.25),.725,"Performance plot uses unipolar span");
    f.w->set_in_low(.25); f.w->set_in_high(.75); f.w->set_inverted(true); near(f.w->invoke_modulation_y(.25),.05,"Plot projects input bounds and inversion");
}
void processorMath() {
    Fixture f;
    near(f.w->invoke_response(.5),.5,"Filter cutoff preview"); near(f.w->invoke_processor_y(.5),.5,"Processor paint reverses vertical direction");
    f.w->set_processor_value(0); near(f.w->invoke_response(.01),.5,"Filter zero normalization has finite floor");
    f.w->set_processor_kind("compressor"); f.w->set_processor_value(-30); near(f.w->invoke_response(.25),.25,"Compressor below threshold"); near(f.w->invoke_response(.9),.6,"Compressor ratio above threshold");
    f.w->set_ratio(0); near(f.w->invoke_response(.9),.9,"Invalid compression ratio is bounded"); near(f.w->invoke_response(-1),0,"Compressor clips below zero"); near(f.w->invoke_response(2),1,"Compressor clips above one");
    f.w->set_processor_kind("eq"); f.w->set_processor_value(1); near(f.w->invoke_response(0),.75,"EQ left side gain"); near(f.w->invoke_response(1),.25,"EQ right side gain");
    f.w->set_processor_value(20); near(f.w->invoke_response(0),.95,"EQ top bounded"); near(f.w->invoke_response(1),.05,"EQ bottom bounded");
    f.w->set_processor_kind("saturate"); f.w->set_processor_value(.5); near(f.w->invoke_response(0),1.f/3,"Saturator negative half"); near(f.w->invoke_response(1),2.f/3,"Saturator positive half");
    f.w->set_processor_kind("bitcrush"); f.w->set_processor_value(4); near(f.w->invoke_response(.3),.25,"Bitcrush quantizes preview"); f.w->set_processor_value(0); near(f.w->invoke_response(.3),.5,"Bitcrush minimum step count");
    f.w->set_processor_kind("decay"); f.w->set_processor_value(.5); near(f.w->invoke_response(.5),.325,"Generic decay preview");
    f.w->set_primary_minimum(.5); f.w->set_primary_maximum(.5); near(f.w->invoke_response(.5),.15,"Degenerate processor range remains finite");
}
void velocityGeometry() {
    using namespace dandrum::slint_ui;
    near(velocity_blend_gain(0,1,64,32,"linear"),1,"Soft layer starts at full level");
    near(velocity_blend_gain(49.f/127,1,64,32,"linear"),.5,"Soft linear midpoint");
    near(velocity_blend_gain(79.f/127,64,127,32,"linear"),.5,"Hard linear midpoint");
    near(velocity_blend_gain(49.f/127,1,64,32,"equal-power"),std::sqrt(.5f),"Equal-power midpoint");
    near(velocity_blend_gain(1,1,64,0,"linear"),0,"Fade width denominator has finite minimum");
    near(velocity_blend_gain(0,1,64,-1,"unknown"),1,"Unknown curve retains equal-power fallback");
    for(auto [t,v]:std::array<std::pair<float,float>,4>{{{0,0},{63.f/127,1},{1,1},{.1f,0}}}) near(velocity_blend_gain(t,64,127,0,"none"),v,"No-crossfade range uses inclusive endpoints");
    auto path=velocity_blend_path(1,127,0,"none",false); std::string expected="M 0 0";
    for(int i=1;i<=100;++i) expected+=" L "+std::to_string(i)+" 0";
    require(path==expected,"Unfilled full-range path has all 101 known coordinates");
    require(velocity_blend_path(1,127,0,"none",true)=="M 0 100 L 0 0"+expected.substr(5)+" L 100 100 Z","Filled full-range path closes to the baseline");
    Fixture f;
    near(f.w->get_text_x()/1.f,8,"Soft band caption starts at inset"); near(f.w->get_text_width()/1.f,120,"Soft band caption width");
    near(f.w->get_fill_opacity(),1,"Unity gain is opaque"); f.w->set_gain(-20); near(f.w->get_fill_opacity(),.1,"Gain dB drives fill opacity");
    require(f.w->get_band_color()==slint::Color::from_rgb_uint8(0x6a,0x58,0x4a)&&f.w->get_band_stroke()==slint::Color::from_rgb_uint8(0x8a,0x75,0x60),"Soft band retains supplied-theme colors");
    f.w->set_gain(12); near(f.w->get_fill_opacity(),1,"Positive gain opacity clamps"); f.w->set_low(64); near(f.w->get_text_x()/1.f,272,"Hard band caption stays right aligned"); near(f.w->get_text_width()/1.f,120,"Hard band caption width");
    require(f.w->get_band_color()==slint::Color::from_rgb_uint8(0xe0,0x8a,0x4e)&&f.w->get_band_stroke()==slint::Color::from_rgb_uint8(0xe0,0x8a,0x4e),"Hard band retains its highlight colors");
    f.w->set_low(128); near(f.w->get_text_width()/1.f,0,"Offscreen caption width never becomes negative");
    f.drag(40,275,0); require(f.w->get_selections()==1,"Band click forwards selection");
}
void gestures() {
    Fixture f;
    for(float x:{504.f,554.f,220.f}) {
        const bool processor=x==220; const float y=processor?180:60;
        const auto value=[&]{return processor?f.w->get_processor_value():f.w->get_range_value();};
        float initial=value(); int begins=f.w->get_begins(),updates=f.w->get_updates(),ends=f.w->get_ends(),cancels=f.w->get_cancels();
        f.w->set_accept_begin(false); f.drag(x,y,10); require(f.w->get_begins()==begins+1&&f.w->get_updates()==updates&&f.w->get_ends()==ends&&!f.w->get_captured(),"Refused drag produces no preview or commit");
        f.w->set_accept_begin(true); f.drag(x,y,10.4,false); near(value(),initial+(processor?.026f:10.f),"Accepted drag preserves continuous or integral scaling"); require(f.w->get_captured(),"Accepted begin captures input");
        f.key(slint::platform::key_codes::Escape); f.release(); near(value(),initial,"Escape asks host to restore its value"); require(f.w->get_cancels()==cancels+1&&f.w->get_ends()==ends&&!f.w->get_captured(),"Escape cannot subsequently commit");
        f.drag(x,y,-10); near(value(),initial-(processor?.025f:10.f),"Negative drag retains signed delta"); require(f.w->get_ends()==ends+1,"Release commits once"); initial=value();
        f.drag(x,y,10,false); f.w->set_controls_enabled(false); f.settle(); f.release(); near(value(),initial,"Disable rolls captured value back"); require(f.w->get_cancels()==cancels+2,"Disable cancels once");
        begins=f.w->get_begins(); f.drag(x,y,10); require(f.w->get_begins()==begins,"Disabled control rejects begin"); f.w->set_controls_enabled(true); f.settle();
        f.drag(x,y,10,false); f.w->window().dispatch_window_active_changed_event(false); f.settle(); f.release(); near(value(),initial,"Window loss cancels drag"); f.w->window().dispatch_window_active_changed_event(true); f.settle();
        f.drag(x,y,10,false); f.w->window().dispatch_pointer_exit_event(); f.settle(); f.release(); near(value(),initial,"Pointer cancellation restores host value");
        f.drag(x,y,10,false); f.w->invoke_finish(true); f.release(); near(value(),initial,"Explicit finish cancellation is idempotent");
    }
    f.w->set_axis_size(0); auto value=f.w->get_range_value(); f.drag(504,60,10); near(f.w->get_range_value(),value,"Zero plot extent never divides or updates");
    f.w->window().dispatch_pointer_press_event(slint::LogicalPosition{{504,60}},slint::PointerEventButton::Right); f.w->window().dispatch_pointer_release_event(slint::LogicalPosition{{504,60}},slint::PointerEventButton::Right); f.settle(); require(f.w->get_contexts()==1,"Range context callback remains available");
}
void render(const char* directory) {
    auto w=dandrum_ui::SlintEditorModulationLayers::create();
    w->set_fill_path(slint::SharedString(dandrum::slint_ui::velocity_blend_path(1,64,32,"linear",true))); w->set_stroke_path(slint::SharedString(dandrum::slint_ui::velocity_blend_path(1,64,32,"linear",false)));
    std::string error; slint::Timer timer;
    timer.start(slint::TimerMode::SingleShot,std::chrono::milliseconds(300),[&]{try {
        auto pixels=w->window().take_snapshot(); require(pixels.has_value(),"Native renderer creates pixels");
        std::ofstream out(std::string(directory)+"/modulation-layers.pam",std::ios::binary); out<<"P7\nWIDTH "<<pixels->width()<<"\nHEIGHT "<<pixels->height()<<"\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n"; out.write(reinterpret_cast<const char*>(pixels->begin()),pixels->width()*pixels->height()*4);
        auto bright=[&](int x,int y,int minimum=200){for(int dy=-2;dy<=2;++dy)for(int dx=-1;dx<=1;++dx){auto p=pixels->begin()[(y+dy)*pixels->width()+x+dx];if(p.r>minimum&&p.g>minimum&&p.b>minimum)return true;}return false;};
        require(bright(70,25)&&bright(170,115),"Signed sine renders its independently known positive and negative extrema");
        require(!bright(70,115)&&!bright(170,25),"Signed curve is not vertically inverted");
        require(bright(220,190)&&bright(120,146),"Processor response uses independently known half and quarter positions");
        auto brown=[&](int x,int y){for(int dy=-1;dy<=1;++dy){auto p=pixels->begin()[(y+dy)*pixels->width()+x];if(p.r>115&&p.r<155&&p.g>95&&p.g<135&&p.b>75&&p.b<115)return true;}return false;};
        require(brown(180,276),"Velocity blend stroke passes through its known linear fade coordinate");
        require(bright(120,315,130)&&bright(320,325,130),"Zero segment count falls back to a finite diagonal stroke");
        const auto red=[&](int x,int y){auto p=pixels->begin()[y*pixels->width()+x];return p.r>180&&p.g<100&&p.b<100;};
        require(red(503,40)&&red(500,95)&&red(553,40)&&!red(550,95),"Range cap and uncapped split preserve their distinct paint geometry");
    }catch(const std::exception& e){error=e.what();}slint::quit_event_loop();}); w->run(); require(error.empty(),error);
}
}
int main(int argc,char** argv) {
    int failures=0;
    if(argc>1){try{render(argv[1]);}catch(const std::exception& e){++failures;std::cerr<<e.what()<<'\n';}}
    else {slint::testing::init();for(auto test:{modulationMath,processorMath,velocityGeometry,gestures})try{test();}catch(const std::exception& e){++failures;std::cerr<<e.what()<<'\n';}}
    std::cout<<checks<<" standalone modulation/layers checks; "<<failures<<" failed groups\n";return failures?1:0;
}
