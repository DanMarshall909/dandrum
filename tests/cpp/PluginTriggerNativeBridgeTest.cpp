#include "InstrumentHostWebBridge.h"

#include <chrono>
#include <atomic>
#include <cmath>
#include <iostream>
#include <stdexcept>
#include <thread>
#include "PluginTriggerWebRuntimeTest.h"

namespace
{
void require (bool condition, const char* message)
{
    if (! condition) throw std::runtime_error (message);
}

juce::var call (InstrumentHostWebBridge& bridge, const char* name,
                juce::Array<juce::var> args = {})
{
    for (const auto& entry : bridge.nativeFunctions())
        if (entry.first.toString() == name)
        {
            juce::var reply;
            std::atomic<bool> done { false };
            entry.second (args, [&] (juce::var value) { reply = value; done.store (true, std::memory_order_release); });
            const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds (10);
            while (! done.load (std::memory_order_acquire) && std::chrono::steady_clock::now() < deadline)
                std::this_thread::sleep_for (std::chrono::milliseconds (1));
            require (done.load (std::memory_order_acquire), "native bridge function did not complete");
            return reply;
        }
    throw std::runtime_error (std::string ("Missing native function: ") + name);
}

juce::var waitJob (InstrumentHostWebBridge& bridge, const juce::var& ticket,
                   const char* method = "getUiJobStatus")
{
    require (ticket["status"].toString() == "accepted", "preparation was not admitted");
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds (10);
    do
    {
        auto status = call (bridge, method, { ticket["job_id"] });
        if (status["state"].toString() != "running") return status;
        std::this_thread::sleep_for (std::chrono::milliseconds (2));
    } while (std::chrono::steady_clock::now() < deadline);
    throw std::runtime_error ("preparation timed out");
}

float hit (DandrumAudioProcessor& processor, int note)
{
    juce::AudioBuffer<float> buffer (2, 16);
    // Let previous one-shot voices finish before checking a fresh onset.
    juce::MidiBuffer empty;
    for (int block = 0; block < 4000; ++block)
    {
        buffer.clear();
        processor.processBlock (buffer, empty);
    }
    buffer.clear();
    juce::MidiBuffer midi;
    midi.addEvent (juce::MidiMessage::noteOn (1, note, juce::uint8 (100)), 0);
    processor.processBlock (buffer, midi);
    return buffer.getSample (0, 0);
}
}

int main (int argc, char** argv)
{
    if (argc > 1 && (juce::String (argv[1]) == "--web-runtime"
        || juce::String (argv[1]) == "--juce-gtkwebkitfork-child"))
        return runTriggerWebRuntime (argc, argv);
    const juce::ScopedJuceInitialiser_GUI gui;
    try
    {
        DandrumAudioProcessor processor (InstrumentDemoConfiguration::sampler());
        processor.setPlayConfigDetails (0, 2, 48000, 16);
        processor.prepareToPlay (48000, 16);
        InstrumentHostWebBridge bridge (processor);
        const auto session = call (bridge, "getSamplerSession");
        require (session["document"]["sources"].size() == 1, "prepared sources missing");
        require (juce::File::isAbsolutePath (session["sourcePath"].toString()), "source provenance missing");
        const auto generation = session["document"]["generation"];
        const auto snapshotPath = juce::File::getSpecialLocation (juce::File::tempDirectory)
            .getChildFile ("dandrum-trigger-real-session.json");
        snapshotPath.replaceWithText (juce::JSON::toString (session));
        const auto original = session["yaml"].toString();
        require (original.contains ("sample_map_player"), "native YAML missing");
        juce::Array<juce::var> selected;
        selected.add (juce::File (session["sourcePath"].toString()).getSiblingFile ("assets/advanced-drums.wav").getFullPathName());
        const auto imported = call (bridge, "importSamplerFiles", { juce::var (selected), generation });
        require (imported.size() == 1 && static_cast<int> (imported[0]["sampleRate"]) == 48000,
                 "native import did not decode actual metadata");
        const juce::File retained (imported[0]["uri"].toString());
        require (retained.existsAsFile() && retained.isAChildOf (processor.currentInstrumentFile().getParentDirectory()),
                 "native import escaped the authoring root");
        // Only this test's uniquely owned copy is removed; source/user caches survive.
        require (retained.getParentDirectory().deleteRecursively(), "test-owned import cache cleanup failed");
        require (std::abs (hit (processor, 36) + 0.5f) < 0.0001f, "known signed sample incorrect");
        const auto parameterReply = call (bridge, "setParameter",
            { juce::var ("drums.level"), juce::var (0.125), generation });
        require (parameterReply["status"].toString() == "accepted", "representative knob command rejected");
        require (std::abs (hit (processor, 36) + 0.25f) < 0.0001f, "bridge knob did not change actual sample level");
        call (bridge, "setParameter", { juce::var ("drums.level"), juce::var (0.25), generation });
        const auto waveform = waitJob (bridge, call (bridge, "requestSourceWaveform",
            { juce::var ("drums"), juce::var (0), juce::var (32), generation }), "getWaveformJobStatus");
        require (waveform["state"].toString() == "ready", "whole-source waveform failed");
        require (waveform["result"]["startFrame"].toString() == "0"
            && waveform["result"]["endFrame"].toString() == "51000", "whole-source waveform used cropped region geometry");
        auto edited = original.replace ("key_range: [36, 36]", "key_range: [60, 60]");
        const auto success = waitJob (bridge, call (bridge, "prepareSampler", { edited, generation }));
        require (success["state"].toString() == "completed", "owned YAML replacement failed");
        require (std::abs (hit (processor, 60) + 0.5f) < 0.0001f, "accepted mapping did not render actual sample");
        const auto activeGeneration = processor.getParameterSurfaceGeneration();
        const auto rejected = call (bridge, "prepareSampler", { original, generation });
        require (rejected.isString(), "stale generation accepted");
        const auto failed = waitJob (bridge, call (bridge, "prepareSampler",
            { juce::var ("invalid: ["), juce::var (static_cast<juce::int64> (activeGeneration)) }));
        require (failed["state"].toString() == "failed", "invalid YAML accepted");
        require (processor.getParameterSurfaceGeneration() == activeGeneration, "failure changed generation");
        require (std::abs (hit (processor, 60) + 0.5f) < 0.0001f, "failed edit lost working sample");
        const auto undo = waitJob (bridge, call (bridge, "prepareSampler",
            { original, static_cast<juce::int64> (activeGeneration) }));
        require (undo["state"].toString() == "completed", "inverse edit failed");
        require (std::abs (hit (processor, 36) + 0.5f) < 0.0001f, "inverse edit did not restore sample map");
        for (int index = 1; index < argc; ++index)
        {
            const auto file = juce::File (juce::String (argv[index]));
            const auto compiled = waitJob (bridge, call (bridge, "prepareSampler",
                { file.loadFileAsString(), juce::var (static_cast<juce::int64> (processor.getParameterSurfaceGeneration())) }));
            require (compiled["state"].toString() == "completed", "React compiler graph failed native preparation");
            const bool empty = file.getFileName().contains ("empty");
            const int note = file.getFileName().contains ("edited") ? 40 : 36;
            require (std::abs (hit (processor, note) - (empty ? 0.0f : -0.5f)) < 0.0001f,
                     "React compiler graph did not produce expected signed sample output");
        }
        std::cout << "PASS: native shell session, signed playback, edit/inverse, stale and failure preservation\n";
        return 0;
    }
    catch (const std::exception& error)
    {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
