#pragma once

#include "InstrumentUiMeterDelivery.h"
#include "InstrumentUiDocument.h"

#include <juce_gui_basics/juce_gui_basics.h>

#include <cstddef>
#include <memory>
#include <vector>

class DandrumAudioProcessor;

class NativeOutputBuses final : public juce::Component
{
public:
    explicit NativeOutputBuses (DandrumAudioProcessor& hostProcessor);
    ~NativeOutputBuses() override;
    void setDocument (const InstrumentUiDocument& document);
    void setPacket (const InstrumentUiMeterDelivery::Packet& packet);
    void clear();
    void paint (juce::Graphics& graphics) override;
    void resized() override;

private:
    struct BusRow;

    DandrumAudioProcessor& processor;
    juce::Label heading, count, unavailable;
    juce::Component content;
    juce::Viewport viewport;
    std::vector<std::unique_ptr<BusRow>> rows;
};
