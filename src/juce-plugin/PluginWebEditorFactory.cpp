#include "PluginEditor.h"

juce::AudioProcessorEditor* DandrumAudioProcessor::createEditor()
{
    return new DandrumAudioProcessorEditor (*this);
}
