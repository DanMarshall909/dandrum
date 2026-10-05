#pragma once

#include "NativeOutputBuses.h"
#include "PluginProcessor.h"
#include "DesignTokens.h"

#include <cmath>
#include <iostream>
#include <memory>

inline juce::Component* findOutputComponent (juce::Component& owner, const juce::String& id)
{
    if (owner.getComponentID() == id) return &owner;
    for (int index = 0; index < owner.getNumChildComponents(); ++index)
        if (auto* result = findOutputComponent (*owner.getChildComponent (index), id))
            return result;
    return nullptr;
}

inline int runNativeMeterCharacterization()
{
    auto config = InstrumentDemoConfiguration::kick();
    config.instrumentPath = juce::File (juce::String (DANDRUM_SOURCE_ROOT))
        .getChildFile ("tests/fixtures/plugin-ui-knob.yaml").getFullPathName().toStdString();
    DandrumAudioProcessor processor (config);
    processor.setPlayConfigDetails (0, 2, 48000.0, 64);
    processor.prepareToPlay (48000.0, 64);
    const auto session = processor.uiCommands().createSession();
    processor.subscribeMeter (session, processor.getParameterSurfaceGeneration());
    processor.setMeterSessionVisible (session, true);
    std::unique_ptr<juce::AudioProcessorEditor> editor (processor.createEditor());
    auto* meter = dynamic_cast<NativeOutputBuses*> (editor->findChildWithID ("output-buses"));
    auto* left = dynamic_cast<juce::TextButton*> (findOutputComponent (*meter, "clip-left"));
    auto* right = dynamic_cast<juce::TextButton*> (findOutputComponent (*meter, "clip-right"));
    left->onClick(); // No measured clip: inert.
    auto* level = processor.getParameterForPublicId ("fixture.level");
    level->setValueNotifyingHost (1.0f);
    juce::AudioBuffer<float> audio (2, 64);
    juce::MidiBuffer midi;
    processor.processBlock (audio, midi);
    for (int frame = 0; frame < 64; ++frame)
        if (std::abs (audio.getSample (0, frame) - 1.0f) > 0.00001f
            || std::abs (audio.getSample (1, frame)) > 0.00001f)
        { std::cerr << "native meter fixture must render signed +1/0 PCM\n"; return 1; }
    InstrumentUiMeterDelivery::Packet packet;
    packet.display.valid = packet.display.complete = true;
    packet.display.generation = processor.getParameterSurfaceGeneration();
    packet.display.peak = packet.display.rms = { 1.0, 0.0 };
    packet.clip = processor.getMeterClipSnapshot();
    meter->setPacket (packet);
    editor->createComponentSnapshot (editor->getLocalBounds());
    if (! left->isEnabled() || right->isEnabled())
    { std::cerr << "native measured clip must latch only left\n"; return 1; }
    left->onClick();
    if (left->isEnabled() || processor.getMeterClipSnapshot().latched[0])
    { std::cerr << "native clip acknowledgement must clear the measured left ticket\n"; return 1; }
    meter->setPacket (packet); // The acknowledged ticket is now obsolete.
    left->onClick();
    if (! left->isEnabled())
    { std::cerr << "native rejected clip acknowledgement must retain its latch\n"; return 1; }
    meter->clear();
    editor->setSize (1200, 800);
    editor->createComponentSnapshot (editor->getLocalBounds());
    if (left->isEnabled() || right->isEnabled())
    { std::cerr << "native clearing must retire all clip actions\n"; return 1; }
    processor.unsubscribeMeter (session);
    processor.uiCommands().closeSession (session);
    return 0;
}

inline int runNativeOutputBusesCheck()
{
    DandrumAudioProcessor processor (InstrumentDemoConfiguration::sampler());
    processor.setPlayConfigDetails (0, 2, 48000.0, 64);
    processor.prepareToPlay (48000.0, 64);
    std::unique_ptr<juce::AudioProcessorEditor> editor (processor.createEditor());
    editor->addToDesktop (juce::ComponentPeer::windowIsTemporary);
    editor->setVisible (true);
    editor->toFront (true);
    juce::MessageManager::getInstance()->runDispatchLoopUntil (20);
    auto* name = dynamic_cast<juce::Label*> (findOutputComponent (*editor, "output:0-name"));
    auto* channels = dynamic_cast<juce::Label*> (findOutputComponent (*editor, "output:0-channels"));
    if (name == nullptr || channels == nullptr || name->getText() != "Output"
        || channels->getText() != "2 channels · L/R")
    { std::cerr << "native output view must enumerate actual named Output bus and L/R channels\n"; return 1; }
    auto* panel = dynamic_cast<NativeOutputBuses*> (editor->findChildWithID ("output-buses"));
    const auto label = [&] (const juce::String& id) -> juce::Label*
    { return dynamic_cast<juce::Label*> (findOutputComponent (*panel, id)); };
    if (label ("output:0-feeds")->getText() != "Feed details unavailable"
        || label ("output:0-status")->getText() != "WAITING FOR AUDIO"
        || name->getFont().getTypefacePtr()->getName() != "Barlow Semi Condensed"
        || channels->getFont().getTypefacePtr()->getName() != "JetBrains Mono")
    { std::cerr << "native prepared bus must distinguish missing feeds and waiting audio with supplied fonts\n"; return 1; }

    // Owned metadata, including layouts not exposed by this stereo-only example.
    // Actual JUCE layout capture is calibrated separately by cxx-output-bus-metadata.
    InstrumentUiDocument document;
    document.generation = processor.getParameterSurfaceGeneration();
    document.outputBuses = {
        { "mono", "Centre", { "C" }, true, {} },
        { "aux", "Auxiliary", { "L", "R" }, false, "master" },
        { "unbound", "Unmeasured stereo", { "L", "R" }, false, {} },
        { "surround", "Surround", { "L", "R", "C", "Lfe", "Ls", "Rs" }, false, {} },
        { "disabled", "Parked", {}, false, {} } };
    panel->setDocument (document);
    document.outputBuses.clear(); // Display may not borrow the caller's storage.
    if (label ("mono-name")->getText() != "Centre"
        || label ("mono-channels")->getText() != "1 channel · C"
        || label ("surround-channels")->getText() != "6 channels · L/R/C/Lfe/Ls/Rs"
        || label ("disabled-channels")->getText() != "Disabled"
        || label ("mono-status")->getText() != "Measurements unavailable"
        || label ("aux-status")->getText() != "WAITING FOR AUDIO"
        || label ("unbound-status")->getText() != "Measurements unavailable"
        || ! label ("mono-main")->isVisible() || label ("aux-main")->isVisible()
        || findOutputComponent (*panel, "surround-channel:5") == nullptr)
    { std::cerr << "native output view must preserve owned mono/surround/disabled channel facts and explicit tap availability\n"; return 1; }

    InstrumentUiMeterDelivery::Packet packet;
    packet.display.valid = packet.display.complete = true;
    packet.display.generation = document.generation;
    packet.display.peak = { 0.5, 0.25 }; packet.display.rms = { 0.4, 0.125 };
    packet.clip.valid = true; packet.clip.generation = document.generation;
    packet.clip.latched = { true, true }; packet.clip.ticket = { 73, 91 };
    panel->setPacket (packet);
    auto* measured = findOutputComponent (*panel, "aux-channel:0");
    auto* measuredRight = findOutputComponent (*panel, "aux-channel:1");
    if (label ("aux-status")->getText() != "LIVE"
        || ! measured->getName().contains ("master")
        || ! measured->getName().contains ("generation " + juce::String (document.generation))
        || ! measured->getName().contains ("peak 0.500")
        || ! measured->getName().contains ("RMS 0.400")
        || ! measuredRight->getName().contains ("peak 0.250")
        || ! measuredRight->getName().contains ("RMS 0.125")
        || label ("mono-status")->getText() != "Measurements unavailable"
        || label ("unbound-status")->getText() != "Measurements unavailable")
    { std::cerr << "native measurements must identify the explicit tap/channel/generation and independent peak/RMS\n"; return 1; }
    const auto assertBars = [&] ()
    {
        const auto image = measured->createComponentSnapshot (measured->getLocalBounds());
        return image.getPixelAt (35, 10) == juce::Colour (dandrum::ui::tokens::dd_vermilion_lo)
            && image.getPixelAt (35, 11) == juce::Colour (dandrum::ui::tokens::dd_vermilion_hi)
            && image.getPixelAt (100, 10) == juce::Colour (dandrum::ui::tokens::surface_well);
    };
    for (const auto& size : { juce::Point<int> (1200, 800), juce::Point<int> (820, 560) })
    {
        editor->setSize (size.x, size.y);
        if (label ("aux-channels")->getWidth() < 84 || ! assertBars())
        { std::cerr << "native named bus layout must retain channel summaries and literal 120x4 peak/RMS bars at both sizes\n"; return 1; }
        auto* clip = dynamic_cast<juce::Button*> (findOutputComponent (*panel, "clip-left"));
        if (clip == nullptr || clip->getWidth() < 24 || clip->getHeight() < 24
            || measured->getWidth() < 182 || measured->getHeight() < 24)
        { std::cerr << "native measured channel must retain a 24-pixel clip target\n"; return 1; }
        const auto feedsBounds = label ("aux-feeds")->getBounds();
        if (size.x == 820 && measured->getY() <= feedsBounds.getBottom())
        { std::cerr << "compact native bus measurements must wrap below feed details\n"; return 1; }
        auto* row = findOutputComponent (*panel, "aux");
        auto* rightClip = findOutputComponent (*panel, "clip-right");
        if (! row->getLocalBounds().reduced (10).contains (
                row->getLocalArea (rightClip, rightClip->getLocalBounds())))
        { std::cerr << "native channel clip target must stay inside the padded bus row\n"; return 1; }
        if (! row->getWantsKeyboardFocus())
        { std::cerr << "native output inspection row must accept keyboard focus\n"; return 1; }
        row->grabKeyboardFocus();
        if (! row->hasKeyboardFocus (false))
        { std::cerr << "native output inspection row must receive real keyboard focus\n"; return 1; }
        auto* viewport = row->findParentComponentOfClass<juce::Viewport>();
        if (row->getY() < viewport->getViewPositionY()
            || row->getBottom() > viewport->getViewPositionY() + viewport->getViewHeight())
        { std::cerr << "native keyboard focus must scroll the inspected bus row into view\n"; return 1; }
        const auto focusImage = row->createComponentSnapshot (row->getLocalBounds());
        if (focusImage.getPixelAt (20, 0) != juce::Colour (dandrum::ui::tokens::color_focus))
        { std::cerr << "native output inspection must paint a cream focus outline\n"; return 1; }
        editor->createComponentSnapshot (editor->getLocalBounds());
        findOutputComponent (*panel, "mono")->grabKeyboardFocus();
    }
    packet.display.complete = false;
    panel->setPacket (packet);
    if (label ("aux-status")->getText() != "HISTORY GAP")
    { std::cerr << "native incomplete readings must disclose a history gap\n"; return 1; }
    packet.display.peak = packet.display.rms = { 0.0, 0.0 };
    panel->setPacket (packet);
    if (! measured->getName().contains ("peak 0.000"))
    { std::cerr << "native measured silence must remain a measured value\n"; return 1; }
    document.generation += 1;
    document.outputBuses = { { "new", "Replacement", { "L", "R" }, true, "master" } };
    panel->setDocument (document);
    panel->setPacket (packet);
    if (label ("new-status")->getText() != "WAITING FOR AUDIO"
        || findOutputComponent (*panel, "aux") != nullptr
        || findOutputComponent (*panel, "new-channel:0")->getName().contains ("peak"))
    { std::cerr << "native generation replacement must retire readings and reject old packets\n"; return 1; }
    panel->clear();
    document.outputBuses.clear(); panel->setDocument (document);
    if (label ("output-bindings-unavailable")->getText() != "Output bindings unavailable")
    { std::cerr << "native empty bindings must explicitly report unavailable outputs\n"; return 1; }
    return 0;
}
