#pragma once
#include "FilterProcessor.h"
namespace filter_spike {
struct SlintCheckGeometry {
    juce::Rectangle<float> graph;
    juce::Point<float> frequencyKnob;
    juce::Point<float> aboutButton, aboutClose;
    bool aboutOpen = false;
    bool knobValueVisible = false;
};
SlintCheckGeometry readSlintCheckGeometry(juce::AudioProcessorEditor&);
bool checkSlintInteractions(FilterProcessor&, juce::AudioProcessorEditor&, juce::String& error);
}
