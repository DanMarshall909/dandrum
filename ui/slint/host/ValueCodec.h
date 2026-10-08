#pragma once

#include <algorithm>
#include <charconv>
#include <cmath>
#include <limits>
#include <string>
#include <string_view>
#include <system_error>
#include <type_traits>

namespace dandrum::slint_ui {
struct ParsedNumber { bool valid = false; float value = 0; };

namespace detail {
inline std::string_view trim(std::string_view text) {
    constexpr auto whitespace = " \t\r\n";
    const auto first = text.find_first_not_of(whitespace);
    if (first == std::string_view::npos) return {};
    const auto last = text.find_last_not_of(whitespace);
    return text.substr(first, last - first + 1);
}
inline ParsedNumber checked(double value) {
    if (!std::isfinite(value) || std::abs(value) > std::numeric_limits<float>::max()) return {};
    return { true, static_cast<float>(value) };
}
inline bool ratio_unit(std::string_view unit) { return unit == "x" || unit == "×"; }
} // namespace detail

// Values are in the parameter's declared units. Clamping/stepping belongs to the
// range control; this codec only parses complete, finite values unambiguously.
inline ParsedNumber parse_value(std::string_view text, std::string_view unit) {
    text = detail::trim(text); unit = detail::trim(unit);
    if (text.empty()) return {};
    std::string normalized(text);
    if (normalized.starts_with("−")) normalized.replace(0, std::string_view("−").size(), "-");
    text = normalized;
    if (unit == "pan") {
        if (text == "C" || text == "c") return { true, 0 };
        if (text.front() == 'L' || text.front() == 'l' || text.front() == 'R' || text.front() == 'r') {
            const bool left = text.front() == 'L' || text.front() == 'l';
            const auto amount = parse_value(text.substr(1), "");
            if (!amount.valid || amount.value < 0 || amount.value > 50) return {};
            return { true, (left ? -1.0f : 1.0f) * amount.value / 50.0f };
        }
    }
    if (text.front() == '+') text.remove_prefix(1);
    if (text.empty()) return {};
    double value = 0;
    const auto parsed = std::from_chars(text.data(), text.data() + text.size(), value, std::chars_format::general);
    if (parsed.ec != std::errc{} || parsed.ptr == text.data() || !std::isfinite(value)) return {};
    const auto suffix = detail::trim(std::string_view(parsed.ptr, text.data() + text.size() - parsed.ptr));
    if (suffix.empty()) return detail::checked(value);
    if (unit == "Hz" && suffix == "kHz") return detail::checked(value * 1000);
    if (unit == "kHz" && suffix == "Hz") return detail::checked(value / 1000);
    if (detail::ratio_unit(unit) && detail::ratio_unit(suffix)) return detail::checked(value);
    if (!unit.empty() && suffix == unit) return detail::checked(value);
    return {};
}

// The unit is displayed by ValueEditor separately. Only domain notation remains
// in the formatted number (Unicode minus; L/C/R pan). No locale-dependent commas.
inline std::string format_value(float value, std::string_view unit) {
    if (!std::isfinite(value)) return "—";
    if (unit == "pan") {
        const auto steps = std::lround(std::abs(std::clamp(value, -1.0f, 1.0f)) * 50);
        return steps == 0 ? "C" : (value < 0 ? "L" : "R") + std::to_string(steps);
    }
    const bool fixed_ratio = detail::ratio_unit(unit);
    const bool fixed_db = unit == "dB";
    const int decimals = fixed_db ? 1 : fixed_ratio ? 3 : unit == "%" || unit == "ms" ? 0 : 3;
    char buffer[96];
    const auto result = std::to_chars(buffer, buffer + sizeof(buffer), static_cast<double>(value), std::chars_format::fixed, decimals);
    if (result.ec != std::errc{}) return "—";
    std::string output(buffer, result.ptr);
    if (!fixed_db && !fixed_ratio && decimals > 0) {
        while (output.ends_with('0')) output.pop_back();
        if (output.ends_with('.')) output.pop_back();
    }
    if (output.front() == '-') {
        const bool zero = output.find_first_not_of("-0.") == std::string::npos;
        if (zero) output.erase(0, 1);
        else output.replace(0, 1, "−");
    }
    return output;
}

// Generated Slint types are template arguments so this header is also usable and
// testable without the Slint SDK. Typical call after creating the window:
// bind_value_codec<ParsedValue>(window->global<ValueCodec>());
template <class Parsed, class Codec>
void bind_value_codec(const Codec& codec) {
    codec.on_parse([](const auto& text, const auto& unit) {
        const auto parsed = parse_value(std::string_view(text.data()), std::string_view(unit.data()));
        return Parsed { parsed.valid, parsed.value };
    });
    codec.on_format([](float value, const auto& unit) {
        using SharedText = std::decay_t<decltype(unit)>;
        const auto formatted = format_value(value, std::string_view(unit.data()));
        return SharedText(formatted.c_str());
    });
}
} // namespace dandrum::slint_ui
