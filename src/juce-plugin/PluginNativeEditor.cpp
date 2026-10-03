#include "NativeMasterMeter.h"
#include "PluginProcessor.h"
#include "InstrumentUiWaveformGeometry.h"
#include "InstrumentUiSpectralGeometry.h"
#include "DesignTokens.h"
#include "NativeKnobFontBinaryData.h"

#include <juce_gui_basics/juce_gui_basics.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <cmath>
#include <charconv>
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
juce::Font nativeKnobFont (bool value, float height)
{
    static const auto uiFace = juce::Typeface::createSystemTypefaceFor (
        NativeKnobFontBinaryData::BarlowSemiCondensedSemiBold_ttf,
        NativeKnobFontBinaryData::BarlowSemiCondensedSemiBold_ttfSize);
    static const auto valueFace = juce::Typeface::createSystemTypefaceFor (
        NativeKnobFontBinaryData::JetBrainsMonoMedium_ttf,
        NativeKnobFontBinaryData::JetBrainsMonoMedium_ttfSize);
    return juce::FontOptions (value ? valueFace : uiFace).withHeight (height);
}

class NativeKnobReadout final : public juce::TextButton
{
public:
    void paint (juce::Graphics& graphics) override
    {
        namespace tokens = dandrum::ui::tokens;
        const auto bounds = getLocalBounds().toFloat().reduced (0.5f);
        graphics.setColour (juce::Colour (tokens::dd_ink_2));
        graphics.fillRoundedRectangle (bounds, tokens::radius_1);
        graphics.setColour (juce::Colour (tokens::border_control));
        graphics.drawRoundedRectangle (bounds, tokens::radius_1, 1.0f);
        graphics.setColour (juce::Colour (tokens::text_primary));
        graphics.setFont (nativeKnobFont (true, tokens::type_value));
        graphics.drawText (getButtonText(), getLocalBounds().reduced (6, 0),
                           juce::Justification::centredLeft, true);
    }
};

class NativeKnobValueEditor final : public juce::TextEditor
{
public:
    std::function<void()> onCommit;
    std::function<void()> onCancel;
    std::function<void()> onBlur;

    bool keyPressed (const juce::KeyPress& key) override
    {
        if (key.getKeyCode() == juce::KeyPress::returnKey)
        {
            const auto callback = onCommit;
            if (callback)
                callback();
            return true;
        }
        if (key.getKeyCode() == juce::KeyPress::escapeKey)
        {
            const auto callback = onCancel;
            if (callback)
                callback();
            return true;
        }
        return juce::TextEditor::keyPressed (key);
    }

    void focusLost (FocusChangeType cause) override
    {
        // Keep caret/undo cleanup, but never queue a host completion for a later entry.
        juce::TextEditor::focusLost (cause);
        const auto callback = onBlur;
        if (callback)
            callback();
    }
};

class NativeHostKnob final : public juce::Slider
{
public:
    NativeHostKnob()
    {
        setRange (0.0, 1.0);
        setSliderStyle (juce::Slider::RotaryVerticalDrag);
        setTextBoxStyle (juce::Slider::NoTextBox, false, 0, 0);
        setWantsKeyboardFocus (true);
        caption.setComponentID ("primary-knob-label");
        caption.setJustificationType (juce::Justification::centred);
        caption.setColour (juce::Label::textColourId,
                           juce::Colour (dandrum::ui::tokens::text_secondary));
        caption.setFont (nativeKnobFont (false, 12.0f));
        caption.setInterceptsMouseClicks (false, false);
        addAndMakeVisible (caption);
        readout.setComponentID ("primary-knob-readout");
        readout.setTooltip ("Click to type an actual value");
        readout.onClick = [this] { startEditing(); };
        readout.addMouseListener (this, true);
        addChildComponent (readout);
        valueEditor.setComponentID ("primary-knob-value-editor");
        valueEditor.setFont (nativeKnobFont (true, dandrum::ui::tokens::type_value));
        valueEditor.setColour (juce::TextEditor::backgroundColourId,
                               juce::Colour (dandrum::ui::tokens::dd_ink_2));
        valueEditor.setColour (juce::TextEditor::textColourId,
                               juce::Colour (dandrum::ui::tokens::text_primary));
        valueEditor.setColour (juce::TextEditor::outlineColourId,
                               juce::Colour (dandrum::ui::tokens::dd_paper_2));
        valueEditor.setColour (juce::TextEditor::focusedOutlineColourId,
                               juce::Colour (dandrum::ui::tokens::dd_paper_2));
        valueEditor.onCommit = [this] { finishEditing (false, true); };
        valueEditor.onCancel = [this] { finishEditing (true, true); };
        valueEditor.onBlur = [this] { finishEditing (false, false); };
        valueEditor.addMouseListener (this, true);
        addChildComponent (valueEditor);
    }

    ~NativeHostKnob() override
    {
        // The child editor must not commit its draft while members are destroyed.
        valueEditor.onCommit = {};
        valueEditor.onCancel = {};
        valueEditor.onBlur = {};
        readout.removeMouseListener (this);
        valueEditor.removeMouseListener (this);
    }

    void setPreparedParameter (std::optional<InstrumentUiDocument::Parameter> next,
                               std::uint32_t generation)
    {
        // These are owned copies. A reload cannot invalidate range/default data.
        if (next && (! std::isfinite (next->minValue) || ! std::isfinite (next->maxValue)
                     || next->maxValue <= next->minValue))
            next.reset();
        edit.reset();
        drag.reset();
        nudgeUntil = 0.0;
        popupHoldUntil = 0.0;
        valueEditor.setVisible (false);
        prepared = std::move (next);
        preparedGeneration = generation;
        setEnabled (prepared.has_value());
        updatePopup();
    }

    void observeValue (double value)
    {
        authoritative = value;
        if (! edit && ! drag)
            setValue (value, juce::dontSendNotification);
        updatePopup();
    }

    bool keyPressed (const juce::KeyPress& key) override
    {
        if (! prepared)
            return false;
        if (key.getKeyCode() == juce::KeyPress::returnKey)
        {
            startEditing();
            return true;
        }
        if (edit)
            return false;
        const auto step = key.getModifiers().isShiftDown() ? 0.01 : 0.05;
        double next = getValue();
        const auto code = key.getKeyCode();
        if (code == juce::KeyPress::upKey || code == juce::KeyPress::rightKey)
            next += step;
        else if (code == juce::KeyPress::downKey || code == juce::KeyPress::leftKey)
            next -= step;
        else if (code == juce::KeyPress::homeKey)
            next = 0.0;
        else if (code == juce::KeyPress::endKey)
            next = 1.0;
        else if (code == juce::KeyPress::deleteKey || code == juce::KeyPress::backspaceKey)
            next = prepared->normalisedDefaultValue;
        else
            return false;
        commitValue (next);
        return true;
    }

    void mouseDoubleClick (const juce::MouseEvent& event) override
    {
        if (prepared && ! edit && ! isPopupEvent (event)
            && event.mods.isLeftButtonDown())
            commitValue (prepared->normalisedDefaultValue);
    }

    void mouseWheelMove (const juce::MouseEvent& event,
                         const juce::MouseWheelDetails& wheel) override
    {
        if (! prepared || edit || isPopupEvent (event)
            || ! (wheel.deltaY > 0.0f || wheel.deltaY < 0.0f))
            return;
        const auto step = event.mods.isShiftDown() ? 0.01 : 0.02;
        commitValue (getValue() + (wheel.deltaY > 0.0f ? step : -step));
    }

    void mouseDown (const juce::MouseEvent& event) override
    {
        if (! prepared || edit || drag || isPopupEvent (event)
            || ! event.mods.isLeftButtonDown())
            return;
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        grabKeyboardFocus();
        if (safeThis == nullptr)
            return;
        drag = Drag { event.position.y, getValue() };
        const auto callback = onDragStart;
        if (callback)
            callback();
        if (safeThis != nullptr)
            safeThis->updatePopup();
    }

    void mouseDrag (const juce::MouseEvent& event) override
    {
        if (! drag || event.originalComponent != this)
            return;
        const auto distance = event.mods.isShiftDown() ? 800.0 : 200.0;
        commitValue (drag->value + (drag->y - event.position.y) / distance, false);
    }

    void mouseUp (const juce::MouseEvent&) override { finishDrag(); }

    void mouseEnter (const juce::MouseEvent& event) override
    {
        if (event.eventComponent == this)
            hovered = true;
        else
            popupHovered = true;
        popupHoldUntil = 0.0;
        updatePopup();
    }

    void mouseExit (const juce::MouseEvent& event) override
    {
        if (event.eventComponent == this)
            hovered = false;
        else
            popupHovered = false;
        popupHoldUntil = juce::Time::getMillisecondCounterHiRes() + 250.0;
        updatePopup();
    }

    void focusGained (FocusChangeType) override { updatePopup(); }
    void focusLost (FocusChangeType) override
    {
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        if (! hasKeyboardFocus (true))
            finishDrag();
        if (safeThis != nullptr)
            safeThis->updatePopup();
    }
    void focusOfChildComponentChanged (FocusChangeType) override
    {
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        if (! hasKeyboardFocus (true))
            finishDrag();
        if (safeThis != nullptr)
            safeThis->updatePopup();
    }

    void startEditing()
    {
        if (! prepared || edit)
            return;
        edit = Edit { *prepared, preparedGeneration, actualText (getValue()) };
        valueEditor.setText (edit->initial, false);
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        updatePopup();
        if (safeThis == nullptr)
            return;
        valueEditor.grabKeyboardFocus();
        if (safeThis != nullptr)
            safeThis->valueEditor.selectAll();
    }

    void setCaption (const juce::String& text)
    {
        caption.setText (text.toUpperCase(), juce::dontSendNotification);
        caption.setTooltip (text);
        setName (text);
    }

    void resized() override
    {
        caption.setBounds (6, 6, getWidth() - 12, 14);
        readout.setBounds ((getWidth() - 104) / 2 + 6, 118, 92, 24);
        valueEditor.setBounds (readout.getBounds());
    }

    void paint (juce::Graphics& graphics) override
    {
        namespace tokens = dandrum::ui::tokens;
        const float size = tokens::knob_lg;
        const float cx = static_cast<float> (getWidth()) / 2.0f;
        const float cy = 25.0f + size / 2.0f;
        const float trackMax = 4.0f, trackMin = 1.5f;
        const float rTrack = size / 2.0f - 6.5f - trackMax / 2.0f;
        const float rCap = rTrack - trackMax / 2.0f - 2.5f;
        const auto drawArc = [&] (float radius, float from, float to,
                                 juce::Colour colour, float width)
        {
            juce::Path path;
            path.addCentredArc (cx, cy, radius, radius, 0.0f,
                                juce::degreesToRadians (from), juce::degreesToRadians (to), true);
            graphics.setColour (colour);
            graphics.strokePath (path, juce::PathStrokeType (
                width, juce::PathStrokeType::curved, juce::PathStrokeType::rounded));
        };
        drawArc (rTrack, -135.0f, 135.0f,
                 juce::Colour (isEnabled() ? tokens::color_track : tokens::dd_ink_4), trackMin);
        if (isEnabled())
            drawArc (rTrack, -135.0f, -135.0f + static_cast<float> (getValue()) * 270.0f,
                     juce::Colour (tokens::color_value),
                     drag || nudgeUntil > juce::Time::getMillisecondCounterHiRes() ? trackMax : trackMin);
        const auto point = [&] (float radius)
        {
            const auto angle = juce::degreesToRadians (-135.0f);
            return juce::Point<float> { cx + radius * std::sin (angle),
                                        cy - radius * std::cos (angle) };
        };
        graphics.setColour (juce::Colour (tokens::dd_paper_3).withAlpha (isEnabled() ? 1.0f : 0.4f));
        graphics.drawLine ({ point (rTrack + trackMin / 2.0f + 1.0f),
                            point (rTrack + trackMin / 2.0f + 4.0f) }, 1.5f);
        graphics.setColour (juce::Colours::black.withAlpha (0.45f));
        graphics.fillEllipse (cx - rCap, cy - rCap + 1.5f, rCap * 2.0f, rCap * 2.0f);
        graphics.setColour (juce::Colour (! isEnabled() ? tokens::dd_ink_4
            : drag ? tokens::dd_ink_6
            : hovered ? tokens::dd_cap_hover : tokens::dd_ink_5));
        graphics.fillEllipse (cx - rCap, cy - rCap, rCap * 2.0f, rCap * 2.0f);
        graphics.setColour (juce::Colour (tokens::dd_line_3).withAlpha (isEnabled() ? 0.6f : 0.3f));
        graphics.drawEllipse (cx - rCap, cy - rCap, rCap * 2.0f, rCap * 2.0f, 1.0f);
        drawArc (rCap - 1.0f, -60.0f, 60.0f, juce::Colours::white.withAlpha (0.09f), 1.0f);
        if (hasKeyboardFocus (true))
        {
            graphics.setColour (juce::Colour (tokens::color_focus));
            graphics.drawRoundedRectangle (
                juce::Rectangle<float> { 2.0f, 2.0f, static_cast<float> (getWidth()) - 4.0f, 110.0f },
                tokens::radius_2, 2.0f);
        }
        if (popupVisible)
        {
            const auto bounds = popupBounds();
            graphics.setColour (juce::Colour (tokens::dd_ink_0));
            graphics.fillRoundedRectangle (bounds, tokens::radius_2);
            graphics.setColour (juce::Colour (tokens::border_strong));
            graphics.drawRoundedRectangle (bounds.reduced (0.5f), tokens::radius_2, 1.0f);
            juce::Path arrow;
            arrow.startNewSubPath (cx - 4.0f, 112.0f);
            arrow.lineTo (cx, 108.0f);
            arrow.lineTo (cx + 4.0f, 112.0f);
            graphics.setColour (juce::Colour (tokens::dd_ink_0));
            graphics.fillPath (arrow);
            graphics.setColour (juce::Colour (tokens::border_strong));
            graphics.strokePath (arrow, juce::PathStrokeType (1.0f));
        }
    }

private:
    juce::Rectangle<float> popupBounds() const
    {
        return { static_cast<float> ((getWidth() - 104) / 2), 112.0f, 104.0f, 38.0f };
    }

    bool isPopupEvent (const juce::MouseEvent& event) const
    {
        return event.originalComponent != this
            || (popupVisible && popupBounds().contains (event.position));
    }

    struct Drag { float y; double value; };

    void finishDrag()
    {
        if (! drag)
            return;
        drag.reset();
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        const auto callback = onDragEnd;
        if (callback)
            callback();
        if (safeThis != nullptr)
            safeThis->updatePopup();
    }

    void commitValue (double next, bool nudge = true)
    {
        if (nudge)
            nudgeUntil = juce::Time::getMillisecondCounterHiRes() + 600.0;
        setValue (std::clamp (next, 0.0, 1.0), juce::dontSendNotification);
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        const auto callback = onValueChange;
        if (callback)
            callback();
        if (safeThis == nullptr)
            return;
        authoritative = getValue();
        updatePopup();
    }

    struct Edit
    {
        InstrumentUiDocument::Parameter parameter;
        std::uint32_t generation;
        juce::String initial;
    };

    juce::String actualText (double value) const
    {
        if (! prepared)
            return {};
        const double actual = prepared->minValue
            + value * (static_cast<double> (prepared->maxValue) - prepared->minValue);
        std::array<char, 64> buffer {};
        const auto result = std::to_chars (buffer.data(), buffer.data() + buffer.size(),
                                           actual, std::chars_format::general, 6);
        return juce::String::fromUTF8 (buffer.data(), static_cast<int> (result.ptr - buffer.data()));
    }

    void updatePopup()
    {
        const auto now = juce::Time::getMillisecondCounterHiRes();
        popupVisible = prepared && (edit.has_value() || drag.has_value() || hasKeyboardFocus (true)
            || hovered || popupHovered || popupHoldUntil > now || nudgeUntil > now);
        if (! edit)
            readout.setButtonText (actualText (getValue()));
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        readout.setVisible (popupVisible && ! edit);
        if (safeThis == nullptr)
            return;
        valueEditor.setVisible (popupVisible && edit.has_value());
        if (safeThis != nullptr)
            safeThis->repaint();
    }

    void finishEditing (bool cancelled, bool returnFocus)
    {
        if (! edit)
            return;
        const auto captured = *edit;
        const auto text = valueEditor.getText();
        edit.reset();
        juce::Component::SafePointer<NativeHostKnob> safeThis (this);
        valueEditor.setVisible (false);
        if (safeThis == nullptr)
            return;
        std::optional<double> next;
        if (! cancelled && prepared && captured.generation == preparedGeneration
            && captured.parameter.id == prepared->id && text != captured.initial)
        {
            auto decimal = text.trim().replaceCharacter (0x2212, '-').toStdString();
            if (decimal.size() > 1 && decimal.front() == '+' && decimal[1] != '-')
                decimal.erase (0, 1);
            double actual = 0.0;
            const auto result = std::from_chars (decimal.data(), decimal.data() + decimal.size(), actual);
            if (result.ec == std::errc {} && result.ptr == decimal.data() + decimal.size()
                && std::isfinite (actual) && actual >= captured.parameter.minValue
                && actual <= captured.parameter.maxValue)
                next = (actual - captured.parameter.minValue)
                    / (static_cast<double> (captured.parameter.maxValue) - captured.parameter.minValue);
        }
        if (next)
            commitValue (*next);
        else
            setValue (authoritative, juce::dontSendNotification);
        if (safeThis == nullptr)
            return;
        updatePopup();
        if (safeThis != nullptr && returnFocus)
            grabKeyboardFocus();
    }

    juce::Label caption;
    NativeKnobReadout readout;
    NativeKnobValueEditor valueEditor;
    std::optional<InstrumentUiDocument::Parameter> prepared;
    std::optional<Edit> edit;
    std::optional<Drag> drag;
    std::uint32_t preparedGeneration = 0;
    double authoritative = 0.0;
    bool popupVisible = false;
    bool hovered = false;
    bool popupHovered = false;
    double popupHoldUntil = 0.0;
    double nudgeUntil = 0.0;
};

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
                    const auto column = plot.getX() + static_cast<int> (std::lround (*x));
                    const auto high = plot.getY() + static_cast<int> (std::lround (geometry->sampleY (bucket.maximum)));
                    const auto low = plot.getY() + static_cast<int> (std::lround (geometry->sampleY (bucket.minimum)));
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
            const auto x = std::clamp (plot.getX() + static_cast<int> (std::lround (marker.x)),
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

class NativePreparedSpectrum final : public juce::Component
{
public:
    NativePreparedSpectrum()
    {
        setComponentID ("prepared-spectrum");
        const std::array ids { "spectral-frequency-min", "spectral-frequency-max",
                              "spectral-time-start", "spectral-time-end", "spectral-settings" };
        for (std::size_t i = 0; i < labels.size(); ++i)
        {
            labels[i].setComponentID (ids[i]);
            labels[i].setFont (nativeKnobFont (true, 10.0f));
            labels[i].setColour (juce::Label::textColourId, juce::Colour (dandrum::ui::tokens::text_secondary));
            labels[i].setBorderSize (juce::BorderSize<int> (0));
            addAndMakeVisible (labels[i]);
        }
        labels[1].setJustificationType (juce::Justification::centredLeft);
        labels[3].setJustificationType (juce::Justification::centredRight);
        clear();
    }
    void clear()
    {
        source.reset(); region.reset(); result.reset(); geometry.reset(); image = {};
        for (auto& label : labels) label.setText ({}, juce::dontSendNotification);
        setName ("NO PREPARED SAMPLE"); repaint();
    }
    void setPreparedRegion (const InstrumentUiDocument::Source& preparedSource,
                            const InstrumentUiDocument::Region& preparedRegion)
    {
        clear(); source = preparedSource; region = preparedRegion;
        setName ("PREPARING SPECTRUM");
    }
    void setUnavailable (const std::string& error)
    {
        result.reset(); geometry.reset(); image = {};
        setName ("SPECTRUM UNAVAILABLE: " + juce::String (error)); repaint();
    }
    void setResult (std::shared_ptr<const InstrumentUiSpectralService::Result> ready)
    {
        result = std::move (ready); updateImage();
        if (geometry)
            setName ("Prepared spectrum: " + juce::String (source->id) + "." + juce::String (region->id));
        else
            setUnavailable ("Incoherent prepared spectral data");
    }
    void resized() override
    {
        const auto plot = plotBounds();
        labels[0].setBounds (16, plot.getBottom() - 16, 46, 16);
        labels[1].setBounds (16, plot.getY(), 46, 16);
        labels[2].setBounds (plot.getX(), plot.getBottom() + 2, 120, 18);
        labels[3].setBounds (plot.getRight() - 120, plot.getBottom() + 2, 120, 18);
        labels[4].setBounds (16, getHeight() - 27, std::max (1, getWidth() - 32), 18);
        updateImage();
    }
    void paint (juce::Graphics& graphics) override
    {
        namespace tokens = dandrum::ui::tokens;
        graphics.setColour (juce::Colour (tokens::dd_ink_2));
        graphics.fillRoundedRectangle (getLocalBounds().toFloat(), 10.0f);
        graphics.setColour (juce::Colour (tokens::border_control));
        graphics.drawRoundedRectangle (getLocalBounds().toFloat().reduced (0.5f), 10.0f, 1.0f);
        graphics.setColour (juce::Colour (tokens::text_primary));
        graphics.setFont (nativeKnobFont (false, 14.0f));
        graphics.drawText ("PREPARED SPECTRUM", 16, 10, getWidth() - 32, 20, juce::Justification::centredLeft);
        graphics.setColour (juce::Colour (tokens::text_secondary));
        graphics.setFont (nativeKnobFont (false, 11.0f));
        graphics.drawText (getName(), 16, 30, getWidth() - 32, 18, juce::Justification::centredLeft);
        const auto plot = plotBounds();
        graphics.setColour (juce::Colour (tokens::dd_ink_0)); graphics.fillRect (plot);
        if (image.isValid()) graphics.drawImageAt (image, plot.getX(), plot.getY());
    }
private:
    juce::Rectangle<int> plotBounds() const
    { return { 64, 52, std::max (1, getWidth() - 80), std::max (1, getHeight() - 110) }; }
    static juce::Colour colour (float db, float floor)
    {
        namespace tokens = dandrum::ui::tokens;
        constexpr std::array ramp { tokens::dd_ink_0, tokens::dd_ink_5, tokens::dd_vermilion_lo,
                                    tokens::dd_vermilion, tokens::dd_paper_1 };
        const auto level = std::clamp ((double (db) - floor) / -floor, 0.0, 1.0) * (ramp.size() - 1);
        const auto index = static_cast<std::size_t> (std::floor (level));
        const auto fraction = level - index;
        const juce::Colour low (ramp[index]), high (ramp[std::min (index + 1, ramp.size() - 1)]);
        const auto mix = [fraction] (juce::uint8 a, juce::uint8 b)
        { return static_cast<juce::uint8> (std::lround (a + (double (b) - a) * fraction)); };
        return juce::Colour::fromRGB (mix (low.getRed(), high.getRed()), mix (low.getGreen(), high.getGreen()),
                                      mix (low.getBlue(), high.getBlue()));
    }
    void updateImage()
    {
        const auto plot = plotBounds();
        geometry = source && region && result ? InstrumentUiSpectralGeometry::fromPrepared (
            *source, *region, *result, plot.getWidth(), plot.getHeight()) : std::nullopt;
        image = {};
        if (! geometry) return;
        image = juce::Image (juce::Image::RGB, plot.getWidth(), plot.getHeight(), true);
        juce::Graphics pixels (image);
        for (const auto& column : result->columns)
        {
            const auto left = static_cast<int> (std::lround (geometry->columnX (column.startFrame)));
            const auto span = result->endFrame - column.startFrame;
            const auto right = static_cast<int> (std::lround (geometry->columnX (
                column.startFrame + std::min (span, result->settings.hopFrames))));
            for (int row = 0; row < plot.getHeight(); ++row)
            {
                pixels.setColour (colour (geometry->rowMagnitude (column, row), result->settings.floorDbFS));
                pixels.fillRect (left, row, std::max (1, right - left), 1);
            }
        }
        for (const auto& marker : geometry->markers())
        {
            using Kind = InstrumentUiWaveformGeometry::MarkerKind;
            pixels.setColour (juce::Colour (marker.kind == Kind::loopStart || marker.kind == Kind::loopEnd
                ? 0xffe2bf72 : marker.kind == Kind::sliceStart || marker.kind == Kind::sliceEnd ? 0xffab9ee9 : 0xff8da79a));
            pixels.fillRect (std::clamp (static_cast<int> (std::lround (marker.x)), 0, plot.getWidth() - 1), 0, 1, plot.getHeight());
        }
        labels[0].setText (juce::String (std::lround (result->frequencyHz[1])) + " Hz", juce::dontSendNotification);
        labels[1].setText (juce::String (result->frequencyHz.back() / 1000.0, 0) + " kHz", juce::dontSendNotification);
        labels[2].setText (juce::String (geometry->startSeconds(), 3) + " s", juce::dontSendNotification);
        labels[3].setText (juce::String (geometry->endSeconds(), 3) + " s", juce::dontSendNotification);
        const auto separator = juce::String::fromUTF8 (" · ");
        labels[4].setText ("Hann" + separator + "FFT " + juce::String (static_cast<int> (result->settings.fftSize))
            + separator + "hop " + juce::String (std::to_string (result->settings.hopFrames))
            + separator + juce::String (result->settings.floorDbFS, 0) + "..0 dBFS" + separator + "ch "
            + juce::String (result->channel + 1) + separator + juce::String (result->sampleRateHz) + " Hz"
            + separator + "DC omitted",
            juce::dontSendNotification);
        repaint();
    }
    std::optional<InstrumentUiDocument::Source> source;
    std::optional<InstrumentUiDocument::Region> region;
    std::shared_ptr<const InstrumentUiSpectralService::Result> result;
    std::optional<InstrumentUiSpectralGeometry> geometry;
    juce::Image image;
    std::array<juce::Label, 5> labels;
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
        primaryKnob.onDragStart = [this] { beginPrimaryGesture(); };
        primaryKnob.onValueChange = [this] { writePrimaryValue(); };
        primaryKnob.onDragEnd = [this]
        {
            juce::Component::SafePointer<DandrumNativeEditor> safeThis (this);
            endPrimaryGesture();
            if (safeThis != nullptr)
                safeThis->refreshPrimaryKnob();
        };
        addAndMakeVisible (primaryKnob);
        refreshPrimaryKnob();

        addAndMakeVisible (meter);
        addAndMakeVisible (waveform);
        addChildComponent (spectrum);
        waveToggle.setButtonText ("Wave"); spectralToggle.setButtonText ("Spectral");
        waveToggle.setComponentID ("sample-display-wave"); spectralToggle.setComponentID ("sample-display-spectral");
        waveToggle.onClick = [this] { setSpectralDisplay (false); };
        spectralToggle.onClick = [this] { setSpectralDisplay (true); };
        addAndMakeVisible (waveToggle); addAndMakeVisible (spectralToggle);
        waveToggle.setToggleState (true, juce::dontSendNotification);
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
        primaryKnob.onDragStart = {};
        primaryKnob.onValueChange = {};
        primaryKnob.onDragEnd = {};
        processor.cancelPreparedWaveformSession (meterSession);
        processor.cancelPreparedSpectrumSession (meterSession);
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
        primaryKnob.setBounds (24, 302, 152, 184);
        waveform.setBounds (200, 302, getWidth() - 224, getHeight() - 326);
        spectrum.setBounds (waveform.getBounds());
        waveToggle.setBounds (getWidth() - 176, getHeight() - (showSpectral ? 108 : 74), 64, 20);
        spectralToggle.setBounds (getWidth() - 108, getHeight() - (showSpectral ? 108 : 74), 68, 20);
    }

    void timerCallback() override
    {
        juce::Component::SafePointer<DandrumNativeEditor> safeThis (this);
        refreshPrimaryKnob();
        if (safeThis == nullptr)
            return;
        refreshWaveform();
        refreshSpectrum();
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

    void setSpectralDisplay (bool enabled)
    {
        if (spectralJob) processor.cancelPreparedSpectrumJob (*spectralJob);
        spectralJob.reset(); spectralRequested = false; showSpectral = enabled;
        waveform.setVisible (! enabled); spectrum.setVisible (enabled);
        waveToggle.setToggleState (! enabled, juce::dontSendNotification);
        spectralToggle.setToggleState (enabled, juce::dontSendNotification);
        resized();
        refreshSpectrum();
    }
    void refreshSpectrum()
    {
        if (! showSpectral || ! isShowing())
        {
            if (spectralJob) processor.cancelPreparedSpectrumJob (*spectralJob);
            spectralJob.reset(); spectralRequested = false;
            return;
        }
        if (! spectralRequested && spectralSelection)
        {
            spectralRequested = true;
            spectralJob = processor.requestPreparedSpectrum (waveformGeneration,
                spectralSelection->first, spectralSelection->second, 0, meterSession);
            if (! spectralJob) spectrum.setUnavailable ("Request not admitted; click Spectral to retry");
        }
        if (spectralJob)
        {
            const auto status = processor.getPreparedSpectrumJobStatus (*spectralJob);
            if (status && status->state != InstrumentUiSpectralService::State::running)
            {
                if (status->state == InstrumentUiSpectralService::State::ready && status->generation == waveformGeneration)
                    spectrum.setResult (status->result);
                else
                    spectrum.setUnavailable (status->error.empty() ? "Request retired; click Spectral to retry" : status->error);
                spectralJob.reset();
            }
        }
    }

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
            if (spectralJob) processor.cancelPreparedSpectrumJob (*spectralJob);
            spectralJob.reset(); spectralSelection.reset(); spectralRequested = false; spectrum.clear();
            const auto document = processor.getPreparedUiDocument();
            if (document && document->capabilities.preparedWaveform)
                for (const auto& source : document->sources)
                    if (! source.regions.empty())
                    {
                        const auto& region = source.regions.front();
                        waveform.setPreparedRegion (source, region);
                        spectrum.setPreparedRegion (source, region);
                        spectralSelection = std::pair { source.id, region.id };
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
        if (! primaryBindingKnown || state.generation != primaryGeneration || id != primaryId)
        {
            juce::Component::SafePointer<DandrumNativeEditor> safeThis (this);
            endPrimaryGesture();
            if (safeThis == nullptr)
                return;
            primaryBindingKnown = true;
            primaryGeneration = state.generation;
            primaryId = id;
            const auto label = selected != nullptr
                ? juce::String (selected->id == "amp.release_ms" ? "RELEASE"
                    : selected->name.empty() ? selected->id : selected->name)
                : juce::String ("NO PUBLIC CONTROL");
            primaryKnob.setCaption (label);
            std::optional<InstrumentUiDocument::Parameter> descriptor;
            if (const auto document = processor.getPreparedUiDocument();
                document && document->generation == primaryGeneration)
                for (const auto& parameter : document->parameters)
                    if (parameter.id == primaryId)
                    {
                        descriptor = parameter;
                        break;
                    }
            primaryKnob.setPreparedParameter (std::move (descriptor), primaryGeneration);
        }
        if (selected != nullptr && dragState == DragState::idle)
            primaryKnob.observeValue (selected->normalisedValue);
    }

    void beginPrimaryGesture()
    {
        if (primaryId.empty() || dragState != DragState::idle)
            return;
        dragState = DragState::rejected;
        dragGeneration = primaryGeneration;
        dragId = primaryId;
        juce::Component::SafePointer<DandrumNativeEditor> safeThis (this);
        const auto reply = processor.uiCommands().beginGesture (
            { dragGeneration, dragId, meterSession });
        if (safeThis != nullptr && reply.status == InstrumentUiCommandStatus::accepted)
            safeThis->dragState = DragState::active;
    }

    void writePrimaryValue()
    {
        if (primaryId.empty() || dragState == DragState::rejected)
            return;
        juce::Component::SafePointer<DandrumNativeEditor> safeThis (this);
        const auto reply = processor.uiCommands().setParameter (
            { primaryGeneration, primaryId, primaryKnob.getValue(),
              dragState == DragState::active ? meterSession : 0 });
        if (safeThis != nullptr && reply.status != InstrumentUiCommandStatus::accepted)
        {
            endPrimaryGesture();
            if (safeThis != nullptr)
                safeThis->refreshPrimaryKnob();
        }
    }

    void endPrimaryGesture()
    {
        const auto active = dragState == DragState::active;
        const InstrumentUiGestureRequest request { dragGeneration, dragId, meterSession };
        dragState = DragState::idle;
        if (active)
            processor.uiCommands().endGesture (request);
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
    bool primaryBindingKnown = false;
    DragState dragState = DragState::idle;
    juce::Label title;
    juce::Label summary;
    NativeHostKnob primaryKnob;
    NativeMasterMeter meter;
    NativePreparedWaveform waveform;
    NativePreparedSpectrum spectrum;
    juce::TextButton waveToggle, spectralToggle;
    bool showSpectral = false, spectralRequested = false;
    std::optional<std::pair<std::string, std::string>> spectralSelection;
    std::optional<std::uint64_t> spectralJob;
};
}

juce::AudioProcessorEditor* DandrumAudioProcessor::createEditor()
{
    return new DandrumNativeEditor (*this);
}
