#include "PluginProcessor.h"
#include "PluginEditor.h"
#include "DefaultPatch.h"

#include <algorithm>
#include <array>
#include <bit>
#include <cctype>
#include <cstdint>
#include <cstdlib>
#include <map>
#include <memory>
#include <set>
#include <vector>

namespace
{
bool sameBitPattern (float a, float b) noexcept
{
    return std::bit_cast<std::uint32_t> (a) == std::bit_cast<std::uint32_t> (b);
}

juce::String stripYamlQuotes (juce::String value)
{
    value = value.trim();
    if (value.length() >= 2
        && ((value.startsWithChar ('"') && value.endsWithChar ('"'))
            || (value.startsWithChar ('\'') && value.endsWithChar ('\''))))
    {
        return value.substring (1, value.length() - 1);
    }

    return value;
}

juce::String stripYamlComment (const juce::String& line)
{
    return line.upToFirstOccurrenceOf ("#", false, false).trimEnd();
}

int leadingSpaces (const juce::String& line)
{
    const auto text = line.toStdString();
    int count = 0;
    for (const auto ch : text)
    {
        if (ch != ' ')
            break;
        ++count;
    }
    return count;
}

bool parseDouble (const juce::String& text, double& value)
{
    const auto raw = text.trim().toStdString();
    if (raw.empty())
        return false;

    char* end = nullptr;
    const auto parsed = std::strtod (raw.c_str(), &end);
    if (end == raw.c_str())
        return false;

    while (end != nullptr && *end != '\0')
    {
        if (! std::isspace (static_cast<unsigned char> (*end)))
            return false;
        ++end;
    }

    value = parsed;
    return true;
}

bool isStructuralPresetField (const juce::String& key)
{
    static const std::set<juce::String> structuralFields = {
        "module_definitions", "modules", "connections", "render", "events", "event_sequence",
        "scripts", "scheduling", "schedule", "feedback"
    };

    return structuralFields.contains (key);
}

juce::File writeStateRestoreInstrumentFile (const juce::String& yamlText)
{
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile ("dandrum_restored_instrument_"
                                   + juce::String (juce::Random::getSystemRandom().nextInt())
                                   + ".yaml");
    file.replaceWithText (yamlText);
    return file;
}
} // namespace

juce::String DandrumAudioProcessor::publicSlotParameterId (int slotIndex)
{
    std::array<char, 32> buffer {};
    std::snprintf (buffer.data(), buffer.size(), "dandrum.slot.%02d", slotIndex);
    return juce::String (buffer.data());
}

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

juce::AudioProcessorValueTreeState::ParameterLayout DandrumAudioProcessor::createParameterLayout()
{
    juce::AudioProcessorValueTreeState::ParameterLayout layout;

    for (int slotIndex = 0; slotIndex < kPublicParameterSlotCount; ++slotIndex)
    {
        layout.add (std::make_unique<juce::AudioParameterFloat> (
            juce::ParameterID { publicSlotParameterId (slotIndex), 1 },
            "Public Parameter Slot " + juce::String (slotIndex + 1),
            juce::NormalisableRange<float> (0.0f, 1.0f),
            0.0f));
    }

    return layout;
}

bool DandrumAudioProcessor::readInstrumentIdentity (const juce::String& yaml, juce::String& instrumentId, int& schemaVersion)
{
    juce::String section;
    juce::StringArray lines;
    lines.addLines (yaml);

    for (const auto& rawLine : lines)
    {
        const auto line = stripYamlComment (rawLine);
        const auto trimmed = line.trim();
        if (trimmed.isEmpty())
            continue;

        const auto indent = leadingSpaces (line);
        if (indent == 0 && trimmed.endsWithChar (':'))
        {
            section = trimmed.dropLastCharacters (1).trim();
            continue;
        }

        const auto key = trimmed.upToFirstOccurrenceOf (":", false, false).trim();
        const auto value = stripYamlQuotes (trimmed.fromFirstOccurrenceOf (":", false, false));

        if (section == "instrument" && indent >= 2)
        {
            if (key == "id")
                instrumentId = value;
            else if (key == "preset_schema_version")
                schemaVersion = value.getIntValue();
        }
    }

    return instrumentId.isNotEmpty() && schemaVersion > 0;
}

DandrumAudioProcessor::ParsedPreset DandrumAudioProcessor::parsePresetFile (const juce::File& presetFile)
{
    ParsedPreset preset;

    if (! presetFile.existsAsFile())
    {
        preset.error = "Preset file does not exist: " + presetFile.getFullPathName();
        return preset;
    }

    preset.yamlContent = presetFile.loadFileAsString();
    juce::String section;
    juce::StringArray lines;
    lines.addLines (preset.yamlContent);

    for (const auto& rawLine : lines)
    {
        const auto line = stripYamlComment (rawLine);
        const auto trimmed = line.trim();
        if (trimmed.isEmpty())
            continue;

        const auto indent = leadingSpaces (line);
        const auto key = trimmed.upToFirstOccurrenceOf (":", false, false).trim();
        const auto value = stripYamlQuotes (trimmed.fromFirstOccurrenceOf (":", false, false));

        if (indent == 0)
        {
            if (isStructuralPresetField (key))
            {
                preset.error = "Preset document cannot declare structural field: " + key;
                return preset;
            }

            section = key;
            if (key == "name" && value.isNotEmpty())
                preset.name = value;
            continue;
        }

        if (section == "instrument" && indent >= 2)
        {
            if (key == "id")
                preset.instrumentId = value;
            else if (key == "preset_schema_version")
                preset.presetSchemaVersion = value.getIntValue();
        }
        else if (section == "values" && indent >= 2)
        {
            double parsed = 0.0;
            if (! parseDouble (value, parsed))
            {
                preset.error = "Preset value for " + key + " is not numeric";
                return preset;
            }
            preset.values[key] = parsed;
        }
    }

    if (preset.name.isEmpty())
        preset.error = "Preset is missing name";
    else if (preset.instrumentId.isEmpty() || preset.presetSchemaVersion <= 0)
        preset.error = "Preset is missing instrument identity/schema version";

    return preset;
}

float DandrumAudioProcessor::clampToDescriptorRange (const PublicParameterDescriptor& descriptor, float value) noexcept
{
    if (descriptor.minValue <= descriptor.maxValue)
        return juce::jlimit (descriptor.minValue, descriptor.maxValue, value);

    return value;
}

float DandrumAudioProcessor::normalisePublicValue (const PublicParameterDescriptor& descriptor, float value) noexcept
{
    const auto clamped = clampToDescriptorRange (descriptor, value);
    const auto width = descriptor.maxValue - descriptor.minValue;
    if (width <= 0.0f)
        return 0.0f;

    return juce::jlimit (0.0f, 1.0f, (clamped - descriptor.minValue) / width);
}

float DandrumAudioProcessor::denormalisePublicValue (const PublicParameterDescriptor& descriptor, float normalisedValue) noexcept
{
    const auto width = descriptor.maxValue - descriptor.minValue;
    if (width <= 0.0f)
        return descriptor.defaultValue;

    return clampToDescriptorRange (descriptor, descriptor.minValue + juce::jlimit (0.0f, 1.0f, normalisedValue) * width);
}

DandrumAudioProcessor::DandrumAudioProcessor()
    : juce::AudioProcessor (BusesProperties()
                                 .withInput ("Input", juce::AudioChannelSet::stereo(), true)
                                 .withOutput ("Output", juce::AudioChannelSet::stereo(), true)),
      parameters (*this, nullptr, "DandrumState", createParameterLayout()),
      engine (dandrum_engine_create())
{
    parameterSlots.resize (kPublicParameterSlotCount);
    for (int slotIndex = 0; slotIndex < kPublicParameterSlotCount; ++slotIndex)
    {
        parameterSlots[slotIndex].slotParameterId = publicSlotParameterId (slotIndex);
        parameterSlots[slotIndex].rawValue = parameters.getRawParameterValue (parameterSlots[slotIndex].slotParameterId);
    }

    instrumentLoaded = loadDefaultInstrument();
    if (instrumentLoaded)
        preparePublicParameterSlots (loadPublicParameterDescriptors (dandrum::defaultPatchPath().string()), nullptr, false);
}

DandrumAudioProcessor::~DandrumAudioProcessor()
{
    dandrum_engine_destroy (engine.load (std::memory_order_relaxed));
}

bool DandrumAudioProcessor::loadDefaultInstrument()
{
    auto* activeEngine = engine.load (std::memory_order_relaxed);
    if (activeEngine == nullptr)
    {
        lastLoadError = "Rust engine was not created";
        return false;
    }

    const auto patchPath = dandrum::defaultPatchPath();
    const juce::File patchFile (juce::String (patchPath.string()));
    if (! dandrum_engine_load_patch (activeEngine, patchPath.string().c_str()))
    {
        lastLoadError = juce::String ("Failed to load default patch: ") + juce::String (patchPath.string());
        return false;
    }

    const auto yamlText = patchFile.loadFileAsString();
    juce::String instrumentId;
    int schemaVersion = 0;
    readInstrumentIdentity (yamlText, instrumentId, schemaVersion);

    lastLoadError = {};
    loadedInstrument.sourceFile = patchFile;
    loadedInstrument.yamlContent = yamlText;
    loadedInstrument.instrumentId = instrumentId;
    loadedInstrument.presetSchemaVersion = schemaVersion;
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

bool DandrumAudioProcessor::replaceActiveEngineFromFile (const juce::File& yamlFile,
                                                         const juce::File& sourceHint,
                                                         const juce::String& yamlText,
                                                         bool requirePreparedHost,
                                                         bool preferCurrentSlotValues,
                                                         juce::String* reloadWarning)
{
    if (! yamlFile.existsAsFile())
    {
        lastLoadError = "Instrument file does not exist: " + yamlFile.getFullPathName();
        replacementState.store (static_cast<int> (ReplacementState::Failed), std::memory_order_relaxed);
        return false;
    }

    const auto sampleRate = getSampleRate() > 0.0 ? getSampleRate() : 44100.0;
    const auto blockSize = getBlockSize() > 0 ? getBlockSize() : 512;
    if (requirePreparedHost && getSampleRate() <= 0.0)
    {
        lastLoadError = "Cannot reload before prepareToPlay has been called (sample rate is not yet known)";
        replacementState.store (static_cast<int> (ReplacementState::Failed), std::memory_order_relaxed);
        return false;
    }

    replacementState.store (static_cast<int> (ReplacementState::Validating), std::memory_order_relaxed);

    auto* candidate = dandrum_engine_create();
    if (candidate == nullptr)
    {
        lastLoadError = "Rust engine allocation failed";
        replacementState.store (static_cast<int> (ReplacementState::Failed), std::memory_order_relaxed);
        return false;
    }

    dandrum_engine_prepare_realtime (candidate,
                                     static_cast<float> (sampleRate),
                                     static_cast<std::size_t> (juce::jmax (1, blockSize)));

    replacementState.store (static_cast<int> (ReplacementState::Compiling), std::memory_order_relaxed);
    const auto path = yamlFile.getFullPathName().toStdString();
    if (! dandrum_engine_load_patch (candidate, path.c_str()))
    {
        lastLoadError = "Failed to load instrument: " + yamlFile.getFullPathName();
        dandrum_engine_destroy (candidate);
        replacementState.store (static_cast<int> (ReplacementState::Failed), std::memory_order_relaxed);
        return false;
    }

    juce::String instrumentId;
    int schemaVersion = 0;
    readInstrumentIdentity (yamlText, instrumentId, schemaVersion);

    replacementState.store (static_cast<int> (ReplacementState::Muted), std::memory_order_relaxed);
    suspendProcessing (true);
    setMuted (true);

    auto* previous = engine.exchange (candidate, std::memory_order_acq_rel);

    instrumentLoaded = true;
    lastLoadError.clear();
    loadedInstrument.sourceFile = sourceHint;
    loadedInstrument.yamlContent = yamlText;
    loadedInstrument.instrumentId = instrumentId;
    loadedInstrument.presetSchemaVersion = schemaVersion;
    preparePublicParameterSlots (yamlFile, reloadWarning, preferCurrentSlotValues);

    juce::Thread::sleep (5);

    if (previous != nullptr)
        dandrum_engine_destroy (previous);

    setMuted (false);
    suspendProcessing (false);
    replacementState.store (static_cast<int> (ReplacementState::Running), std::memory_order_relaxed);

    return true;
}

void DandrumAudioProcessor::preparePublicParameterSlots (const juce::File& instrumentFile,
                                                          juce::String* droppedParametersWarning,
                                                          bool preferCurrentSlotValues)
{
    preparePublicParameterSlots (loadPublicParameterDescriptors (instrumentFile.getFullPathName().toStdString()),
                                  droppedParametersWarning,
                                  preferCurrentSlotValues);
}

void DandrumAudioProcessor::preparePublicParameterSlots (const std::vector<PublicParameterDescriptor>& descriptors,
                                                          juce::String* droppedParametersWarning,
                                                          bool preferCurrentSlotValues)
{
    if (droppedParametersWarning != nullptr)
        droppedParametersWarning->clear();

    auto* activeEngine = engine.load (std::memory_order_relaxed);
    if (activeEngine == nullptr || ! instrumentLoaded)
        return;

    std::map<juce::String, float> carriedValuesByPublicId;
    juce::StringArray oldPublicIds;
    for (const auto& slot : parameterSlots)
    {
        if (! slot.active)
            continue;

        oldPublicIds.add (slot.descriptor.id);
        if (slot.rawValue != nullptr)
            carriedValuesByPublicId[slot.descriptor.id] = denormalisePublicValue (slot.descriptor, slot.rawValue->load (std::memory_order_relaxed));
    }

    std::set<juce::String> newPublicIds;
    juce::StringArray droppedParameterIds;

    for (auto& slot : parameterSlots)
    {
        slot.active = false;
        slot.descriptor = {};
        slot.engineSlotIndices.clear();
        slot.lastAppliedNormalisedValue = slot.rawValue != nullptr ? slot.rawValue->load (std::memory_order_relaxed) : 0.0f;
    }

    const auto activeCount = juce::jmin (static_cast<int> (descriptors.size()), kPublicParameterSlotCount);
    for (int slotIndex = 0; slotIndex < activeCount; ++slotIndex)
    {
        auto& slot = parameterSlots[slotIndex];
        slot.active = true;
        slot.descriptor = descriptors[static_cast<std::size_t> (slotIndex)];
        newPublicIds.insert (slot.descriptor.id);

        float actualValue = slot.descriptor.defaultValue;
        if (preferCurrentSlotValues && slot.rawValue != nullptr)
            actualValue = denormalisePublicValue (slot.descriptor, slot.rawValue->load (std::memory_order_relaxed));
        else if (const auto carried = carriedValuesByPublicId.find (slot.descriptor.id); carried != carriedValuesByPublicId.end())
            actualValue = clampToDescriptorRange (slot.descriptor, carried->second);

        const auto normalisedValue = normalisePublicValue (slot.descriptor, actualValue);
        setSlotNormalisedValue (slotIndex, normalisedValue, false);

        const auto targetCount = dandrum_engine_public_numeric_parameter_target_count (activeEngine, slot.descriptor.id.toRawUTF8());
        for (std::size_t targetIndex = 0; targetIndex < targetCount; ++targetIndex)
        {
            slot.engineSlotIndices.push_back (dandrum_engine_prepare_public_numeric_parameter_slot_at (
                activeEngine, slot.descriptor.id.toRawUTF8(), targetIndex));
        }

        applySlotToEngine (slot, normalisedValue, activeEngine);
        slot.lastAppliedNormalisedValue = normalisedValue;
    }

    for (const auto& publicId : oldPublicIds)
        if (! newPublicIds.contains (publicId))
            droppedParameterIds.add (publicId);

    if (droppedParametersWarning != nullptr)
    {
        if (! droppedParameterIds.isEmpty())
            *droppedParametersWarning = "Instrument no longer defines: " + droppedParameterIds.joinIntoString (", ");

        if (static_cast<int> (descriptors.size()) > kPublicParameterSlotCount)
        {
            if (droppedParametersWarning->isNotEmpty())
                *droppedParametersWarning << "; ";
            *droppedParametersWarning << "Instrument declares " << static_cast<int> (descriptors.size())
                                      << " public parameters; only " << kPublicParameterSlotCount
                                      << " fixed host slots are available";
        }
    }

    parameterSurfaceGeneration.fetch_add (1, std::memory_order_relaxed);
}

void DandrumAudioProcessor::setSlotNormalisedValue (int slotIndex, float normalisedValue, bool notifyHost)
{
    if (! juce::isPositiveAndBelow (slotIndex, kPublicParameterSlotCount))
        return;

    auto* parameter = parameters.getParameter (publicSlotParameterId (slotIndex));
    if (parameter == nullptr)
        return;

    const auto value = juce::jlimit (0.0f, 1.0f, normalisedValue);
    if (notifyHost)
        parameter->setValueNotifyingHost (value);
    else
        parameter->setValue (value);
}

void DandrumAudioProcessor::applySlotToEngine (ParameterSlot& slot, float normalisedValue, DandrumEngine* activeEngine) noexcept
{
    if (activeEngine == nullptr || ! slot.active || slot.engineSlotIndices.empty())
        return;

    const auto actualValue = denormalisePublicValue (slot.descriptor, normalisedValue);
    bool appliedAny = false;
    for (const auto slotIndex : slot.engineSlotIndices)
    {
        if (slotIndex == kNoEngineSlot)
            continue;

        if (dandrum_engine_set_public_numeric_parameter_by_slot (
                activeEngine, static_cast<std::size_t> (slotIndex), actualValue))
        {
            appliedAny = true;
        }
    }

    if (appliedAny)
        slot.lastAppliedNormalisedValue = normalisedValue;
}

void DandrumAudioProcessor::applyChangedParameters (DandrumEngine* activeEngine) noexcept
{
    if (activeEngine == nullptr)
        return;

    for (auto& slot : parameterSlots)
    {
        if (! slot.active || slot.rawValue == nullptr || slot.engineSlotIndices.empty())
            continue;

        const auto currentValue = slot.rawValue->load (std::memory_order_relaxed);
        if (sameBitPattern (currentValue, slot.lastAppliedNormalisedValue))
            continue;

        applySlotToEngine (slot, currentValue, activeEngine);
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
    return loadedPreset.name;
}

void DandrumAudioProcessor::changeProgramName (int, const juce::String&) {}

void DandrumAudioProcessor::getStateInformation (juce::MemoryBlock& destData)
{
    auto state = parameters.copyState();
    state.setProperty ("dandrum_schema_version", kPluginStateSchemaVersion, nullptr);
    state.setProperty ("instrument_path", loadedInstrument.sourceFile.getFullPathName(), nullptr);
    state.setProperty ("instrument_yaml", loadedInstrument.yamlContent, nullptr);
    state.setProperty ("instrument_id", loadedInstrument.instrumentId, nullptr);
    state.setProperty ("preset_schema_version", loadedInstrument.presetSchemaVersion, nullptr);
    state.setProperty ("preset_path", loadedPreset.sourceFile.getFullPathName(), nullptr);
    state.setProperty ("preset_name", loadedPreset.name, nullptr);
    state.setProperty ("preset_yaml", loadedPreset.yamlContent, nullptr);

    std::unique_ptr<juce::XmlElement> xml (state.createXml());
    if (xml != nullptr)
        copyXmlToBinary (*xml, destData);
}

void DandrumAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    std::unique_ptr<juce::XmlElement> xml (getXmlFromBinary (data, sizeInBytes));
    if (xml == nullptr || ! xml->hasTagName (parameters.state.getType()))
        return;

    auto state = juce::ValueTree::fromXml (*xml);
    const auto schemaVersion = static_cast<int> (state.getProperty ("dandrum_schema_version", 0));
    if (schemaVersion != 0 && schemaVersion != kPluginStateSchemaVersion)
    {
        lastLoadError = "Unsupported Dandrum plugin state schema version: " + juce::String (schemaVersion);
        return;
    }

    const auto instrumentYaml = state.getProperty ("instrument_yaml").toString();
    const juce::File sourceHint (state.getProperty ("instrument_path").toString());
    juce::File restoreFile;

    DandrumEngine* candidate = nullptr;
    if (instrumentYaml.isNotEmpty())
    {
        restoreFile = writeStateRestoreInstrumentFile (instrumentYaml);
        candidate = dandrum_engine_create();
        if (candidate == nullptr)
        {
            lastLoadError = "Rust engine allocation failed while restoring plugin state";
            return;
        }

        const auto sampleRate = getSampleRate() > 0.0 ? getSampleRate() : 44100.0;
        const auto blockSize = getBlockSize() > 0 ? getBlockSize() : 512;
        dandrum_engine_prepare_realtime (candidate,
                                         static_cast<float> (sampleRate),
                                         static_cast<std::size_t> (juce::jmax (1, blockSize)));

        if (! dandrum_engine_load_patch (candidate, restoreFile.getFullPathName().toStdString().c_str()))
        {
            dandrum_engine_destroy (candidate);
            lastLoadError = "Failed to restore embedded instrument from plugin state";
            return;
        }
    }

    const std::lock_guard<std::mutex> reloadLock (reloadMutex);
    parameters.replaceState (state);

    if (candidate != nullptr)
    {
        suspendProcessing (true);
        setMuted (true);
        auto* previous = engine.exchange (candidate, std::memory_order_acq_rel);

        juce::String instrumentId;
        int schema = 0;
        readInstrumentIdentity (instrumentYaml, instrumentId, schema);
        instrumentLoaded = true;
        loadedInstrument.sourceFile = sourceHint;
        loadedInstrument.yamlContent = instrumentYaml;
        loadedInstrument.instrumentId = instrumentId;
        loadedInstrument.presetSchemaVersion = schema;
        preparePublicParameterSlots (restoreFile, &lastReloadWarning, true);

        juce::Thread::sleep (5);
        if (previous != nullptr)
            dandrum_engine_destroy (previous);

        setMuted (false);
        suspendProcessing (false);
    }
    else if (instrumentLoaded)
    {
        preparePublicParameterSlots (loadedInstrument.sourceFile, &lastReloadWarning, true);
    }

    loadedPreset.sourceFile = juce::File (state.getProperty ("preset_path").toString());
    loadedPreset.name = state.getProperty ("preset_name").toString();
    loadedPreset.yamlContent = state.getProperty ("preset_yaml").toString();
    lastLoadError.clear();
}

bool DandrumAudioProcessor::isInstrumentLoaded() const noexcept
{
    return instrumentLoaded;
}

const juce::String& DandrumAudioProcessor::getLastLoadError() const noexcept
{
    return lastLoadError;
}

const juce::String& DandrumAudioProcessor::getLastPresetError() const noexcept
{
    return lastPresetError;
}

bool DandrumAudioProcessor::hasPublicParameter (juce::StringRef parameterId) const
{
    return getParameterForPublicId (parameterId) != nullptr;
}

juce::RangedAudioParameter* DandrumAudioProcessor::getParameterForPublicId (juce::StringRef parameterId) const
{
    for (const auto& slot : parameterSlots)
    {
        if (slot.active && slot.descriptor.id == parameterId)
            return parameters.getParameter (slot.slotParameterId);
    }

    return nullptr;
}

juce::String DandrumAudioProcessor::getPublicParameterDisplayName (juce::StringRef parameterId) const
{
    for (const auto& slot : parameterSlots)
        if (slot.active && slot.descriptor.id == parameterId)
            return slot.descriptor.name.isNotEmpty() ? slot.descriptor.name : slot.descriptor.id;

    return {};
}

juce::StringArray DandrumAudioProcessor::getActivePublicParameterIds() const
{
    juce::StringArray ids;
    for (const auto& slot : parameterSlots)
        if (slot.active)
            ids.add (slot.descriptor.id);

    return ids;
}

std::uint32_t DandrumAudioProcessor::getParameterSurfaceGeneration() const noexcept
{
    return parameterSurfaceGeneration.load (std::memory_order_relaxed);
}

const juce::File& DandrumAudioProcessor::currentInstrumentFile() const noexcept
{
    return loadedInstrument.sourceFile;
}

const juce::String& DandrumAudioProcessor::currentInstrumentYaml() const noexcept
{
    return loadedInstrument.yamlContent;
}

const juce::String& DandrumAudioProcessor::currentPresetName() const noexcept
{
    return loadedPreset.name;
}

const juce::String& DandrumAudioProcessor::currentPresetYaml() const noexcept
{
    return loadedPreset.yamlContent;
}

const juce::String& DandrumAudioProcessor::getLastReloadWarning() const noexcept
{
    return lastReloadWarning;
}

juce::String DandrumAudioProcessor::replacementTransactionState() const
{
    switch (static_cast<ReplacementState> (replacementState.load (std::memory_order_relaxed)))
    {
        case ReplacementState::Running: return "running";
        case ReplacementState::Validating: return "validating";
        case ReplacementState::Muted: return "muted";
        case ReplacementState::Compiling: return "compiling";
        case ReplacementState::Failed: return "failed";
    }

    return "unknown";
}

std::size_t DandrumAudioProcessor::getDroppedMidiEventCount() const noexcept
{
    return droppedMidiEventCount.load (std::memory_order_relaxed);
}

bool DandrumAudioProcessor::reloadInstrumentFromFile (const juce::File& yamlFile)
{
    const std::lock_guard<std::mutex> reloadLock (reloadMutex);
    const auto yamlText = yamlFile.loadFileAsString();
    loadedPreset = {};
    return replaceActiveEngineFromFile (yamlFile, yamlFile, yamlText, true, false, &lastReloadWarning);
}

bool DandrumAudioProcessor::loadPresetFromFile (const juce::File& presetFile)
{
    lastPresetError.clear();

    if (! instrumentLoaded)
    {
        lastPresetError = "Cannot load preset before an instrument is loaded";
        return false;
    }

    auto preset = parsePresetFile (presetFile);
    if (preset.error.isNotEmpty())
    {
        lastPresetError = preset.error;
        return false;
    }

    if (preset.instrumentId != loadedInstrument.instrumentId
        || preset.presetSchemaVersion != loadedInstrument.presetSchemaVersion)
    {
        lastPresetError = "Preset targets " + preset.instrumentId + " schema " + juce::String (preset.presetSchemaVersion)
                          + ", but loaded instrument is " + loadedInstrument.instrumentId
                          + " schema " + juce::String (loadedInstrument.presetSchemaVersion);
        return false;
    }

    for (const auto& [publicId, _] : preset.values)
    {
        if (! hasPublicParameter (publicId))
        {
            lastPresetError = "Preset value targets unknown public parameter: " + publicId;
            return false;
        }
    }

    auto* activeEngine = engine.load (std::memory_order_relaxed);
    if (activeEngine == nullptr)
    {
        lastPresetError = "Rust engine is not available";
        return false;
    }

    for (int slotIndex = 0; slotIndex < static_cast<int> (parameterSlots.size()); ++slotIndex)
    {
        auto& slot = parameterSlots[static_cast<std::size_t> (slotIndex)];
        if (! slot.active)
            continue;

        const auto found = preset.values.find (slot.descriptor.id);
        if (found == preset.values.end())
            continue;

        const auto normalised = normalisePublicValue (slot.descriptor, static_cast<float> (found->second));
        setSlotNormalisedValue (slotIndex, normalised, true);
        applySlotToEngine (slot, normalised, activeEngine);
    }

    loadedPreset.sourceFile = presetFile;
    loadedPreset.name = preset.name;
    loadedPreset.yamlContent = preset.yamlContent;
    return true;
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new DandrumAudioProcessor();
}
