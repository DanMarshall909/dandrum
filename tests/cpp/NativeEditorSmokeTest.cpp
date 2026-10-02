#include "NativeMasterMeter.h"
#include "PluginProcessor.h"

#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <iostream>
#include <memory>

namespace
{
struct HostGestureListener final : juce::AudioProcessorListener
{
    void audioProcessorParameterChanged (juce::AudioProcessor*, int, float) override {}
    void audioProcessorChanged (juce::AudioProcessor*, const ChangeDetails&) override {}
    void audioProcessorParameterChangeGestureBegin (juce::AudioProcessor*, int slot) override
    {
        ++begins;
        lastSlot = slot;
    }
    void audioProcessorParameterChangeGestureEnd (juce::AudioProcessor*, int slot) override
    {
        ++ends;
        lastSlot = slot;
    }
    int begins = 0;
    int ends = 0;
    int lastSlot = -1;
};
}

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
        HostGestureListener listener;
        processor.addListener (&listener);

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
        auto* knob = dynamic_cast<juce::Slider*> (
            editor->findChildWithID ("primary-knob-slider"));
        auto* knobLabel = dynamic_cast<juce::Label*> (
            editor->findChildWithID ("primary-knob-label"));
        const auto state = processor.getUiParameterState();
        if (state.parameters.empty())
        {
            std::cerr << "native editor had no prepared public control\n";
            return 1;
        }
        const auto preferred = std::find_if (
            state.parameters.begin(), state.parameters.end(), [] (const auto& value)
            { return value.id == "amp.release_ms"; });
        const auto& selected = preferred != state.parameters.end()
            ? *preferred : state.parameters.front();
        const auto expectedLabel = selected.id == "amp.release_ms"
            ? juce::String ("RELEASE") : juce::String (selected.name);
        if (knob == nullptr || knobLabel == nullptr
            || ! knob->isEnabled() || ! knob->onDragStart || ! knob->onDragEnd
            || knobLabel->getText() != expectedLabel
            || std::abs (knob->getValue() - selected.normalisedValue) > 0.00001)
        {
            std::cerr << "native editor did not bind an authoritative public knob\n";
            return 1;
        }
        const auto publicId = juce::String (selected.id);
        auto* hostParameter = processor.getParameterForPublicId (publicId);
        if (hostParameter == nullptr)
            return 1;
        const auto hostSlot = processor.getParameters().indexOf (hostParameter);
        knob->onDragStart();
        knob->setValue (0.25, juce::sendNotificationSync);
        knob->setValue (0.75, juce::sendNotificationSync);
        knob->onDragEnd();
        if (listener.begins != 1 || listener.ends != 1 || listener.lastSlot != hostSlot
            || std::abs (hostParameter->getValue() - 0.75f) > 0.00001f)
        {
            std::cerr << "native knob drag did not use one gesture and the public host slot\n";
            return 1;
        }
        knob->setValue (0.33, juce::sendNotificationSync);
        if (listener.begins != 2 || listener.ends != 2 || listener.lastSlot != hostSlot
            || std::abs (hostParameter->getValue() - 0.33f) > 0.00001f)
        {
            std::cerr << "native knob typed value did not complete one host gesture\n";
            return 1;
        }
        hostParameter->setValueNotifyingHost (0.4f);
        const auto deadline = juce::Time::getMillisecondCounterHiRes() + 1500.0;
        while (std::abs (knob->getValue() - 0.4) > 0.00001
               && juce::Time::getMillisecondCounterHiRes() < deadline)
        {
            juce::Thread::sleep (20);
            juce::Timer::callPendingTimersSynchronously();
        }
        if (std::abs (knob->getValue() - 0.4) > 0.00001)
        {
            std::cerr << "native knob did not observe authoritative host automation: knob="
                      << knob->getValue() << " host=" << hostParameter->getValue()
                      << " snapshot=" << hostParameter->getValue()
                      << "\n";
            return 1;
        }
        knob->onDragStart();
        editor.reset();
        if (listener.begins != 3 || listener.ends != 3)
        {
            std::cerr << "closing the native editor left a host gesture open\n";
            return 1;
        }
        processor.removeListener (&listener);
    }

    auto fixtureConfiguration = InstrumentDemoConfiguration::kick();
    fixtureConfiguration.instrumentPath = juce::File (juce::String (DANDRUM_SOURCE_ROOT))
        .getChildFile ("tests/fixtures/plugin-ui-knob.yaml").getFullPathName().toStdString();
    DandrumAudioProcessor fixtureProcessor (fixtureConfiguration);
    fixtureProcessor.setPlayConfigDetails (0, 2, 48000.0, 64);
    fixtureProcessor.prepareToPlay (48000.0, 64);
    if (! fixtureProcessor.isInstrumentLoaded())
    {
        std::cerr << "signed UI knob fixture did not prepare: "
                  << fixtureProcessor.getLastLoadError() << '\n';
        return 1;
    }
    HostGestureListener fixtureListener;
    fixtureProcessor.addListener (&fixtureListener);
    std::unique_ptr<juce::AudioProcessorEditor> fixtureEditor (fixtureProcessor.createEditor());
    auto* fixtureKnob = dynamic_cast<juce::Slider*> (
        fixtureEditor->findChildWithID ("primary-knob-slider"));
    if (fixtureKnob == nullptr || ! fixtureKnob->isEnabled())
        return 1;
    juce::AudioBuffer<float> output (2, 64);
    juce::MidiBuffer midi;
    const auto renders = [&] (float expectedLeft, float expectedRight)
    {
        output.clear();
        fixtureProcessor.processBlock (output, midi);
        for (int channel = 0; channel < 2; ++channel)
            for (int frame = 0; frame < output.getNumSamples(); ++frame)
                if (std::abs (output.getSample (channel, frame)
                               - (channel == 0 ? expectedLeft : expectedRight)) > 0.00001f)
                    return false;
        return true;
    };
    fixtureKnob->onDragStart();
    fixtureKnob->setValue (0.25, juce::sendNotificationSync);
    if (! renders (-0.5f, 0.0f))
    {
        std::cerr << "native knob did not render signed left -0.5/right 0 output: "
                  << output.getSample (0, 0) << ", " << output.getSample (1, 0) << '\n';
        return 1;
    }
    fixtureKnob->setValue (0.75, juce::sendNotificationSync);
    if (! renders (0.5f, 0.0f))
    {
        std::cerr << "native knob did not render signed left +0.5/right 0 output: "
                  << output.getSample (0, 0) << ", " << output.getSample (1, 0) << '\n';
        return 1;
    }
    fixtureKnob->onDragEnd();
    if (fixtureListener.begins != 1 || fixtureListener.ends != 1)
    {
        std::cerr << "signed native knob schedule did not preserve one host gesture\n";
        return 1;
    }
    fixtureEditor.reset();
    fixtureProcessor.removeListener (&fixtureListener);
    return 0;
}
