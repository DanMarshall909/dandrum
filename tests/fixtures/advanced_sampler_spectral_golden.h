#pragma once
#include <array>
#include <cstdint>
#include <string_view>
// Independent oracle generated from the unchanged imported Spectrogram JavaScript.
// Source SHA-256: 03c64a7d103c629ce50cac281350d0b79d10382bde85233b125e7bb349b5b3b7
namespace dandrum::spectral_test {
struct Pixel { unsigned x, y; std::uint32_t rgba; };
struct Case { std::string_view profile; bool reversed; double start, end; std::uint32_t hash; std::array<Pixel, 4> pixels; };
inline constexpr std::array<std::uint32_t, 5> palette = { 0x130f0c, 0x41362c, 0xb0662f, 0xe08a4e, 0xf2e6d3 };
inline constexpr std::array<Case, 12> cases = {{
    {"kick", false, 0, 1, 140650304u, {{{0, 0, 0x5a412dff}, {0, 63, 0xf2e6d3ff}, {1, 17, 0x130f0cff}, {3, 42, 0x2c241eff}}}},
    {"kick", true, 0.25, 0.5, 3635752480u, {{{0, 0, 0x130f0cff}, {0, 63, 0x65462dff}, {1, 17, 0x130f0cff}, {3, 42, 0x5d422dff}}}},
    {"snare", false, 0, 1, 229039119u, {{{0, 0, 0xbc6f37ff}, {0, 63, 0xedcdafff}, {1, 17, 0x6b482dff}, {3, 42, 0x191410ff}}}},
    {"snare", true, 0.25, 0.5, 4063426338u, {{{0, 0, 0x130f0cff}, {0, 63, 0x352c24ff}, {1, 17, 0x130f0cff}, {3, 42, 0x6e4a2dff}}}},
    {"snare-soft", false, 0, 1, 1179563402u, {{{0, 0, 0x774e2dff}, {0, 63, 0xe9b891ff}, {1, 17, 0x2d251eff}, {3, 42, 0x130f0cff}}}},
    {"snare-soft", true, 0.25, 0.5, 3415016426u, {{{0, 0, 0x130f0cff}, {0, 63, 0x322a22ff}, {1, 17, 0x130f0cff}, {3, 42, 0x59412dff}}}},
    {"hat-closed", false, 0, 1, 96656027u, {{{0, 0, 0xefd9c0ff}, {0, 63, 0x8c562eff}, {1, 17, 0xe39963ff}, {3, 42, 0x9a5d2eff}}}},
    {"hat-closed", true, 0.25, 0.5, 168709712u, {{{0, 0, 0x5c422dff}, {0, 63, 0x1f1914ff}, {1, 17, 0x130f0cff}, {3, 42, 0x63452dff}}}},
    {"hat-open", false, 0, 1, 1026927423u, {{{0, 0, 0xefd6bcff}, {0, 63, 0x9c5d2eff}, {1, 17, 0xe39a65ff}, {3, 42, 0xa9632fff}}}},
    {"hat-open", true, 0.25, 0.5, 1373041431u, {{{0, 0, 0x5a412dff}, {0, 63, 0x221c16ff}, {1, 17, 0x130f0cff}, {3, 42, 0x69472dff}}}},
    {"break", false, 0, 1, 3702634868u, {{{0, 0, 0x130f0cff}, {0, 63, 0xf2e6d3ff}, {1, 17, 0x130f0cff}, {3, 42, 0x82522eff}}}},
    {"break", true, 0.25, 0.5, 2137206501u, {{{0, 0, 0x130f0cff}, {0, 63, 0x65462dff}, {1, 17, 0x130f0cff}, {3, 42, 0x58402dff}}}},
}};
} // namespace dandrum::spectral_test
