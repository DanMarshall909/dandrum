#include "AdvancedSamplerDomainPages.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/slint/host/ValueCodec.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <cmath>
#include <iostream>
#include <stdexcept>

namespace {
int checks = 0;
void require(bool value, std::string message) {
    ++checks;
    if (!value) throw std::runtime_error(message);
}
struct Fixture {
    slint::ComponentHandle<AdvancedSamplerDomainPages> window = AdvancedSamplerDomainPages::create();
    std::shared_ptr<dandrum::sampler::SessionBinding<Session>> binding = dandrum::sampler::bindWindowSession<Session>(window);
    const Session& session = window->global<Session>();
    Fixture() { dandrum::slint_ui::bind_value_codec<ParsedValue>(window->global<ValueCodec>()); window->show(); }
    ~Fixture() { window->hide(); }
    auto& model() { return *binding->model(); }
    void settle() { slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop(); }
    void command(std::string a, std::string t = {}, std::string text = {}, float v = 0) {
        require(session.invoke_command(a.c_str(), t.c_str(), text.c_str(), v), "Native command accepted: " + a); settle();
    }
    auto element(std::string label) {
        settle(); auto elements = slint::testing::ElementHandle::find_by_accessible_label(window, label);
        if (elements.empty()) throw std::runtime_error("Missing native control: " + label);
        return elements[0];
    }
    bool exists(std::string label) { settle(); return !slint::testing::ElementHandle::find_by_accessible_label(window, label).empty(); }
    void action(std::string label) {
        settle(); for (auto e : slint::testing::ElementHandle::find_by_accessible_label(window, label)) {
            if (e.accessible_role() == slint::language::AccessibleRole::Button) { e.invoke_accessible_default_action(); settle(); return; }
        }
        throw std::runtime_error("Missing native button: " + label);
    }
    slint::LogicalPosition point(std::string label, float x = .5, float y = .5) {
        auto e = element(label); auto p = e.absolute_position(); auto size = e.size(); return slint::LogicalPosition{{p.x + size.width*x, p.y + size.height*y}};
    }
    void click(slint::LogicalPosition p, slint::PointerEventButton button = slint::PointerEventButton::Left) {
        window->window().dispatch_pointer_press_event(p, button); window->window().dispatch_pointer_release_event(p, button); settle();
    }
    void openKeys() {
        command("patch.load", "keys"); command("zone.select", "k48p"); window->set_page("mapping"); settle();
    }
    void keysMenu() {
        auto p = point("Velocity high Felt C3 p", .4); p.y += 24;
        click(p, slint::PointerEventButton::Right); require(exists("Copy key range"), "Actual mapping context menu opens");
    }
    void key(slint::SharedString text) {
        window->window().dispatch_key_press_event(text); window->window().dispatch_key_release_event(text); settle();
    }
    void menuItem(int index) {
        for (int i = 0; i <= index; ++i) key(slint::platform::key_codes::DownArrow);
        key(slint::platform::key_codes::Return);
    }
    void drag(std::string label, float dx, float dy, bool cancel = false) {
        auto start = point(label); slint::LogicalPosition end{{start.x+dx, start.y+dy}};
        window->window().dispatch_pointer_press_event(start, slint::PointerEventButton::Left);
        window->window().dispatch_pointer_move_event(slint::LogicalPosition{{start.x+dx/2,start.y+dy/2}}); settle();
        window->window().dispatch_pointer_move_event(end); settle();
        if (cancel) { window->window().dispatch_key_press_event("\x1b"); window->window().dispatch_key_release_event("\x1b"); }
        window->window().dispatch_pointer_release_event(end, slint::PointerEventButton::Left); settle();
    }
};
void mappingCopyAndPaste() {
    Fixture f; f.openKeys(); auto original=f.model().definition(); auto history=f.model().undoCount();
    f.keysMenu(); f.action("Copy group"); require(f.session.get_zone_clipboard_count()==10, "Actual Copy group retains all ten Keys zones");
    require(f.model().undoCount()==history, "Copy group adds no edit");
    f.keysMenu(); f.action("Copy key range"); require(f.session.get_zone_clipboard_count()==2, "Copy key range retains both velocity layers");
    auto count=f.model().records("zone").size(); f.keysMenu(); f.action("Paste as layer");
    require(f.model().records("zone").size()==count+2, "Native Paste as layer preserves old zones and adds both copied layers");
    require(f.model().find("zone","k48p")->number("hi")==53, "Layer paste leaves original key range intact");
    require(f.model().undoCount()==history+1, "Layer paste is one Undo"); f.command("undo"); require(f.model().definition()==original, "Layer paste Undo restores complete definition");
    f.keysMenu(); f.action("Paste at D3"); require(f.model().find("zone","k48p")->number("hi")==49, "Ordinary paste replaces matching key range at hovered note");
    require(f.model().undoCount()==history+1, "Ordinary paste is one Undo"); f.command("undo"); require(f.model().definition()==original, "Ordinary paste Undo restores complete definition");
}
void mappingDeleteAndDismiss() {
    Fixture f; f.openKeys(); auto original=f.model().definition(); auto count=f.model().records("zone").size(); auto history=f.model().undoCount();
    f.keysMenu(); f.action("Delete zone"); require(f.model().records("zone").size()==count-1, "Native Delete removes exactly its target");
    require(f.model().find("zone","k55p")->number("lo")==48, "Delete closes the gap with the matching neighbour"); f.command("undo"); require(f.model().definition()==original, "Delete Undo restores all ranges");
    f.keysMenu(); f.action("Delete, leave gap"); require(!f.model().find("zone","k48p"), "Delete leave gap removes target");
    require(f.model().find("zone","k55p")->number("lo")==54, "Delete leave gap preserves neighbour"); f.command("undo"); require(f.model().definition()==original, "Leave gap Undo restores definition");
    f.keysMenu(); f.window->window().dispatch_key_press_event("\x1b"); f.window->window().dispatch_key_release_event("\x1b"); f.settle(); require(!f.exists("Copy key range"), "Escape dismisses actual mapping popup");
    f.keysMenu(); f.click(slint::LogicalPosition{{620,1100}}); require(!f.exists("Copy key range"), "Outside pointer dismisses popup");
    f.keysMenu(); f.click(slint::LogicalPosition{{620,1100}},slint::PointerEventButton::Right); require(!f.exists("Copy key range"), "Outside right-click dismisses popup");
    require(f.model().definition()==original&&f.model().undoCount()==history, "Dismissal never edits definition or history");
    f.action("Select mapping Felt C4 p"); require(f.session.get_selected_zone()=="k60p"&&f.session.get_selected_source()=="keys-source", "Actual inventory button selects native zone/source");
}
void mappingGap() {
    Fixture f; f.command("patch.load","kit"); f.window->set_page("mapping");
    auto hi=f.element("Velocity high Kick"),lo=f.element("Velocity low Kick");auto hp=hi.absolute_position(); auto lp=lo.absolute_position();auto size=hi.size();
    float height=(lp.y-hp.y)*127/126;slint::LogicalPosition p{{hp.x+size.width*9.5f,hp.y+2.5f+height*47/126}};
    auto original=f.model().definition();auto count=f.model().records("zone").size();f.click(p,slint::PointerEventButton::Right);f.action("Add zone in empty keys");
    auto zone=f.model().find("zone",f.model().text("selected-zone"));require(zone&&zone->number("lo")==43&&zone->number("hi")==45&&zone->number("root-note")==45, "Actual empty-key menu fills the bounded gap at note45");
    require(zone->text("source").empty()&&f.model().records("zone").size()==count+1, "Gap creates one truthful empty zone");f.command("undo");require(f.model().definition()==original,"Gap Undo restores definition");
}
void layersSplitAndOrder() {
    Fixture f;f.command("patch.load","kit");f.command("zone.select","snS");f.window->set_page("layers");
    auto old=f.model().definition();auto low=f.model().find("zone","snH")->number("vel-lo"),high=f.model().find("zone","snS")->number("vel-hi");auto undo=f.model().undoCount();
    f.drag("Velocity split",30,0);auto delta=f.model().find("zone","snS")->number("vel-hi")-high;
    require(delta>0&&f.model().find("zone","snH")->number("vel-lo")==low+delta, "Actual white split drag translates both boundaries equally");
    require(f.model().find("zone","snHB")->number("vel-lo")==low+delta,"Split keeps alternate velocity band aligned");
    require(f.model().undoCount()==undo+1,"Split drag coalesces one Undo");f.command("undo");require(f.model().definition()==old,"Split Undo restores all zone fields");
    f.drag("Velocity split",30,0,true);require(f.model().definition()==old&&f.model().undoCount()==undo,"Cancelled split restores all fields without Undo");
    f.command("zone.select","snHB");f.command("selector.mode","","round-robin");old=f.model().definition();undo=f.model().undoCount();
    f.drag("Move candidate Snare hard B",0,-88);f.command("selector.reset");require(f.model().selectCandidates(38,110).front()=="snHB","Actual row drag changes first played alternate");
    require(f.session.get_candidates()->row_data(0)->id=="snHB","Native candidate rows reflect reordered model");require(f.model().undoCount()==undo+1,"Candidate drag is one Undo");f.command("undo");require(f.model().definition()==old,"Candidate drag Undo restores definition");
    f.drag("Move candidate Snare hard B",0,-88,true);require(f.model().definition()==old&&f.model().undoCount()==undo,"Cancelled candidate drag preserves complete state/history");
}
void mappingCornerAndZoom() {
    Fixture f;f.openKeys();auto edge=f.element("Velocity high Felt C3 p");auto before=edge.absolute_position();auto size=edge.size();auto anchor=f.point("Velocity high Felt C3 p",.4);
    f.window->window().dispatch_key_press_event("\x11");f.window->window().dispatch_pointer_scroll_event(anchor,0,120);f.window->window().dispatch_key_release_event("\x11");f.settle();
    auto zoomed=f.element("Velocity high Felt C3 p");auto position=zoomed.absolute_position();auto zoomSize=zoomed.size();
    require(zoomSize.width>size.width,"Actual Ctrl+wheel zooms key map");require(std::abs((anchor.x-position.x)/zoomSize.width-.4)<.025,"Wheel zoom remains anchored at actual pointer note");
    f.window->window().dispatch_pointer_scroll_event(anchor,-60,0);f.settle();auto panned=f.element("Velocity high Felt C3 p");require(panned.absolute_position().x<position.x&&std::abs(panned.size().width-zoomSize.width)<.01,"Horizontal wheel pans without changing zoom");
    f.action("Fit");auto original=f.model().definition();auto history=f.model().undoCount();auto old=*f.model().find("zone","k48p");
    f.drag("Resize high key and high velocity Felt C3 p",20,-12);auto changed=f.model().find("zone","k48p");require(changed->number("hi")>old.number("hi")&&changed->number("vel-hi")>old.number("vel-hi"),"Actual corner changes both note and velocity boundaries");
    require(f.model().undoCount()==history+1,"Compound corner is one Undo");f.command("undo");require(f.model().definition()==original,"Corner Undo restores full ranges including pushed neighbour");
    f.drag("Resize high key and high velocity Felt C3 p",20,-12,true);require(f.model().definition()==original&&f.model().undoCount()==history,"Cancelled corner restores all fields without Undo");
}
void modulationDestinations() {
    Fixture f;f.command("state.select","mod");f.window->set_page("mod");f.action("Filter envelope");
    f.action("Add destination from selected modulator");require(f.window->get_last_context()=="modulator-destination"&&f.window->get_last_target()=="filter-env","Actual selected editor forwards canonical source to root picker");
}
void routeMenus() {
    Fixture f;f.command("state.select","mod");f.window->set_page("mod");auto old=*f.model().find("route","filter-route");
    auto open=[&]{f.click(f.point("Select route Filter envelope to Filter cutoff"),slint::PointerEventButton::Right);};
    f.command("route.select","amp-default");open();f.menuItem(0);require(f.session.get_selected_route()=="filter-route","Right-click Edit targets clicked route independently of old selection");
    auto history=f.model().undoCount();open();f.menuItem(1);require(!f.model().find("route","filter-route")->number("on"),"Popup Bypass changes targeted route");f.command("undo");require(f.model().find("route","filter-route")->fields==old.fields,"Bypass Undo restores complete route");
    open();f.menuItem(2);require(f.model().find("route","filter-route")->number("amount")==-old.number("amount"),"Popup Invert changes clicked amount");f.command("undo");
    open();f.menuItem(3);require(f.window->get_last_context()=="parameter-destination"&&f.window->get_last_target()=="cutoff","Popup destination forwards actual typed parameter");
    open();f.menuItem(4);require(!f.model().find("route","filter-route"),"Popup Remove deletes clicked route");f.command("undo");require(f.model().find("route","filter-route")->fields==old.fields,"Remove Undo restores route");
    f.click(f.point("Select route Amp envelope to Amp gain"),slint::PointerEventButton::Right);f.menuItem(4);require(f.model().text("amp-status")=="detached"&&!f.model().find("route","amp-default"),"Default route removal uses canonical Amp detach");f.command("undo");require(f.model().find("route","amp-default")&&f.model().undoCount()==history,"Amp detach Undo restores baseline/history");
}
}
int main() {
    slint::testing::init();int failures=0;
    for(auto test: {mappingCopyAndPaste,mappingDeleteAndDismiss,mappingGap,layersSplitAndOrder,mappingCornerAndZoom,modulationDestinations,routeMenus}) {
        try { test(); } catch(const std::exception& error) { ++failures; std::cerr<<"FAIL: "<<error.what()<<'\n'; }
    }
    std::cout<<checks<<" native interaction checks; "<<failures<<" failed groups\n";return failures?1:0;
}
