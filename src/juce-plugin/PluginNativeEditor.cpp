#include "NativeMasterMeter.h"
#include "PluginProcessor.h"
#include "InstrumentUiWaveformGeometry.h"

#include <juce_gui_basics/juce_gui_basics.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>

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
class NativePreparedWaveform final : public juce::Component
{
public:
    NativePreparedWaveform()
    {
        setComponentID ("prepared-waveform");
        clear();
    }

    void clear()
    {
        source.reset();
        region.reset();
        geometry.reset();
        result.reset();
        setName ("NO PREPARED SAMPLE");
        repaint();
    }

    void setPreparedRegion (const InstrumentUiDocument::Source& preparedSource,
                            const InstrumentUiDocument::Region& preparedRegion)
    {
        // Copies survive a processor reload; the view never borrows engine metadata.
        source = preparedSource;
        region = preparedRegion;
        result.reset();
        updateGeometry();
        setName ("PREPARING WAVEFORM");
        repaint();
    }

    void setResult (std::shared_ptr<const InstrumentUiWaveformService::Result> ready)
    {
        if (! source || ! region || ! ready || ready->sourceId != source->id
            || ready->regionId != region->id || ready->sampleRateHz != source->sampleRateHz
            || ready->startFrame != region->startFrame || ready->endFrame != region->endFrame)
            return;
        result = std::move (ready);
        setName ("Prepared waveform: " + juce::String (source->id) + "."
                 + juce::String (region->id));
        repaint();
    }

    void setUnavailable()
    {
        result.reset();
        setName ("WAVEFORM UNAVAILABLE");
        repaint();
    }

    void resized() override { updateGeometry(); }

    void paint (juce::Graphics& graphics) override
    {
        graphics.setColour (juce::Colour (0xff232a27));
        graphics.fillRoundedRectangle (getLocalBounds().toFloat(), 10.0f);
        graphics.setColour (juce::Colour (0xff66796d));
        graphics.drawRoundedRectangle (getLocalBounds().toFloat().reduced (0.5f), 10.0f, 1.0f);
        graphics.setColour (juce::Colour (0xffdce9de));
        graphics.setFont (juce::FontOptions (14.0f).withStyle ("bold"));
        graphics.drawText ("PREPARED SAMPLE", 16, 12, getWidth() - 32, 22,
                           juce::Justification::centredLeft);
        graphics.setColour (juce::Colour (0xff9eafa2));
        graphics.setFont (juce::FontOptions (11.0f));
        graphics.drawText (getName(), 16, 32, getWidth() - 32, 18,
                           juce::Justification::centredLeft);

        const auto plot = plotBounds();
        graphics.setColour (juce::Colour (0xff111916));
        graphics.fillRect (plot);
        if (! geometry)
            return;

        graphics.setColour (juce::Colour (0xff414d45));
        graphics.fillRect (plot.getX(), plot.getCentreY(), plot.getWidth(), 1);
        if (result)
        {
            graphics.setColour (juce::Colour (0xff7ce0aa));
            for (const auto& bucket : result->buckets)
                if (const auto x = geometry->bucketX (bucket.startFrame, bucket.endFrame))
                {
                    const auto column = plot.getX() + static_cast<int> (*x);
                    const auto high = plot.getY() + static_cast<int> (geometry->sampleY (bucket.maximum));
                    const auto low = plot.getY() + static_cast<int> (geometry->sampleY (bucket.minimum));
                    graphics.fillRect (column, high, 1, std::max (2, low - high + 1));
                }
        }
        for (const auto& marker : geometry->markers())
        {
            const auto color = marker.kind == InstrumentUiWaveformGeometry::MarkerKind::loopStart
                                   || marker.kind == InstrumentUiWaveformGeometry::MarkerKind::loopEnd
                ? 0xffe2bf72 : marker.kind == InstrumentUiWaveformGeometry::MarkerKind::sliceStart
                                   || marker.kind == InstrumentUiWaveformGeometry::MarkerKind::sliceEnd
                ? 0xffab9ee9 : 0xff8da79a;
            graphics.setColour (juce::Colour (color));
            const auto x = std::clamp (plot.getX() + static_cast<int> (marker.x),
                                       plot.getX(), plot.getRight() - 1);
            graphics.fillRect (x, plot.getY(), 1, plot.getHeight());
        }
        graphics.setColour (juce::Colour (0xff9eafa2));
        graphics.drawText (juce::String (source->sampleRateHz) + " Hz · "
                               + juce::String (geometry->durationSeconds(), 3) + " s",
                           16, getHeight() - 22, getWidth() - 32, 18,
                           juce::Justification::centredLeft);
    }

private:
    juce::Rectangle<int> plotBounds() const
    {
        return { 16, 52, std::max (1, getWidth() - 32), std::max (1, getHeight() - 76) };
    }

    void updateGeometry()
    {
        const auto plot = plotBounds();
        geometry = source && region
            ? InstrumentUiWaveformGeometry::fromPrepared (
                  *source, *region, plot.getWidth(), plot.getHeight())
            : std::nullopt;
        repaint();
    }

    std::optional<InstrumentUiDocument::Source> source;
    std::optional<InstrumentUiDocument::Region> region;
    std::optional<InstrumentUiWaveformGeometry> geometry;
    std::shared_ptr<const InstrumentUiWaveformService::Result> result;
};

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

        primaryKnob.setComponentID ("primary-knob-slider");
        primaryKnob.setSliderStyle (juce::Slider::RotaryHorizontalVerticalDrag);
        primaryKnob.setTextBoxStyle (juce::Slider::TextBoxBelow, false, 76, 24);
        primaryKnob.setRange (0.0, 1.0);
        primaryKnob.setNumDecimalPlacesToDisplay (3);
        primaryKnob.setColour (juce::Slider::rotarySliderFillColourId,
                               juce::Colour (0xff7ce0aa));
        primaryKnob.setColour (juce::Slider::rotarySliderOutlineColourId,
                               juce::Colour (0xff414d45));
        primaryKnob.setColour (juce::Slider::thumbColourId,
                               juce::Colour (0xffdce9de));
        primaryKnob.setWantsKeyboardFocus (true);
        primaryKnob.onDragStart = [this] { beginPrimaryGesture(); };
        primaryKnob.onValueChange = [this] { writePrimaryValue(); };
        primaryKnob.onDragEnd = [this]
        {
            endPrimaryGesture();
            refreshPrimaryKnob();
        };
        addAndMakeVisible (primaryKnob);
        primaryLabel.setComponentID ("primary-knob-label");
        primaryLabel.setJustificationType (juce::Justification::centred);
        primaryLabel.setColour (juce::Label::textColourId, juce::Colour (0xffdce9de));
        addAndMakeVisible (primaryLabel);
        refreshPrimaryKnob();

        addAndMakeVisible (meter);
        addAndMakeVisible (waveform);
        meterGeneration = processor.getParameterSurfaceGeneration();
        processor.subscribeMeter (meterSession, meterGeneration);
        processor.setMeterSessionVisible (meterSession, false);
        setResizable (true, true);
        setResizeLimits (620, 420, 1600, 1100);
        setSize (820, 560);
        refreshWaveform();
        startTimerHz (30);
    }

    ~DandrumNativeEditor() override
    {
        stopTimer();
        processor.cancelPreparedWaveformSession (meterSession);
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
        primaryLabel.setBounds (24, 302, 152, 26);
        primaryKnob.setBounds (24, 328, 152, 158);
        waveform.setBounds (200, 302, getWidth() - 224, getHeight() - 326);
    }

    void timerCallback() override
    {
        refreshPrimaryKnob();
        refreshWaveform();
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
    enum class DragState { idle, active, rejected };

    void refreshWaveform()
    {
        const auto generation = processor.getParameterSurfaceGeneration();
        if (generation != waveformGeneration)
        {
            if (waveformJob)
                processor.cancelPreparedWaveformJob (*waveformJob);
            waveformJob.reset();
            waveformGeneration = generation;
            waveform.clear();
            const auto document = processor.getPreparedUiDocument();
            if (document && document->capabilities.preparedWaveform)
                for (const auto& source : document->sources)
                    if (! source.regions.empty())
                    {
                        const auto& region = source.regions.front();
                        waveform.setPreparedRegion (source, region);
                        waveformJob = processor.requestPreparedWaveform (
                            generation, source.id, region.id, 0,
                            static_cast<std::size_t> (std::min<std::uint64_t> (
                                512, region.endFrame - region.startFrame)), meterSession);
                        if (! waveformJob)
                            waveform.setUnavailable();
                        break;
                    }
        }
        if (waveformJob)
            if (const auto status = processor.getPreparedWaveformJobStatus (*waveformJob))
            {
                if (status->state == InstrumentUiWaveformService::State::ready
                    && status->generation == waveformGeneration)
                {
                    waveform.setResult (status->result);
                    waveformJob.reset();
                }
                else if (status->state != InstrumentUiWaveformService::State::running)
                {
                    waveform.setUnavailable();
                    waveformJob.reset();
                }
            }
    }

    void refreshPrimaryKnob()
    {
        const auto state = processor.getUiParameterState();
        const auto preferred = std::find_if (
            state.parameters.begin(), state.parameters.end(), [] (const auto& value)
            { return value.id == "amp.release_ms"; });
        const auto* selected = state.parameters.empty() ? nullptr
            : preferred != state.parameters.end() ? &*preferred : &state.parameters.front();
        const auto id = selected != nullptr ? selected->id : std::string {};
        if (state.generation != primaryGeneration || id != primaryId)
        {
            endPrimaryGesture();
            primaryGeneration = state.generation;
            primaryId = id;
            const auto label = selected != nullptr
                ? juce::String (selected->id == "amp.release_ms" ? "RELEASE"
                    : selected->name.empty() ? selected->id : selected->name)
                : juce::String ("NO PUBLIC CONTROL");
            primaryLabel.setText (label, juce::dontSendNotification);
            primaryKnob.setName (label);
            primaryKnob.setEnabled (selected != nullptr);
        }
        if (selected != nullptr && dragState == DragState::idle)
            primaryKnob.setValue (selected->normalisedValue, juce::dontSendNotification);
    }

    void beginPrimaryGesture()
    {
        if (primaryId.empty() || dragState != DragState::idle)
            return;
        dragState = DragState::rejected;
        dragGeneration = primaryGeneration;
        dragId = primaryId;
        if (processor.uiCommands().beginGesture (
                { dragGeneration, dragId, meterSession }).status
            == InstrumentUiCommandStatus::accepted)
            dragState = DragState::active;
    }

    void writePrimaryValue()
    {
        if (primaryId.empty() || dragState == DragState::rejected)
            return;
        const auto reply = processor.uiCommands().setParameter (
            { primaryGeneration, primaryId, primaryKnob.getValue(),
              dragState == DragState::active ? meterSession : 0 });
        if (reply.status != InstrumentUiCommandStatus::accepted)
        {
            endPrimaryGesture();
            refreshPrimaryKnob();
        }
    }

    void endPrimaryGesture()
    {
        if (dragState == DragState::active)
            processor.uiCommands().endGesture ({ dragGeneration, dragId, meterSession });
        dragState = DragState::idle;
    }

    DandrumAudioProcessor& processor;
    std::uint64_t meterSession = 0;
    std::uint32_t meterGeneration = 0;
    std::uint32_t waveformGeneration = 0;
    std::optional<std::uint64_t> waveformJob;
    std::uint32_t primaryGeneration = 0;
    std::uint32_t dragGeneration = 0;
    std::string primaryId;
    std::string dragId;
    DragState dragState = DragState::idle;
    juce::Label title;
    juce::Label summary;
    juce::Label primaryLabel;
    juce::Slider primaryKnob;
    NativeMasterMeter meter;
    NativePreparedWaveform waveform;
};
}

juce::AudioProcessorEditor* DandrumAudioProcessor::createEditor()
{
    return new DandrumNativeEditor (*this);
}
