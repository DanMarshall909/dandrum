#include "../../ui/slint/host/ValueCodec.h"

#include <cmath>
#include <functional>
#include <iostream>
#include <stdexcept>
#include <string>
#include <string_view>

namespace {
using dandrum::slint_ui::parse_value;
using dandrum::slint_ui::format_value;
struct StringCodec {
    using Parse = std::function<dandrum::slint_ui::ParsedNumber(const std::string&, const std::string&)>;
    using Format = std::function<std::string(float, const std::string&)>;
    mutable Parse parse;
    mutable Format format;
    void on_parse(Parse callback) const { parse = callback; }
    void on_format(Format callback) const { format = callback; }
};
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
void string_binding() {
    // Unlike Slint SharedString, this supported callback string type retains NUL bytes.
    const StringCodec codec;
    dandrum::slint_ui::bind_value_codec<dandrum::slint_ui::ParsedNumber>(codec);
    const std::string nul_text("2\0junk", 6), nul_decibels("dB\0junk", 7), nul_pan("pan\0junk", 8);
    require(!codec.parse(nul_text, "").valid, "Generic binding must parse the complete embedded-NUL value");
    require(!codec.parse("2 dB", nul_decibels).valid, "Generic binding must not truncate a unit into decibels");
    require(!codec.parse("L20", nul_pan).valid, "Generic binding must not truncate a unit into pan notation");
    const auto bare = codec.parse("2", nul_decibels);
    require(bare.valid && bare.value == 2, "Generic binding accepts bare numbers for an unknown unit");
    require(codec.format(3.04f, nul_decibels) == "3.04", "Generic binding retains the complete unit instead of applying decibel precision");
    require(codec.format(-0.4f, nul_pan) == "−0.4", "Generic binding retains the complete unit instead of applying pan notation");
    const auto ordinary = codec.parse("-12 dB", "dB");
    require(ordinary.valid && ordinary.value == -12, "Generic binding preserves ordinary unit parsing");
    require(codec.format(-12, "dB") == "−12.0", "Generic binding preserves ordinary Unicode formatting");
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
        parses("c", "pan", 0); parses("l25", "pan", -0.5f); parses("r25", "pan", 0.5f);
        parses("0.125 kHz", "Hz", 125); parses("125 Hz", "kHz", 0.125f);
        rejects("", "dB"); rejects("  ", ""); rejects("garbage", "dB");
        rejects("3 dB junk", "dB"); rejects("3 Hz", "dB"); rejects("3 dB", "");
        rejects("--3", "dB"); rejects("3.0.1", ""); rejects("nan", "");
        rejects("NaN", ""); rejects("inf", ""); rejects("-inf", "");
        rejects("1e309", ""); rejects("1e39", ""); rejects("1e38 kHz", "Hz");
        rejects("R51", "pan"); rejects("L-20", "pan"); rejects("Rnan", "pan");
        rejects(std::string_view("2\0junk", 6), "");
        rejects("2 dB", std::string_view("dB\0junk", 7));
        rejects("L20", std::string_view("pan\0junk", 8));
        parses("2", std::string_view("dB\0junk", 7), 2);
        require(format_value(-3.04f, "dB") == "−3.0", "Decibels use Unicode minus and one decimal");
        require(format_value(1.5f, "×") == "1.500", "Ratios use three decimals without duplicating the unit");
        require(format_value(12.7f, "%") == "13", "Percent values use integer display");
        require(format_value(-0.4f, "pan") == "L20", "Pan display uses the guide's L50 to R50 convention");
        require(format_value(0, "pan") == "C", "Centered pan has an explicit C label");
        require(format_value(1, "pan") == "R50", "Right edge is R50");
        require(format_value(123.6f, "ms") == "124", "Milliseconds use integer display");
        require(format_value(1.23456f, "") == "1.235", "Generic values retain up to three useful decimals");
        require(format_value(-0.00001f, "") == "0", "Formatting does not display negative zero");
        require(format_value(-2, "pan") == "L50" && format_value(2, "pan") == "R50", "Pan notation clamps to its display extent");
        require(format_value(0.9996f, "x") == "1.000", "Ratio precision preserves trailing decimals after rounding");
        require(format_value(3.04f, "dB") == "3.0", "Decibel precision preserves its trailing decimal");
        require(format_value(3.04f, std::string_view("dB\0junk", 7)) == "3.04", "An embedded NUL does not turn an unknown unit into decibels");
        require(format_value(-0.4f, std::string_view("pan\0junk", 8)) == "−0.4", "An embedded NUL does not turn an unknown unit into pan notation");
        require(format_value(std::numeric_limits<float>::infinity(), "") == "—", "Nonfinite display uses the unavailable marker");
        string_binding();
        std::cout << "PASS: " << checks << " native Slint value codec checks\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
