#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "DefaultPatch.h"

#include <array>
#include <bit>
#include <cstdint>
#include <map>
#include <memory>
#include <vector>

namespace
{
bool sameBitPattern (float a, float b) noexcept
{
    return std::bit_cast<std::uint32_t> (a) == std::bit_cast<std::uint32_t> (b);
}
} // namespace

std::vector<DandrumAudioProcessor::PublicParameterDescriptor> DandrumAudioProcessor::loadPublicParameterDescriptors (
    const std::string& patchPath)
{
    const auto count = dandrum_patch_public_numeric_parameter_count (patchPath.c_str());
    std::vector<PublicParameterDescriptor> descriptors;
    descriptors.reserve (count);

    for (std::size_t index = 0; index < count; ++index)
    {
        std::array<char, 128> id {};
        std::array<char, 128> name {};
        double defaultValue = 0.0;
        double minValue = 0.0;
        double maxValue = 1.0;

        if (dandrum_patch_public_numeric_parameter_descriptor (patchPath.c_str(),
                                                               index,
                                                               id.data(),
                                                               id.size(),
                                                               name.data(),
                                                               name.size(),
                                                               &defaultValue,
                                                               &minValue,
                                                               &maxValue))
        {
            descriptors.push_back ({ juce::String (id.data()),
                                     juce::String (name.data()),
                                     static_cast<float> (defaultValue),
                                     static_cast<float> (minValue),
                                     static_cast<float> (maxValue) });
        }
    }

    return descriptors;
}

juce::AudioProcessorValueTreeState::ParameterLayout DandrumAudioProcessor::createParameterLayout (
    const std::vector<PublicParameterDescriptor>& descriptors)
{
    juce::AudioProcessorValueTreeState::ParameterLayout layout;

    for (const auto& descriptor : descriptors)
    {
        layout.add (std::make_unique<juce::AudioParameterFloat> (
            juce::ParameterID { descriptor.id, 1 },
            descriptor.name,
            juce::NormalisableRange<float> (descriptor.minValue, descriptor.maxValue),
            descriptor.defaultValue));
    }

    return layout;
}

DandrumAudioProcessor::DandrumAudioProcessor()
    : juce::AudioProcessor (BusesProperties()
                                 .withInput ("Input", juce::AudioChannelSet::stereo(), true)
                                 .withOutput ("Output", juce::AudioChannelSet::stereo(), true)),
      defaultParameterDescriptors (loadPublicParameterDescriptors (dandrum::defaultPatchPath().string())),
      parameters (*this, nullptr, "DandrumState", createParameterLayout (defaultParameterDescriptors)),
      engine (dandrum_engine_create())
{
    instrumentLoaded = loadDefaultInstrument();
    preparePublicParameterSlots (defaultParameterDescriptors);
}

DandrumAudioProcessor::~DandrumAudioProcessor()
{
    dandrum_engine_destroy (engine.load (std::memory_order_relaxed));
}

bool DandrumAudioProcessor::loadDefaultInstrument()
{
    if (engine.load (std::memory_order_relaxed) == nullptr)
    {
        lastLoadError = "Rust engine was not created";
        return false;
    }

    const auto patchPath = dandrum::defaultPatchPath();
    const juce::File patchFile (juce::String (patchPath.string()));
    if (! dandrum_engine_load_patch (engine.load (std::memory_order_relaxed), patchPath.string().c_str()))
    {
        lastLoadError = juce::String ("Failed to load default patch: ") + juce::String (patchPath.string());
        return false;
    }

    lastLoadError = {};
    loadedInstrument.sourceFile = patchFile;
    loadedInstrument.yamlContent = patchFile.loadFileAsString();
    return true;
}

void DandrumAudioProcessor::prepareToPlay (double sampleRate, int samplesPerBlock)
{
    auto* activeEngine = engine.load (std::memory_order_relaxed);
    if (activeEngine == nullptr || ! instrumentLoaded)
        return;

    dandrum_engine_prepare_realtime (activeEngine,
                                     static_cast<float> (sampleRate),
                                     static_cast<std::size_t> (juce::jmax (1, samplesPerBlock)));
}

void DandrumAudioProcessor::releaseResources() {}

bool DandrumAudioProcessor::isBusesLayoutSupported (const BusesLayout& layouts) const
{
    if (layouts.getMainOutputChannelSet() != juce::AudioChannelSet::stereo())
        return false;

    const auto input = layouts.getMainInputChannelSet();
    return input == juce::AudioChannelSet::disabled() || input == juce::AudioChannelSet::stereo();
}

void DandrumAudioProcessor::renderSilence (juce::AudioBuffer<float>& buffer) const
{
    buffer.clear();
}

void DandrumAudioProcessor::preparePublicParameterSlots (const juce::File& instrumentFile,
                                                          juce::String* droppedParametersWarning)
{
    preparePublicParameterSlots (loadPublicParameterDescriptors (instrumentFile.getFullPathName().toStdString()),
                                  droppedParametersWarning);
}

void DandrumAudioProcessor::preparePublicParameterSlots (const std::vector<PublicParameterDescriptor>& descriptors,
                                                          juce::String* droppedParametersWarning)
{
    parameterSlots.clear();
    if (droppedParametersWarning != nullptr)
        droppedParametersWarning->clear();

    auto* activeEngine = engine.load (std::memory_order_relaxed);
    if (activeEngine == nullptr || ! instrumentLoaded)
        return;

    // The APVTS parameter layout is fixed at construction time from the
    // default bundled instrument and cannot be safely resized at runtime
    // (see design.md's open question on dynamic parameter layouts). A
    // replacement instrument can therefore only resolve slots for parameter
    // IDs that already exist in that fixed layout; a replacement instrument's
    // own new parameters cannot be exposed as controls in this v1 slice.
    std::map<juce::String, PublicParameterDescriptor> descriptorsById;
    for (const auto& descriptor : descriptors)
        descriptorsById.emplace (descriptor.id, descriptor);

    juce::StringArray droppedParameterIds;

    for (auto* parameter : getParameters())
    {
        auto* hosted = dynamic_cast<juce::HostedAudioProcessorParameter*> (parameter);
        if (hosted == nullptr)
            continue;

        const auto parameterId = hosted->getParameterID();
        ParameterSlot slot;
        slot.rawValue = parameters.getRawParameterValue (parameterId);

        const auto found = descriptorsById.find (parameterId);
        if (found != descriptorsById.end())
        {
            const auto targetCount = dandrum_engine_public_numeric_parameter_target_count (
                activeEngine, parameterId.toRawUTF8());
            for (std::size_t targetIndex = 0; targetIndex < targetCount; ++targetIndex)
            {
                slot.engineSlotIndices.push_back (dandrum_engine_prepare_public_numeric_parameter_slot_at (
                    activeEngine, parameterId.toRawUTF8(), targetIndex));
            }

            const auto currentValue = slot.rawValue != nullptr
                                           ? slot.rawValue->load (std::memory_order_relaxed)
                                           : found->second.defaultValue;

            // Carry the existing APVTS value over onto the replacement
            // instrument's slot(s) now, off the audio thread. Without this,
            // the new engine would keep its own YAML default because
            // applyChangedParameters() only reacts to *changes* in the raw
            // parameter value, and the raw value hasn't changed here — only
            // which engine it's bound to has.
            for (const auto slotIndex : slot.engineSlotIndices)
            {
                if (slotIndex != kNoEngineSlot)
                {
                    dandrum_engine_set_public_numeric_parameter_by_slot (
                        activeEngine, static_cast<std::size_t> (slotIndex), currentValue);
                }
            }
            slot.lastAppliedValue = currentValue;
        }
        else
        {
            // This parameter no longer exists in the replacement instrument;
            // leave engineSlotIndices empty so applyChangedParameters() skips
            // it, and surface the drop instead of absorbing it silently.
            droppedParameterIds.add (parameterId);
        }

        parameterSlots.push_back (slot);
    }

    if (droppedParametersWarning != nullptr && ! droppedParameterIds.isEmpty())
    {
        *droppedParametersWarning = "Instrument no longer defines: "
                                     + droppedParameterIds.joinIntoString (", ");
    }
}

void DandrumAudioProcessor::applyChangedParameters (DandrumEngine* activeEngine) noexcept
{
    if (activeEngine == nullptr)
        return;

    for (auto& slot : parameterSlots)
    {
        if (slot.rawValue == nullptr || slot.engineSlotIndices.empty())
            continue;

        const auto currentValue = slot.rawValue->load (std::memory_order_relaxed);
        if (sameBitPattern (currentValue, slot.lastAppliedValue))
            continue;

        bool appliedAny = false;
        for (const auto slotIndex : slot.engineSlotIndices)
        {
            if (slotIndex == kNoEngineSlot)
                continue;

            if (dandrum_engine_set_public_numeric_parameter_by_slot (
                    activeEngine, static_cast<std::size_t> (slotIndex), currentValue))
            {
                appliedAny = true;
            }
        }

        if (appliedAny)
            slot.lastAppliedValue = currentValue;
    }
}

void DandrumAudioProcessor::setMuted (bool shouldMute) noexcept
{
    muted.store (shouldMute, std::memory_order_relaxed);
}

bool DandrumAudioProcessor::isMuted() const noexcept
{
    return muted.load (std::memory_order_relaxed);
}

void DandrumAudioProcessor::processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    juce::ScopedNoDenormals noDenormals;

    const auto numSamples = buffer.getNumSamples();

    for (auto channel = 2; channel < buffer.getNumChannels(); ++channel)
        buffer.clear (channel, 0, numSamples);

    // Single acquire load: synchronizes-with the release-store in
    // reloadInstrumentFromFile()'s engine swap, and every use below reads the
    // same consistent pointer value for this block.
    auto* activeEngine = engine.load (std::memory_order_acquire);

    if (activeEngine == nullptr || ! instrumentLoaded || isMuted() || numSamples <= 0 || buffer.getNumChannels() <= 0)
    {
        renderSilence (buffer);
        return;
    }

    applyChangedParameters (activeEngine);

    for (const auto metadata : midiMessages)
    {
        const auto message = metadata.getMessage();
        const auto frameOffset = static_cast<std::size_t> (juce::jlimit (0, numSamples - 1, metadata.samplePosition));

        if (message.isNoteOn())
        {
            dandrum_engine_note_on_at (activeEngine,
                                       static_cast<unsigned char> (message.getNoteNumber()),
                                       static_cast<unsigned char> (message.getVelocity() * 127.0f),
                                       frameOffset);
        }
        else if (message.isNoteOff())
        {
            dandrum_engine_note_off_at (activeEngine,
                                        static_cast<unsigned char> (message.getNoteNumber()),
                                        frameOffset);
        }
    }

    auto* left = buffer.getWritePointer (0);
    auto* right = buffer.getNumChannels() > 1 ? buffer.getWritePointer (1) : left;

    dandrum_engine_render (activeEngine, left, right, static_cast<std::size_t> (numSamples));
}

juce::AudioProcessorEditor* DandrumAudioProcessor::createEditor()
{
    return new DandrumAudioProcessorEditor (*this);
}

bool DandrumAudioProcessor::hasEditor() const
{
    return true;
}

const juce::String DandrumAudioProcessor::getName() const
{
    return JucePlugin_Name;
}

bool DandrumAudioProcessor::acceptsMidi() const
{
    return true;
}

bool DandrumAudioProcessor::producesMidi() const
{
    return false;
}

double DandrumAudioProcessor::getTailLengthSeconds() const
{
    return 0.0;
}

int DandrumAudioProcessor::getNumPrograms()
{
    return 1;
}

int DandrumAudioProcessor::getCurrentProgram()
{
    return 0;
}

void DandrumAudioProcessor::setCurrentProgram (int) {}

const juce::String DandrumAudioProcessor::getProgramName (int)
{
    return {};
}

void DandrumAudioProcessor::changeProgramName (int, const juce::String&) {}

void DandrumAudioProcessor::getStateInformation (juce::MemoryBlock& destData)
{
    const auto state = parameters.copyState();
    std::unique_ptr<juce::XmlElement> xml (state.createXml());

    if (xml != nullptr)
        copyXmlToBinary (*xml, destData);
}

void DandrumAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    std::unique_ptr<juce::XmlElement> xml (getXmlFromBinary (data, sizeInBytes));

    if (xml != nullptr && xml->hasTagName (parameters.state.getType()))
        parameters.replaceState (juce::ValueTree::fromXml (*xml));
}

bool DandrumAudioProcessor::isInstrumentLoaded() const noexcept
{
    return instrumentLoaded;
}

const juce::String& DandrumAudioProcessor::getLastLoadError() const noexcept
{
    return lastLoadError;
}

bool DandrumAudioProcessor::hasPublicParameter (juce::StringRef parameterId) const
{
    return parameters.getParameter (parameterId) != nullptr;
}

const juce::File& DandrumAudioProcessor::currentInstrumentFile() const noexcept
{
    return loadedInstrument.sourceFile;
}

const juce::String& DandrumAudioProcessor::currentInstrumentYaml() const noexcept
{
    return loadedInstrument.yamlContent;
}

const juce::String& DandrumAudioProcessor::getLastReloadWarning() const noexcept
{
    return lastReloadWarning;
}

bool DandrumAudioProcessor::reloadInstrumentFromFile (const juce::File& yamlFile)
{
    // Serializes this method against itself: without this lock, two
    // concurrent callers could each publish a candidate engine and then
    // destroy the *other* caller's just-published engine as their own
    // "previous", freeing an engine still in use.
    const std::lock_guard<std::mutex> reloadLock (reloadMutex);

    if (! yamlFile.existsAsFile())
    {
        lastLoadError = "Instrument file does not exist: " + yamlFile.getFullPathName();
        return false;
    }

    if (getSampleRate() <= 0.0)
    {
        lastLoadError = "Cannot reload before prepareToPlay has been called (sample rate is not yet known)";
        return false;
    }

    // Build and validate the replacement engine off the audio thread, without
    // touching the currently active engine at all. If anything here fails,
    // the previously loaded instrument keeps running untouched.
    auto* candidate = dandrum_engine_create();
    if (candidate == nullptr)
    {
        lastLoadError = "Rust engine allocation failed";
        return false;
    }

    dandrum_engine_prepare_realtime (candidate,
                                     static_cast<float> (getSampleRate()),
                                     static_cast<std::size_t> (juce::jmax (1, getBlockSize())));

    const auto path = yamlFile.getFullPathName().toStdString();
    if (! dandrum_engine_load_patch (candidate, path.c_str()))
    {
        lastLoadError = "Failed to load instrument: " + yamlFile.getFullPathName();
        dandrum_engine_destroy (candidate);
        return false;
    }

    const auto yamlText = yamlFile.loadFileAsString();

    // The candidate is fully prepared and valid. Suspend host calls to
    // processBlock and mute as a fallback for hosts/formats that don't fully
    // honour suspension, then publish the replacement.
    suspendProcessing (true);
    setMuted (true);

    auto* previous = engine.exchange (candidate, std::memory_order_acq_rel);

    instrumentLoaded = true;
    lastLoadError.clear();
    loadedInstrument.sourceFile = yamlFile;
    loadedInstrument.yamlContent = yamlText;
    preparePublicParameterSlots (yamlFile, &lastReloadWarning);

    // Give any processBlock call already in flight before suspension a brief
    // moment to finish before the previous engine is destroyed.
    juce::Thread::sleep (5);

    if (previous != nullptr)
        dandrum_engine_destroy (previous);

    setMuted (false);
    suspendProcessing (false);

    return true;
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new DandrumAudioProcessor();
}
