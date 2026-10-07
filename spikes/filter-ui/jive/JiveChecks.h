#pragma once
#include "../shared/FilterProcessor.h"
namespace filter_spike {
// Focused spike smoke evidence; call on JUCE's message thread with an open editor.
bool checkJiveInteractions(FilterProcessor&, juce::AudioProcessorEditor&, juce::String& failure);
}
