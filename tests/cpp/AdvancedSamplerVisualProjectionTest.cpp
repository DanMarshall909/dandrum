#include "AppWindow.h"
#include "ui/advanced-sampler/engine/Bindings.h"
#include "ui/advanced-sampler/theme/ThemeBinding.h"
#include <slint-testing.h>
#include <slint-platform.h>
#include <iostream>
#include <stdexcept>
#include <set>

int main() {
    int checks = 0;
    try {
        using namespace dandrum_ui;
        slint::testing::init();
        auto window = AppWindow::create();
        auto binding = dandrum::sampler::bindWindowSession<Session>(window);
        auto model = binding->model();
        const auto& session = window->global<Session>();
        dandrum::advanced_sampler::apply_theme(window->global<SamplerTheme>(), dandrum::advanced_sampler::resolve_theme({}));
        const auto require = [&](bool ok, const char* why) { ++checks; if (!ok) throw std::runtime_error(why); };
        require(session.invoke_command("state.select", "sample", "", 0), "Open actual source metadata");
        require(session.get_sample_name()=="felt_C4_f.wav", "Source labels project the real cached filename independently of Inspector selection");
        require(session.get_sample_progress()==1, "Cached native source is fully decoded");
        require(session.invoke_command("state.select","loading","",0) && std::abs(session.get_sample_progress()-.62)<.000001,
                "Loading view projects the actual selected source's decoded fraction");
        require(session.invoke_command("state.select","sample","",0), "Restore actual source metadata");
        bool format=false, channels=false;
        for(std::size_t i=0;i<session.get_inspector_rows()->row_count();++i){
            const auto row=*session.get_inspector_rows()->row_data(i);
            format |= row.label=="Format" && row.value=="48 kHz · 24-bit";
            channels |= row.label=="Channels" && row.value=="Stereo";
        }
        require(format && channels, "Source format and channel count retain separate truthful readouts");
        require(session.invoke_command("state.select", "pads", "", 0), "Open guide kit pads");
        auto snare = session.get_pads()->row_data(38).value();
        require(snare.name == "Snare" && snare.layers == 2 && snare.alternates == 2,
                "Kit Snare summarizes two velocity bands and two hard alternates");
        require(!session.get_pads()->row_data(40)->mapped,
                "Kit pad population excludes the unrelated hybrid and keys inventory");
        require(model->records("zone").size() == 27 && session.get_mapped_high() == 96,
                "Pad scoping preserves the complete native inventory and keyboard Fit bounds");
        require(session.get_mapping_low() == 24 && session.get_mapping_high() == 47,
                "Kit map retains its separate two-octave viewport");
        require(session.get_buses()->row_count() == 5 && session.get_routing_buses()->row_count() == 3,
                "Routing projects three plugin outputs while preserving all five bus destinations");
        auto mainBus = session.get_routing_buses()->row_data(0).value();
        const auto snareBus = session.get_routing_buses()->row_data(2).value();
        require(snareBus.name == "Snare 5/6" && snareBus.channel_summary == "Out 5/6",
                "Snare plugin-output caption identifies its physical stereo pair");
        require(mainBus.channels->row_count() == 2
                && mainBus.channels->row_data(0)->peak != mainBus.channels->row_data(1)->peak
                && mainBus.channels->row_data(0)->ticket == "guide-fixture",
                "Routing meters use independent native fixture channels, labelled as mock measurements");
        require(session.get_inspector_title() == "Snare", "Pad inspector uses the same native sound owner");
        require(session.invoke_command("state.select", "routing", "", 0), "Open actual Snare routing");
        bool ampDefault=false, filterOrdinary=false;
        for (std::size_t i=0;i<session.get_routes()->row_count();++i) {
            const auto row=*session.get_routes()->row_data(i);
            ampDefault |= row.id=="amp-default" && row.default_route;
            filterOrdinary |= row.id=="filter-route" && !row.default_route;
        }
        require(ampDefault && filterOrdinary,
                "Default-route badges project actual route metadata rather than source names");
        const auto sendText = [&](std::string_view label) {
            for (std::size_t i = 0; i < session.get_inspector_rows()->row_count(); ++i) {
                const auto row = session.get_inspector_rows()->row_data(i).value();
                if (std::string_view(row.label.data()) == label) return std::string(row.value.data());
            }
            throw std::runtime_error("Missing routing send value");
        };
        require(sendText("Reverb") == "-10.5 dB" && sendText("Delay") == "-14.9 dB",
                "Routing inspector presents actual linear sends in decibels");
        require(session.invoke_command("send.set", "snare", "Reverb", 0)
                && sendText("Reverb") == "−∞ dB",
                "A silent native send displays negative infinity instead of a finite gain");
        require(session.invoke_command("state.select", "voice", "", 0), "Open selected Filter module");
        bool filterType = false, filterSlope = false;
        for (std::size_t i = 0; i < session.get_inspector_rows()->row_count(); ++i) {
            const auto row = session.get_inspector_rows()->row_data(i).value();
            filterType |= row.label == "Type" && row.value == "Ladder";
            filterSlope |= row.label == "Slope" && row.value == "24 dB/oct";
        }
        require(filterType && filterSlope, "Filter inspector projects the processor's persisted type and slope");
        std::set<std::string> destinations;
        for (std::size_t i = 0; i < session.get_inspector_rows()->row_count(); ++i) {
            const auto row = session.get_inspector_rows()->row_data(i).value();
            if (row.kind == "modulation") destinations.insert(row.parameter_id.data());
        }
        require(destinations.contains("cutoff") && !destinations.contains("pitch") && !destinations.contains("pan"),
                "Filter inspector lists routes for the selected processor only");
        require(session.invoke_command("state.select", "overview", "", 0), "Open native overview");
        bool releaseEnabled = false, mainArticulation = false;
        for (std::size_t i = 0; i < session.get_overview()->row_count(); ++i) {
            const auto row = session.get_overview()->row_data(i).value();
            releaseEnabled |= row.id == "release-sample" && row.value_text == "Yes";
            mainArticulation |= row.id == "articulation" && row.value_text == "Main hit";
        }
        require(releaseEnabled && mainArticulation,
                "Overview avoids inventing take counts or keyswitch assignments from boolean settings");
        require(session.invoke_command("state.select", "sample", "", 0), "Open source history");
        window->show();
        slint::platform::update_timers_and_animations();
        slint::invoke_from_event_loop([] { slint::quit_event_loop(); }); slint::run_event_loop();
        const auto inspectors = slint::testing::ElementHandle::find_by_element_type_name(window, "Inspector");
        require(inspectors.size() == 1, "One native inspector is composed");
        bool heading = false;
        for (const auto& row : slint::testing::ElementHandle::find_by_element_type_name(window, "Text")) {
            const auto text = row.accessible_label();
            if (text && *text == "HISTORY") {
                const auto p = row.absolute_position(); const auto ip = inspectors[0].absolute_position();
                require(p.x == ip.x + 13 && p.y < ip.y + 90,
                        "Source HISTORY is anchored above operations instead of centered over them");
                heading = true;
            }
        }
        require(heading, "Source HISTORY has a visible accessible heading");
        window->hide();
        std::cout << "PASS: " << checks << " native visual projection checks\n";
    } catch (const std::exception& e) { std::cerr << "FAIL: " << e.what() << '\n'; return 1; }
}
