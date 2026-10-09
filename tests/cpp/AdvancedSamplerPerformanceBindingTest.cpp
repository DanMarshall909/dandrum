#include "AdvancedSamplerPerformance.h"
#include "../../ui/advanced-sampler/performance/Bindings.h"
#include <slint-testing.h>
#include <iostream>

int main()
{
    slint::testing::init();
    auto fixture = dandrum_ui::AdvancedSamplerPerformanceTest::create();
    dandrum::sampler::bind_patch_search(fixture->global<dandrum_ui::PatchFilter>());
    int octaveEvents = 0, releasedSources = 0, pcEvents = 0, lastOctave = 0;
    auto& state = fixture->global<dandrum_ui::PerformanceState>();
    state.on_octave_changed([&](int note) { ++octaveEvents; lastOctave = note; });
    state.on_pc_changed([&](bool) { ++pcEvents; });
    state.on_release_notes([&]() { ++releasedSources; });
    if (!fixture->invoke_run_tests()) {
        std::cerr << fixture->get_test_report() << '\n';
        return 1;
    }
    if (octaveEvents != 2 || lastOctave != 108 || pcEvents != 2 || releasedSources != 3) {
        std::cerr << "Performance controls did not publish balanced native octave/PC/release signals\n";
        return 1;
    }
    std::cout << "PASS: 3 native performance signal contracts\n";
    std::cout << fixture->get_test_report() << '\n';
    return 0;
}
