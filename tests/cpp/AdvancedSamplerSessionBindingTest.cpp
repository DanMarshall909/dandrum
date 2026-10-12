#include "BindingFixture.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include <slint-testing.h>
#include <stdexcept>
#include <iostream>
#include <functional>
#define CHECK(x) do { ++checks; if(!(x)) throw std::runtime_error(#x); } while(false)
struct RefreshChanges : slint::private_api::ModelChangeListener {
 int changes=0;
 void row_added(std::size_t,std::size_t) override { ++changes; }
 void row_removed(std::size_t,std::size_t) override { ++changes; }
 void row_changed(std::size_t) override { ++changes; }
 void reset() override { ++changes; }
};
struct ProjectionRow {
 slint::SharedString id;
 int value=0;
 std::shared_ptr<slint::Model<int>> children;
 bool operator==(const ProjectionRow&) const = default;
};
struct CountingProjection : slint::VectorModel<ProjectionRow> {
 using slint::VectorModel<ProjectionRow>::VectorModel;
 mutable std::size_t reads=0;
 mutable bool invalidRead=false;
 std::optional<ProjectionRow> row_data(std::size_t i) const override {
  ++reads;invalidRead|=i>=row_count();return slint::VectorModel<ProjectionRow>::row_data(i);
 }
};
void stableProjection(int& checks) {
 using namespace dandrum::sampler;
 std::vector<Record> records;
 std::vector<ProjectionRow> initial;
 std::map<std::string,std::shared_ptr<slint::Model<int>>> children;
 for(int i=0;i<128;++i) {
  auto id="row"+std::to_string(i);
  auto child=std::make_shared<slint::VectorModel<int>>(std::vector<int>{i});
  records.push_back({id,{{"value",std::to_string(i)}}});
  initial.push_back({binding::str(id),i,child});children.emplace(id,child);
 }
 auto counted=std::make_shared<CountingProjection>(std::move(initial));
 std::shared_ptr<slint::Model<ProjectionRow>> model=counted;
 auto fill=[](ProjectionRow& row,const Record& record) {
  row.id=binding::str(record.id);row.value=record.number("value");
  if(!row.children)row.children=std::make_shared<slint::VectorModel<int>>(std::vector<int>{row.value});
 };
 auto verify=[&](bool moved=false) {
  auto oldCount=model->row_count();counted->reads=0;counted->invalidRead=false;
  auto result=binding::project(model,records,fill);
  CHECK(result==model);
  CHECK(counted->reads<=(moved?3:2)*std::max(oldCount,records.size()));
  CHECK(!counted->invalidRead);CHECK(result->row_count()==records.size());
  for(std::size_t i=0;i<records.size();++i) {
   auto row=result->row_data(i);CHECK(row.has_value());CHECK(row->id==binding::str(records[i].id));
   CHECK(row->value==records[i].number("value"));
   auto existing=children.find(records[i].id);
   if(existing!=children.end())CHECK(row->children==existing->second);
   else {
    for(const auto& [id,child]:children)CHECK(row->children!=child);
    CHECK(row->children->row_count()==1);CHECK(row->children->row_data(0)==row->value);
    children.emplace(records[i].id,row->children);
   }
  }
 };
 verify();
 records.front().fields["value"]="999";verify();
 records.insert(records.begin(),{"inserted",{{"value","42"}}});verify(true);
 records.erase(records.begin()+1);verify(true);
 std::reverse(records.begin(),records.end());verify(true);
 records.clear();verify();
 records.push_back({"fresh",{{"value","7"}}});verify();
}
template<class Binding> void stableRefresh(const std::shared_ptr<Binding>& binding,const Session& session,int& checks) {
 std::vector<std::function<void()>> verify;
 auto watch=[&](auto original,auto current) {
  auto changes=std::make_shared<RefreshChanges>();original->attach_peer(changes);
  verify.push_back([&,original,current,changes]{CHECK(original==current());CHECK(changes->changes==0);});
 };
 watch(session.get_waveform(),[&]{return session.get_waveform();});
 watch(session.get_sounding_notes(),[&]{return session.get_sounding_notes();});
 watch(session.get_selected_zones(),[&]{return session.get_selected_zones();});
 watch(session.get_selected_candidates(),[&]{return session.get_selected_candidates();});
 watch(session.get_macros(),[&]{return session.get_macros();});
 watch(session.get_buses(),[&]{return session.get_buses();});
 watch(session.get_routing_buses(),[&]{return session.get_routing_buses();});
 watch(session.get_layers(),[&]{return session.get_layers();});
 for(std::size_t i=0;i<session.get_macros()->row_count();++i) {
  watch(session.get_macros()->row_data(i)->destinations,[&,i]{return session.get_macros()->row_data(i)->destinations;});
  watch(session.get_macros()->row_data(i)->assignments,[&,i]{return session.get_macros()->row_data(i)->assignments;});
 }
 for(std::size_t i=0;i<session.get_buses()->row_count();++i) {
  watch(session.get_buses()->row_data(i)->feeds,[&,i]{return session.get_buses()->row_data(i)->feeds;});
  watch(session.get_buses()->row_data(i)->channels,[&,i]{return session.get_buses()->row_data(i)->channels;});
 }
 for(std::size_t i=0;i<session.get_routing_buses()->row_count();++i) {
  watch(session.get_routing_buses()->row_data(i)->feeds,[&,i]{return session.get_routing_buses()->row_data(i)->feeds;});
  watch(session.get_routing_buses()->row_data(i)->channels,[&,i]{return session.get_routing_buses()->row_data(i)->channels;});
 }
 for(std::size_t i=0;i<session.get_layers()->row_count();++i)
  watch(session.get_layers()->row_data(i)->parameters,[&,i]{return session.get_layers()->row_data(i)->parameters;});
 binding->refresh();binding->refresh();for(auto& assertion:verify)assertion();
 auto notes=session.get_sounding_notes();CHECK(session.invoke_note_on(36,100,"refresh-test"));
 CHECK(session.get_sounding_notes()==notes);CHECK(notes->row_count()==1);CHECK(notes->row_data(0)==36);
 session.invoke_note_off(36,"refresh-test");CHECK(session.get_sounding_notes()==notes);CHECK(notes->row_count()==0);
 auto selected=session.get_selected_zones();CHECK(session.invoke_command("zone.select","snS","",0));
 CHECK(session.get_selected_zones()==selected);CHECK(selected->row_data(0)=="snS");
 CHECK(session.invoke_command("zone.select","kick","",0));
 auto parameters=session.get_layers()->row_data(0)->parameters;
 auto parameterChanges=std::make_shared<RefreshChanges>();parameters->attach_peer(parameterChanges);
 CHECK(session.invoke_command("parameter.set","pitch","",3));CHECK(session.get_layers()->row_data(0)->parameters==parameters);
 CHECK(parameters->row_data(0)->value==3);CHECK(parameterChanges->changes==1);
 CHECK(session.invoke_command("undo","","",0));
 auto destinations=session.get_macros()->row_data(0)->destinations;auto count=destinations->row_count();
 CHECK(session.invoke_command("macro.bind","tone","pan",.25));
 CHECK(session.get_macros()->row_data(0)->destinations==destinations);CHECK(destinations->row_count()==count+1);
 CHECK(session.invoke_command("undo","","",0));CHECK(destinations->row_count()==count);
 auto layers=session.get_layers();auto retained=layers->row_data(2).value();
 CHECK(session.invoke_command("zone.delete","snS","",0));CHECK(session.get_layers()==layers);
 bool preserved=false;
 for(std::size_t i=0;i<layers->row_count();++i)if(auto row=layers->row_data(i);row->id==retained.id){CHECK(row->parameters==retained.parameters);preserved=true;}
 CHECK(preserved);CHECK(session.invoke_command("undo","","",0));
}
int main(){int checks=0;try{
 stableProjection(checks);
 slint::testing::init();auto w=SessionBindingFixture::create();auto b=dandrum::sampler::bindWindowSession<Session>(w);const auto& s=w->global<Session>();
 CHECK(w->get_spectral_image().size().width==0);CHECK(s.get_current_patch_id()=="empty");CHECK(s.get_tree()->row_count()==1);CHECK(s.get_pads()->row_count()==128);CHECK(s.get_sources()->row_count()==0);
 CHECK(s.invoke_command("patch.load","kit","",0));CHECK(w->get_spectral_image().size().width==256);CHECK(s.get_current_patch_id()=="kit");CHECK(s.get_zones()->row_count()==15);CHECK(s.get_modules()->row_count()==6);CHECK(s.get_waveform()->row_count()==256);CHECK(s.get_pads()->row_data(38)->alternates==2);
 stableRefresh(b,s,checks);
 CHECK(s.get_inspector_type()=="Sample");CHECK(s.get_inspector_title()=="Kick.wav");CHECK(s.get_sample_duration()==0.8f);CHECK(s.get_inspector_rows()->row_count()>4);
 CHECK(s.get_candidates()->row_count()==1);CHECK(s.invoke_command("zone.select","snS","",0));CHECK(s.get_candidates()->row_count()==3);CHECK(s.invoke_command("selector.mode","","round-robin",0));CHECK(s.get_candidates()->row_count()==1);CHECK(s.invoke_command("zone.select","snH","",0));CHECK(s.get_candidates()->row_count()==2);CHECK(s.invoke_command("zone.select","kick","",0));CHECK(s.invoke_command("selector.mode","","velocity",0));
 CHECK(s.invoke_command("page.select","mapping","",0));CHECK(s.get_inspector_type()=="Zone");CHECK(s.get_inspector_title()=="Kick");
 CHECK(s.invoke_command("modulator.select","lfo1","",0));CHECK(s.get_inspector_type()=="Modulator");CHECK(s.get_inspector_title()=="LFO 1");
 CHECK(s.invoke_command("page.select","sample","",0));
 CHECK(s.invoke_command("fx.add","drums","Compressor",0));auto compressor=b->model()->records("fx").back().id;CHECK(s.invoke_command("selection.set",compressor.c_str(),"",0));CHECK(s.invoke_command("page.select","fx","",0));CHECK(s.get_inspector_parameters()->row_count()==6);auto effectParameters=s.get_effect_modules()->row_data(2)->parameters;b->refresh();CHECK(s.get_effect_modules()->row_data(2)->parameters==effectParameters);CHECK(s.invoke_command("page.select","sample","",0));
 auto pads=s.get_pads();auto macros=s.get_macros();auto zones=s.get_zones();auto parameters=s.get_parameters();b->refresh();CHECK(s.get_pads()==pads);CHECK(s.get_macros()==macros);CHECK(s.get_zones()==zones);CHECK(s.get_parameters()==parameters);
 CHECK(s.invoke_parameter_gesture("cutoff",0,6800));CHECK(s.invoke_parameter_gesture("cutoff",1,1234));CHECK(s.invoke_parameter_gesture("cutoff",2,1234));CHECK(s.get_can_undo());CHECK(b->model()->number("cutoff")==1234);CHECK(s.invoke_command("undo","","",0));CHECK(b->model()->number("cutoff")==6800);
 CHECK(s.invoke_note_on(38,112,"pad"));CHECK(s.invoke_note_on(38,100,"keys"));CHECK(s.get_sounding_notes()->row_count()==1);s.invoke_note_off(38,"pad");CHECK(s.get_sounding_notes()->row_count()==1);s.invoke_release_notes();CHECK(s.get_sounding_notes()->row_count()==0);
 CHECK(s.invoke_command("analysis.start","transients","",0));CHECK(s.get_analysis_state()=="running");b->model()->tick(1);b->refresh();CHECK(s.get_analysis_candidates()->row_count()>=2);CHECK(s.invoke_command("analysis.apply","","",0));CHECK(s.get_analysis_candidates()->row_count()==0);
 CHECK(s.invoke_command("patch.search","","Multisampled",0));CHECK(s.get_presets()->row_count()==1);CHECK(s.get_presets()->row_data(0)->id=="keys");CHECK(s.invoke_command("macro.bind","tone","pan",0.25));auto macro=s.get_macros()->row_data(0);CHECK(macro->destinations->row_count()==4);CHECK(s.invoke_command("macro.host","tone","",1));CHECK(s.get_macros()->row_data(0)->host_automated);
 CHECK(b->model()->selectState("mapping"));b->refresh();CHECK(s.get_zones()->row_count()==13);CHECK(s.get_tree()->row_count()==9);CHECK(s.get_mapped_low()==24);CHECK(s.get_mapped_high()==96);CHECK(s.get_mapping_low()==24);CHECK(s.get_mapping_high()==47);CHECK(s.get_modulators()->row_count()==12);CHECK(s.get_routes()->row_count()==14);CHECK(s.get_routes()->row_data(1)->source_name=="Filter envelope");CHECK(s.get_routes()->row_data(1)->destination_name=="Filter cutoff");CHECK(s.invoke_command("module.add","voice","Noise",0));auto noise=s.get_voice_modules()->row_data(s.get_voice_modules()->row_count()-1);CHECK(noise->kind=="noise");CHECK(noise->mode=="white");CHECK(noise->parameters->row_count()==2);CHECK(s.get_filter_mode()=="lp");CHECK(s.invoke_command("routing.output","drums","Drums 3/4",0));CHECK(s.invoke_command("selection.set","kick","",0));CHECK(s.get_resolved_output()=="Drums 3/4");bool inheritedKick=false;for(std::size_t i=0;i<s.get_tree()->row_count();++i){auto node=s.get_tree()->row_data(i);if(node->id=="kick")inheritedKick=node->resolved_output=="Drums 3/4";}CHECK(inheritedKick);bool drumsFeeds=false;for(std::size_t i=0;i<s.get_buses()->row_count();++i){auto bus=s.get_buses()->row_data(i);if(bus->id=="drums-out"){drumsFeeds=bus->feeds->row_count()==3;bool openHat=false;for(std::size_t j=0;j<bus->feeds->row_count();++j)openHat|=bus->feeds->row_data(j)=="Open Hat";CHECK(openHat);}}CHECK(drumsFeeds);
 CHECK(s.invoke_command("fx.add","drums","Compressor",0));auto typedOwner=b->model()->records("fx").back().id;auto typedTarget="fx:"+typedOwner+":threshold";CHECK(s.invoke_command("route.add","filter-env",typedTarget.c_str(),.4));CHECK(s.get_routes()->row_data(s.get_routes()->row_count()-1)->destination_name=="Threshold");auto macroCount=s.get_macro_bindings()->row_count();CHECK(s.invoke_command("parameter.learn",typedTarget.c_str(),"",0));CHECK(s.invoke_command("midi.receive","74","",127));CHECK(s.get_macro_bindings()->row_count()==macroCount);CHECK(s.invoke_command("parameter.host",typedTarget.c_str(),"",1));CHECK(s.invoke_command("selection.set",typedOwner.c_str(),"",0));CHECK(s.invoke_command("page.select","fx","",0));CHECK(s.get_inspector_parameters()->row_data(0)->host_automated);
 CHECK(s.invoke_command("state.select","mod","",0));CHECK(s.get_route_grouping()=="source");CHECK(s.get_route_groups()->row_count()==12);auto groups=s.get_route_groups();auto nested=groups->row_data(4)->routes;CHECK(nested->row_count()==2);b->refresh();CHECK(s.get_route_groups()==groups);CHECK(s.get_route_groups()->row_data(4)->routes==nested);CHECK(s.invoke_command("route.expand","source:velocity-mod","",0));CHECK(!s.get_route_groups()->row_data(4)->expanded);CHECK(s.invoke_command("route.group","destination","",0));CHECK(s.get_route_groups()->row_count()==9);
 auto ink0=slint::Color::from_rgb_uint8(10,10,10),ink5=slint::Color::from_rgb_uint8(40,40,40),accentLow=slint::Color::from_rgb_uint8(100,80,60),accent=slint::Color::from_rgb_uint8(200,180,150),paper=slint::Color::from_rgb_uint8(240,240,240);auto spectral=s.invoke_spectral_preview(s.get_waveform(),false,0,1,ink0,ink5,accentLow,accent,paper);CHECK(spectral.size().width==s.get_waveform()->row_count());CHECK(spectral.size().height==64);auto cached=s.invoke_spectral_preview(s.get_waveform(),false,0,1,ink0,ink5,accentLow,accent,paper);CHECK(cached==spectral);auto recoloured=s.invoke_spectral_preview(s.get_waveform(),false,0,1,ink0,ink5,accentLow,paper,paper);CHECK(recoloured!=spectral);
 CHECK(s.get_source_groups() && s.get_source_groups()->row_count()==4);
 auto sourceGroups=s.get_source_groups();CHECK(sourceGroups->row_data(0)->name=="Drums");CHECK(sourceGroups->row_data(0)->expanded);CHECK(sourceGroups->row_data(0)->sources->row_count()==2);CHECK(!sourceGroups->row_data(1)->expanded);auto groupSources=sourceGroups->row_data(0)->sources;b->refresh();CHECK(s.get_source_groups()==sourceGroups);CHECK(s.get_source_groups()->row_data(0)->sources==groupSources);CHECK(s.invoke_command("asset.group-toggle","Drums","",0));CHECK(!s.get_source_groups()->row_data(0)->expanded);CHECK(!s.invoke_command("asset.group-toggle","Absent","",0));
 CHECK(s.get_parameter_catalog() && s.get_parameter_catalog()->row_count()>s.get_parameters()->row_count());CHECK(s.get_parameter_catalog()->row_count()<=4096);
 for(auto target:{"fx:comp:threshold","modulator:lfo1:depth","pad:38:pan"}){bool discovered=false;auto catalog=s.get_parameter_catalog();for(std::size_t i=0;i<catalog->row_count();++i)if(catalog->row_data(i)->id==target)discovered=true;CHECK(discovered);CHECK(s.invoke_command("route.add","lfo1",target,.25));CHECK(b->model()->records("route").back().text("destination")==target);CHECK(s.invoke_command("macro.bind","tone",target,.3));CHECK(b->model()->find("binding",std::string("tone:")+target));}
 CHECK(s.invoke_command("module.add","voice","Noise",0));auto noiseOwner=b->model()->records("module").back().id;auto noiseGain="module:"+noiseOwner+":gain";bool noiseDiscovered=false;for(std::size_t i=0;i<s.get_parameter_catalog()->row_count();++i)if(s.get_parameter_catalog()->row_data(i)->id==noiseGain.c_str())noiseDiscovered=true;CHECK(noiseDiscovered);CHECK(s.invoke_command("route.add","lfo1",noiseGain.c_str(),.2));
 CHECK(s.invoke_slice_source(s.get_slice_sources(),"sl3")=="break-source");CHECK(s.invoke_slice_source(s.get_slice_sources(),"absent")=="");CHECK(s.invoke_command("asset.select","kick-source","",0));CHECK(s.invoke_slice_source(s.get_slice_sources(),"sl3")=="break-source");CHECK(s.invoke_command("slice.preview","sl3","",0));CHECK(s.get_selected_source()=="break-source");CHECK(s.get_selected_slice()=="sl3");CHECK(s.get_sounding_notes()->row_count()==0);
 CHECK(s.invoke_command("patch.load","kit","",0));CHECK(w->get_slice_owner()=="break-source");auto ownersBefore=s.get_slice_sources();auto provenance=b->model()->definition();auto sliceRecord=provenance.find("record \"slice\" \"sl3\"");CHECK(sliceRecord!=std::string::npos);auto ownerField=provenance.find("\"source\" \"break-source\"",sliceRecord);CHECK(ownerField!=std::string::npos);provenance.replace(ownerField,std::string("\"source\" \"break-source\"").size(),"\"source\" \"keys-source\"");CHECK(b->model()->importDefinition(provenance));b->refresh();CHECK(s.invoke_slice_source(s.get_slice_sources(),"sl3")=="keys-source");CHECK(s.get_slice_sources()==ownersBefore);CHECK(w->get_slice_owner()=="keys-source");
 auto projectedWave=s.get_waveform();auto beforeWaveImage=w->get_spectral_image();auto firstBucket=projectedWave->row_data(0).value();firstBucket.minimum=-1;firstBucket.maximum=1;projectedWave->set_row_data(0,firstBucket);CHECK(s.get_waveform()==projectedWave);CHECK(w->get_spectral_image()!=beforeWaveImage);
 CHECK(s.invoke_command("patch.load","kit","",0));
 for(const auto& name:{"amp","filter"})for(const auto& field:{"attack","decay","sustain","release"}){
  std::string parameter=std::string(name)+"-"+field;std::string owner=std::string(name)+"-env";
  auto expected=b->model()->find("parameter",parameter)->number("value");bool found=false;
  for(std::size_t i=0;i<s.get_modulators()->row_count();++i){auto mod=s.get_modulators()->row_data(i).value();if(mod.id==owner.c_str()){found=true;double projected=std::string(field)=="attack"?mod.attack:std::string(field)=="decay"?mod.decay:std::string(field)=="sustain"?mod.sustain:mod.release;CHECK(std::abs(projected-expected)<.0001);}}
  CHECK(found);CHECK(s.invoke_command("parameter.set",parameter.c_str(),"",expected+.1));CHECK(s.invoke_command("undo","","",0));CHECK(b->model()->find("modulator",owner)->number(field)==expected);
 }
 CHECK(s.invoke_command("state.select","pads","",0));CHECK(s.invoke_command("pad.select","","",127));CHECK(s.get_selected_pad()==127);
 auto selectedPadSource=s.get_selected_source();auto zoneCount=b->model()->records("zone").size();bool addFound=false;
 for(std::size_t i=0;i<s.get_inspector_rows()->row_count();++i){auto row=s.get_inspector_rows()->row_data(i).value();if(row.label=="Add zone…"){addFound=true;CHECK(row.action=="zone.add");CHECK(row.target==selectedPadSource);CHECK(row.amount==127);CHECK(s.invoke_command(row.action,row.target,row.field,row.amount));break;}}
 CHECK(addFound);CHECK(b->model()->records("zone").size()==zoneCount+1);CHECK(b->model()->records("zone").back().number("lo")==127);CHECK(s.invoke_command("undo","","",0));CHECK(b->model()->records("zone").size()==zoneCount);
 CHECK(s.invoke_command("patch.load","empty","",0));CHECK(s.get_inspector_type()=="Instrument");bool emptySounds=false;
 for(std::size_t i=0;i<s.get_inspector_rows()->row_count();++i){auto row=s.get_inspector_rows()->row_data(i).value();if(row.label=="Sounds")emptySounds=row.value=="None";CHECK(row.action!="zone.add");}CHECK(emptySounds);
 CHECK(b->model()->selectState("fx"));b->refresh();CHECK(s.get_fx_chains()->row_count()==5);std::size_t groupCount=0,busCount=0,processorCount=0;for(std::size_t i=0;i<s.get_fx_chains()->row_count();++i){auto chain=s.get_fx_chains()->row_data(i);CHECK(chain->processors!=nullptr);if(chain->kind=="group")++groupCount;else if(chain->kind=="bus")++busCount;for(std::size_t j=0;j<chain->processors->row_count();++j){auto processor=chain->processors->row_data(j);CHECK(processor->parent==chain->id);++processorCount;}}CHECK(groupCount==3);CHECK(busCount==2);CHECK(processorCount==9);CHECK(s.invoke_command("selector.axis","","macro",0));CHECK(s.get_selector_axis()=="macro");CHECK(s.invoke_command("selector.source","tone","",0));CHECK(s.get_selector_source()=="tone");CHECK(s.invoke_command("selector.lock","","",0));CHECK(!s.get_take_lock());CHECK(s.invoke_command("undo","","",0));CHECK(s.get_take_lock());CHECK(s.invoke_command("articulation.select","","stac",0));CHECK(s.get_articulation()=="stac");CHECK(s.invoke_command("patch.load","keys","",0));CHECK(s.invoke_command("zone.copy-group","k48p","",0));CHECK(s.get_zone_clipboard_count()==10);
 std::cout<<checks<<" native Session binding checks passed\n";return 0;
 }catch(const std::exception&e){std::cerr<<e.what()<<'\n';return 1;}}
