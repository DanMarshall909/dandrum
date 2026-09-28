#include "PluginProcessor.h"
#include "DefaultPatch.h"

#include <cmath>
#include <iterator>
#include <iostream>
#include <memory>
#include <thread>
#include <vector>

namespace
{
bool bufferIsFinite (const juce::AudioBuffer<float>& buffer)
{
    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
    {
        const auto* samples = buffer.getReadPointer (channel);
        for (int i = 0; i < buffer.getNumSamples(); ++i)
            if (! std::isfinite (samples[i]))
                return false;
    }

    return true;
}

bool bufferHasSignal (const juce::AudioBuffer<float>& buffer)
{
    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
    {
        const auto* samples = buffer.getReadPointer (channel);
        for (int i = 0; i < buffer.getNumSamples(); ++i)
            if (std::abs (samples[i]) > 0.000001f)
                return true;
    }

    return false;
}

float tailRms (const juce::AudioBuffer<float>& buffer, int fromSample)
{
    double sumSquares = 0.0;
    int count = 0;

    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
    {
        const auto* samples = buffer.getReadPointer (channel);
        for (int i = fromSample; i < buffer.getNumSamples(); ++i)
        {
            sumSquares += static_cast<double> (samples[i]) * samples[i];
            ++count;
        }
    }

    return count == 0 ? 0.0f : static_cast<float> (std::sqrt (sumSquares / count));
}

juce::RangedAudioParameter* findParameter (DandrumAudioProcessor& processor, const juce::String& publicParameterId)
{
    return processor.getParameterForPublicId (publicParameterId);
}

float renderTailRms (DandrumAudioProcessor& processor, int blockSize, int numBlocks, int noteNumber)
{
    juce::AudioBuffer<float> full (2, blockSize * numBlocks);
    full.clear();

    for (int block = 0; block < numBlocks; ++block)
    {
        juce::AudioBuffer<float> blockBuffer (2, blockSize);
        blockBuffer.clear();
        juce::MidiBuffer midi;
        if (block == 0)
            midi.addEvent (juce::MidiMessage::noteOn (1, (juce::uint8) noteNumber, (juce::uint8) 100), 0);

        processor.processBlock (blockBuffer, midi);

        for (int channel = 0; channel < 2; ++channel)
            full.copyFrom (channel, block * blockSize, blockBuffer, channel, 0, blockSize);
    }

    return tailRms (full, (numBlocks - 1) * blockSize);
}

float renderKickTailRms (float normalizedDecayValue, int blockSize, int numBlocks)
{
    auto processor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    processor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    processor->prepareToPlay (48000.0, blockSize);

    auto* decayParam = findParameter (*processor, "kick.decay_ms");
    if (decayParam != nullptr)
        decayParam->setValueNotifyingHost (normalizedDecayValue);

    const auto result = renderTailRms (*processor, blockSize, numBlocks, 36);
    processor->releaseResources();
    return result;
}

juce::File defaultPatchFile()
{
    return juce::File (juce::String (dandrum::defaultPatchPath().string()));
}

juce::String decayControlDefault (const juce::String& value)
{
    return "{ name: decay_ms, direction: input, signal: control, channels: 1, default: " + value + ",";
}

juce::File examplePresetFile (const juce::String& name)
{
    const auto path = dandrum::defaultPatchPath().parent_path().parent_path() / "presets" / name.toStdString();
    return juce::File (juce::String (path.string()));
}

// Writes a copy of the bundled default instrument with a target line replaced,
// so reload tests can prove a genuinely different instrument was loaded (not
// just the same file re-read). Returns an invalid (default-constructed) File
// if targetLine isn't found in the bundled patch, rather than silently
// writing out an unmodified copy — callers must not mistake "no match" for
// "modified".
juce::File writeModifiedKickPatch (const juce::String& targetLine, const juce::String& newLine)
{
    const auto original = defaultPatchFile();
    auto content = original.loadFileAsString();
    if (! content.contains (targetLine))
        return {};

    content = content.replace (targetLine, newLine);

    auto modified = juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getChildFile ("dandrum_modified_kick_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    modified.replaceWithText (content);
    return modified;
}

juce::File writePatchWithAdditionalPublicParameter()
{
    const auto original = defaultPatchFile();
    auto content = original.loadFileAsString();
    const juce::String rootMarker = "  - { name: sub_decay_ms, direction: input, signal: control, channels: 1, default: 800, min: 50, max: 2000, maps_to: voices.sub_decay_ms }";
    const juce::String aliasMarker = "    - { name: kick.sub_decay_ms, maps_to: sub_decay_ms }";
    if (! content.contains (rootMarker) || ! content.contains (aliasMarker))
        return {};

    content = content.replace (rootMarker,
                               "  - { name: extra_click, direction: input, signal: control, channels: 1, default: 0.25, min: 0, max: 1 }\n"
                               + rootMarker);
    content = content.replace (aliasMarker,
                               "    - { name: kick.extra_click, maps_to: extra_click }\n"
                               + aliasMarker);
    auto modified = juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getChildFile ("dandrum_added_public_parameter_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    modified.replaceWithText (content);
    return modified;
}

// A minimal but valid instrument with no preset_surface parameters at all, so
// reloading to it drops every public parameter the default instrument exposed
// from the dynamic editor/public-id surface.
juce::File writePatchWithNoPublicParameters()
{
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile ("dandrum_no_public_parameters_" + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    file.replaceWithText (
        "metadata:\n"
        "  name: No Public Parameters\n"
        "instrument:\n"
        "  id: dandrum.no-public-parameters\n"
        "  preset_schema_version: 1\n"
        "ports:\n"
        "  - { name: master, direction: output, signal: audio, channels: 2, maps_from: tone.out }\n"
        "modules:\n"
        "  - { id: tone, type: control_to_audio, static: { channels: 2 }, defaults: { in: 0.25 } }\n"
        "connections: []\n");
    return file;
}

juce::File writePresetFile (const juce::String& filePrefix, const juce::String& yaml)
{
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile (filePrefix + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    file.replaceWithText (yaml);
    return file;
}

bool nearlyEqual (float actual, float expected, float tolerance)
{
    return std::abs (actual - expected) <= tolerance;
}

bool freshInstrumentUsesAuthoredDefaults()
{
    constexpr int frames = 512;
    DandrumAudioProcessor processor (InstrumentDemoConfiguration::kick());
    processor.setPlayConfigDetails (0, 2, 48000.0, frames);
    processor.prepareToPlay (48000.0, frames);

    const auto patch = defaultPatchFile();
    const DandrumKernelBusDeclaration master { "master", 2, 2 };
    std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)> reference (
        dandrum_kernel_prepare_file (patch.getFullPathName().toRawUTF8(), 48000, frames, &master, 1),
        &dandrum_kernel_destroy);
    if (reference == nullptr)
        return false;

    const auto count = dandrum_patch_public_numeric_parameter_count (patch.getFullPathName().toRawUTF8());
    if (count == 0)
        return false;
    for (std::size_t index = 0; index < count; ++index)
    {
        char id[128] {}, name[128] {};
        double authoredDefault = 0.0, minimum = 0.0, maximum = 0.0;
        if (! dandrum_patch_public_numeric_parameter_descriptor (
                patch.getFullPathName().toRawUTF8(), index, id, sizeof (id), name, sizeof (name),
                &authoredDefault, &minimum, &maximum))
            return false;
        const auto* hostParameter = processor.getParameterForPublicId (id);
        const auto expectedNormalised = static_cast<float> ((authoredDefault - minimum) / (maximum - minimum));
        if (hostParameter == nullptr || ! nearlyEqual (hostParameter->getValue(), expectedNormalised, 0.00001f))
            return false;
    }

    juce::AudioBuffer<float> actual (2, frames), expected (2, frames);
    actual.clear();
    expected.clear();
    juce::MidiBuffer noHostMidi;
    if (! processor.enqueueEditorNoteOn (36, 100.0f / 127.0f))
        return false;
    dandrum_kernel_note_on_at (reference.get(), 36, 100, 0);
    processor.processBlock (actual, noHostMidi);
    float* channels[] { expected.getWritePointer (0), expected.getWritePointer (1) };
    const DandrumKernelOutputBusView output { "master", channels, 2, frames };
    if (dandrum_kernel_render (reference.get(), nullptr, 0, &output, 1, frames) != frames)
        return false;
    if (! bufferHasSignal (expected))
        return false;
    for (int channel = 0; channel < 2; ++channel)
        for (int frame = 0; frame < frames; ++frame)
            if (! nearlyEqual (actual.getSample (channel, frame), expected.getSample (channel, frame), 0.00001f))
            {
                std::cerr << "fresh default mismatch at frame " << frame << ": host="
                          << actual.getSample (channel, frame) << " Rust=" << expected.getSample (channel, frame) << '\n';
                return false;
            }
    processor.releaseResources();
    return true;
}

bool hostMidiVelocityMatchesRustEvent()
{
    constexpr int frames = 512;
    constexpr int offset = 7;
    constexpr juce::uint8 velocity = 100;
    const auto patch = juce::File (juce::String (
        dandrum::findRepositoryExample ("examples/patches/tb303-acid.yaml").string()));
    DandrumAudioProcessor processor (InstrumentDemoConfiguration::kick());
    processor.setPlayConfigDetails (0, 2, 48000.0, frames);
    processor.prepareToPlay (48000.0, frames);
    if (! processor.reloadInstrumentFromFile (patch))
    {
        std::cerr << "acid kernel patch did not reload in plugin\n";
        return false;
    }

    const DandrumKernelBusDeclaration buses[] {
        { "master", 2, 2 }, { "filter_cutoff", 1, 1 }, { "filter_resonance", 1, 1 },
        { "filter_envelope_modulation", 1, 1 }, { "filter_decay_ms", 1, 1 },
        { "accent_brightness", 1, 1 }, { "amp_release_ms", 1, 1 }, { "slide_time_ms", 1, 1 }
    };
    std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)> reference (
        dandrum_kernel_prepare_file (patch.getFullPathName().toRawUTF8(), 48000, frames,
                                     buses, std::size (buses)),
        &dandrum_kernel_destroy);
    if (reference == nullptr)
    {
        std::cerr << "acid kernel reference did not prepare\n";
        return false;
    }

    const auto count = dandrum_patch_public_numeric_parameter_count (patch.getFullPathName().toRawUTF8());
    if (count == 0)
    {
        std::cerr << "acid kernel patch exposed no public controls\n";
        return false;
    }
    for (std::size_t index = 0; index < count; ++index)
    {
        char id[128] {}, name[128] {};
        double authoredDefault = 0.0, minimum = 0.0, maximum = 0.0;
        if (! dandrum_patch_public_numeric_parameter_descriptor (
                patch.getFullPathName().toRawUTF8(), index, id, sizeof (id), name, sizeof (name),
                &authoredDefault, &minimum, &maximum))
            return false;
        auto* hostParameter = processor.getParameterForPublicId (id);
        if (hostParameter == nullptr)
        {
            std::cerr << "plugin missing public control " << id << '\n';
            return false;
        }
        hostParameter->setValueNotifyingHost (0.5f);
        if (! dandrum_kernel_set_public_numeric_parameter_by_slot (reference.get(), index,
                                                                  (minimum + maximum) / 2.0))
        {
            std::cerr << "kernel reference rejected public control " << id << '\n';
            return false;
        }
    }

    juce::AudioBuffer<float> actual (2, frames), expected (2, frames);
    actual.clear();
    expected.clear();
    juce::MidiBuffer hostMidi;
    hostMidi.addEvent (juce::MidiMessage::noteOn (1, 60, velocity), offset);
    dandrum_kernel_note_on_at (reference.get(), 60, velocity, offset);
    processor.processBlock (actual, hostMidi);
    float* channels[] { expected.getWritePointer (0), expected.getWritePointer (1) };
    const DandrumKernelOutputBusView output { "master", channels, 2, frames };
    if (dandrum_kernel_render (reference.get(), nullptr, 0, &output, 1, frames) != frames)
    {
        std::cerr << "kernel reference did not render master\n";
        return false;
    }
    if (! bufferHasSignal (expected))
    {
        std::cerr << "kernel reference master was silent\n";
        return false;
    }
    for (int channel = 0; channel < 2; ++channel)
        for (int frame = 0; frame < frames; ++frame)
            if (! nearlyEqual (actual.getSample (channel, frame), expected.getSample (channel, frame), 0.00001f))
            {
                std::cerr << "MIDI velocity mismatch at frame " << frame << ": host="
                          << actual.getSample (channel, frame) << " Rust=" << expected.getSample (channel, frame) << '\n';
                return false;
            }
    processor.releaseResources();
    return true;
}

bool preparationPreservesRestoredInstrumentAndHostSlots()
{
    juce::TemporaryFile modifiedPatch (".yaml");
    auto yaml = defaultPatchFile().loadFileAsString();
    if (! yaml.contains ("defaults: { sustain: 0, attack: 0 }")
        || ! modifiedPatch.getFile().replaceWithText (yaml.replace ("defaults: { sustain: 0, attack: 0 }",
                                                                 "defaults: { sustain: 0, attack: 50 }")))
        return false;
    DandrumAudioProcessor source (InstrumentDemoConfiguration::kick());
    source.setPlayConfigDetails (0, 2, 48000.0, 64);
    source.prepareToPlay (48000.0, 64);
    if (! source.reloadInstrumentFromFile (modifiedPatch.getFile()))
        return false;
    const std::pair<const char*, float> values[] {
        { "kick.tune_hz", 0.5f }, { "kick.decay_ms", 0.25f }, { "kick.punch", 0.375f },
        { "kick.click", 0.75f }, { "kick.sub_decay_ms", 0.125f }, { "kick.sub_level", 0.625f }
    };
    for (const auto& [id, normalized] : values)
    {
        auto* parameter = source.getParameterForPublicId (id);
        if (parameter == nullptr)
            return false;
        parameter->setValueNotifyingHost (normalized);
    }
    juce::MemoryBlock saved;
    source.getStateInformation (saved);

    for (const bool restoreState : { false, true })
    {
        DandrumAudioProcessor processor (InstrumentDemoConfiguration::kick());
        const auto slots = processor.getParameters();
        juce::StringArray slotIds;
        for (const auto* slot : slots)
            slotIds.add (static_cast<const juce::RangedAudioParameter*> (slot)->paramID);
        if (restoreState)
            processor.setStateInformation (saved.getData(), static_cast<int> (saved.getSize()));
        else
            for (const auto& [id, normalized] : values)
                processor.getParameterForPublicId (id)->setValueNotifyingHost (normalized);

        for (const auto& [rate, frames] : { std::pair { 44100.0, 64 }, { 48000.0, 1024 }, { 96000.0, 32 } })
        {
            processor.setPlayConfigDetails (0, 2, rate, frames);
            processor.prepareToPlay (rate, frames);
            const auto missing = juce::File::getSpecialLocation (juce::File::tempDirectory)
                                     .getNonexistentChildFile ("dandrum_lifecycle_missing", ".yaml");
            if (processor.reloadInstrumentFromFile (missing) || processor.getLastLoadError().isEmpty())
                return false;
            const auto& current = processor.getParameters();
            if (current.size() != 64 || current.size() != slots.size())
                return false;
            for (int index = 0; index < current.size(); ++index)
                if (current[index] != slots[index]
                    || static_cast<juce::RangedAudioParameter*> (current[index])->paramID != slotIds[index])
                    return false;

            // Independent named-bus host path. The custom deleter owns the
            // FFI handle even when a later comparison fails.
            const auto referencePatch = restoreState ? modifiedPatch.getFile() : defaultPatchFile();
            const DandrumKernelBusDeclaration buses[] {
                { "master", 2, 2 }, { "tune_hz", 1, 1 }, { "decay_ms", 1, 1 },
                { "punch", 1, 1 }, { "click", 1, 1 }, { "sub_decay_ms", 1, 1 },
                { "sub_level", 1, 1 }
            };
            std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)> reference (
                dandrum_kernel_prepare_file (referencePatch.getFullPathName().toRawUTF8(),
                                             static_cast<std::uint32_t> (rate),
                                             static_cast<std::size_t> (frames), buses, std::size (buses)),
                &dandrum_kernel_destroy);
            if (reference == nullptr)
                return false;
            // Physical values come from the authored ranges and inputs above.
            const std::pair<const char*, double> physicalValues[] {
                { "kick.tune_hz", 70.0 }, { "kick.decay_ms", 537.5 }, { "kick.punch", 0.375 },
                { "kick.click", 0.75 }, { "kick.sub_decay_ms", 293.75 }, { "kick.sub_level", 0.625 }
            };
            for (std::size_t index = 0; index < std::size (physicalValues); ++index)
                if (! dandrum_kernel_set_public_numeric_parameter_by_slot (reference.get(), index,
                                                                            physicalValues[index].second))
                    return false;
            for (const auto& [id, normalized] : values)
            {
                auto* parameter = processor.getParameterForPublicId (id);
                if (parameter == nullptr || ! nearlyEqual (parameter->getValue(), normalized, 0.00001f))
                    return false;
            }
            bool audible = false;
            for (int block = 0; block < 8; ++block)
            {
                juce::AudioBuffer<float> actual (2, frames), expected (2, frames);
                actual.clear();
                expected.clear();
                juce::MidiBuffer midi;
                if (block == 0)
                {
                    midi.addEvent (juce::MidiMessage::noteOn (1, 36, (juce::uint8) 100), 5);
                    dandrum_kernel_note_on_at (reference.get(), 36, 100, 5);
                }
                processor.processBlock (actual, midi);
                float* channels[] { expected.getWritePointer (0), expected.getWritePointer (1) };
                const DandrumKernelOutputBusView output { "master", channels, 2,
                                                          static_cast<std::size_t> (frames) };
                if (dandrum_kernel_render (reference.get(), nullptr, 0, &output, 1,
                                           static_cast<std::size_t> (frames)) != static_cast<std::size_t> (frames))
                    return false;
                audible = audible || bufferHasSignal (actual);
                for (int channel = 0; channel < 2; ++channel)
                    for (int frame = 0; frame < frames; ++frame)
                        if (! nearlyEqual (actual.getSample (channel, frame), expected.getSample (channel, frame), 0.00001f))
                        {
                            std::cerr << "preparation audio mismatch: rate=" << rate << " restored=" << restoreState
                                      << " block=" << block << " frame=" << frame
                                      << " actual=" << actual.getSample (channel, frame)
                                      << " expected=" << expected.getSample (channel, frame)
                                      << " loaded=" << processor.isInstrumentLoaded()
                                      << " muted=" << processor.isMuted()
                                      << " next=" << actual.getSample (channel, 6)
                                      << " end=" << actual.getSample (channel, frames - 1) << '\n';
                            return false;
                        }
            }
            if (! audible)
                return false;
            processor.releaseResources();
        }
    }
    return true;
}
} // namespace

int main()
{
    constexpr int blockSize = 64;

    const auto kernelFile = juce::File::getSpecialLocation (juce::File::tempDirectory)
                                .getChildFile ("dandrum-plugin-kernel-"
                                               + juce::String (juce::Random::getSystemRandom().nextInt())
                                               + ".yaml");
    kernelFile.replaceWithText (
        "metadata: { name: plugin-kernel }\n"
        "ports:\n"
        "  - { name: master, direction: output, signal: audio, channels: 2, maps_from: source.out }\n"
        "modules:\n"
        "  - { id: source, type: control_to_audio, static: { channels: 2 }, defaults: { in: 0.25 } }\n"
        "connections: []\n");
    auto kernelConfiguration = InstrumentDemoConfiguration::kick();
    kernelConfiguration.instrumentPath = kernelFile.getFullPathName().toStdString();
    DandrumAudioProcessor kernelProcessor (kernelConfiguration);
    kernelProcessor.setPlayConfigDetails (0, 2, 48000.0, blockSize);
    kernelProcessor.prepareToPlay (48000.0, blockSize);
    juce::AudioBuffer<float> kernelBuffer (2, blockSize);
    kernelBuffer.clear();
    juce::MidiBuffer kernelMidi;
    kernelProcessor.processBlock (kernelBuffer, kernelMidi);
    if (! kernelProcessor.isInstrumentLoaded()
        || ! nearlyEqual (kernelBuffer.getSample (0, 0), 0.25f, 0.00001f))
    {
        std::cerr << "plugin did not render kernel master output\n";
        return 1;
    }
    kernelFile.replaceWithText (
        "metadata: { name: plugin-kernel-reload }\n"
        "ports:\n"
        "  - { name: master, direction: output, signal: audio, channels: 2, maps_from: source.out }\n"
        "modules:\n"
        "  - { id: source, type: control_to_audio, static: { channels: 2 }, defaults: { in: -0.5 } }\n"
        "connections: []\n");
    if (! kernelProcessor.reloadInstrumentFromFile (kernelFile))
    {
        std::cerr << "plugin did not reload kernel master output\n";
        return 1;
    }
    kernelBuffer.clear();
    kernelProcessor.processBlock (kernelBuffer, kernelMidi);
    if (! nearlyEqual (kernelBuffer.getSample (0, 0), -0.5f, 0.00001f))
    {
        std::cerr << "plugin kernel reload did not change master output\n";
        return 1;
    }
    const auto legacyPatch = juce::File (juce::String (DANDRUM_SOURCE_ROOT))
                                 .getChildFile ("src/rust-engine/tests/fixtures/unify-graph-kernel/legacy/polyphonic-pad.yaml");
    if (kernelProcessor.reloadInstrumentFromFile (legacyPatch)
        || kernelProcessor.getLastLoadError().isEmpty())
    {
        std::cerr << "plugin accepted a legacy instrument after kernel migration\n";
        return 1;
    }
    kernelBuffer.clear();
    kernelProcessor.processBlock (kernelBuffer, kernelMidi);
    if (! nearlyEqual (kernelBuffer.getSample (0, 0), -0.5f, 0.00001f))
    {
        std::cerr << "rejected legacy instrument replaced the running kernel\n";
        return 1;
    }
    juce::MemoryBlock kernelState;
    kernelProcessor.getStateInformation (kernelState);
    DandrumAudioProcessor restoredKernel (InstrumentDemoConfiguration::kick());
    restoredKernel.setPlayConfigDetails (0, 2, 48000.0, blockSize);
    restoredKernel.prepareToPlay (48000.0, blockSize);
    restoredKernel.setStateInformation (kernelState.getData(), static_cast<int> (kernelState.getSize()));
    kernelBuffer.clear();
    restoredKernel.processBlock (kernelBuffer, kernelMidi);
    if (! nearlyEqual (kernelBuffer.getSample (0, 0), -0.5f, 0.00001f))
    {
        std::cerr << "plugin did not restore embedded kernel master output\n";
        return 1;
    }
    if (! kernelProcessor.reloadInstrumentFromFile (defaultPatchFile())
        || ! kernelProcessor.hasPublicParameter ("kick.tune_hz"))
    {
        std::cerr << "plugin did not switch back to the default kick patch\n";
        return 1;
    }
    kernelFile.deleteFile();

    if (! hostMidiVelocityMatchesRustEvent())
    {
        std::cerr << "host MIDI velocity did not reach Rust unchanged\n";
        return 1;
    }

    if (! freshInstrumentUsesAuthoredDefaults())
    {
        std::cerr << "fresh plugin parameters did not retain authored engine defaults\n";
        return 1;
    }

    if (! preparationPreservesRestoredInstrumentAndHostSlots())
    {
        std::cerr << "host preparation did not preserve instrument audio, restored values or fixed slots\n";
        return 1;
    }

    auto processor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    if (! processor->isInstrumentLoaded())
    {
        std::cerr << processor->getLastLoadError() << '\n';
        return 1;
    }

    if (! processor->hasPublicParameter ("kick.tune_hz"))
    {
        std::cerr << "default plugin instrument did not expose kick.tune_hz\n";
        return 1;
    }

    juce::MemoryBlock state;
    processor->getStateInformation (state);
    if (state.getSize() == 0)
    {
        std::cerr << "plugin state serialization produced an empty state block\n";
        return 1;
    }

    auto restored = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    restored->setStateInformation (state.getData(), static_cast<int> (state.getSize()));
    if (! restored->hasPublicParameter ("kick.tune_hz"))
    {
        std::cerr << "restored plugin did not keep the public parameter mapping\n";
        return 1;
    }

    processor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    processor->prepareToPlay (48000.0, blockSize);

    juce::AudioBuffer<float> buffer (2, blockSize);
    buffer.clear();

    juce::MidiBuffer midi;
    midi.addEvent (juce::MidiMessage::noteOn (1, 60, (juce::uint8) 100), 10);

    processor->processBlock (buffer, midi);

    if (! bufferIsFinite (buffer))
    {
        std::cerr << "plugin processBlock produced non-finite samples\n";
        return 1;
    }

    if (! bufferHasSignal (buffer))
    {
        std::cerr << "plugin processBlock rendered silence after default instrument note-on\n";
        return 1;
    }

    processor->releaseResources();

    // The web editor runs on JUCE's message thread, so it feeds a bounded
    // queue instead of touching the Rust engine while the audio callback may
    // be rendering. Prove that an editor-originated note reaches that callback.
    auto editorMidiProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    editorMidiProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    editorMidiProcessor->prepareToPlay (48000.0, blockSize);
    if (! editorMidiProcessor->enqueueEditorNoteOn (36, 1.0f))
    {
        std::cerr << "plugin rejected a web-editor note-on while its MIDI queue was empty\n";
        return 1;
    }

    juce::AudioBuffer<float> editorMidiBuffer (2, blockSize);
    editorMidiBuffer.clear();
    juce::MidiBuffer noHostMidi;
    editorMidiProcessor->processBlock (editorMidiBuffer, noHostMidi);
    if (! bufferIsFinite (editorMidiBuffer) || ! bufferHasSignal (editorMidiBuffer))
    {
        std::cerr << "web-editor note-on did not produce finite audio signal\n";
        return 1;
    }

    if (! editorMidiProcessor->enqueueEditorNoteOff (36))
    {
        std::cerr << "plugin rejected a web-editor note-off while its MIDI queue was empty\n";
        return 1;
    }
    editorMidiProcessor->processBlock (editorMidiBuffer, noHostMidi);
    editorMidiProcessor->releaseResources();

    auto boundedQueueProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    bool queueRejectedEvent = false;
    for (int event = 0; event < 256; ++event)
        queueRejectedEvent = ! boundedQueueProcessor->enqueueEditorNoteOn (36, 0.8f)
                             || queueRejectedEvent;

    if (! queueRejectedEvent || boundedQueueProcessor->getDroppedMidiEventCount() == 0)
    {
        std::cerr << "web-editor MIDI queue did not bound and report overflow\n";
        return 1;
    }

    // Prove that changing a public parameter (via the same APVTS path the
    // generic JUCE knobs use) actually reaches the running instrument, rather
    // than only updating the host-facing parameter value cosmetically.
    constexpr int decayBlockSize = 128;
    constexpr int decayNumBlocks = 40; // ~106ms at 48kHz
    const auto shortDecayTailRms = renderKickTailRms (0.0f, decayBlockSize, decayNumBlocks);
    const auto longDecayTailRms = renderKickTailRms (1.0f, decayBlockSize, decayNumBlocks);

    if (! (longDecayTailRms > shortDecayTailRms * 2.0f))
    {
        std::cerr << "changing kick.decay_ms did not audibly change the render: short="
                   << shortDecayTailRms << " long=" << longDecayTailRms << '\n';
        return 1;
    }

    // Public parameter changes must never mutate or reload the loaded YAML.
    {
        auto yamlCheckProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        yamlCheckProcessor->setPlayConfigDetails (0, 2, 48000.0, decayBlockSize);
        yamlCheckProcessor->prepareToPlay (48000.0, decayBlockSize);
        const auto yamlBefore = yamlCheckProcessor->currentInstrumentYaml();

        auto* tuneParam = findParameter (*yamlCheckProcessor, "kick.tune_hz");
        if (tuneParam == nullptr)
        {
            std::cerr << "kick.tune_hz parameter missing\n";
            return 1;
        }
        tuneParam->setValueNotifyingHost (0.75f);

        juce::AudioBuffer<float> yamlCheckBuffer (2, decayBlockSize);
        yamlCheckBuffer.clear();
        juce::MidiBuffer yamlCheckMidi;
        yamlCheckMidi.addEvent (juce::MidiMessage::noteOn (1, 36, (juce::uint8) 100), 0);
        yamlCheckProcessor->processBlock (yamlCheckBuffer, yamlCheckMidi);

        if (yamlCheckProcessor->currentInstrumentYaml() != yamlBefore)
        {
            std::cerr << "changing a public parameter mutated the loaded instrument YAML\n";
            return 1;
        }
        yamlCheckProcessor->releaseResources();
    }

    // Muting should silence the processor even with an active note.
    auto mutedProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    mutedProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    mutedProcessor->prepareToPlay (48000.0, blockSize);
    mutedProcessor->setMuted (true);
    if (! mutedProcessor->isMuted())
    {
        std::cerr << "setMuted(true) did not update isMuted()\n";
        return 1;
    }

    juce::AudioBuffer<float> mutedBuffer (2, blockSize);
    mutedBuffer.clear();
    juce::MidiBuffer mutedMidi;
    mutedMidi.addEvent (juce::MidiMessage::noteOn (1, 60, (juce::uint8) 100), 0);
    mutedProcessor->processBlock (mutedBuffer, mutedMidi);

    if (bufferHasSignal (mutedBuffer))
    {
        std::cerr << "processBlock produced signal while the processor was muted\n";
        return 1;
    }

    mutedProcessor->setMuted (false);
    mutedProcessor->releaseResources();

    // Reload success + parameter carry-over: an explicit value set before
    // reload should still apply after the swap, overriding whatever default
    // the replacement YAML declares (design.md: "parameters that still exist
    // keep their current value"). A live parameter change after the reload
    // should also still reach the (new) running engine.
    constexpr int reloadBlockSize = 128;
    constexpr int reloadNumBlocks = 40; // ~106ms at 48kHz

    auto reloadProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    reloadProcessor->setPlayConfigDetails (0, 2, 48000.0, reloadBlockSize);
    reloadProcessor->prepareToPlay (48000.0, reloadBlockSize);

    auto* decayParam = findParameter (*reloadProcessor, "kick.decay_ms");
    if (decayParam == nullptr)
    {
        std::cerr << "kick.decay_ms parameter missing on the default instrument\n";
        return 1;
    }
    decayParam->setValueNotifyingHost (0.0f); // normalized minimum: shortest decay

    const auto shortBeforeReloadTailRms = renderTailRms (*reloadProcessor, reloadBlockSize, reloadNumBlocks, 36);

    // The replacement file declares a much longer default (1900ms), so if
    // carry-over were broken and the new file's own default won instead, this
    // render would come out clearly longer/louder than shortBeforeReloadTailRms.
    const auto longDefaultFile = writeModifiedKickPatch (decayControlDefault ("650"), decayControlDefault ("1900"));
    if (! reloadProcessor->reloadInstrumentFromFile (longDefaultFile))
    {
        std::cerr << "reloadInstrumentFromFile failed unexpectedly: "
                   << reloadProcessor->getLastLoadError() << '\n';
        return 1;
    }

    if (! reloadProcessor->isInstrumentLoaded()
        || reloadProcessor->currentInstrumentFile() != longDefaultFile
        || ! reloadProcessor->currentInstrumentYaml().contains (decayControlDefault ("1900")))
    {
        std::cerr << "reloadInstrumentFromFile did not update loaded-instrument bookkeeping\n";
        return 1;
    }

    if (! reloadProcessor->getLastReloadWarning().isEmpty())
    {
        std::cerr << "reloadInstrumentFromFile warned about dropped parameters when none were dropped: "
                   << reloadProcessor->getLastReloadWarning() << '\n';
        return 1;
    }

    const auto matchedSnapshotYaml = longDefaultFile.loadFileAsString().replace (
        decayControlDefault ("1900"), decayControlDefault ("1700"));
    if (! reloadProcessor->reloadInstrumentFromYaml (matchedSnapshotYaml, longDefaultFile)
        || reloadProcessor->currentInstrumentFile() != longDefaultFile
        || reloadProcessor->currentInstrumentYaml() != matchedSnapshotYaml)
    {
        std::cerr << "reloadInstrumentFromYaml did not load and retain the immutable matched snapshot: "
                  << reloadProcessor->getLastLoadError() << '\n';
        return 1;
    }

    const auto carriedOverTailRms = renderTailRms (*reloadProcessor, reloadBlockSize, reloadNumBlocks, 36);
    // The carried-over 50ms decay is fully silent this far into the tail, so its
    // tail RMS is ~0 and can't support a multiplicative bound. A broken carry-over
    // would instead apply the replacement file's 1900ms default and leave a
    // clearly audible tail here, comparable to the long-decay render measured
    // above. Bound against a small fraction of that known-audible level.
    const auto carriedOverTailCeiling = juce::jmax (shortBeforeReloadTailRms * 2.0f, longDecayTailRms * 0.1f);
    if (! (carriedOverTailRms < carriedOverTailCeiling))
    {
        std::cerr << "explicit kick.decay_ms value was not carried over across reload: before="
                   << shortBeforeReloadTailRms << " afterReload=" << carriedOverTailRms
                   << " ceiling=" << carriedOverTailCeiling << '\n';
        return 1;
    }

    // The parameter bridge should still be live on the freshly-reloaded
    // engine: a further explicit change should audibly take effect.
    decayParam = findParameter (*reloadProcessor, "kick.decay_ms");
    if (decayParam == nullptr)
    {
        std::cerr << "kick.decay_ms parameter disappeared after reload\n";
        return 1;
    }
    decayParam->setValueNotifyingHost (1.0f); // normalized maximum: longest decay
    const auto longAfterReloadTailRms = renderTailRms (*reloadProcessor, reloadBlockSize, reloadNumBlocks, 36);
    if (! (longAfterReloadTailRms > carriedOverTailRms * 1.3f))
    {
        std::cerr << "kick.decay_ms changes stopped taking effect after reload: short="
                   << carriedOverTailRms << " long=" << longAfterReloadTailRms << '\n';
        return 1;
    }

    reloadProcessor->releaseResources();

    // A replacement instrument that introduces a new public parameter should
    // expose it through an unused fixed host slot and initialise it from the
    // YAML-declared default value.
    {
        auto addProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        addProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        addProcessor->prepareToPlay (48000.0, blockSize);

        const auto addedParameterFile = writePatchWithAdditionalPublicParameter();
        if (addedParameterFile == juce::File())
        {
            std::cerr << "writePatchWithAdditionalPublicParameter could not patch the default instrument\n";
            return 1;
        }
        if (! addProcessor->reloadInstrumentFromFile (addedParameterFile))
        {
            std::cerr << "reloadInstrumentFromFile failed for added-parameter patch: "
                      << addProcessor->getLastLoadError() << '\n';
            return 1;
        }

        auto* extraClick = findParameter (*addProcessor, "kick.extra_click");
        if (extraClick == nullptr)
        {
            std::cerr << "replacement instrument did not expose newly-added public parameter kick.extra_click\n";
            return 1;
        }
        if (! nearlyEqual (extraClick->getValue(), 0.25f, 0.002f))
        {
            std::cerr << "kick.extra_click was not initialised from YAML default: " << extraClick->getValue() << '\n';
            return 1;
        }

        addProcessor->releaseResources();
        addedParameterFile.deleteFile();
    }

    // Reload failure: the previous instrument keeps running unchanged.
    auto failedReloadProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
    failedReloadProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    failedReloadProcessor->prepareToPlay (48000.0, blockSize);

    const juce::File missingFile ("/nonexistent/path/to/dandrum_missing_instrument.yaml");
    if (failedReloadProcessor->reloadInstrumentFromFile (missingFile))
    {
        std::cerr << "reloadInstrumentFromFile unexpectedly succeeded for a missing file\n";
        return 1;
    }
    if (failedReloadProcessor->getLastLoadError().isEmpty())
    {
        std::cerr << "reloadInstrumentFromFile failure did not set an error message\n";
        return 1;
    }

    juce::AudioBuffer<float> stillWorkingBuffer (2, blockSize);
    stillWorkingBuffer.clear();
    juce::MidiBuffer stillWorkingMidi;
    stillWorkingMidi.addEvent (juce::MidiMessage::noteOn (1, 36, (juce::uint8) 100), 0);
    failedReloadProcessor->processBlock (stillWorkingBuffer, stillWorkingMidi);

    if (! bufferIsFinite (stillWorkingBuffer) || ! bufferHasSignal (stillWorkingBuffer))
    {
        std::cerr << "processor stopped rendering correctly after a failed reload attempt\n";
        return 1;
    }

    failedReloadProcessor->releaseResources();

    // writeModifiedKickPatch must fail loudly (an invalid File) rather than
    // silently writing an unmodified copy when the target line isn't found,
    // so a future drift in the bundled patch can't silently defang the
    // reload/carry-over test above.
    const auto noSuchTargetFile = writeModifiedKickPatch (decayControlDefault ("this-value-does-not-exist"), decayControlDefault ("1900"));
    if (noSuchTargetFile != juce::File())
    {
        std::cerr << "writeModifiedKickPatch did not report failure for an unmatched target line\n";
        return 1;
    }

    // reloadInstrumentFromFile must not silently prepare a candidate engine
    // with an invalid (zero) sample rate when called before the host has ever
    // called prepareToPlay.
    {
        auto unpreparedProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        if (unpreparedProcessor->reloadInstrumentFromFile (defaultPatchFile()))
        {
            std::cerr << "reloadInstrumentFromFile succeeded before prepareToPlay was ever called\n";
            return 1;
        }
        if (unpreparedProcessor->getLastLoadError().isEmpty())
        {
            std::cerr << "reloadInstrumentFromFile rejected a pre-prepareToPlay call without setting an error\n";
            return 1;
        }
    }

    // Concurrent reloadInstrumentFromFile calls must never destroy an engine
    // another in-flight reload is still using: two threads hammering reload
    // on the same processor must leave it in a valid, still-rendering state.
    {
        auto racingProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        racingProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        racingProcessor->prepareToPlay (48000.0, blockSize);

        const auto defaultFile = defaultPatchFile();
        const auto alternateFile = writeModifiedKickPatch (decayControlDefault ("650"), decayControlDefault ("700"));

        constexpr int threadCount = 4;
        constexpr int iterationsPerThread = 20;
        std::vector<std::thread> threads;
        for (int t = 0; t < threadCount; ++t)
        {
            threads.emplace_back ([&, t]
            {
                for (int i = 0; i < iterationsPerThread; ++i)
                {
                    const auto& file = ((t + i) % 2 == 0) ? defaultFile : alternateFile;
                    racingProcessor->reloadInstrumentFromFile (file);
                }
            });
        }
        for (auto& thread : threads)
            thread.join();

        if (! racingProcessor->isInstrumentLoaded())
        {
            std::cerr << "processor was left without a loaded instrument after concurrent reloads\n";
            return 1;
        }

        juce::AudioBuffer<float> racingBuffer (2, blockSize);
        racingBuffer.clear();
        juce::MidiBuffer racingMidi;
        racingMidi.addEvent (juce::MidiMessage::noteOn (1, 36, (juce::uint8) 100), 0);
        racingProcessor->processBlock (racingBuffer, racingMidi);

        if (! bufferIsFinite (racingBuffer))
        {
            std::cerr << "processor produced non-finite samples after concurrent reloadInstrumentFromFile calls\n";
            return 1;
        }

        racingProcessor->releaseResources();
        alternateFile.deleteFile();
    }

    // Reloading to an instrument that drops previously-live public parameters
    // must be visibly reconciled (design.md), not silently absorbed.
    {
        auto dropProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        dropProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        dropProcessor->prepareToPlay (48000.0, blockSize);

        const auto noPublicParametersFile = writePatchWithNoPublicParameters();
        if (! dropProcessor->reloadInstrumentFromFile (noPublicParametersFile))
        {
            std::cerr << "reloadInstrumentFromFile failed unexpectedly for a patch with no public parameters: "
                       << dropProcessor->getLastLoadError() << '\n';
            return 1;
        }

        if (dropProcessor->hasPublicParameter ("kick.tune_hz"))
        {
            std::cerr << "reload kept kick.tune_hz visible after the replacement instrument removed it\n";
            return 1;
        }

        if (dropProcessor->getLastReloadWarning().isEmpty())
        {
            std::cerr << "reloadInstrumentFromFile did not warn about parameters dropped by the new instrument\n";
            return 1;
        }

        dropProcessor->releaseResources();
        noPublicParametersFile.deleteFile();
    }

    // Compatible presets should apply as public value changes for the loaded
    // instrument. They must not replace or mutate the immutable instrument YAML.
    {
        auto presetProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        presetProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        presetProcessor->prepareToPlay (48000.0, blockSize);
        const auto yamlBefore = presetProcessor->currentInstrumentYaml();
        const auto presetFile = examplePresetFile ("tight-808-kick.yaml");

        if (! presetProcessor->loadPresetFromFile (presetFile))
        {
            std::cerr << "compatible preset failed to load: " << presetProcessor->getLastPresetError() << '\n';
            return 1;
        }

        if (presetProcessor->currentPresetName() != "Tight 808 Kick" || presetProcessor->currentPresetYaml().isEmpty())
        {
            std::cerr << "loaded preset identity/content was not retained\n";
            return 1;
        }

        auto* decay = findParameter (*presetProcessor, "kick.decay_ms");
        if (decay == nullptr || ! nearlyEqual (decay->getValue(), (420.0f - 50.0f) / (2000.0f - 50.0f), 0.01f))
        {
            std::cerr << "preset value for kick.decay_ms was not applied to mutable parameter state\n";
            return 1;
        }

        if (presetProcessor->currentInstrumentYaml() != yamlBefore)
        {
            std::cerr << "loading a preset mutated the loaded instrument YAML\n";
            return 1;
        }

        presetProcessor->releaseResources();
    }

    // Incompatible or structural presets should be reported, not applied.
    {
        auto rejectProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        rejectProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        rejectProcessor->prepareToPlay (48000.0, blockSize);

        const auto wrongInstrumentPreset = writePresetFile (
            "dandrum_wrong_instrument_preset_",
            "name: Wrong\n"
            "instrument:\n"
            "  id: dandrum.other\n"
            "  preset_schema_version: 1\n"
            "values:\n"
            "  kick.decay_ms: 420\n");
        if (rejectProcessor->loadPresetFromFile (wrongInstrumentPreset))
        {
            std::cerr << "preset targeting another instrument was applied\n";
            return 1;
        }
        if (rejectProcessor->getLastPresetError().isEmpty())
        {
            std::cerr << "wrong-instrument preset did not report an error\n";
            return 1;
        }

        const auto structuralPreset = writePresetFile (
            "dandrum_structural_preset_",
            "name: Structural\n"
            "instrument:\n"
            "  id: dandrum.synthetic-808-kick\n"
            "  preset_schema_version: 1\n"
            "modules: []\n");
        if (rejectProcessor->loadPresetFromFile (structuralPreset))
        {
            std::cerr << "structural preset was applied\n";
            return 1;
        }
        if (! rejectProcessor->getLastPresetError().contains ("structural"))
        {
            std::cerr << "structural preset rejection did not explain the structural field: "
                      << rejectProcessor->getLastPresetError() << '\n';
            return 1;
        }

        rejectProcessor->releaseResources();
        wrongInstrumentPreset.deleteFile();
        structuralPreset.deleteFile();
    }

    // State persistence should embed enough instrument and preset information
    // to restore without depending only on the original absolute file paths.
    {
        auto stateProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        stateProcessor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
        stateProcessor->prepareToPlay (48000.0, blockSize);

        const auto restoredLongDefaultFile = writeModifiedKickPatch (decayControlDefault ("650"), decayControlDefault ("1750"));
        if (! stateProcessor->reloadInstrumentFromFile (restoredLongDefaultFile))
        {
            std::cerr << "state restore setup failed to reload modified instrument: "
                      << stateProcessor->getLastLoadError() << '\n';
            return 1;
        }
        if (! stateProcessor->loadPresetFromFile (examplePresetFile ("tight-808-kick.yaml")))
        {
            std::cerr << "state restore setup failed to load preset: "
                      << stateProcessor->getLastPresetError() << '\n';
            return 1;
        }

        juce::MemoryBlock savedState;
        stateProcessor->getStateInformation (savedState);

        auto restoredStateProcessor = std::make_unique<DandrumAudioProcessor> (InstrumentDemoConfiguration::kick());
        restoredStateProcessor->setStateInformation (savedState.getData(), static_cast<int> (savedState.getSize()));

        if (! restoredStateProcessor->currentInstrumentYaml().contains (decayControlDefault ("1750")))
        {
            std::cerr << "state restore did not restore embedded instrument YAML\n";
            return 1;
        }
        if (restoredStateProcessor->currentPresetName() != "Tight 808 Kick"
            || restoredStateProcessor->currentPresetYaml().isEmpty())
        {
            std::cerr << "state restore did not restore preset identity/content\n";
            return 1;
        }
        if (! restoredStateProcessor->hasPublicParameter ("kick.decay_ms"))
        {
            std::cerr << "state restore did not rebuild the public parameter surface\n";
            return 1;
        }

        stateProcessor->releaseResources();
        restoredStateProcessor->releaseResources();
        restoredLongDefaultFile.deleteFile();
    }

    longDefaultFile.deleteFile();
    return 0;
}
