#include "AdvancedSamplerGuideControls.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include <slint-testing.h>
#include <iostream>
#include <stdexcept>
#include <array>
namespace {
int checks=0;
void require(bool condition,const std::string& message){++checks;if(!condition)throw std::runtime_error(message);}
template<class Window> void activate(const Window& window,std::string_view label){
    for(const auto& element:slint::testing::ElementHandle::find_by_accessible_label(window,label))
        if(element.accessible_role()==slint::language::AccessibleRole::Button){
            const auto p=element.absolute_position();const auto s=element.size();
            const slint::LogicalPosition centre{{p.x+s.width/2,p.y+s.height/2}};
            window->window().dispatch_pointer_press_event(centre,slint::PointerEventButton::Left);
            window->window().dispatch_pointer_release_event(centre,slint::PointerEventButton::Left);return;
        }
    throw std::runtime_error("Missing actual native guide action: "+std::string(label));
}
bool contains(const auto& window,std::string_view label){return !slint::testing::ElementHandle::find_by_accessible_label(window,label).empty();}
struct State{const char* id;const char* label;const char* page;};
constexpr std::array states={
State{"empty","01 Empty","empty"},State{"loaded","02 Single sample","sample"},State{"sample","03 Sample editing","sample"},
State{"loop","04 Loop editing","sample"},State{"slices","05 Slice editing","slices"},State{"transients","06 Transient analysis","slices"},
State{"mapping","07 Key map","mapping"},State{"vel","08 Velocity layers","layers"},State{"rr","09 Round robin","layers"},
State{"voice","10 Voice shaping","voice"},State{"mod","11 Modulation","mod"},State{"macro","12 Macro editing","macros"},
State{"routing","13 Routing","routing"},State{"fx","14 Effects","fx"},State{"missing","15 Missing asset","sample"},
State{"unsupported","16 Unsupported asset","sample"},State{"loading","17 Loading","sample"},State{"running","18 Analysis running","slices"},
State{"failed","19 Analysis failed","slices"},State{"min","20 Minimum 820×560","sample"},State{"default","21 Default 1200×800","mapping"},
State{"exp","22 Expanded 1600×1000","voice"},State{"drop","23 Multi-file drop","empty"},State{"perf","24 Compact · macros only","perform"},
State{"pads","25 Pads · performance","pads"},State{"recon","26 Reload pending","mapping"},State{"diag","27 Diagnostics","diagnostics"},
State{"overview","28 Overview · inheritance","overview"},State{"freeze","29 Freeze","voice"},State{"macros","30 Macros","macros"}};
}
int main(){try{
    slint::testing::init();auto window=dandrum_ui::AdvancedSamplerGuideControlsTest::create();
    auto binding=dandrum::sampler::bindWindowSession<dandrum_ui::Session>(window);
    const auto& session=window->global<dandrum_ui::Session>();const auto& presentation=window->global<dandrum_ui::GuidePresentation>();
    window->window().show();
    require(!presentation.get_enabled()&&window->get_editor_origin()==0,"Normal editor crop is unchanged by the opt-in harness");
    require(!contains(window,"Acceptance patch Expressive kit"),"Normal editor has no guide-only patch controls");
    presentation.set_enabled(true);
    for(float width:{820.f,1200.f,1600.f}){
        window->window().set_size(slint::LogicalSize{{width,1000}});
        for(const auto& state:states){
            auto elements=slint::testing::ElementHandle::find_by_accessible_label(window,state.label);
            require(!elements.empty(),"Every guide state remains accessible at each editor size");
            for(const auto& element:elements)if(element.accessible_role()==slint::language::AccessibleRole::Button){
                const auto position=element.absolute_position();const auto size=element.size();
                require(position.x>=0&&size.width>=180&&position.x+size.width<=width&&position.y>=0&&position.y+size.height<=window->get_editor_origin(),"Guide state action fits entirely above the editor crop: "+std::string(state.label)+" at width "+std::to_string(width)+" x="+std::to_string(position.x)+" y="+std::to_string(position.y)+" w="+std::to_string(size.width)+" h="+std::to_string(size.height)+" editor="+std::to_string(window->get_editor_origin()));
            }
        }
    }
    window->window().set_size(slint::LogicalSize{{1200,760}});
    for(const auto& patch:std::array{std::array{"keys","Multisampled keys","mapping"},std::array{"hybrid","Sample + synth","voice"},std::array{"kit","Expressive kit","pads"}}){
        activate(window,std::string("Acceptance patch ")+patch[1]);
        require(session.get_current_patch_id()==patch[0],"Actual guide patch selector loads "+std::string(patch[0]));
        require(session.get_page()==patch[2],"Patch selection opens its guide acceptance page");
        require(session.get_tree()->row_count()>1&&session.get_sources()->row_count()>0,"Acceptance patch loads actual native inventory");
        require(!session.get_can_undo(),"Patch loading retains its native new-session history boundary");
    }
    for(const char* patch:{"keys","hybrid","kit"}){
        require(session.invoke_command("patch.load",patch,"",0),"Load acceptance patch for state-preservation family");
        require(session.invoke_command("parameter.set","cutoff","",4321),"Create a native edit before guide presentation navigation");
        require(session.invoke_command("parameter.set","resonance","",.7)&&session.invoke_command("undo","","",0),"Retain a redo entry before presentation navigation");
        const auto definition=binding->model()->definition();const auto undo=binding->model()->undoCount();
        const auto redo=binding->model()->redoCount();
        const auto sources=session.get_sources()->row_count();const auto zones=session.get_zones()->row_count();
        for(const auto& state:states){
            activate(window,state.label);
            require(session.get_state_id()==state.id,"Actual state action selects "+std::string(state.id));
            require(session.get_page()==state.page,"Selected guide state opens its native page");
            require(session.get_current_patch_id()==patch,"Guide state preserves selected acceptance patch");
            require(binding->model()->definition()==definition&&binding->model()->undoCount()==undo&&binding->model()->redoCount()==redo,"Guide presentation navigation preserves definition, undo and redo history");
            require(session.get_sources()->row_count()==sources&&session.get_zones()->row_count()==zones,"Guide state navigation preserves native patch inventory");
        }
    }
    require(window->get_requested_width()==860&&window->get_requested_height()==610,"Compact performance state requests exact reference editor size");
    for(const auto& layout:std::array{std::array{"20 Minimum 820×560", "820", "560"},std::array{"21 Default 1200×800","1200","800"},std::array{"22 Expanded 1600×1000","1600","1000"}}){
        activate(window,layout[0]);require(window->get_requested_width()==std::stof(layout[1])&&window->get_requested_height()==std::stof(layout[2]),"Actual size state requests exact guide editor dimensions");
    }
    const auto definition=binding->model()->definition();const auto undo=binding->model()->undoCount();
    require(!session.invoke_command("guide.select","unknown-state","",0),"Unknown guide states are rejected");
    require(session.get_state_id()=="exp"&&session.get_page()=="voice"&&binding->model()->definition()==definition,"Rejected state preserves current view and patch");
    activate(window,"Engine contract annotations");
    require(presentation.get_annotations(),"Actual annotation toggle changes presentation state");
    require(contains(window,"Engine annotation Voice Template (curated view)"),"Annotation toggle presents the actual selected-page engine contract chip");
    require(contains(window,"Engine annotation Telemetry"),"Enabled annotations present header engine telemetry ownership");
    require(binding->model()->definition()==definition&&binding->model()->undoCount()==undo,"Annotation presentation preserves patch definition and history");
    for(const auto& page:std::array{
        std::array{"03 Sample editing","Asset binding on AssetPlayer"},
        std::array{"05 Slice editing","Analysis (async)"},
        std::array{"07 Key map","Trigger Rules: note × velocity"},
        std::array{"08 Velocity layers","Selector policy"},
        std::array{"11 Modulation","Modulation Route"},
        std::array{"13 Routing","Bus → plugin output Connection"},
        std::array{"14 Effects","Processor modules + Connections"},
        std::array{"24 Compact · macros only","Macro Parameters + Bindings"},
        std::array{"25 Pads · performance","PadGrid: playable Trigger Rules"},
        std::array{"27 Diagnostics","Diagnostics"},
        std::array{"28 Overview · inheritance","Property inheritance"},
        std::array{"30 Macros","Macro Parameters + Bindings"},
        std::array{"01 Empty","Asset service"}}){
        activate(window,page[0]);
        require(contains(window,std::string("Engine annotation ")+page[1]),"Annotations describe the selected page's engine ownership");
    }
    require(session.invoke_note_on(60,100,"guide-test"),"Begin a note before guide navigation");
    activate(window,"03 Sample editing");
    require(binding->model()->heldNotes().empty(),"Guide navigation releases held notes");
    activate(window,"Engine contract annotations");
    require(!presentation.get_annotations()&&!contains(window,"Engine annotation Telemetry"),"Disabling annotations removes actual annotation presentation");
    window->window().dispatch_key_press_event(" ");window->window().dispatch_key_release_event(" ");
    require(presentation.get_annotations(),"The annotation control is also keyboard operable");
    require(binding->model()->definition()==definition&&binding->model()->undoCount()==undo,"All annotation views preserve the edited native patch");
    presentation.set_enabled(false);require(window->get_editor_origin()==0,"Disabling harness restores the exact ordinary crop");
    window->window().hide();std::cout<<"PASS: "<<checks<<" actual native guide control checks\n";
}catch(const std::exception& error){std::cerr<<"FAIL: "<<error.what()<<'\n';return 1;}}
