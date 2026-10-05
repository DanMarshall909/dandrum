#pragma once

#include "NativeKnobFontBinaryData.h"
#include <juce_gui_basics/juce_gui_basics.h>

namespace dandrum::ui
{
inline juce::Font nativeFont (bool value, float height)
{
    static const auto uiFace = juce::Typeface::createSystemTypefaceFor (
        NativeKnobFontBinaryData::BarlowSemiCondensedSemiBold_ttf,
        NativeKnobFontBinaryData::BarlowSemiCondensedSemiBold_ttfSize);
    static const auto valueFace = juce::Typeface::createSystemTypefaceFor (
        NativeKnobFontBinaryData::JetBrainsMonoMedium_ttf,
        NativeKnobFontBinaryData::JetBrainsMonoMedium_ttfSize);
    return juce::FontOptions (value ? valueFace : uiFace).withHeight (height);
}
}
