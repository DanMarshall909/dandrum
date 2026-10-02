#pragma once

#include "InstrumentUiMeterDelivery.h"

#include <juce_gui_basics/juce_gui_basics.h>

#include <cstddef>

class DandrumAudioProcessor;

class NativeMasterMeter final : public juce::Component
{
public:
    explicit NativeMasterMeter (DandrumAudioProcessor& hostProcessor);
    void setPacket (const InstrumentUiMeterDelivery::Packet& packet);
    void clear();
    void paint (juce::Graphics& graphics) override;
    void resized() override;

private:
    void acknowledge (std::size_t channel);
    void updateClipButtons();

    DandrumAudioProcessor& processor;
    InstrumentUiMeterDisplay::Snapshot display;
    InstrumentUiMeterCapture::ClipSnapshot clip;
    juce::TextButton leftClip;
    juce::TextButton rightClip;
};
