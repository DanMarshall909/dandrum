#include "PluginProcessor.h"

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
#if defined(DANDRUM_DEFAULT_SAMPLER_PLUGIN)
    return new DandrumAudioProcessor (InstrumentDemoConfiguration::sampler());
#else
    return new DandrumAudioProcessor();
#endif
}
