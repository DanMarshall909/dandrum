#include "NativeOutputBuses.h"
#include "PluginProcessor.h"
#include "NativeUiFonts.h"
#include "DesignTokens.h"

#include <algorithm>
#include <utility>

namespace
{
class NativeBusClip final : public juce::TextButton
{
public:
    void paintButton (juce::Graphics& graphics, bool hovered, bool down) override
    {
        namespace tokens = dandrum::ui::tokens;
        const auto bounds = getLocalBounds().toFloat().reduced (0.5f);
        graphics.setColour (juce::Colour (isEnabled() ? tokens::color_action
            : down ? tokens::surface_control_pressed
            : hovered ? tokens::surface_control_hover : tokens::surface_control));
        graphics.fillRoundedRectangle (bounds, tokens::radius_2);
        graphics.setColour (juce::Colour (hasKeyboardFocus (false) ? tokens::color_focus : tokens::border_control));
        graphics.drawRoundedRectangle (bounds, tokens::radius_2, hasKeyboardFocus (false) ? 2.0f : 1.0f);
        graphics.setColour (juce::Colour (isEnabled() ? tokens::text_on_accent : tokens::text_disabled));
        graphics.setFont (dandrum::ui::nativeFont (false, 11.0f));
        graphics.drawText ("CLIP", getLocalBounds(), juce::Justification::centred);
    }
};

class NativeBusChannel final : public juce::Component
{
public:
    NativeBusChannel (const InstrumentUiDocument::OutputBus& bus, std::size_t index, bool bound)
        : channelName (bus.channels[index]), hasBinding (bound)
    {
        setComponentID (juce::String (bus.id) + "-channel:" + juce::String (index));
        if (bound)
        {
            clipButton.setComponentID (index == 0 ? "clip-left" : "clip-right");
            clipButton.setTitle ("Acknowledge " + juce::String (bus.name) + " " + juce::String (channelName) + " clip");
            addAndMakeVisible (clipButton);
        }
    }

    void showReading (const juce::String& identity, bool valid, double peakValue, double rmsValue, bool clipped)
    {
        measured = valid; peak = peakValue; rms = rmsValue;
        setName (identity + (valid ? " · peak " + juce::String (peak, 3) + " · RMS " + juce::String (rms, 3)
                                  : hasBinding ? " · Waiting" : " · Unavailable"));
        clipButton.setEnabled (clipped);
        repaint();
    }

    void paint (juce::Graphics& graphics) override
    {
        namespace tokens = dandrum::ui::tokens;
        graphics.setFont (dandrum::ui::nativeFont (true, 11.0f));
        graphics.setColour (juce::Colour (tokens::text_secondary));
        graphics.drawText (channelName, 0, 0, 24, 24, juce::Justification::centredLeft);
        const juce::Rectangle<float> track (29.0f, 10.0f, 120.0f, 4.0f);
        if (measured)
        {
            graphics.setColour (juce::Colour (tokens::surface_well));
            graphics.fillRoundedRectangle (track, tokens::radius_1);
            graphics.setColour (juce::Colour (tokens::dd_vermilion_lo));
            graphics.fillRect (track.withWidth (120.0f * static_cast<float> (std::clamp (peak, 0.0, 1.0))));
            graphics.setColour (juce::Colour (tokens::dd_vermilion_hi));
            graphics.fillRect (track.withWidth (120.0f * static_cast<float> (std::clamp (rms, 0.0, 1.0)))
                                    .withHeight (2.0f).translated (0.0f, 1.0f));
        }
        else
        {
            graphics.setColour (juce::Colour (tokens::text_tertiary));
            graphics.drawText (hasBinding ? "Waiting" : "Unavailable", 29, 0, 120, 24,
                               juce::Justification::centredLeft);
        }
    }

    void resized() override { clipButton.setBounds (154, 0, 28, 24); }
    NativeBusClip clipButton;
private:
    juce::String channelName;
    bool hasBinding = false, measured = false;
    double peak = 0.0, rms = 0.0;
};
}

struct NativeOutputBuses::BusRow final : juce::Component
{
    BusRow (DandrumAudioProcessor& owner, juce::Viewport& scrollView,
            InstrumentUiDocument::OutputBus prepared, std::uint32_t revision)
        : processor (owner), viewport (scrollView), bus (std::move (prepared)), generation (revision),
          bound (bus.meterBusId == "master" && bus.channels.size() == 2)
    {
        namespace tokens = dandrum::ui::tokens;
        setComponentID (bus.id); setName (juce::String (bus.name) + " output bus");
        setWantsKeyboardFocus (true);
        for (auto* label : { &name, &summary, &feeds, &status })
        {
            label->setFont (dandrum::ui::nativeFont (label == &summary, 11.0f));
            label->setColour (juce::Label::textColourId, juce::Colour (
                label == &name ? tokens::text_primary : label == &summary ? tokens::text_secondary : tokens::text_tertiary));
            label->setMinimumHorizontalScale (1.0f);
            label->setBorderSize (juce::BorderSize<int> (0));
            addAndMakeVisible (*label);
        }
        name.setComponentID (juce::String (bus.id) + "-name");
        main.setComponentID (juce::String (bus.id) + "-main");
        main.setText ("Main", juce::dontSendNotification);
        main.setFont (dandrum::ui::nativeFont (false, 11.0f));
        main.setColour (juce::Label::textColourId, juce::Colour (tokens::text_secondary));
        main.setColour (juce::Label::backgroundColourId, juce::Colour (tokens::surface_well));
        main.setJustificationType (juce::Justification::centred);
        addChildComponent (main); main.setVisible (bus.main);
        summary.setComponentID (juce::String (bus.id) + "-channels");
        feeds.setComponentID (juce::String (bus.id) + "-feeds");
        status.setComponentID (juce::String (bus.id) + "-status");
        name.setText (bus.name, juce::dontSendNotification);
        name.setTooltip (bus.main ? "Main output bus" : "Output bus");
        juce::StringArray labels;
        for (const auto& channel : bus.channels) labels.add (channel);
        summary.setText (labels.isEmpty() ? "Disabled" : juce::String (labels.size())
            + (labels.size() == 1 ? " channel · " : " channels · ") + labels.joinIntoString ("/"), juce::dontSendNotification);
        summary.setTooltip (summary.getText());
        feeds.setText ("Feed details unavailable", juce::dontSendNotification);
        for (std::size_t index = 0; index < bus.channels.size(); ++index)
        {
            auto channel = std::make_unique<NativeBusChannel> (bus, index, bound);
            channel->clipButton.onClick = [this, index] { acknowledge (index); };
            addAndMakeVisible (*channel); channels.push_back (std::move (channel));
        }
        clear();
    }

    int heightForWidth (int width) const
    {
        return (width < 800 ? 88 : 40) + 24 * static_cast<int> (channels.size());
    }
    void resized() override
    {
        const int width = getWidth() - 20;
        const bool compact = getWidth() < 800;
        const int measurementsX = compact ? 10 : getWidth() - 210;
        const int statusY = compact ? 58 : 10;
        const int nameOffset = bus.main ? 44 : 0;
        main.setBounds (10, compact ? 13 : (getHeight() - 18) / 2, 38, 18);
        if (compact)
        {
            name.setBounds (10 + nameOffset, 10, width / 2 - 6 - nameOffset, 24);
            summary.setBounds (16 + width / 2, 10, width / 2 - 6, 30);
            feeds.setBounds (10, 38, width, 18);
        }
        else
        {
            const int nameWidth = getWidth() - 630;
            name.setBounds (10 + nameOffset, 10, nameWidth - nameOffset, getHeight() - 20);
            summary.setBounds (22 + nameWidth, 10, 220, getHeight() - 20);
            feeds.setBounds (254 + nameWidth, 10, 152, getHeight() - 20);
        }
        status.setBounds (measurementsX, statusY, compact ? width : 200, 18);
        for (std::size_t index = 0; index < channels.size(); ++index)
            channels[index]->setBounds (measurementsX, statusY + 20 + 24 * static_cast<int> (index), 182, 24);
    }
    void paint (juce::Graphics& graphics) override
    {
        namespace tokens = dandrum::ui::tokens;
        const auto bounds = getLocalBounds().toFloat().reduced (4.0f);
        graphics.setColour (juce::Colour (tokens::surface_control));
        graphics.fillRoundedRectangle (bounds, tokens::radius_2);
        graphics.setColour (juce::Colour (tokens::border_control));
        graphics.drawRoundedRectangle (bounds.reduced (0.5f), tokens::radius_2, 1.0f);
        if (hasKeyboardFocus (true))
        {
            graphics.setColour (juce::Colour (tokens::color_focus));
            graphics.drawRoundedRectangle (getLocalBounds().toFloat().reduced (1.0f), tokens::radius_3, 2.0f);
        }
    }
    void focusGained (FocusChangeType) override
    {
        const int top = viewport.getViewPositionY();
        if (getY() < top || getBottom() > top + viewport.getViewHeight())
            viewport.setViewPosition (0, getY());
        repaint();
    }
    void focusLost (FocusChangeType) override { repaint(); }
    void clear()
    {
        display = {}; clip = {};
        status.setText (bound ? "WAITING FOR AUDIO" : "Measurements unavailable", juce::dontSendNotification);
        updateChannels();
    }
    void setPacket (const InstrumentUiMeterDelivery::Packet& packet)
    {
        if (! bound || ! packet.display.valid || packet.display.generation != generation)
            return;
        display = packet.display; clip = packet.clip;
        status.setText (display.complete ? "LIVE" : "HISTORY GAP", juce::dontSendNotification);
        updateChannels();
    }
    void acknowledge (std::size_t channel)
    {
        if (! clip.valid || clip.generation != generation || ! clip.latched[channel] || clip.ticket[channel] == 0)
            return;
        if (processor.acknowledgeMeterClip (channel, generation, clip.ticket[channel]))
        {
            clip.latched[channel] = false;
            display.clipped[channel] = false;
            updateChannels();
        }
    }
    void updateChannels()
    {
        for (std::size_t index = 0; index < channels.size(); ++index)
        {
            const auto measured = bound && display.valid;
            channels[index]->showReading (juce::String (bus.name) + " " + juce::String (bus.channels[index])
                + " · " + juce::String (bus.meterBusId) + " · generation " + juce::String (generation),
                measured, measured ? display.peak[index] : 0.0, measured ? display.rms[index] : 0.0,
                measured && clip.valid && clip.generation == generation && clip.latched[index] && clip.ticket[index] != 0);
        }
    }
    DandrumAudioProcessor& processor;
    juce::Viewport& viewport;
    const InstrumentUiDocument::OutputBus bus;
    const std::uint32_t generation;
    const bool bound;
    juce::Label name, summary, feeds, status, main;
    std::vector<std::unique_ptr<NativeBusChannel>> channels;
    InstrumentUiMeterDisplay::Snapshot display;
    InstrumentUiMeterCapture::ClipSnapshot clip;
};

NativeOutputBuses::NativeOutputBuses (DandrumAudioProcessor& hostProcessor) : processor (hostProcessor)
{
    namespace tokens = dandrum::ui::tokens;
    setComponentID ("output-buses"); setName ("Output buses");
    heading.setText ("Output buses", juce::dontSendNotification);
    heading.setFont (dandrum::ui::nativeFont (false, 12.0f));
    count.setFont (dandrum::ui::nativeFont (true, 11.0f));
    unavailable.setFont (dandrum::ui::nativeFont (false, 11.0f));
    for (auto* label : { &heading, &count, &unavailable })
    {
        label->setColour (juce::Label::textColourId, juce::Colour (tokens::text_secondary));
        addAndMakeVisible (*label);
    }
    unavailable.setComponentID ("output-bindings-unavailable");
    unavailable.setText ("Output bindings unavailable", juce::dontSendNotification);
    viewport.setViewedComponent (&content, false); viewport.setScrollBarsShown (true, false);
    addAndMakeVisible (viewport);
    if (const auto document = processor.getPreparedUiDocument()) setDocument (*document);
}
NativeOutputBuses::~NativeOutputBuses() = default;

void NativeOutputBuses::setDocument (const InstrumentUiDocument& document)
{
    rows.clear();
    for (const auto& bus : document.outputBuses)
    {
        auto row = std::make_unique<BusRow> (processor, viewport, bus, document.generation);
        content.addAndMakeVisible (*row); rows.push_back (std::move (row));
    }
    count.setText (juce::String (rows.size()) + (rows.size() == 1 ? " bus" : " buses"), juce::dontSendNotification);
    unavailable.setVisible (rows.empty()); viewport.setVisible (! rows.empty());
    resized();
}
void NativeOutputBuses::setPacket (const InstrumentUiMeterDelivery::Packet& packet)
{
    for (auto& row : rows) row->setPacket (packet);
}
void NativeOutputBuses::clear()
{
    for (auto& row : rows) row->clear();
}
void NativeOutputBuses::paint (juce::Graphics& graphics)
{
    namespace tokens = dandrum::ui::tokens;
    graphics.setColour (juce::Colour (tokens::surface_panel));
    graphics.fillRoundedRectangle (getLocalBounds().toFloat(), tokens::radius_3);
    graphics.setColour (juce::Colour (tokens::surface_panel_header));
    graphics.fillRect (0, 0, getWidth(), getWidth() < 800 ? 24 : 28);
}
void NativeOutputBuses::resized()
{
    const bool compact = getWidth() < 800;
    const int padding = compact ? 8 : 12, header = compact ? 24 : 28;
    heading.setBounds (padding, 0, getWidth() - padding * 2 - 80, header);
    count.setBounds (getWidth() - padding - 80, 0, 80, header);
    unavailable.setBounds (padding, header + padding, getWidth() - padding * 2, 24);
    viewport.setBounds (padding, header + 4, getWidth() - padding * 2, getHeight() - header - padding);
    const int width = viewport.getWidth() - viewport.getScrollBarThickness();
    int y = 0;
    for (auto& row : rows)
    {
        const int height = row->heightForWidth (width);
        row->setBounds (0, y, width, height); y += height + 4;
    }
    content.setSize (width, y);
}
