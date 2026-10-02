#include "PluginProcessor.h"

#include <juce_gui_basics/juce_gui_basics.h>

namespace
{
class DandrumNativeEditor final : public juce::AudioProcessorEditor
{
public:
    explicit DandrumNativeEditor (DandrumAudioProcessor& hostProcessor)
        : juce::AudioProcessorEditor (&hostProcessor)
    {
        setName (juce::String (hostProcessor.demoConfiguration().title));
        title.setText (juce::String (hostProcessor.demoConfiguration().title),
                       juce::dontSendNotification);
        title.setJustificationType (juce::Justification::centredLeft);
        title.setFont (juce::FontOptions (24.0f));
        title.setColour (juce::Label::textColourId, juce::Colours::white);
        addAndMakeVisible (title);

        summary.setText (juce::String (hostProcessor.getActivePublicParameterIds().size())
                             + " public controls", juce::dontSendNotification);
        summary.setJustificationType (juce::Justification::centredLeft);
        summary.setColour (juce::Label::textColourId, juce::Colour (0xffaab5ad));
        addAndMakeVisible (summary);
        setSize (820, 560);
    }

    void paint (juce::Graphics& graphics) override
    {
        graphics.fillAll (juce::Colour (0xff171a18));
    }

    void resized() override
    {
        title.setBounds (24, 20, getWidth() - 48, 38);
        summary.setBounds (24, 70, getWidth() - 48, 24);
    }

private:
    juce::Label title;
    juce::Label summary;
};
}

juce::AudioProcessorEditor* DandrumAudioProcessor::createEditor()
{
    return new DandrumNativeEditor (*this);
}
