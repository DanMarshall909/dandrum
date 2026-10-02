#include "NativeMasterMeter.h"
#include "PluginProcessor.h"

#include <juce_gui_basics/juce_gui_basics.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>

NativeMasterMeter::NativeMasterMeter (DandrumAudioProcessor& hostProcessor)
    : processor (hostProcessor)
{
    setComponentID ("master-meter");
    leftClip.setComponentID ("clip-left");
    rightClip.setComponentID ("clip-right");
    leftClip.setButtonText ("CLIP");
    rightClip.setButtonText ("CLIP");
    addAndMakeVisible (leftClip);
    addAndMakeVisible (rightClip);
    leftClip.onClick = [this] { acknowledge (0); };
    rightClip.onClick = [this] { acknowledge (1); };
    updateClipButtons();
}

void NativeMasterMeter::setPacket (const InstrumentUiMeterDelivery::Packet& packet)
{
    display = packet.display;
    clip = packet.clip;
    updateClipButtons();
    repaint();
}

void NativeMasterMeter::clear()
{
    display = {};
    clip = {};
    updateClipButtons();
    repaint();
}

void NativeMasterMeter::paint (juce::Graphics& graphics)
{
    graphics.setColour (juce::Colour (0xff232a27));
    graphics.fillRoundedRectangle (getLocalBounds().toFloat(), 10.0f);
    graphics.setColour (juce::Colour (0xff66796d));
    graphics.drawRoundedRectangle (getLocalBounds().toFloat().reduced (0.5f), 10.0f, 1.0f);
    graphics.setColour (juce::Colour (0xffdce9de));
    graphics.setFont (juce::FontOptions (14.0f).withStyle ("bold"));
    graphics.drawText ("MASTER OUTPUT", 20, 12, getWidth() - 40, 24,
                       juce::Justification::centredLeft);
    graphics.setColour (juce::Colour (0xff9eafa2));
    graphics.setFont (juce::FontOptions (11.0f));
    graphics.drawText (! display.valid ? "WAITING FOR AUDIO"
                       : display.complete ? "LIVE" : "HISTORY GAP",
                       getWidth() - 170, 14, 145, 20, juce::Justification::centredRight);

    for (std::size_t channel = 0; channel < 2; ++channel)
    {
        const auto y = 51 + static_cast<int> (channel) * 53;
        graphics.setColour (juce::Colour (0xffdce9de));
        graphics.setFont (juce::FontOptions (16.0f).withStyle ("bold"));
        graphics.drawText (channel == 0 ? "L" : "R", 24, y - 3, 30, 26,
                           juce::Justification::centredLeft);

        const juce::Rectangle<float> track (64.0f, static_cast<float> (y),
                                             static_cast<float> (getWidth() - 158), 19.0f);
        graphics.setColour (juce::Colour (0xff111916));
        graphics.fillRoundedRectangle (track, 4.0f);
        const auto peak = display.valid ? std::clamp (display.peak[channel], 0.0, 1.0) : 0.0;
        const auto rms = display.valid ? std::clamp (display.rms[channel], 0.0, 1.0) : 0.0;
        graphics.setColour (juce::Colour (0xff3b9576));
        graphics.fillRoundedRectangle (track.withWidth (track.getWidth()
                                                         * static_cast<float> (peak)), 4.0f);
        graphics.setColour (juce::Colour (0xff7ce0aa));
        graphics.fillRoundedRectangle (track.withWidth (track.getWidth()
                                                         * static_cast<float> (rms))
                                             .withHeight (8.0f).translated (0.0f, 5.5f), 3.0f);
    }
}

void NativeMasterMeter::resized()
{
    leftClip.setBounds (getWidth() - 82, 46, 60, 29);
    rightClip.setBounds (getWidth() - 82, 99, 60, 29);
}

void NativeMasterMeter::acknowledge (std::size_t channel)
{
    if (! clip.valid || ! clip.latched[channel])
        return;
    if (processor.acknowledgeMeterClip (channel, clip.generation, clip.ticket[channel]))
    {
        clip.latched[channel] = false;
        display.clipped[channel] = false;
        updateClipButtons();
        repaint();
    }
}

void NativeMasterMeter::updateClipButtons()
{
    const std::array<juce::TextButton*, 2> buttons { &leftClip, &rightClip };
    for (std::size_t channel = 0; channel < buttons.size(); ++channel)
    {
        const bool latched = clip.valid && clip.latched[channel];
        buttons[channel]->setEnabled (latched);
        buttons[channel]->setColour (juce::TextButton::buttonColourId,
                                     juce::Colour (latched ? 0xffd25245 : 0xff414d45));
        buttons[channel]->setColour (juce::TextButton::textColourOffId,
                                     juce::Colour (latched ? 0xffffffff : 0xff9eafa2));
    }
}

namespace
{
class DandrumNativeEditor final : public juce::AudioProcessorEditor,
                                  private juce::Timer
{
public:
    explicit DandrumNativeEditor (DandrumAudioProcessor& hostProcessor)
        : juce::AudioProcessorEditor (&hostProcessor),
          processor (hostProcessor),
          meterSession (hostProcessor.uiCommands().createSession()),
          meter (hostProcessor)
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
        addAndMakeVisible (meter);
        meterGeneration = processor.getParameterSurfaceGeneration();
        processor.subscribeMeter (meterSession, meterGeneration);
        processor.setMeterSessionVisible (meterSession, false);
        setResizable (true, true);
        setResizeLimits (620, 420, 1600, 1100);
        setSize (820, 560);
        startTimerHz (30);
    }

    ~DandrumNativeEditor() override
    {
        stopTimer();
        processor.unsubscribeMeter (meterSession);
        processor.uiCommands().closeSession (meterSession);
    }

    void paint (juce::Graphics& graphics) override
    {
        graphics.fillAll (juce::Colour (0xff171a18));
    }

    void resized() override
    {
        title.setBounds (24, 20, getWidth() - 48, 38);
        summary.setBounds (24, 70, getWidth() - 48, 24);
        meter.setBounds (24, 118, getWidth() - 48, 160);
    }

    void timerCallback() override
    {
        const auto generation = processor.getParameterSurfaceGeneration();
        if (generation != meterGeneration)
        {
            processor.unsubscribeMeter (meterSession);
            meterGeneration = generation;
            processor.subscribeMeter (meterSession, meterGeneration);
            meter.clear();
        }
        processor.setMeterSessionVisible (meterSession, isShowing());
        if (! isShowing())
            return;
        processor.pollMeterDelivery();
        if (const auto packet = processor.takeMeterPacket (meterSession))
        {
            meter.setPacket (*packet);
            processor.acknowledgeMeterPacket (meterSession, packet->meter.generation,
                                              packet->sequence);
        }
    }

private:
    DandrumAudioProcessor& processor;
    std::uint64_t meterSession = 0;
    std::uint32_t meterGeneration = 0;
    juce::Label title;
    juce::Label summary;
    NativeMasterMeter meter;
};
}

juce::AudioProcessorEditor* DandrumAudioProcessor::createEditor()
{
    return new DandrumNativeEditor (*this);
}
