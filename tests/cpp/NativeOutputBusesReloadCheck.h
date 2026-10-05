#pragma once

#include <stdexcept>

// Linux's test-only linker wrapper schedules a real activation immediately
// after the editor has copied a prepared document. No production test hook.
namespace nativeOutputReloadCheck
{
inline int readsUntilAction = 0;
inline std::uint32_t copiedGeneration = 0;
inline bool reloadAccepted = false;
inline bool unavailableCopy = false;
}

extern "C" std::optional<InstrumentUiDocument>
__real__ZNK21DandrumAudioProcessor21getPreparedUiDocumentEv (const DandrumAudioProcessor*);
extern "C" std::optional<InstrumentUiDocument>
__wrap__ZNK21DandrumAudioProcessor21getPreparedUiDocumentEv (const DandrumAudioProcessor* processor)
{
    auto document = __real__ZNK21DandrumAudioProcessor21getPreparedUiDocumentEv (processor);
    if (nativeOutputReloadCheck::readsUntilAction > 0
        && --nativeOutputReloadCheck::readsUntilAction == 0)
    {
        if (nativeOutputReloadCheck::unavailableCopy) return std::nullopt;
        nativeOutputReloadCheck::copiedGeneration = document ? document->generation : 0;
        nativeOutputReloadCheck::reloadAccepted = const_cast<DandrumAudioProcessor*> (processor)
            ->reloadInstrumentFromFile (juce::File (juce::String (
                InstrumentDemoConfiguration::sampler().instrumentPath.string())));
    }
    return document;
}

struct PluginConstructionTestProbe
{
    static bool meterVisible (const DandrumAudioProcessor& processor)
    { return processor.meterDelivery.visibleCount() != 0; }
};

namespace nativeOutputReloadCheck
{
inline juce::Component* find (juce::Component& owner, const juce::String& id)
{
    if (owner.getComponentID() == id) return &owner;
    for (auto* child : owner.getChildren())
        if (auto* found = find (*child, id)) return found;
    return nullptr;
}

inline void check (bool valid, const char* reason)
{
    if (! valid) throw std::runtime_error (reason);
}

template <typename Predicate>
void await (Predicate ready, const char* reason)
{
    const auto deadline = juce::Time::getMillisecondCounterHiRes() + 2000.0;
    while (! ready() && juce::Time::getMillisecondCounterHiRes() < deadline)
        juce::MessageManager::getInstance()->runDispatchLoopUntil (5);
    check (ready(), reason);
}

inline int run()
{
    try
    {
        // Read 1 constructs output rows; read 2 obtains the knob descriptor;
        // read 3 admits the meter snapshot. This linker seam keeps the real
        // processor/editor and schedules activation at either meter boundary.
        struct Scenario { int activationAfterRead; bool unavailableCopy; };
        for (const auto scenario : { Scenario { 0, false }, Scenario { 1, false },
                                     Scenario { 3, false }, Scenario { 3, true } })
        {
            DandrumAudioProcessor processor (InstrumentDemoConfiguration::sampler());
            processor.setPlayConfigDetails (0, 2, 48000.0, 64);
            processor.prepareToPlay (48000.0, 64);
            const auto originalGeneration = processor.getParameterSurfaceGeneration();
            copiedGeneration = 0; reloadAccepted = false;
            readsUntilAction = scenario.activationAfterRead;
            unavailableCopy = scenario.unavailableCopy;
            std::unique_ptr<juce::AudioProcessorEditor> editor (processor.createEditor());
            check (editor != nullptr, "reload interleaving must open the real native editor");
            editor->addToDesktop (juce::ComponentPeer::windowIsTemporary);
            editor->setVisible (true);
            if (scenario.unavailableCopy)
            {
                auto* unavailable = find (*editor, "output-bindings-unavailable");
                check (unavailable && unavailable->isVisible()
                    && find (*editor, "output:0-channel:0") == nullptr,
                    "unavailable prepared snapshot must clear output rows and report missing bindings");
            }
            else if (scenario.activationAfterRead != 0)
                check (reloadAccepted && copiedGeneration == originalGeneration
                    && processor.getParameterSurfaceGeneration() == originalGeneration + 1,
                    "reload interleaving must activate a real new generation after copying the old document");
            await ([&] { return PluginConstructionTestProbe::meterVisible (processor); },
                   "original native meter session must become visible without a second subscriber");
            const auto generation = processor.getParameterSurfaceGeneration();
            auto* reading = find (*editor, "output:0-channel:0");
            check (reading && reading->getName().contains ("generation " + juce::String (generation)),
                   "editor construction crossing a reload must retire old output rows");

            juce::AudioBuffer<float> audio (2, 64);
            juce::MidiBuffer midi;
            midi.addEvent (juce::MidiMessage::noteOn (1, 36, static_cast<juce::uint8> (96)), 0);
            processor.processBlock (audio, midi);
            check (std::abs (audio.getSample (0, 0) + 0.5f) < 0.00001f
                && std::abs (audio.getSample (1, 0) + 0.5f) < 0.00001f,
                "reloaded sampler must still render the known signed -0.5/-0.5 kick");
            await ([&]
            {
                auto* current = find (*editor, "output:0-channel:0");
                return current && current->getName().contains ("peak")
                    && current->getName().contains ("RMS")
                    && current->getName().contains ("generation " + juce::String (generation));
            }, "original native timer must show real audio measurements in the current generation after construction/reload");
            std::cout << "Native output startup: action after document " << scenario.activationAfterRead
                      << ", unavailable copy " << scenario.unavailableCopy
                      << ", generation " << generation << ", signed -0.5/-0.5, "
                      << find (*editor, "output:0-channel:0")->getName() << '\n';
        }
        return 0;
    }
    catch (const std::exception& error)
    {
        readsUntilAction = 0;
        std::cerr << error.what() << '\n';
        return 1;
    }
}
}
