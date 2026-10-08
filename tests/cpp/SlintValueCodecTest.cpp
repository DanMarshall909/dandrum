#include "../../ui/slint/host/ValueCodec.h"

#include <cmath>
#include <iostream>
#include <stdexcept>
#include <string_view>

namespace {
using dandrum::slint_ui::parse_value;
using dandrum::slint_ui::format_value;
int checks = 0;
void require(bool condition, std::string_view reason) {
    ++checks;
    if (!condition) throw std::runtime_error(std::string(reason));
}
void parses(std::string_view text, std::string_view unit, float expected) {
    const auto result = parse_value(text, unit);
    require(result.valid && std::abs(result.value - expected) < 0.0001f,
            "The native codec must parse a complete value in the declared unit");
}
void rejects(std::string_view text, std::string_view unit) {
    require(!parse_value(text, unit).valid, "Malformed, nonfinite, mismatched, or overflowing values must not silently become zero");
}
}
int main() {
    try {
        parses("-3", "dB", -3); parses("-3 dB", "dB", -3);
        parses("−3.5 dB", "dB", -3.5f); parses("  +2.5 dB \t", "dB", 2.5f);
        parses("1.500×", "×", 1.5f); parses("1.5x", "×", 1.5f);
        parses("1.5 ×", "x", 1.5f); parses("25%", "%", 25);
        parses("120 ms", "ms", 120); parses("1.25 kHz", "Hz", 1250);
        parses("250 Hz", "kHz", 0.25f); parses("2e3 Hz", "Hz", 2000);
        parses("L20", "pan", -0.4f); parses("R50", "pan", 1);
        parses("C", "pan", 0); parses("-0.25", "pan", -0.25f);
        rejects("", "dB"); rejects("  ", ""); rejects("garbage", "dB");
        rejects("3 dB junk", "dB"); rejects("3 Hz", "dB"); rejects("3 dB", "");
        rejects("--3", "dB"); rejects("3.0.1", ""); rejects("nan", "");
        rejects("NaN", ""); rejects("inf", ""); rejects("-inf", "");
        rejects("1e309", ""); rejects("1e39", ""); rejects("1e38 kHz", "Hz");
        rejects("R51", "pan"); rejects("L-20", "pan"); rejects("Rnan", "pan");
        require(format_value(-3.04f, "dB") == "−3.0", "Decibels use Unicode minus and one decimal");
        require(format_value(1.5f, "×") == "1.500", "Ratios use three decimals without duplicating the unit");
        require(format_value(12.7f, "%") == "13", "Percent values use integer display");
        require(format_value(-0.4f, "pan") == "L20", "Pan display uses the guide's L50 to R50 convention");
        require(format_value(0, "pan") == "C", "Centered pan has an explicit C label");
        require(format_value(1, "pan") == "R50", "Right edge is R50");
        require(format_value(123.6f, "ms") == "124", "Milliseconds use integer display");
        require(format_value(1.23456f, "") == "1.235", "Generic values retain up to three useful decimals");
        require(format_value(-0.00001f, "") == "0", "Formatting does not display negative zero");
        std::cout << "PASS: " << checks << " native Slint value codec checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
