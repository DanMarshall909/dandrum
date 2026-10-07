#pragma once
#include <juce_gui_basics/juce_gui_basics.h>
#include "../shared/FilterViewModel.h"

namespace filter_spike::jive_ui {
inline juce::Colour background() { return juce::Colour(0xff11151c); }
inline juce::Colour panel() { return juce::Colour(0xff19202b); }
inline juce::Colour muted() { return juce::Colour(0xff8b9bb1); }
inline juce::Colour bandColour(int band) {
    constexpr std::array<juce::uint32, 3> colours { 0xff6fe0ce, 0xffffbc70, 0xffab9fff };
    return juce::Colour(colours[static_cast<size_t>(band)]);
}
inline void label(juce::Graphics& g, const juce::String& text, juce::Rectangle<int> bounds,
                  juce::Justification alignment = juce::Justification::centredLeft) {
    g.setFont(juce::FontOptions(12.0f));
    g.setColour(muted());
    g.drawText(text, bounds, alignment);
}
using Model = std::shared_ptr<FilterViewModel>;
}
