#include "AdvancedSamplerDomainRender.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <chrono>
#include <cmath>
#include <fstream>
#include <cstdlib>
#include <iostream>
#include <stdexcept>
// Render the actual native Slint paths on the caller's private virtual display.
int main(int argc,char**argv){if(argc!=2)return 2;int checks=0;try{auto w=AdvancedSamplerDomainRender::create();auto b=dandrum::sampler::bindWindowSession<Session>(w);const auto&s=w->global<Session>();dandrum::slint_ui::bind_value_codec<ParsedValue>(w->global<ValueCodec>());s.invoke_command("patch.load","kit","",0);s.invoke_command("modulator.select","filter-env","",0);int phase=std::getenv("DANDRUM_DOMAIN_ROUTE_COLORS")?3:0;if(phase==3)s.invoke_command("state.select","mod","",0);std::string failure;slint::Timer timer;
 auto require=[&](bool ok,std::string text){++checks;if(!ok)throw std::runtime_error(text);};
 timer.start(slint::TimerMode::Repeated,std::chrono::milliseconds(150),[&]{try{auto image=w->window().take_snapshot();require(image.has_value(),"Actual native renderer supplies snapshot");std::ofstream output(std::string(argv[1])+"/domain-"+std::to_string(phase)+".pam",std::ios::binary);output<<"P7\nWIDTH "<<image->width()<<"\nHEIGHT "<<image->height()<<"\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";output.write(reinterpret_cast<const char*>(image->begin()),image->width()*image->height()*4);if(phase==3){
 const auto& theme=w->global<SamplerTheme>();
 for(auto [source,destination,color]:std::vector<std::tuple<std::string,std::string,slint::Color>>{{"Amp envelope","Amp gain",theme.get_paper_3()},{"Filter envelope","Filter cutoff",theme.get_mod_a()},{"LFO 1","Pitch",theme.get_mod_b()},{"Velocity","Amp gain",theme.get_mod_c()},{"Macro · Tone","Filter cutoff",theme.get_mod_d()}}){
  auto fields=slint::testing::ElementHandle::find_by_accessible_label(w,"Amount "+source+" to "+destination);require(!fields.empty(),"Actual route amount exists: "+source);
  auto field=fields[0];auto position=field.absolute_position();
  int x=std::lround(position.x-6-96/2+1),y=std::lround(position.y+11);
  require(x>=0&&y>=0&&x<image->width()&&y<image->height(),"Route color sample lies inside native image");
  bool matches=false;for(int dx=-1;dx<=1;++dx){auto pixel=image->begin()[y*image->width()+x+dx];matches=matches||(std::abs(int(pixel.r)-color.red())<3&&std::abs(int(pixel.g)-color.green())<3&&std::abs(int(pixel.b)-color.blue())<3);}
  require(matches,"Actual route bar uses guide semantic color: "+source);
 }
 ++phase;s.invoke_command("state.select","vel","",0);w->set_page("vel");return;
}else if(phase==4){
 const auto ink=w->global<SamplerTheme>().get_ink_0();
 const double alpha=std::pow(10.,-2./20.);
 const int expected=std::lround(ink.green()*(1-alpha)+88*alpha);
 for(int x=150;x<270;++x){const auto p=image->begin()[220*image->width()+x];require(std::abs(int(p.g)-expected)<=1,
  "Velocity fill has continuous independently calibrated -2 dB gain at column "+std::to_string(x));}
 timer.stop();slint::quit_event_loop();return;
}std::string label=phase==0?"Modulator graph Filter envelope":phase==1?"Modulator graph LFO 1":"Processor graph Compressor";auto plots=slint::testing::ElementHandle::find_by_accessible_label(w,label);require(!plots.empty(),"Native graph exists: "+label);if(phase==1){auto rates=slint::testing::ElementHandle::find_by_accessible_label(w,"Rate"),depths=slint::testing::ElementHandle::find_by_accessible_label(w,"Depth");require(!rates.empty()&&!depths.empty(),"Actual LFO controls exist");require(rates[0].absolute_position().x+rates[0].size().width<=depths[0].absolute_position().x+1,"Visible LFO Rate and Depth controls do not overlap");}auto p=plots[0].absolute_position();auto size=plots[0].size();double tx=phase==0?.3:phase==1?.125:.85,ty=phase==0?.68:phase==1?.275:.2625;bool near=false;for(int dy=-2;dy<=2;++dy)for(int dx=-2;dx<=2;++dx){int x=std::lround(p.x+tx*size.width)+dx,y=std::lround(p.y+ty*size.height)+dy;if(x<0||y<0||x>=image->width()||y>=image->height())continue;auto pixel=image->begin()[y*image->width()+x];near=near||(phase<2?pixel.g>135&&pixel.b>135&&pixel.r<130:pixel.r>160&&pixel.g>130&&pixel.b<160);}require(near,"Native graph stroke passes known normalized point in phase "+std::to_string(phase));if(++phase==3){s.invoke_command("state.select","mod","",0);w->set_page("mod");}else if(phase==1)s.invoke_command("modulator.select","lfo1","",0);else{s.invoke_command("fx.add","drums","Compressor",0);w->set_page("fx");}}catch(const std::exception&e){failure=e.what();timer.stop();slint::quit_event_loop();}});w->run();if(!failure.empty())throw std::runtime_error(failure);require(phase==4,"Envelope/LFO/compressor, route colors and continuous velocity fill actual renderer cases completed");std::cout<<"PASS: "<<checks<<" actual native domain renderer checks\n";return 0;}catch(const std::exception&e){std::cerr<<"FAIL: "<<e.what()<<'\n';return 1;}}
