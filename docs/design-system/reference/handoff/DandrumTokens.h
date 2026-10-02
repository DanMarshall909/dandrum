#pragma once
// Dandrum design tokens for JUCE. Generated from tokens/*.css — keep the two in sync.
// All sizes are logical pixels at 100% scale; JUCE applies display + plugin zoom.
#include <juce_graphics/juce_graphics.h>

namespace dd
{
namespace colour
{
    // Graphite ramp
    inline const juce::Colour ink0  { 0xFF130F0C }; // wells, header/status bars
    inline const juce::Colour ink1  { 0xFF1A1511 }; // editor background
    inline const juce::Colour ink2  { 0xFF211B16 }; // panel surface
    inline const juce::Colour ink3  { 0xFF2A231D }; // panel header, menu
    inline const juce::Colour ink4  { 0xFF342B23 }; // control face, pad
    inline const juce::Colour ink5  { 0xFF41362C }; // knob cap, hover
    inline const juce::Colour ink6  { 0xFF524437 }; // pressed, knob track
    inline const juce::Colour line1 { 0xFF2F271F };
    inline const juce::Colour line2 { 0xFF42372C };
    inline const juce::Colour line3 { 0xFF5F4F40 };
    // Text
    inline const juce::Colour paper1 { 0xFFF2E6D3 }; // primary text, value arc, pointer
    inline const juce::Colour paper2 { 0xFFCBB9A0 }; // labels, prepared values
    inline const juce::Colour paper3 { 0xFFA8957D }; // tertiary, units
    inline const juce::Colour paper4 { 0xFF706252 }; // disabled
    inline const juce::Colour onAccent { 0xFF1E1209 };
    inline const juce::Colour capHover { 0xFF4A3D31 };
    inline const juce::Colour padHover { 0xFF3A3027 };
    // Accent: ember, faded orange (selection + primary action)
    inline const juce::Colour vermilion     { 0xFFE08A4E };
    inline const juce::Colour vermilionHi   { 0xFFEAA170 };
    inline const juce::Colour vermilionLo   { 0xFFB0662F };
    inline const juce::Colour vermilionWash { 0xFF3A2618 };
    // Modulation slots (pair with glyph: A circle, B triangle, C square, D diamond)
    inline const juce::Colour modA { 0xFF3FD0C9 };
    inline const juce::Colour modB { 0xFFA98BFF };
    inline const juce::Colour modC { 0xFF8EDB5A };
    inline const juce::Colour modD { 0xFFFF85BE };
    inline const juce::Colour modWash { 0xFF1F2E29 };
    // Host + status
    inline const juce::Colour host  { 0xFF5B9DFF };
    inline const juce::Colour ok    { 0xFF5FD38A };
    inline const juce::Colour warn  { 0xFFE9C15A };
    inline const juce::Colour error { 0xFFF0545E };
    inline const juce::Colour hostWash  { 0xFF1D2330 };
    inline const juce::Colour okWash    { 0xFF1B2719 };
    inline const juce::Colour warnWash  { 0xFF2F2814 };
    inline const juce::Colour errorWash { 0xFF34181A };
    inline const juce::Colour waveform       { 0xFF9E8F7D }; // outside region (draw at 28% alpha)
    inline const juce::Colour waveformRegion { 0xFFE6D6BE };
}

namespace type
{
    // Fonts are embedded via BinaryData (SIL OFL): BarlowSemiCondensed-{Medium,SemiBold,Bold}.ttf,
    // Barlow-Bold.ttf, JetBrainsMono-{Medium,SemiBold}.ttf
    inline constexpr float micro   = 11.0f; // minimum size anywhere
    inline constexpr float label   = 12.0f; // caps, +0.06em tracking
    inline constexpr float body    = 13.0f;
    inline constexpr float value   = 13.0f; // mono
    inline constexpr float valueLg = 15.0f; // mono, under 64px knobs
    inline constexpr float heading = 13.0f; // caps, bold, +0.10em
    inline constexpr float title   = 18.0f;
    inline constexpr float brand   = 20.0f;
    inline constexpr float trackingCaps    = 0.06f; // Font::setExtraKerningFactor
    inline constexpr float trackingHeading = 0.10f;
}

namespace space
{
    inline constexpr int s1 = 2, s2 = 4, s3 = 6, s4 = 8, s5 = 12, s6 = 16, s7 = 24, s8 = 32;
    inline constexpr int panelSeam = 4, panelPad = 12, panelPadCompact = 8, controlGap = 8, groupGap = 16;
}

namespace radius { inline constexpr float field = 2.0f, control = 4.0f, panel = 6.0f; }

namespace stroke
{
    inline constexpr float hairline = 1.0f, control = 1.5f, focus = 2.0f, focusGap = 2.0f;
    inline constexpr float track = 3.0f, trackLg = 4.0f, trackSm = 2.5f;
    inline constexpr float mod = 2.0f, modSm = 1.5f;
}

namespace size
{
    inline constexpr int knobLg = 64, knobMd = 48, knobSm = 36, knobMin = 28;
    inline constexpr int buttonH = 28, buttonHSm = 24, iconButton = 28, iconButtonSm = 24, icon = 16, iconSm = 14;
    inline constexpr int fieldH = 24, fieldW = 72, tabH = 32, tabHSm = 26, segmentH = 24, rowH = 26, rowHSm = 22;
    inline constexpr int menuItemH = 26, menuW = 248, pad = 76, padSm = 52, padMin = 44, padGap = 6;
    inline constexpr int sliderThick = 24, sliderLen = 160, sliderThumb = 12, meterW = 6, meterH = 96;
    inline constexpr int headerH = 44, statusH = 26, panelHeaderH = 28, panelHeaderHSm = 24, hitMin = 24;
}

namespace motion
{
    inline constexpr int hostDecayMs = 400;    // host-automation tint fade after last host change
    inline constexpr int tooltipDelayMs = 500;
    inline constexpr int chokeFlashMs = 180;
    inline constexpr int repaintHz = 30;       // timer for meters, pad levels, cursor, live mod dots
}

namespace shadow
{
    // juce::DropShadow { colour, radius, offset }
    inline const juce::DropShadow cap   { juce::Colours::black.withAlpha (0.55f), 3, { 0, 2 } };
    inline const juce::DropShadow float_{ juce::Colours::black.withAlpha (0.50f), 20, { 0, 8 } };
}
} // namespace dd
