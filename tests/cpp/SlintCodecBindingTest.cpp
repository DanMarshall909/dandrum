#include "Controls.h"
#include "slint-testing.h"
#include "ui/slint/host/ValueCodec.h"
#include <iostream>
#include <stdexcept>

namespace {
void require(bool condition, const char* reason) {
    if (!condition) throw std::runtime_error(reason);
}
}

// The experimental Slint test backend creates the real generated component
// tree without opening a desktop window. Viewer/MCP tests own physical input.
int main() {
    try {
        slint::testing::init();
        const auto window = dandrum_ui::ControlsTest::create();
        const auto& codec = window->global<dandrum_ui::ValueCodec>();
        dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(codec);
        const auto parsed = codec.invoke_parse("-12 dB", "dB");
        require(parsed.valid && parsed.value == -12, "Generated codec callback did not parse actual units");
        const auto matches = slint::testing::ElementHandle::find_by_element_id(window, "ControlsTest::knob");
        require(matches.size() == 1, "Generated control is absent; build with SLINT_EMIT_DEBUG_INFO=1");
        matches[0].set_accessible_value("-12 dB");
        require(matches[0].accessible_value().value() == "−12.0", "Native unit entry did not commit and format correctly");
        matches[0].set_accessible_value("−3 dB");
        require(matches[0].accessible_value().value() == "−3.0", "Native Unicode minus entry did not commit correctly");
        const bool passed = window->invoke_run_tests();
        require(passed, window->get_test_report().data());
        std::cout << "PASS: generated C++ codec binding, native unit and Unicode entry; "
                  << window->get_test_report() << '\n';
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
