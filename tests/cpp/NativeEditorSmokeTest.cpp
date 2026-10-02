#include "NativeMasterMeter.h"
#include "PluginProcessor.h"

#include <cstdlib>
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
        auto* meter = dynamic_cast<NativeMasterMeter*> (
            editor->findChildWithID ("master-meter"));
        if (meter == nullptr || ! meter->isVisible() || meter->getWidth() < 300
            || meter->getHeight() < 80
            || meter->findChildWithID ("clip-left") == nullptr
            || meter->findChildWithID ("clip-right") == nullptr)
        {
            std::cerr << "native editor omitted a visible stereo master meter\n";
            return 1;
        }
        InstrumentUiMeterDelivery::Packet packet;
        packet.display.valid = true;
        packet.display.complete = true;
        packet.display.peak = { 0.5, 0.25 };
        packet.display.rms = { 0.4, 0.125 };
        packet.clip.valid = true;
        packet.clip.latched = { false, true };
        packet.clip.ticket = { 0, 1 };
        meter->setPacket (packet);
        auto* leftClip = dynamic_cast<juce::Button*> (meter->findChildWithID ("clip-left"));
        auto* rightClip = dynamic_cast<juce::Button*> (meter->findChildWithID ("clip-right"));
        if (leftClip == nullptr || rightClip == nullptr || leftClip->isEnabled()
            || ! rightClip->isEnabled())
        {
            std::cerr << "native clip buttons did not reflect independent latch state\n";
            return 1;
        }
        const auto image = editor->createComponentSnapshot (editor->getLocalBounds());
        if (image.getPixelAt (200, 171) != juce::Colour (0xff3b9576)
            || image.getPixelAt (200, 179) != juce::Colour (0xff7ce0aa)
            || image.getPixelAt (500, 171) != juce::Colour (0xff111916))
        {
            std::cerr << "native meter bars did not render known peak levels\n";
            return 1;
        }
        if (configuration.instrumentId == "dandrum.tb303-acid")
            if (const auto* output = std::getenv ("DANDRUM_NATIVE_EDITOR_SNAPSHOT"))
            {
                auto stream = juce::File (output).createOutputStream();
                juce::PNGImageFormat png;
                if (stream == nullptr || ! png.writeImageToStream (image, *stream))
                {
                    std::cerr << "native editor snapshot failed\n";
                    return 1;
                }
            }
        meter->clear();
        if (rightClip->isEnabled())
        {
            std::cerr << "native meter clear retained a clip latch\n";
            return 1;
        }
    }
    return 0;
}
