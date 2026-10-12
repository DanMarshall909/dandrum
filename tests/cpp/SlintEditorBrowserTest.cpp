#include "SlintEditorBrowser.h"
#include "ui/slint/host/SearchFilter.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <iostream>
#include <stdexcept>

struct SparsePresets : slint::Model<dandrum_ui::PresetInfo> {
    std::size_t row_count() const override { return 2; }
    std::optional<dandrum_ui::PresetInfo> row_data(std::size_t i) const override {
        if(i!=1)return std::nullopt;
        dandrum_ui::PresetInfo row;row.id="sparse";row.name="Needle";return row;
    }
};
int main() {
    int checks=0;
    auto require=[&](bool ok,const char* message){++checks;if(!ok)throw std::runtime_error(message);};
    try {
        slint::testing::init();auto window=dandrum_ui::SlintEditorBrowser::create();window->show();
        const auto& filter=window->global<dandrum_ui::PresetFilter>();
        const auto& state=window->global<dandrum_ui::PresetBrowserState>();
        dandrum::slint_ui::bind_search_filter(filter);
        window->on_favourite([&](slint::SharedString id){auto presets=window->get_presets();for(std::size_t i=0;i<presets->row_count();++i){auto row=presets->row_data(i).value();if(row.id==id){row.favourite=!row.favourite;presets->set_row_data(i,row);}}});
        const auto settle=[] {slint::platform::update_timers_and_animations();slint::invoke_from_event_loop([]{slint::quit_event_loop();});slint::run_event_loop();};
        const auto element=[&](const char* label){settle();auto found=slint::testing::ElementHandle::find_by_accessible_label(window,label);for(auto item:found)if(item.accessible_role().has_value())return item;throw std::runtime_error(std::string("Missing ")+label);};
        const auto click=[&](const char* label){auto item=element(label);auto p=item.absolute_position();auto s=item.size();auto at=slint::LogicalPosition{{p.x+s.width/2,p.y+s.height/2}};window->window().dispatch_pointer_press_event(at,slint::PointerEventButton::Left);window->window().dispatch_pointer_release_event(at,slint::PointerEventButton::Left);settle();};
        const auto key=[&](slint::SharedString text){window->window().dispatch_key_press_event(text);window->window().dispatch_key_release_event(text);settle();};
        require(window->get_matched_count()==3&&window->get_chosen_id()=="felt","Standalone browser projects supplied presets");
        require(filter.invoke_count(window->get_presets(),"samples","Keys")==1&&filter.invoke_first(window->get_presets(),"samples","Keys")=="felt","Native filtering combines category and contents metadata");
        require(filter.invoke_count(window->get_presets(),"upright","Drums")==0&&filter.invoke_first(window->get_presets(),"upright","Drums").empty(),"A query match in another category remains excluded");
        element("Search patches").set_accessible_value("UPriGHT");settle();
        require(window->get_matched_count()==1&&window->get_chosen_id()=="felt","Shared search performs case-insensitive substring matching");
        state.set_query("ÉT");require(window->get_matched_count()==1&&window->get_chosen_id()=="user","Shared search lowercases Unicode before UTF-8 substring matching");
        state.set_query("");click("User patches");require(window->get_matched_count()==1&&window->get_chosen_id()=="user","User category filters supplied origin metadata");
        click("All patches");
        for(bool compact:{false,true}) {
            window->set_compact(compact);state.set_selected_id("");
            auto loads=window->get_loads();
            click("Favourite Felt Upright");require(window->get_presets()->row_data(0)->favourite,"Favourite pointer emits stable preset ID");
            auto favourite=element("Remove favourite Felt Upright");
            require(favourite.accessible_checkable()==true&&favourite.accessible_checked()==true,"Favourite exposes a checked accessible action");
            favourite.invoke_accessible_default_action();settle();require(!window->get_presets()->row_data(0)->favourite,"Favourite supports assistive technology");
            key(" ");require(window->get_presets()->row_data(0)->favourite,"Space toggles the focused favourite");
            key(slint::platform::key_codes::Return);require(!window->get_presets()->row_data(0)->favourite&&window->get_loads()==loads,"Favourite Return never loads its containing row");
            click("Dry Drums · Drums · Samples");require(window->get_chosen_id()=="drums","Row pointer selects its supplied ID");
            require(window->get_loads()==loads+(compact?1:0),"Only compact rows load on a single click");
            if(!compact){click("Load");require(window->get_loaded()=="drums","Expanded browser Load emits selected ID");}
        }
        window->set_compact(false);state.set_selected_id("");state.set_query("");
        click("Favourite Felt Upright");click("Favourites patches");require(window->get_matched_count()==1,"Favourite filter observes host model updates");
        click("Remove favourite Felt Upright");require(window->get_matched_count()==0,"Removing favourite updates active filter");
        click("All patches");state.set_query("absent");require(window->get_matched_count()==0&&window->get_chosen_id().empty(),"Unmatched query clears the fallback selection");
        require(element("Load").accessible_enabled()==false,"Unmatched query disables Load");
        state.set_query("");click("Import patch definition");require(window->get_action()=="import","Import action delegates to host");
        click("Export patch definition");require(window->get_action()=="export","Export action delegates to host");
        click("Copy definition");require(window->get_action()=="copy","Copy action delegates to host");
        click("Search patches");key(slint::platform::key_codes::Escape);require(window->get_closes()==1,"Escape from query delegates close");
        // Characterize every former supported index, then exercise larger native catalogs.
        std::vector<dandrum_ui::PresetInfo> rows(65);
        for(std::size_t i=0;i<rows.size();++i){rows[i].id=slint::SharedString(std::to_string(i));rows[i].name="Other";}
        auto bounded=std::make_shared<slint::VectorModel<dandrum_ui::PresetInfo>>(rows);
        for(std::size_t i=0;i<64;++i){
            if(i){rows[i-1].name="Other";bounded->set_row_data(i-1,rows[i-1]);}
            rows[i].name="Needle";bounded->set_row_data(i,rows[i]);
            require(filter.invoke_count(bounded,"needle","All")==1,"Each supported row participates in count");
            require(filter.invoke_first(bounded,"needle","All")==rows[i].id,"Each supported row participates in first match");
        }
        rows[63].name="Other";bounded->set_row_data(63,rows[63]);rows[64].name="Needle";bounded->set_row_data(64,rows[64]);
        require(filter.invoke_count(bounded,"needle","All")==1&&filter.invoke_first(bounded,"needle","All")=="64","Native filtering includes presets beyond the former preview limit");
        state.set_selected_id("");state.set_category("All");state.set_query("needle");window->set_presets(bounded);
        require(window->get_matched_count()==1&&window->get_chosen_id()=="64","Native browser uses the complete supplied catalog");
        rows[0].name="Needle";bounded->set_row_data(0,rows[0]);
        require(window->get_matched_count()==2&&window->get_chosen_id()=="0","Native filter dependencies observe model row updates");
        auto appended=rows[0];appended.id="appended";bounded->push_back(appended);
        require(window->get_matched_count()==3,"Native filter dependencies observe appended matching rows");
        bounded->erase(0);
        require(window->get_matched_count()==2&&window->get_chosen_id()=="64","Native filter dependencies observe removed rows and update the first match");
        require(filter.invoke_count({},"","All")==0&&filter.invoke_first({},"","All").empty(),"Missing preset models produce no match");
        auto sparse=std::make_shared<SparsePresets>();
        require(filter.invoke_count(sparse,"needle","All")==1&&filter.invoke_first(sparse,"needle","All")=="sparse","Missing model rows are skipped safely");
        std::cout<<"PASS: "<<checks<<" standalone preset browser checks\n";
    } catch(const std::exception& error){std::cerr<<"FAIL after "<<checks<<" checks: "<<error.what()<<'\n';return 1;}
}
