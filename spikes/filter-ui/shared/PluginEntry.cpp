#include "FilterProcessor.h"
juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter() { return new filter_spike::FilterProcessor(); }
