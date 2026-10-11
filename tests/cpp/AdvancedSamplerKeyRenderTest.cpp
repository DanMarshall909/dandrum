#include "AdvancedSamplerKeyRender.h"
#include <slint.h>
#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iostream>

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    // Set the explicit test scale inside the process, after the headless runner
    // has normalized its inherited display environment.
#ifdef _WIN32
    ::_putenv_s("SLINT_SCALE_FACTOR", "0.8");
#else
    ::setenv("SLINT_SCALE_FACTOR", "0.8", 1);
#endif
    auto window = dandrum_ui::AdvancedSamplerKeyRenderTest::create();
    bool passed = false;
    slint::Timer timer;
    timer.start(slint::TimerMode::SingleShot, std::chrono::milliseconds(200), [&] {
        const auto image = window->window().take_snapshot();
        if (image) {
            std::ofstream out(std::string(argv[1]) + "/keys.pam", std::ios::binary);
            out << "P7\nWIDTH " << image->width() << "\nHEIGHT " << image->height() << "\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n";
            out.write(reinterpret_cast<const char*>(image->begin()), image->width()*image->height()*4);
            passed = image->width() >= 960 && image->width() <= 961 && image->height() >= 640 && image->height() <= 641;
            for (int edge : {482,492,502,513,523,534}) {
                bool darkSeparator = false;
                for (int x = edge-1; x <= edge+1; ++x) {
                    const auto p = image->begin()[560*image->width()+x];
                    darkSeparator |= p.r < 150 && p.g < 150 && p.b < 150;
                }
                passed &= darkSeparator;
            }
            const auto cap = image->begin()[560*image->width()+475];
            passed &= cap.r > 160 && cap.g > 160 && cap.b > 150;
        }
        std::cout << (passed ? "PASS" : "FAIL") << ": Actual software renderer retains distinct white-key separators at explicit 80% scale\n";
        slint::quit_event_loop();
    });
    window->run();
    return passed ? 0 : 1;
}
