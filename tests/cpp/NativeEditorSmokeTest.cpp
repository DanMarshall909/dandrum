#include "PluginProcessor.h"

#include <iostream>
#include <memory>

int main()
{
    for (const auto& configuration : {
             InstrumentDemoConfiguration::tb303(),
             InstrumentDemoConfiguration::sampler() })
    {
        DandrumAudioProcessor processor (configuration);
        if (! processor.isInstrumentLoaded() || ! processor.hasEditor())
        {
            std::cerr << "native plugin failed to load its configured instrument\n";
            return 1;
        }

        std::unique_ptr<juce::AudioProcessorEditor> editor (processor.createEditor());
        if (editor == nullptr
            || editor->getName() != juce::String (configuration.title)
            || editor->getWidth() != 820 || editor->getHeight() != 560)
        {
            std::cerr << "native editor did not open the configured instrument\n";
            return 1;
        }
    }
    return 0;
}
