#include "ui/advanced-sampler/performance/Bindings.h"
#include "ui/advanced-sampler/engine/Model.h"
#include <functional>
#include <iostream>
#include <stdexcept>

struct PerformanceProbe {
    mutable int base{}; mutable bool pc{};
    mutable std::function<void()> release;
    mutable std::function<void(int)> octave;
    mutable std::function<void(bool)> enabled;
    void set_base_note(int value) const { base=value; }
    void set_pc_enabled(bool value) const { pc=value; }
    template<class F> void on_release_notes(F f) const { release=std::move(f); }
    template<class F> void on_octave_changed(F f) const { octave=std::move(f); }
    template<class F> void on_pc_changed(F f) const { enabled=std::move(f); }
};
struct SessionProbe {
    std::shared_ptr<dandrum::sampler::Model> owned;
    int refreshes{};
    auto model() const { return owned; }
    void refresh() { ++refreshes; }
};
struct SearchProbe {
    mutable std::function<bool(slint::SharedString,slint::SharedString)> contains;
    template<class F> void on_contains(F f) const { contains=std::move(f); }
};
int main() {
    int checks=0;
    const auto expect=[&](bool ok,const char* message){++checks;if(!ok)throw std::runtime_error(message);};
    try {
        auto model=std::make_shared<dandrum::sampler::Model>(); model->command("patch.load","kit");
        auto binding=std::make_shared<SessionProbe>(SessionProbe{model}); PerformanceProbe performance;
        dandrum::sampler::bind_performance_sources(performance,binding);
        expect(performance.base==36&&!performance.pc,"Binding initializes canonical shared state");
        performance.octave(48);expect(model->number("octave-base")==48&&binding->refreshes==1,"Absolute UI octave becomes one native delta and refresh");
        performance.enabled(true);expect(model->number("pc-keys")==1&&binding->refreshes==2,"Enable reaches canonical native state");
        model->noteOn(36,100,"bridge");performance.release();expect(model->heldNotes().empty()&&binding->refreshes==3,"Release balances native ownership and refreshes");
        performance.enabled(false);expect(model->number("pc-keys")==0&&binding->refreshes==4,"Disable reaches canonical native state");
        binding.reset();model->noteOn(36,100,"expired");
        performance.release();performance.octave(72);performance.enabled(true);
        expect(model->heldNotes()==std::vector<int>{36}&&model->number("octave-base")==48&&model->number("pc-keys")==0,"Every retained callback safely ignores an expired session");
        model->releaseNotes();
        SearchProbe search;dandrum::sampler::bind_patch_search(search);
        expect(search.contains("kick.wav","kick")&&!search.contains("kick.wav","snare"),"Native search finds substrings and rejects absent queries");
        expect(search.contains("kick.wav","")&&!search.contains("","kick"),"Native search handles empty strings without special ownership");
        std::cout<<"PASS: "<<checks<<" native performance bridge checks\n";return 0;
    } catch(const std::exception& error){std::cerr<<"FAIL: "<<error.what()<<'\n';return 1;}
}
