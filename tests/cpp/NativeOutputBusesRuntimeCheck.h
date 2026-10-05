#pragma once

#include "NativeOutputBuses.h"
#include "DesignTokens.h"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdlib>
#include <memory>
#include <stdexcept>

namespace outputBusRuntimeCheck
{
inline juce::Component* find (juce::Component& owner, const juce::String& id)
{
    if (owner.getComponentID() == id) return &owner;
    for (auto* child : owner.getChildren())
        if (auto* found = find (*child, id)) return found;
    return nullptr;
}

inline void check (bool valid, const juce::String& reason)
{
    if (! valid) throw std::runtime_error (reason.toStdString());
}

class Application final : public juce::JUCEApplication, private juce::Timer
{
public:
    const juce::String getApplicationName() override { return "Native Output Buses Runtime"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise (const juce::String&) override
    {
        try
        {
            processor.reset (dynamic_cast<DandrumAudioProcessor*> (createPluginFilter()));
            check (processor && processor->demoConfiguration().instrumentId == "dandrum.advanced-drum-kit",
                   "native output runtime must start the original sampler factory");
            processor->setPlayConfigDetails (0, 2, 48000.0, 64);
            processor->prepareToPlay (48000.0, 64);
            generation = processor->getParameterSurfaceGeneration();
            editor.reset (processor->createEditor());
            check (editor != nullptr, "original sampler factory must open its native editor");
            editor->setSize (1200, 800);
            editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
            editor->setVisible (true);
            deadline = juce::Time::getMillisecondCounterHiRes() + 5000.0;
            startTimer (25);
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }
    void shutdown() override { stopTimer(); editor.reset(); processor.reset(); }

private:
    juce::Label& label (const juce::String& id)
    {
        auto* result = dynamic_cast<juce::Label*> (find (*editor, id));
        check (result != nullptr, "native output runtime omitted prepared label " + id);
        return *result;
    }
    void snapshot (const juce::String& name)
    {
        const auto image = editor->createComponentSnapshot (editor->getLocalBounds());
        auto* panel = find (*editor, "output-buses");
        check (panel && image.getPixelAt (panel->getX() + 2, panel->getY() + 40)
            == juce::Colour (dandrum::ui::tokens::surface_panel), "native output panel must use generated warm surface tokens");
        if (const auto* directory = std::getenv ("DANDRUM_NATIVE_OUTPUT_SNAPSHOTS"))
        {
            auto file = juce::File (directory).getChildFile (name + ".png");
            auto stream = file.createOutputStream();
            check (stream != nullptr, "native output snapshot must open " + file.getFullPathName());
            check (juce::PNGImageFormat().writeImageToStream (image, *stream), "native output snapshot must save actual editor pixels");
        }
    }
    void timerCallback() override
    {
        try
        {
            check (juce::Time::getMillisecondCounterHiRes() < deadline, "native output runtime timed out waiting for current readings/reload");
            if (phase == 0)
            {
                // Observe the original editor's subscription; never enable a
                // second session or depend on a guessed startup delay.
                if (! PluginConstructionTestProbe::meterVisible (*processor)) return;
                check (label ("output:0-name").getText() == "Output"
                    && label ("output:0-channels").getText() == "2 channels · L/R"
                    && label ("output:0-main").getText() == "Main" && label ("output:0-main").isVisible(),
                    "native output runtime must show the actual named main stereo bus");
                check (label ("output:0-status").getText() == "WAITING FOR AUDIO", "native original output view must wait for measured audio");
                juce::AudioBuffer<float> audio (2, 64);
                juce::MidiBuffer midi;
                midi.addEvent (juce::MidiMessage::noteOn (1, 36, static_cast<juce::uint8> (96)), 0);
                processor->processBlock (audio, midi);
                for (int channel = 0; channel < 2; ++channel)
                {
                    check (std::abs (audio.getSample (channel, 0) + 0.5f) < 0.00001f,
                           "native sampler output runtime must render known signed bundled kick -0.5 on each channel");
                    double energy = 0.0;
                    for (int frame = 0; frame < 64; ++frame)
                    {
                        const double value = audio.getSample (channel, frame);
                        peak[static_cast<std::size_t> (channel)] = std::max (peak[static_cast<std::size_t> (channel)], std::abs (value));
                        energy += value * value;
                    }
                    rms[static_cast<std::size_t> (channel)] = std::sqrt (energy / 64.0);
                }
                ++phase;
            }
            else if (phase == 1)
            {
                if (label ("output:0-status").getText() != "LIVE") return;
                for (std::size_t channel = 0; channel < 2; ++channel)
                {
                    auto* measured = find (*editor, "output:0-channel:" + juce::String (channel));
                    check (measured && measured->getName().contains ("generation " + juce::String (generation))
                        && measured->getName().contains ("peak " + juce::String (peak[channel], 3))
                        && measured->getName().contains ("RMS " + juce::String (rms[channel], 3)),
                        "native original timer must display independently measured callback peak/RMS and generation");
                }
                snapshot ("native-output-full");
                editor->setSize (820, 560); snapshot ("native-output-compact");
                check (processor->reloadInstrumentFromFile (juce::File (juce::String (
                    InstrumentDemoConfiguration::tb303().instrumentPath.string()))), "native output runtime must admit a real instrument reload");
                ++phase;
            }
            else
            {
                auto* measured = find (*editor, "output:0-channel:0");
                const auto current = processor->getParameterSurfaceGeneration();
                check (current != generation, "native reload must replace the working document generation");
                if (measured == nullptr || ! measured->getName().contains ("generation " + juce::String (current))) return;
                check (label ("output:0-status").getText() == "WAITING FOR AUDIO"
                    && ! measured->getName().contains ("peak"), "native original reload must clear old measurements before new audio");
                snapshot ("native-output-reload");
                std::cout << "Original native sampler output: signed first -0.5/-0.5, peak " << peak[0] << "/" << peak[1]
                          << ", RMS " << rms[0] << "/" << rms[1] << ", generation " << generation << " -> " << current << '\n';
                finish (true, {});
            }
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }
    void finish (bool passed, const juce::String& reason)
    { stopTimer(); if (! passed) std::cerr << reason << '\n'; setApplicationReturnValue (passed ? 0 : 1); quit(); }
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    std::array<double, 2> peak {}, rms {};
    std::uint32_t generation = 0;
    int phase = 0;
    double deadline = 0.0;
};
}
