#include "NativeMasterMeter.h"
#include "PluginProcessor.h"
#include "DesignTokens.h"

#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <iostream>
#include <memory>

namespace
{
juce::Component* findNamedComponent (juce::Component& owner, const juce::String& id)
{
    if (owner.getComponentID() == id)
        return &owner;
    for (int index = 0; index < owner.getNumChildComponents(); ++index)
        if (auto* found = findNamedComponent (*owner.getChildComponent (index), id))
            return found;
    return nullptr;
}

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

struct ClosingHostListener final : juce::AudioProcessorListener
{
    enum class Notification { begin, change, end };
    ClosingHostListener (DandrumAudioProcessor& source,
                         std::unique_ptr<juce::AudioProcessorEditor>& owner,
                         Notification notification, int originalSlot)
        : processor (source), editor (owner), closeOn (notification), expectedSlot (originalSlot)
    {
        processor.addListener (this);
    }
    ~ClosingHostListener() override { processor.removeListener (this); }
    void audioProcessorChanged (juce::AudioProcessor*, const ChangeDetails&) override {}
    void audioProcessorParameterChanged (juce::AudioProcessor*, int slot, float) override
    {
        ++changes;
        observe (Notification::change, slot);
    }
    void audioProcessorParameterChangeGestureBegin (juce::AudioProcessor*, int slot) override
    {
        ++begins;
        observe (Notification::begin, slot);
    }
    void audioProcessorParameterChangeGestureEnd (juce::AudioProcessor*, int slot) override
    {
        ++ends;
        observe (Notification::end, slot);
    }
    void observe (Notification notification, int slot)
    {
        lastSlot = slot;
        allSlotsMatch = allSlotsMatch && slot == expectedSlot;
        if (armed && notification == closeOn)
        {
            armed = false;
            editor.reset();
        }
    }
    DandrumAudioProcessor& processor;
    std::unique_ptr<juce::AudioProcessorEditor>& editor;
    Notification closeOn;
    int expectedSlot;
    bool armed = true;
    bool allSlotsMatch = true;
    int begins = 0, changes = 0, ends = 0, lastSlot = -1;
};

void doubleClick (juce::Slider& knob)
{
    const auto now = juce::Time::getCurrentTime();
    knob.mouseDoubleClick (juce::MouseEvent (
        juce::Desktop::getInstance().getMainMouseSource(), { 32.0f, 32.0f },
        juce::ModifierKeys::leftButtonModifier, 1.0f, 0.0f, 0.0f, 0.0f, 0.0f,
        &knob, &knob, now, { 32.0f, 32.0f }, now, 2, false));
}

void wheelOn (juce::Slider& knob, float delta, bool fine,
               juce::Component* origin = nullptr, float y = 57.0f)
{
    const auto now = juce::Time::getCurrentTime();
    const juce::MouseEvent event (
        juce::Desktop::getInstance().getMainMouseSource(), { 76.0f, y },
        fine ? juce::ModifierKeys::shiftModifier : 0, 1.0f, 0.0f, 0.0f, 0.0f, 0.0f,
        &knob, origin != nullptr ? origin : &knob, now, { 76.0f, 57.0f }, now, 0, false);
    knob.mouseWheelMove (event, { 0.0f, delta, false, false, false });
}

juce::MouseEvent pointerAt (juce::Component& control, float y, int modifiers)
{
    const auto now = juce::Time::getCurrentTime();
    return { juce::Desktop::getInstance().getMainMouseSource(), { 76.0f, y },
             modifiers, 1.0f, 0.0f, 0.0f, 0.0f, 0.0f,
             &control, &control, now, { 76.0f, 57.0f }, now, 1, true };
}
}

int main()
{
    juce::ScopedJuceInitialiser_GUI gui;
    for (const auto& configuration : {
             InstrumentDemoConfiguration::tb303(),
             InstrumentDemoConfiguration::sampler() })
    {
        DandrumAudioProcessor processor (configuration);
        processor.setPlayConfigDetails (0, 2, 48000.0, 64);
        processor.prepareToPlay (48000.0, 64);
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
        auto* waveform = editor->findChildWithID ("prepared-waveform");
        if (waveform == nullptr || ! waveform->isVisible()
            || waveform->getWidth() < 300 || waveform->getHeight() < 100)
        {
            std::cerr << "native editor omitted the prepared waveform panel\n";
            return 1;
        }
        if (configuration.instrumentId == "dandrum.advanced-drum-kit")
        {
            const auto deadline = juce::Time::getMillisecondCounterHiRes() + 1500.0;
            while (! waveform->getName().startsWith ("Prepared waveform:")
                   && juce::Time::getMillisecondCounterHiRes() < deadline)
            {
                juce::Thread::sleep (10);
                juce::Timer::callPendingTimersSynchronously();
            }
            const auto waveformImage = editor->createComponentSnapshot (editor->getLocalBounds());
            bool drewEnvelope = false;
            const auto bounds = waveform->getBounds();
            for (int y = bounds.getY() + 50; y < bounds.getBottom() - 20; ++y)
                for (int x = bounds.getX() + 16; x < bounds.getRight() - 16; ++x)
                    if (waveformImage.getPixelAt (x, y) == juce::Colour (0xff7ce0aa))
                        drewEnvelope = true;
            if (! waveform->getName().startsWith ("Prepared waveform:") || ! drewEnvelope)
            {
                std::cerr << "native sampler did not draw prepared signed PCM\n";
                return 1;
            }
            if (waveformImage.getPixelAt (bounds.getRight() - 17, bounds.getY() + 60)
                != juce::Colour (0xff8da79a))
            {
                std::cerr << "native sampler clipped the prepared region end marker\n";
                return 1;
            }
            if (const auto* output = std::getenv ("DANDRUM_NATIVE_SAMPLER_SNAPSHOT"))
            {
                auto stream = juce::File (output).createOutputStream();
                juce::PNGImageFormat png;
                if (stream == nullptr || ! png.writeImageToStream (waveformImage, *stream))
                    return 1;
            }
            editor->setSize (1200, 800);
            const auto largeImage = editor->createComponentSnapshot (editor->getLocalBounds());
            const auto largeBounds = waveform->getBounds();
            bool drewLargeEnvelope = false;
            for (int y = largeBounds.getY() + 50; y < largeBounds.getBottom() - 20; ++y)
                for (int x = largeBounds.getX() + 16; x < largeBounds.getRight() - 16; ++x)
                    if (largeImage.getPixelAt (x, y) == juce::Colour (0xff7ce0aa))
                        drewLargeEnvelope = true;
            if (largeBounds.getWidth() < 900 || largeBounds.getHeight() < 400
                || ! drewLargeEnvelope)
            {
                std::cerr << "native sampler waveform did not survive full-size layout\n";
                return 1;
            }
            editor->setSize (820, 560);
        }
        else if (waveform->getName() != "NO PREPARED SAMPLE")
        {
            std::cerr << "native TB-303 claimed an unavailable sample waveform\n";
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
            findNamedComponent (*editor, "primary-knob-label"));
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
            || knobLabel->getText() != expectedLabel.toUpperCase()
            || std::abs (knob->getValue() - selected.normalisedValue) > 0.00001)
        {
            std::cerr << "native editor did not bind an authoritative public knob\n";
            return 1;
        }
        if (knob->getTextBoxPosition() != juce::Slider::NoTextBox)
        {
            std::cerr << "native reference knob retained a permanent value field\n";
            return 1;
        }
        auto* nativeValueInput = dynamic_cast<juce::TextEditor*> (
            findNamedComponent (*knob, "primary-knob-value-editor"));
        if (knobLabel->getFont().getTypefacePtr()->getName() != "Barlow Semi Condensed"
            || nativeValueInput == nullptr
            || nativeValueInput->getFont().getTypefacePtr()->getName() != "JetBrains Mono")
        {
            std::cerr << "native reference knob did not use the pinned UI/value font families\n";
            return 1;
        }
        const auto knobImage = knob->createComponentSnapshot (knob->getLocalBounds());
        const auto centreX = knob->getWidth() / 2;
        if (knobImage.getPixelAt (centreX, 57)
                != juce::Colour (dandrum::ui::tokens::dd_ink_5)
            || knobImage.getPixelAt (centreX, 45)
                != juce::Colour (dandrum::ui::tokens::dd_ink_5))
        {
            std::cerr << "native reference knob did not draw its pointer-free 64px cap\n";
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
        if (configuration.instrumentId == "dandrum.advanced-drum-kit")
        {
            const auto replacement = juce::File (juce::String (
                InstrumentDemoConfiguration::tb303().instrumentPath.string()));
            if (! processor.reloadInstrumentFromFile (replacement))
            {
                std::cerr << "native waveform reload fixture failed\n";
                return 1;
            }
            const auto waveformDeadline = juce::Time::getMillisecondCounterHiRes() + 1500.0;
            while (waveform->getName() != "NO PREPARED SAMPLE"
                   && juce::Time::getMillisecondCounterHiRes() < waveformDeadline)
            {
                juce::Thread::sleep (20);
                juce::Timer::callPendingTimersSynchronously();
            }
            if (waveform->getName() != "NO PREPARED SAMPLE")
            {
                std::cerr << "native editor retained a stale sampler waveform after reload\n";
                return 1;
            }
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
    {
        juce::TemporaryFile missing (".yaml");
        auto unavailableConfiguration = fixtureConfiguration;
        unavailableConfiguration.instrumentPath = missing.getFile().getFullPathName().toStdString();
        DandrumAudioProcessor unavailableProcessor (unavailableConfiguration);
        if (unavailableProcessor.isInstrumentLoaded() || unavailableProcessor.getPreparedUiDocument())
            return 1;
        HostGestureListener listener;
        unavailableProcessor.addListener (&listener);
        std::unique_ptr<juce::AudioProcessorEditor> unavailable (unavailableProcessor.createEditor());
        auto* knob = dynamic_cast<juce::Slider*> (unavailable->findChildWithID ("primary-knob-slider"));
        if (knob == nullptr) return 1;
        const auto disabledImage = knob->createComponentSnapshot (knob->getLocalBounds());
        if (knob->isEnabled() || disabledImage.getPixelAt (knob->getWidth() / 2, 57)
                != juce::Colour (dandrum::ui::tokens::dd_ink_4))
        {
            std::cerr << "native reference knob did not draw its unavailable cap state\n";
            return 1;
        }
        knob->setValue (0.75, juce::dontSendNotification);
        doubleClick (*knob);
        wheelOn (*knob, 1.0f, false);
        knob->mouseDown (pointerAt (*knob, 57.0f, juce::ModifierKeys::leftButtonModifier));
        knob->mouseDrag (pointerAt (*knob, 7.0f, juce::ModifierKeys::leftButtonModifier));
        if (knob->keyPressed (juce::KeyPress (juce::KeyPress::upKey))
            || std::abs (knob->getValue() - 0.75) > 0.00001
            || listener.begins != 0 || listener.ends != 0)
        {
            std::cerr << "native unavailable control admitted a key or reset without prepared metadata\n";
            return 1;
        }
        unavailable.reset();
        unavailableProcessor.removeListener (&listener);
    }
    for (const auto& declaredRange : { "default: 0, min: 0, max: 0",
                                      "default: 0, min: -1e100, max: 1e100" })
    {
        juce::TemporaryFile fixedRange (".yaml");
        const auto fixedYaml = juce::File (juce::String (fixtureConfiguration.instrumentPath.string()))
            .loadFileAsString().replace ("default: 0, min: -1, max: 1", declaredRange);
        if (! fixedRange.getFile().replaceWithText (fixedYaml))
            return 1;
        auto fixedConfiguration = fixtureConfiguration;
        fixedConfiguration.instrumentPath = fixedRange.getFile().getFullPathName().toStdString();
        DandrumAudioProcessor fixedProcessor (fixedConfiguration);
        if (! fixedProcessor.isInstrumentLoaded() || ! fixedProcessor.getPreparedUiDocument())
        {
            std::cerr << "native fixed-range fixture did not prepare: " << fixedProcessor.getLastLoadError() << '\n';
            return 1;
        }
        HostGestureListener listener;
        fixedProcessor.addListener (&listener);
        std::unique_ptr<juce::AudioProcessorEditor> fixed (fixedProcessor.createEditor());
        auto* knob = dynamic_cast<juce::Slider*> (fixed->findChildWithID ("primary-knob-slider"));
        if (knob == nullptr || knob->isEnabled()
            || knob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey))
            || knob->keyPressed (juce::KeyPress (juce::KeyPress::upKey)))
        {
            std::cerr << "native zero-width or non-finite actual range admitted an editable knob: "
                      << declaredRange << '\n';
            return 1;
        }
        doubleClick (*knob);
        wheelOn (*knob, 1.0f, false);
        if (listener.begins != 0 || listener.ends != 0)
            return 1;
        fixed.reset();
        fixedProcessor.removeListener (&listener);
    }
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
    auto* fixtureParameter = fixtureProcessor.getParameterForPublicId ("fixture.level");
    if (fixtureParameter == nullptr)
    {
        std::cerr << "reset fixture has no public host parameter\n";
        return 1;
    }
    const auto fixtureSlot = fixtureProcessor.getParameters().indexOf (fixtureParameter);
    doubleClick (*fixtureKnob);
    if (std::abs (fixtureKnob->getValue() - 0.5) > 0.00001
        || std::abs (fixtureParameter->getValue() - 0.5f) > 0.00001f
        || std::abs (fixtureParameter->getDefaultValue()) > 0.00001f
        || fixtureListener.begins != 2 || fixtureListener.ends != 2
        || fixtureListener.lastSlot != fixtureSlot || ! renders (0.0f, 0.0f))
    {
        std::cerr << "native double-click did not reset the loaded control through its stable host slot\n";
        return 1;
    }
    const auto retained = fixtureProcessor.getPreparedUiDocument();
    juce::TemporaryFile replacement (".yaml");
    const auto yaml = juce::File (juce::String (fixtureConfiguration.instrumentPath.string())).loadFileAsString();
    if (! retained || ! yaml.contains ("default: 0, min: -1")
        || ! replacement.getFile().replaceWithText (
            yaml.replace ("default: 0, min: -1", "default: -0.6, min: -1"))
        || ! fixtureProcessor.reloadInstrumentFromFile (replacement.getFile()))
    {
        std::cerr << "reset-default reload fixture did not prepare\n";
        return 1;
    }
    fixtureParameter->setValueNotifyingHost (0.8f);
    const auto deadline = juce::Time::getMillisecondCounterHiRes() + 1500.0;
    while (std::abs (fixtureKnob->getValue() - 0.8) > 0.00001
           && juce::Time::getMillisecondCounterHiRes() < deadline)
    {
        juce::Thread::sleep (20);
        juce::Timer::callPendingTimersSynchronously();
    }
    doubleClick (*fixtureKnob);
    if (fixtureProcessor.getParameterForPublicId ("fixture.level") != fixtureParameter
        || fixtureProcessor.getParameters().indexOf (fixtureParameter) != fixtureSlot
        || std::abs (fixtureKnob->getValue() - 0.2) > 0.00001
        || std::abs (fixtureParameter->getValue() - 0.2f) > 0.00001f
        || std::abs (retained->parameters.front().normalisedDefaultValue - 0.5f) > 0.00001f
        || fixtureListener.begins != 3 || fixtureListener.ends != 3
        || fixtureListener.lastSlot != fixtureSlot || ! renders (-0.6f, 0.0f))
    {
        std::cerr << "native reset retained the previous instrument's default after reload\n";
        return 1;
    }
    fixtureEditor->addToDesktop (juce::ComponentPeer::windowIsTemporary);
    fixtureEditor->setVisible (true);
    fixtureKnob->grabKeyboardFocus();
    auto* readout = dynamic_cast<juce::Button*> (
        findNamedComponent (*fixtureKnob, "primary-knob-readout"));
    if (readout == nullptr || ! readout->isShowing()
        || readout->getButtonText() != "-0.6")
    {
        std::cerr << "native focus did not reveal the prepared actual-value popup\n";
        return 1;
    }
    const auto typeActual = [&] (const juce::String& text, int finishKey)
    {
        if (! fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey)))
            return false;
        auto* input = dynamic_cast<juce::TextEditor*> (
            findNamedComponent (*fixtureKnob, "primary-knob-value-editor"));
        if (input == nullptr || ! input->isShowing())
            return false;
        input->selectAll();
        input->keyPressed (juce::KeyPress (juce::KeyPress::backspaceKey));
        for (const auto character : text)
            input->keyPressed (juce::KeyPress (static_cast<int> (character), {}, character));
        if (input->getText() != text)
            return false;
        input->keyPressed (juce::KeyPress (finishKey));
        juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
        return ! input->isVisible();
    };
    readout->triggerClick();
    juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    if (! typeActual ("0.5", juce::KeyPress::returnKey)
        || std::abs (fixtureParameter->getValue() - 0.75f) > 0.00001f
        || fixtureListener.begins != 4 || fixtureListener.ends != 4
        || fixtureListener.lastSlot != fixtureSlot || ! renders (0.5f, 0.0f)
        || readout->getButtonText() != "0.5")
    {
        std::cerr << "native typed actual value did not reach the stable host slot and signed output\n";
        return 1;
    }
    for (const auto& invalid : { "", "NaN", "0x1", "2", "--0.5", "+-0.5", "0.5 Hz" })
        if (! typeActual (invalid, juce::KeyPress::returnKey)
            || std::abs (fixtureParameter->getValue() - 0.75f) > 0.00001f
            || fixtureListener.begins != 4 || fixtureListener.ends != 4
            || readout->getButtonText() != "0.5")
        {
            std::cerr << "native malformed actual value was not rejected: " << invalid << '\n';
            return 1;
        }
    if (! typeActual ("-0.5", juce::KeyPress::escapeKey)
        || ! typeActual ("0.5", juce::KeyPress::returnKey)
        || std::abs (fixtureParameter->getValue() - 0.75f) > 0.00001f
        || fixtureListener.begins != 4 || fixtureListener.ends != 4)
    {
        std::cerr << "native Escape or unchanged entry rewrote the authoritative value\n";
        return 1;
    }
    if (! typeActual (" +0.5 ", juce::KeyPress::returnKey)
        || std::abs (fixtureParameter->getValue() - 0.75f) > 0.00001f
        || fixtureListener.begins != 5 || fixtureListener.ends != 5)
    {
        std::cerr << "native changed decimal spelling did not make one balanced host gesture\n";
        return 1;
    }
    fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    auto* blurInput = dynamic_cast<juce::TextEditor*> (
        findNamedComponent (*fixtureKnob, "primary-knob-value-editor"));
    if (blurInput == nullptr || ! blurInput->isShowing())
        return 1;
    blurInput->setText (juce::String::fromUTF8 ("−0.5"), false);
    fixtureEditor->setWantsKeyboardFocus (true);
    fixtureEditor->grabKeyboardFocus();
    juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    if (blurInput->isVisible() || ! fixtureEditor->hasKeyboardFocus (false)
        || std::abs (fixtureParameter->getValue() - 0.25f) > 0.00001f
        || fixtureListener.begins != 6 || fixtureListener.ends != 6
        || ! renders (-0.5f, 0.0f))
    {
        std::cerr << "native actual-value blur did not commit and release keyboard focus: value="
                  << fixtureParameter->getValue() << " begins=" << fixtureListener.begins
                  << " ends=" << fixtureListener.ends << " text=" << blurInput->getText()
                  << " visible=" << blurInput->isVisible()
                  << " frameFocus=" << fixtureEditor->hasKeyboardFocus (false) << '\n';
        return 1;
    }
    fixtureParameter->setValueNotifyingHost (0.61234567f);
    const auto preciseDeadline = juce::Time::getMillisecondCounterHiRes() + 1500.0;
    while (readout->getButtonText() != "0.224691"
           && juce::Time::getMillisecondCounterHiRes() < preciseDeadline)
        juce::MessageManager::getInstance()->runDispatchLoopUntil (5);
    if (readout->getButtonText() != "0.224691"
        || ! typeActual (readout->getButtonText(), juce::KeyPress::returnKey)
        || std::abs (fixtureParameter->getValue() - 0.61234567f) > 0.00000001f
        || fixtureListener.begins != 6 || fixtureListener.ends != 6)
    {
        std::cerr << "native rounded readout rewrote a precise host value\n";
        return 1;
    }
    fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    blurInput->setText ("-0.5", false);
    fixtureParameter->setValueNotifyingHost (0.84f);
    juce::MessageManager::getInstance()->runDispatchLoopUntil (40);
    if (blurInput->getText() != "-0.5"
        || std::abs (fixtureKnob->getValue() - 0.61234567) > 0.00001)
    {
        std::cerr << "native host echo overwrote a typed draft: knob=" << fixtureKnob->getValue()
                  << " host=" << fixtureParameter->getValue() << " inputVisible=" << blurInput->isVisible()
                  << " inputFocus=" << blurInput->hasKeyboardFocus (true)
                  << " peerFocus=" << fixtureEditor->getPeer()->isFocused()
                  << " gestures=" << fixtureListener.begins << '/' << fixtureListener.ends << '\n';
        return 1;
    }
    blurInput->keyPressed (juce::KeyPress (juce::KeyPress::escapeKey));
    juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    if (std::abs (fixtureKnob->getValue() - 0.84) > 0.00001
        || fixtureListener.begins != 6 || fixtureListener.ends != 6
        || ! renders (0.68f, 0.0f))
    {
        std::cerr << "native cancelled draft did not recover the latest authoritative value\n";
        return 1;
    }
    fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    blurInput->setText ("-0.5", false);
    if (! fixtureProcessor.reloadInstrumentFromFile (replacement.getFile()))
        return 1;
    juce::MessageManager::getInstance()->runDispatchLoopUntil (40);
    blurInput->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    if (blurInput->isVisible() || fixtureListener.begins != 6 || fixtureListener.ends != 6
        || std::abs (fixtureKnob->getValue() - fixtureParameter->getValue()) > 0.00001
        || fixtureProcessor.getParameterForPublicId ("fixture.level") != fixtureParameter
        || fixtureProcessor.getParameters().indexOf (fixtureParameter) != fixtureSlot)
    {
        std::cerr << "native reload admitted an obsolete typed draft or replaced its host slot\n";
        return 1;
    }
    struct KeyStep { int key; bool fine; float normalised; float signedLeft; };
    fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    blurInput->setText ("-0.5", false);
    wheelOn (*fixtureKnob, 1.0f, false, blurInput);
    if (fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::upKey))
        || blurInput->getText() != "-0.5"
        || fixtureListener.begins != 6 || fixtureListener.ends != 6)
    {
        std::cerr << "native popup interaction escaped into a knob gesture\n";
        return 1;
    }
    blurInput->keyPressed (juce::KeyPress (juce::KeyPress::escapeKey));
    juce::MessageManager::getInstance()->runDispatchLoopUntil (10);
    wheelOn (*fixtureKnob, 1.0f, false, readout);
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 145.0f, juce::ModifierKeys::leftButtonModifier));
    fixtureKnob->mouseDoubleClick (pointerAt (*fixtureKnob, 145.0f, juce::ModifierKeys::leftButtonModifier));
    wheelOn (*fixtureKnob, 1.0f, false, nullptr, 145.0f);
    if (fixtureListener.begins != 6 || fixtureListener.ends != 6
        || std::abs (fixtureParameter->getValue() - 0.84f) > 0.00001f)
    {
        std::cerr << "native popup body admitted a drag, reset or wheel gesture\n";
        return 1;
    }
    const KeyStep keySteps[] {
        { juce::KeyPress::upKey, false, 0.89f, 0.78f },
        { juce::KeyPress::downKey, true, 0.88f, 0.76f },
        { juce::KeyPress::rightKey, false, 0.93f, 0.86f },
        { juce::KeyPress::leftKey, true, 0.92f, 0.84f },
        { juce::KeyPress::homeKey, false, 0.0f, -1.0f },
        { juce::KeyPress::downKey, false, 0.0f, -1.0f },
        { juce::KeyPress::endKey, false, 1.0f, 1.0f },
        { juce::KeyPress::upKey, true, 1.0f, 1.0f },
        { juce::KeyPress::deleteKey, false, 0.2f, -0.6f },
        { juce::KeyPress::backspaceKey, false, 0.2f, -0.6f }
    };
    auto expectedGestures = 6;
    for (const auto& step : keySteps)
    {
        ++expectedGestures;
        const juce::KeyPress key (step.key,
            step.fine ? juce::ModifierKeys::shiftModifier : 0, 0);
        if (! fixtureKnob->keyPressed (key)
            || std::abs (fixtureParameter->getValue() - step.normalised) > 0.00001f
            || std::abs (fixtureKnob->getValue() - step.normalised) > 0.00001
            || fixtureListener.begins != expectedGestures
            || fixtureListener.ends != expectedGestures
            || fixtureListener.lastSlot != fixtureSlot || ! renders (step.signedLeft, 0.0f))
        {
            std::cerr << "native key did not use the reference step and one stable host gesture: "
                      << step.key << '\n';
            return 1;
        }
    }
    doubleClick (*fixtureKnob);
    ++expectedGestures;
    if (fixtureKnob->keyPressed (juce::KeyPress ('z'))
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures)
    {
        std::cerr << "native unchanged double-click reset or unhandled key changed gesture admission\n";
        return 1;
    }
    wheelOn (*fixtureKnob, 1.0f, false);
    ++expectedGestures;
    if (std::abs (fixtureParameter->getValue() - 0.22f) > 0.00001f
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || ! renders (-0.56f, 0.0f))
    {
        std::cerr << "native wheel did not use the reference two-percent step\n";
        return 1;
    }
    wheelOn (*fixtureKnob, -1.0f, true);
    ++expectedGestures;
    wheelOn (*fixtureKnob, 0.0f, false);
    if (std::abs (fixtureParameter->getValue() - 0.21f) > 0.00001f
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || ! renders (-0.58f, 0.0f))
    {
        std::cerr << "native wheel did not use the fine step or ignore zero movement\n";
        return 1;
    }
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 57.0f, juce::ModifierKeys::rightButtonModifier));
    fixtureKnob->mouseDown (pointerAt (*readout, 57.0f, juce::ModifierKeys::leftButtonModifier));
    fixtureKnob->mouseDrag (pointerAt (*fixtureKnob, 7.0f, juce::ModifierKeys::leftButtonModifier));
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 57.0f, juce::ModifierKeys::leftButtonModifier));
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 57.0f, juce::ModifierKeys::leftButtonModifier));
    ++expectedGestures;
    const auto activeImage = fixtureKnob->createComponentSnapshot (fixtureKnob->getLocalBounds());
    fixtureKnob->mouseDrag (pointerAt (*fixtureKnob, 7.0f, juce::ModifierKeys::leftButtonModifier));
    if (activeImage.getPixelAt (fixtureKnob->getWidth() / 2, 57)
            != juce::Colour (dandrum::ui::tokens::dd_ink_6)
        || std::abs (fixtureParameter->getValue() - 0.46f) > 0.00001f
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures - 1
        || ! renders (-0.08f, 0.0f))
    {
        std::cerr << "native actual drag did not use its active cap and 200px sensitivity\n";
        return 1;
    }
    fixtureKnob->mouseDrag (pointerAt (*fixtureKnob, -51.0f, juce::ModifierKeys::leftButtonModifier));
    if (std::abs (fixtureParameter->getValue() - 0.75f) > 0.00001f
        || ! renders (0.5f, 0.0f))
        return 1;
    fixtureParameter->setValueNotifyingHost (0.84f);
    juce::MessageManager::getInstance()->runDispatchLoopUntil (40);
    if (std::abs (fixtureKnob->getValue() - 0.75) > 0.00001)
    {
        std::cerr << "native held drag displayed an incoming automation echo\n";
        return 1;
    }
    fixtureKnob->mouseUp (pointerAt (*fixtureKnob, -51.0f, 0));
    fixtureKnob->mouseUp (pointerAt (*fixtureKnob, -51.0f, 0));
    if (std::abs (fixtureKnob->getValue() - 0.84) > 0.00001
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || ! renders (0.68f, 0.0f))
    {
        std::cerr << "native release did not close once and restore the latest authoritative value\n";
        return 1;
    }
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 57.0f, juce::ModifierKeys::leftButtonModifier));
    ++expectedGestures;
    fixtureKnob->mouseDrag (pointerAt (*fixtureKnob, 257.0f,
        juce::ModifierKeys::leftButtonModifier | juce::ModifierKeys::shiftModifier));
    fixtureKnob->mouseUp (pointerAt (*fixtureKnob, 257.0f, 0));
    if (std::abs (fixtureParameter->getValue() - 0.59f) > 0.00001f
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || ! renders (0.18f, 0.0f))
    {
        std::cerr << "native fine drag did not use 800px sensitivity in one gesture\n";
        return 1;
    }
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 57.0f, juce::ModifierKeys::leftButtonModifier));
    ++expectedGestures;
    fixtureEditor->grabKeyboardFocus();
    if (fixtureListener.ends != expectedGestures)
    {
        std::cerr << "native focus loss did not close its captured host gesture immediately\n";
        return 1;
    }
    fixtureKnob->mouseUp (pointerAt (*fixtureKnob, 57.0f, 0));
    if (fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures)
    {
        std::cerr << "native focus loss left a captured gesture open or closed it twice\n";
        return 1;
    }
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 57.0f, juce::ModifierKeys::leftButtonModifier));
    ++expectedGestures;
    if (! fixtureProcessor.reloadInstrumentFromFile (replacement.getFile()))
        return 1;
    const auto dragReloadDeadline = juce::Time::getMillisecondCounterHiRes() + 1500.0;
    while (fixtureListener.ends != expectedGestures
           && juce::Time::getMillisecondCounterHiRes() < dragReloadDeadline)
        juce::MessageManager::getInstance()->runDispatchLoopUntil (5);
    fixtureKnob->mouseUp (pointerAt (*fixtureKnob, 57.0f, 0));
    if (fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || fixtureProcessor.getParameterForPublicId ("fixture.level") != fixtureParameter
        || std::abs (fixtureKnob->getValue() - fixtureParameter->getValue()) > 0.00001)
    {
        std::cerr << "native reload failed to cancel a drag while preserving its host binding\n";
        return 1;
    }
    fixtureEditor->grabKeyboardFocus();
    fixtureKnob->mouseEnter (pointerAt (*fixtureKnob, 57.0f, 0));
    const auto hoverImage = fixtureKnob->createComponentSnapshot (fixtureKnob->getLocalBounds());
    if (! readout->isShowing() || hoverImage.getPixelAt (fixtureKnob->getWidth() / 2, 57)
            != juce::Colour (dandrum::ui::tokens::dd_cap_hover))
    {
        std::cerr << "native reference hover did not reveal its popup and cap state\n";
        return 1;
    }
    fixtureKnob->mouseExit (pointerAt (*fixtureKnob, 57.0f, 0));
    fixtureKnob->mouseEnter (pointerAt (*readout, 12.0f, 0));
    juce::MessageManager::getInstance()->runDispatchLoopUntil (300);
    if (! readout->isShowing())
    {
        std::cerr << "native popup disappeared while the pointer was over its readout\n";
        return 1;
    }
    const auto hoverDeadline = juce::Time::getMillisecondCounterHiRes() + 400.0;
    fixtureKnob->mouseExit (pointerAt (*readout, 12.0f, 0));
    juce::MessageManager::getInstance()->runDispatchLoopUntil (180);
    if (! readout->isShowing())
    {
        std::cerr << "native popup did not retain its pointer handoff grace period\n";
        return 1;
    }
    while (readout->isShowing() && juce::Time::getMillisecondCounterHiRes() < hoverDeadline)
        juce::MessageManager::getInstance()->runDispatchLoopUntil (5);
    if (readout->isShowing())
    {
        std::cerr << "native hover popup did not expire without focus\n";
        return 1;
    }
    const auto nudgeDeadline = juce::Time::getMillisecondCounterHiRes() + 750.0;
    wheelOn (*fixtureKnob, 1.0f, false);
    ++expectedGestures;
    const auto nudgeImage = fixtureKnob->createComponentSnapshot (fixtureKnob->getLocalBounds());
    juce::MessageManager::getInstance()->runDispatchLoopUntil (500);
    if (! readout->isShowing()
        || nudgeImage.getPixelAt (fixtureKnob->getWidth() / 2, 32)
            != juce::Colour (dandrum::ui::tokens::color_value)
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || ! renders (0.22f, 0.0f))
    {
        std::cerr << "native nudge did not hold its popup and active value arc\n";
        return 1;
    }
    while (readout->isShowing() && juce::Time::getMillisecondCounterHiRes() < nudgeDeadline)
        juce::MessageManager::getInstance()->runDispatchLoopUntil (5);
    const auto restingImage = fixtureKnob->createComponentSnapshot (fixtureKnob->getLocalBounds());
    if (readout->isShowing() || restingImage.getPixelAt (fixtureKnob->getWidth() / 2, 32)
            == juce::Colour (dandrum::ui::tokens::color_value))
    {
        std::cerr << "native nudge popup or thick value arc did not expire\n";
        return 1;
    }
    fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    blurInput->setText ("-0.5", false);
    fixtureEditor->grabKeyboardFocus();
    ++expectedGestures;
    if (blurInput->isVisible() || std::abs (fixtureParameter->getValue() - 0.25f) > 0.00001f
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || ! renders (-0.5f, 0.0f))
    {
        std::cerr << "native blur left its old entry active before immediate reopening\n";
        return 1;
    }
    fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    blurInput->setText ("0.5", false);
    juce::MessageManager::getInstance()->runDispatchLoopUntil (20);
    if (! blurInput->isShowing() || blurInput->getText() != "0.5"
        || std::abs (fixtureParameter->getValue() - 0.25f) > 0.00001f
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures)
    {
        std::cerr << "native previous blur completed a newer entry\n";
        return 1;
    }
    blurInput->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
    ++expectedGestures;
    if (blurInput->isVisible() || std::abs (fixtureParameter->getValue() - 0.75f) > 0.00001f
        || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures
        || ! renders (0.5f, 0.0f))
    {
        std::cerr << "native Enter did not finish its own entry before reopening\n";
        return 1;
    }
    for (const auto& previous : { "Enter", "Escape" })
    {
        fixtureKnob->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
        blurInput->setText ("-0.5", false);
        juce::MessageManager::getInstance()->runDispatchLoopUntil (20);
        if (! blurInput->isShowing() || blurInput->getText() != "-0.5"
            || std::abs (fixtureParameter->getValue() - 0.75f) > 0.00001f
            || fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures)
        {
            std::cerr << "native previous " << previous << " completed a newer entry\n";
            return 1;
        }
        blurInput->keyPressed (juce::KeyPress (juce::KeyPress::escapeKey));
        if (blurInput->isVisible())
        {
            std::cerr << "native Escape left its old entry active before reopening\n";
            return 1;
        }
    }
    fixtureKnob->mouseDown (pointerAt (*fixtureKnob, 57.0f, juce::ModifierKeys::leftButtonModifier));
    ++expectedGestures;
    fixtureEditor.reset();
    if (fixtureListener.begins != expectedGestures || fixtureListener.ends != expectedGestures)
    {
        std::cerr << "native editor closure left an actual drag gesture open\n";
        return 1;
    }
    fixtureProcessor.removeListener (&fixtureListener);
    for (const auto& mode : { "begin", "change", "end", "typed", "focus", "reload" })
    {
        fixtureParameter->setValueNotifyingHost (0.5f);
        std::unique_ptr<juce::AudioProcessorEditor> closing (fixtureProcessor.createEditor());
        closing->addToDesktop (juce::ComponentPeer::windowIsTemporary);
        closing->setVisible (true);
        closing->setWantsKeyboardFocus (true);
        auto* control = dynamic_cast<juce::Slider*> (
            findNamedComponent (*closing, "primary-knob-slider"));
        if (control == nullptr)
            return 1;
        const juce::String name (mode);
        const auto notification = name == "begin" ? ClosingHostListener::Notification::begin
            : name == "change" || name == "typed" ? ClosingHostListener::Notification::change
            : ClosingHostListener::Notification::end;
        ClosingHostListener listener (fixtureProcessor, closing, notification, fixtureSlot);
        if (name == "typed")
        {
            control->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
            auto* input = dynamic_cast<juce::TextEditor*> (
                findNamedComponent (*control, "primary-knob-value-editor"));
            if (input == nullptr || ! input->isShowing())
                return 1;
            input->setText ("0.5", false);
            input->keyPressed (juce::KeyPress (juce::KeyPress::returnKey));
        }
        else
        {
            control->mouseDown (pointerAt (*control, 57.0f, juce::ModifierKeys::leftButtonModifier));
            if (closing)
            {
                control->mouseDrag (pointerAt (*control, 7.0f, juce::ModifierKeys::leftButtonModifier));
                if (name == "focus")
                    closing->grabKeyboardFocus();
                else if (name == "reload")
                {
                    if (! fixtureProcessor.reloadInstrumentFromFile (
                            juce::File (juce::String (fixtureConfiguration.instrumentPath.string()))))
                        return 1;
                    juce::MessageManager::getInstance()->runDispatchLoopUntil (40);
                    juce::Timer::callPendingTimersSynchronously();
                }
                else if (closing)
                    control->mouseUp (pointerAt (*control, 7.0f, 0));
            }
        }
        const float expected = name == "begin" ? 0.5f : 0.75f;
        // Reload republishes the carried value on the same host parameter object.
        const auto expectedChanges = name == "begin" ? 0 : name == "reload" ? 2 : 1;
        if (closing || listener.begins != 1 || listener.ends != 1
            || listener.changes != expectedChanges || ! listener.allSlotsMatch
            || fixtureProcessor.getParameterForPublicId ("fixture.level") != fixtureParameter
            || fixtureProcessor.getParameters().indexOf (fixtureParameter) != fixtureSlot
            || std::abs (fixtureParameter->getValue() - expected) > 0.00001f
            || ! renders (name == "begin" ? 0.0f : 0.5f, 0.0f))
        {
            std::cerr << "native host notification closure did not balance the original gesture: " << name
                      << " begins=" << listener.begins << " changes=" << listener.changes
                      << " ends=" << listener.ends << " editor=" << (closing != nullptr) << '\n';
            return 1;
        }
        closing.reset (fixtureProcessor.createEditor());
        auto* reopened = dynamic_cast<juce::Slider*> (
            findNamedComponent (*closing, "primary-knob-slider"));
        if (reopened == nullptr || ! reopened->keyPressed (juce::KeyPress (juce::KeyPress::homeKey))
            || listener.begins != 2 || listener.ends != 2 || ! listener.allSlotsMatch
            || ! renders (-1.0f, 0.0f))
        {
            std::cerr << "native replacement editor could not gesture after host notification closure: " << name << '\n';
            return 1;
        }
    }
    return 0;
}
