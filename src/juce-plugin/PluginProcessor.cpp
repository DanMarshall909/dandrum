#include "PluginProcessor.h"
#include "PluginEditor.h"

#include <algorithm>
#include <array>
#include <bit>
#include <cctype>
#include <cstdint>
#include <cstdlib>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace
{
bool sameBitPattern (float a, float b) noexcept
{
    return std::bit_cast<std::uint32_t> (a) == std::bit_cast<std::uint32_t> (b);
}

std::string copyUiText (DandrumKernelStringView view)
{
    return view.size == 0 ? std::string() : std::string (view.data, view.size);
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

void readInlineInstrumentFields (const juce::String& value, juce::String& instrumentId,
                                 int& schemaVersion)
{
    const auto text = value.trim();
    if (! text.startsWithChar ('{') || ! text.endsWithChar ('}'))
        return;

    const auto fields = juce::StringArray::fromTokens (text.substring (1, text.length() - 1),
                                                        ",", "\"'");
    for (const auto& field : fields)
    {
        const auto key = field.upToFirstOccurrenceOf (":", false, false).trim();
        const auto fieldValue = stripYamlQuotes (field.fromFirstOccurrenceOf (":", false, false));
        if (key == "id")
            instrumentId = fieldValue;
        else if (key == "preset_schema_version")
            schemaVersion = fieldValue.getIntValue();
    }
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

juce::File writeStateRestoreInstrumentFile (const juce::String& yamlText,
                                            const juce::File& assetRoot)
{
    const auto root = assetRoot.isDirectory()
                          ? assetRoot : juce::File::getSpecialLocation (juce::File::tempDirectory);
    auto file = root.getNonexistentChildFile (".dandrum_restored_instrument_", ".yaml", false);
    if (! file.replaceWithText (yamlText))
        return {};
    return file;
}

DandrumKernelInstrument* prepareKernelWithPublicControls (const std::string& path,
                                                          std::uint32_t sampleRate,
                                                          std::size_t blockSize)
{
    const auto parameterCount = dandrum_patch_public_numeric_parameter_count (path.c_str());
    std::vector<std::string> controlNames;
    controlNames.reserve (parameterCount);
    for (std::size_t index = 0; index < parameterCount; ++index)
    {
        std::array<char, 128> name {};
        if (! dandrum_patch_public_numeric_parameter_port_name (path.c_str(), index,
                                                                name.data(), name.size())
            || name[0] == '\0')
            return nullptr;

        if (std::find (controlNames.begin(), controlNames.end(), name.data()) == controlNames.end())
            controlNames.emplace_back (name.data());
    }

    std::vector<DandrumKernelBusDeclaration> buses;
    buses.reserve (controlNames.size() + 1);
    buses.push_back ({ "master", 2, 2 });
    for (const auto& name : controlNames)
        buses.push_back ({ name.c_str(), 1, 1 });

    return dandrum_kernel_prepare_file (path.c_str(), sampleRate, blockSize,
                                        buses.data(), buses.size());
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

juce::AudioProcessorValueTreeState::ParameterLayout DandrumAudioProcessor::createParameterLayout (
    const InstrumentDemoConfiguration& initialConfiguration)
{
    juce::AudioProcessorValueTreeState::ParameterLayout layout;
    const auto initialDescriptors = loadPublicParameterDescriptors (initialConfiguration.instrumentPath.string());

    for (int slotIndex = 0; slotIndex < kPublicParameterSlotCount; ++slotIndex)
    {
        const auto name = static_cast<std::size_t> (slotIndex) < initialDescriptors.size()
                              ? initialDescriptors[static_cast<std::size_t> (slotIndex)].name
                              : "Public Parameter Slot " + juce::String (slotIndex + 1);
        layout.add (std::make_unique<juce::AudioParameterFloat> (
            juce::ParameterID { publicSlotParameterId (slotIndex), 1 },
            name,
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

        if (indent == 0 && key == "instrument")
        {
            section = key;
            readInlineInstrumentFields (value, instrumentId, schemaVersion);
            continue;
        }

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
            else if (key == "instrument")
                readInlineInstrumentFields (value, preset.instrumentId, preset.presetSchemaVersion);
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

DandrumAudioProcessor::DandrumAudioProcessor (InstrumentDemoConfiguration demo)
    : juce::AudioProcessor (BusesProperties()
                                 .withInput ("Input", juce::AudioChannelSet::stereo(), true)
                                 .withOutput ("Output", juce::AudioChannelSet::stereo(), true)),
      configuration (std::move (demo)),
      parameters (*this, nullptr, "DandrumState", createParameterLayout (configuration))
{
    parameterSlots.resize (kPublicParameterSlotCount);
    for (int slotIndex = 0; slotIndex < kPublicParameterSlotCount; ++slotIndex)
    {
        parameterSlots[slotIndex].slotParameterId = publicSlotParameterId (slotIndex);
        parameterSlots[slotIndex].rawValue = parameters.getRawParameterValue (parameterSlots[slotIndex].slotParameterId);
    }

    instrumentLoaded = loadDefaultInstrument();
    if (instrumentLoaded)
        preparePublicParameterSlots (loadPublicParameterDescriptors (configuration.instrumentPath.string()), nullptr, false);

    instrumentFileWatcher.onReload ([this] (const juce::File& changedFile) { reloadInstrumentFromFile (changedFile); });
    if (instrumentLoaded && loadedInstrument.sourceFile.existsAsFile())
        instrumentFileWatcher.watchFile (loadedInstrument.sourceFile);
}

DandrumAudioProcessor::~DandrumAudioProcessor()
{
    dandrum_kernel_destroy (kernel.load (std::memory_order_relaxed));
}

bool DandrumAudioProcessor::loadDefaultInstrument()
{
    const auto& patchPath = configuration.instrumentPath;
    const juce::File patchFile (juce::String (patchPath.string()));
    auto* preparedKernel = prepareKernelWithPublicControls (patchPath.string(), 44100, 512);
    if (preparedKernel == nullptr)
    {
        lastLoadError = juce::String ("Failed to load default patch: ") + juce::String (patchPath.string());
        return false;
    }
    kernel.store (preparedKernel, std::memory_order_relaxed);

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
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (! instrumentLoaded)
        return;

    const auto blockSize = static_cast<std::size_t> (juce::jmax (1, samplesPerBlock));
    if (auto* activeKernel = kernel.load (std::memory_order_relaxed))
    {
        auto* replacement = prepareKernelWithPublicControls (
            loadedInstrument.sourceFile.getFullPathName().toStdString(),
            static_cast<std::uint32_t> (juce::jmax (1.0, sampleRate)), blockSize);
        if (replacement != nullptr)
        {
            kernel.store (replacement, std::memory_order_relaxed);
            for (auto& slot : parameterSlots)
                if (slot.active && slot.kernelSlotIndex != kNoEngineSlot && slot.rawValue != nullptr)
                    applySlotToKernel (slot, slot.rawValue->load (std::memory_order_relaxed), replacement);
            dandrum_kernel_destroy (activeKernel);
        }
    }
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

    const auto path = yamlFile.getFullPathName().toStdString();
    auto* candidateKernel = prepareKernelWithPublicControls (
        path, static_cast<std::uint32_t> (juce::jmax (1.0, sampleRate)),
        static_cast<std::size_t> (juce::jmax (1, blockSize)));
    if (candidateKernel == nullptr)
    {
        lastLoadError = "Failed to load instrument: " + yamlFile.getFullPathName();
        replacementState.store (static_cast<int> (ReplacementState::Failed), std::memory_order_relaxed);
        return false;
    }

    juce::String instrumentId;
    int schemaVersion = 0;
    readInstrumentIdentity (yamlText, instrumentId, schemaVersion);

    replacementState.store (static_cast<int> (ReplacementState::Muted), std::memory_order_relaxed);
    suspendProcessing (true);
    setMuted (true);

    auto* previousKernel = kernel.exchange (candidateKernel, std::memory_order_acq_rel);

    instrumentLoaded = true;
    lastLoadError.clear();
    loadedInstrument.sourceFile = sourceHint;
    loadedInstrument.yamlContent = yamlText;
    loadedInstrument.instrumentId = instrumentId;
    loadedInstrument.presetSchemaVersion = schemaVersion;
    preparePublicParameterSlots (yamlFile, reloadWarning, preferCurrentSlotValues);

    juce::Thread::sleep (5);

    dandrum_kernel_destroy (previousKernel);

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

    auto* activeKernel = kernel.load (std::memory_order_relaxed);
    if (activeKernel == nullptr || ! instrumentLoaded)
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
        slot.kernelSlotIndex = kNoEngineSlot;
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
        setSlotNormalisedValue (slotIndex, normalisedValue);

        slot.kernelSlotIndex = slotIndex;
        applySlotToKernel (slot, normalisedValue, activeKernel);
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

void DandrumAudioProcessor::setSlotNormalisedValue (int slotIndex, float normalisedValue)
{
    if (! juce::isPositiveAndBelow (slotIndex, kPublicParameterSlotCount))
        return;

    auto* parameter = parameters.getParameter (publicSlotParameterId (slotIndex));
    if (parameter == nullptr)
        return;

    const auto value = juce::jlimit (0.0f, 1.0f, normalisedValue);
    parameter->setValueNotifyingHost (value);
}

void DandrumAudioProcessor::applySlotToKernel (ParameterSlot& slot,
                                              float normalisedValue,
                                              DandrumKernelInstrument* activeKernel) noexcept
{
    if (activeKernel == nullptr || ! slot.active || slot.kernelSlotIndex == kNoEngineSlot)
        return;

    const auto actualValue = denormalisePublicValue (slot.descriptor, normalisedValue);
    if (dandrum_kernel_set_public_numeric_parameter_by_slot (
            activeKernel, static_cast<std::size_t> (slot.kernelSlotIndex), actualValue))
        slot.lastAppliedNormalisedValue = normalisedValue;
}

void DandrumAudioProcessor::applyChangedParameters (DandrumKernelInstrument* activeKernel) noexcept
{
    if (activeKernel == nullptr)
        return;

    for (auto& slot : parameterSlots)
    {
        if (! slot.active || slot.rawValue == nullptr)
            continue;

        const auto currentValue = slot.rawValue->load (std::memory_order_relaxed);
        if (sameBitPattern (currentValue, slot.lastAppliedNormalisedValue))
            continue;

        applySlotToKernel (slot, currentValue, activeKernel);
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

bool DandrumAudioProcessor::enqueueEditorMidiEvent (EditorMidiEvent event) noexcept
{
    const auto scope = editorMidiFifo.write (1);
    if (scope.blockSize1 == 0)
    {
        droppedMidiEventCount.fetch_add (1, std::memory_order_relaxed);
        return false;
    }

    editorMidiEvents[static_cast<std::size_t> (scope.startIndex1)] = event;
    return true;
}

bool DandrumAudioProcessor::enqueueEditorNoteOn (int noteNumber, float velocity) noexcept
{
    const auto note = static_cast<std::uint8_t> (juce::jlimit (0, 127, noteNumber));
    const auto midiVelocity = static_cast<std::uint8_t> (
        juce::roundToInt (juce::jlimit (0.0f, 1.0f, velocity) * 127.0f));
    return enqueueEditorMidiEvent ({ true, note, midiVelocity });
}

bool DandrumAudioProcessor::enqueueEditorNoteOff (int noteNumber) noexcept
{
    const auto note = static_cast<std::uint8_t> (juce::jlimit (0, 127, noteNumber));
    return enqueueEditorMidiEvent ({ false, note, 0 });
}

void DandrumAudioProcessor::deliverEditorKernelMidiEvents (DandrumKernelInstrument* activeKernel) noexcept
{
    const auto scope = editorMidiFifo.read (editorMidiFifo.getNumReady());
    const auto deliverBlock = [this, activeKernel] (int startIndex, int eventCount)
    {
        for (int offset = 0; offset < eventCount; ++offset)
        {
            const auto& event = editorMidiEvents[static_cast<std::size_t> (startIndex + offset)];
            if (event.noteOn)
                dandrum_kernel_note_on_at (activeKernel, event.note, event.velocity, 0);
            else
                dandrum_kernel_note_off_at (activeKernel, event.note, 0);
        }
    };

    deliverBlock (scope.startIndex1, scope.blockSize1);
    deliverBlock (scope.startIndex2, scope.blockSize2);
}

void DandrumAudioProcessor::processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    juce::ScopedNoDenormals noDenormals;

    const auto numSamples = buffer.getNumSamples();

    for (auto channel = 2; channel < buffer.getNumChannels(); ++channel)
        buffer.clear (channel, 0, numSamples);

    auto* activeKernel = kernel.load (std::memory_order_acquire);

    if (activeKernel == nullptr || ! instrumentLoaded || isMuted()
        || numSamples <= 0 || buffer.getNumChannels() <= 0)
    {
        renderSilence (buffer);
        return;
    }

    buffer.clear();
    if (buffer.getNumChannels() < 2)
        return;

    applyChangedParameters (activeKernel);
    deliverEditorKernelMidiEvents (activeKernel);
    const auto preparedBlockSize = static_cast<std::size_t> (juce::jmax (1, getBlockSize()));
    for (std::size_t blockStart = 0; blockStart < static_cast<std::size_t> (numSamples);)
    {
        const auto frames = std::min (preparedBlockSize,
                                      static_cast<std::size_t> (numSamples) - blockStart);
        for (const auto metadata : midiMessages)
        {
            const auto message = metadata.getMessage();
            const auto frameOffset = static_cast<std::size_t> (
                juce::jlimit (0, numSamples - 1, metadata.samplePosition));
            if (frameOffset < blockStart || frameOffset >= blockStart + frames)
                continue;

            const auto localOffset = frameOffset - blockStart;
            if (message.isNoteOn())
                dandrum_kernel_note_on_at (activeKernel,
                                           static_cast<unsigned char> (message.getNoteNumber()),
                                           message.getVelocity(), localOffset);
            else if (message.isNoteOff())
                dandrum_kernel_note_off_at (activeKernel,
                                            static_cast<unsigned char> (message.getNoteNumber()),
                                            localOffset);
        }

        float* channels[] { buffer.getWritePointer (0, static_cast<int> (blockStart)),
                            buffer.getWritePointer (1, static_cast<int> (blockStart)) };
        const DandrumKernelOutputBusView master { "master", channels, 2, frames };
        if (dandrum_kernel_render (activeKernel, nullptr, 0, &master, 1, frames) != frames)
            break;
        blockStart += frames;
    }
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
    const auto savedInstrumentId = state.getProperty ("instrument_id").toString();
    juce::File restoreFile;

    DandrumKernelInstrument* candidateKernel = nullptr;
    if (instrumentYaml.isNotEmpty())
    {
        const auto assetRoot = sourceHint.existsAsFile()
                                   ? sourceHint.getParentDirectory()
                                   : (savedInstrumentId == juce::String (configuration.instrumentId)
                                          ? juce::File (juce::String (configuration.instrumentPath.string())).getParentDirectory()
                                          : juce::File());
        restoreFile = writeStateRestoreInstrumentFile (instrumentYaml, assetRoot);
        if (! restoreFile.existsAsFile())
        {
            lastLoadError = "Could not stage embedded instrument from plugin state";
            return;
        }
        const auto sampleRate = getSampleRate() > 0.0 ? getSampleRate() : 44100.0;
        const auto blockSize = getBlockSize() > 0 ? getBlockSize() : 512;
        const auto restorePath = restoreFile.getFullPathName().toStdString();
        candidateKernel = prepareKernelWithPublicControls (
            restorePath, static_cast<std::uint32_t> (juce::jmax (1.0, sampleRate)),
            static_cast<std::size_t> (juce::jmax (1, blockSize)));
        if (candidateKernel == nullptr)
        {
            restoreFile.deleteFile();
            lastLoadError = "Failed to restore embedded instrument from plugin state";
            return;
        }
    }

    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    parameters.replaceState (state);

    if (candidateKernel != nullptr)
    {
        suspendProcessing (true);
        setMuted (true);
        auto* previousKernel = kernel.exchange (candidateKernel, std::memory_order_acq_rel);

        juce::String instrumentId;
        int schema = 0;
        readInstrumentIdentity (instrumentYaml, instrumentId, schema);
        instrumentLoaded = true;
        loadedInstrument.sourceFile = sourceHint;
        loadedInstrument.yamlContent = instrumentYaml;
        loadedInstrument.instrumentId = instrumentId;
        loadedInstrument.presetSchemaVersion = schema;
        preparePublicParameterSlots (restoreFile, &lastReloadWarning, true);
        restoreFile.deleteFile();

        juce::Thread::sleep (5);
        dandrum_kernel_destroy (previousKernel);

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

    // Watch the restored instrument's original file only if it still exists on
    // this machine; the embedded YAML is the source of truth for restore, and a
    // stale path from another machine must not become a phantom watch target.
    if (loadedInstrument.sourceFile.existsAsFile())
        instrumentFileWatcher.watchFile (loadedInstrument.sourceFile);
    else
        instrumentFileWatcher.stopWatching();
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

const InstrumentDemoConfiguration& DandrumAudioProcessor::demoConfiguration() const noexcept
{
    return configuration;
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

std::vector<DandrumAudioProcessor::PublicParameterSnapshotEntry>
DandrumAudioProcessor::getPublicParameterSnapshot() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    std::vector<PublicParameterSnapshotEntry> values;
    values.reserve (parameterSlots.size());
    for (const auto& slot : parameterSlots)
    {
        if (! slot.active)
            continue;

        // Every slot has a fixed JUCE parameter object for the processor lifetime.
        const auto* parameter = parameters.getParameter (slot.slotParameterId);
        values.push_back ({ slot.descriptor.id, slot.descriptor.name, parameter->getValue() });
    }
    return values;
}

std::optional<InstrumentUiDocument> DandrumAudioProcessor::getPreparedUiDocument() const
{
    std::unique_ptr<DandrumKernelUiSnapshot, decltype (&dandrum_kernel_ui_snapshot_destroy)> snapshot (
        nullptr, &dandrum_kernel_ui_snapshot_destroy);
    InstrumentUiDocument document;
    {
        const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
        auto* activeKernel = kernel.load (std::memory_order_acquire);
        if (! instrumentLoaded || activeKernel == nullptr)
            return std::nullopt;

        // Only this copy and the parameter snapshot need the engine/reload lock.
        snapshot.reset (dandrum_kernel_ui_snapshot_create (activeKernel));
        if (! snapshot)
            return std::nullopt;

        document.generation = parameterSurfaceGeneration.load (std::memory_order_relaxed);
        document.instrumentId = loadedInstrument.instrumentId.toStdString();
        document.parameters.reserve (parameterSlots.size());
        for (std::size_t index = 0; index < parameterSlots.size(); ++index)
        {
            const auto& slot = parameterSlots[index];
            if (! slot.active)
                continue;
            std::int32_t group = 0;
            if (! dandrum_kernel_ui_public_control_group (snapshot.get(), index, &group))
                return std::nullopt;
            const auto* parameter = parameters.getParameter (slot.slotParameterId);
            if (parameter == nullptr)
                return std::nullopt;
            InstrumentUiDocument::Parameter value;
            value.id = slot.descriptor.id.toStdString();
            value.name = slot.descriptor.name.toStdString();
            value.normalisedValue = parameter->getValue();
            value.minValue = slot.descriptor.minValue;
            value.maxValue = slot.descriptor.maxValue;
            if (group > 0)
            {
                value.scope = InstrumentUiDocument::ControlScope::sampleGroup;
                value.controlGroup = group;
            }
            document.parameters.push_back (std::move (value));
        }
    }

    const auto sourceCount = dandrum_kernel_ui_source_count (snapshot.get());
    document.sources.reserve (sourceCount);
    for (std::size_t sourceIndex = 0; sourceIndex < sourceCount; ++sourceIndex)
    {
        DandrumKernelUiSource sourceView {};
        if (! dandrum_kernel_ui_source (snapshot.get(), sourceIndex, &sourceView))
            return std::nullopt;
        InstrumentUiDocument::Source source;
        source.id = copyUiText (sourceView.id);
        source.sampleRateHz = sourceView.sampleRateHz;
        source.channelCount = sourceView.channelCount;
        source.frameCount = sourceView.frameCount;
        const auto regionCount = dandrum_kernel_ui_region_count (snapshot.get(), sourceIndex);
        source.regions.reserve (regionCount);
        for (std::size_t regionIndex = 0; regionIndex < regionCount; ++regionIndex)
        {
            DandrumKernelUiRegion regionView {};
            if (! dandrum_kernel_ui_region (snapshot.get(), sourceIndex, regionIndex, &regionView))
                return std::nullopt;
            InstrumentUiDocument::Region region;
            region.id = copyUiText (regionView.id);
            region.startFrame = regionView.startFrame;
            region.endFrame = regionView.endFrame;
            if (regionView.rootNote >= 0)
                region.rootNote = regionView.rootNote;
            if (regionView.hasGainDb)
                region.gainDb = regionView.gainDb;
            if (regionView.hasPan)
                region.pan = regionView.pan;
            region.reverse = regionView.reverse;
            region.fadeInMs = regionView.fadeInMs;
            region.fadeOutMs = regionView.fadeOutMs;
            if (regionView.hasLoop)
                region.loop = InstrumentUiDocument::RegionLoop {
                    copyUiText (regionView.loopMode), regionView.loopStartFrame,
                    regionView.loopEndFrame, regionView.loopCrossfadeMs };
            source.regions.push_back (std::move (region));
        }
        const auto sliceCount = dandrum_kernel_ui_slice_count (snapshot.get(), sourceIndex);
        source.slices.reserve (sliceCount);
        for (std::size_t sliceIndex = 0; sliceIndex < sliceCount; ++sliceIndex)
        {
            DandrumKernelUiSlice sliceView {};
            if (! dandrum_kernel_ui_slice (snapshot.get(), sourceIndex, sliceIndex, &sliceView))
                return std::nullopt;
            source.slices.push_back ({ copyUiText (sliceView.id), sliceView.startFrame, sliceView.endFrame });
        }
        document.sources.push_back (std::move (source));
    }

    const auto mapCount = dandrum_kernel_ui_map_count (snapshot.get());
    document.maps.reserve (mapCount);
    for (std::size_t mapIndex = 0; mapIndex < mapCount; ++mapIndex)
    {
        DandrumKernelUiMap mapView {};
        if (! dandrum_kernel_ui_map (snapshot.get(), mapIndex, &mapView))
            return std::nullopt;
        InstrumentUiDocument::Map map;
        map.id = copyUiText (mapView.id);
        map.selectionMode = copyUiText (mapView.selectionMode);
        map.selectionSeed = mapView.selectionSeed;
        const auto zoneCount = dandrum_kernel_ui_zone_count (snapshot.get(), mapIndex);
        map.zones.reserve (zoneCount);
        for (std::size_t zoneIndex = 0; zoneIndex < zoneCount; ++zoneIndex)
        {
            DandrumKernelUiZone zoneView {};
            if (! dandrum_kernel_ui_zone (snapshot.get(), mapIndex, zoneIndex, &zoneView))
                return std::nullopt;
            InstrumentUiDocument::Zone zone;
            zone.id = copyUiText (zoneView.id);
            zone.sourceIndex = zoneView.sourceIndex;
            zone.regionIndex = zoneView.regionIndex;
            zone.startFrame = zoneView.startFrame;
            zone.endFrame = zoneView.endFrame;
            zone.keyLow = zoneView.keyLow;
            zone.keyHigh = zoneView.keyHigh;
            zone.velocityLow = zoneView.velocityLow;
            zone.velocityHigh = zoneView.velocityHigh;
            zone.roundRobinGroup = copyUiText (zoneView.roundRobinGroup);
            zone.chokeGroup = copyUiText (zoneView.chokeGroup);
            if (zoneView.controlGroup > 0)
                zone.controlGroup = zoneView.controlGroup;
            zone.weight = zoneView.weight;
            if (zoneView.hasGainDb)
                zone.gainDb = zoneView.gainDb;
            if (zoneView.hasPan)
                zone.pan = zoneView.pan;
            if (zoneView.hasPitchSemitones)
                zone.pitchSemitones = zoneView.pitchSemitones;
            map.zones.push_back (std::move (zone));
        }
        document.maps.push_back (std::move (map));
    }
    document.capabilities.sampleKeyMap = ! document.maps.empty();
    return document;
}

InstrumentUiParameterState DandrumAudioProcessor::getUiParameterState() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    InstrumentUiParameterState state;
    state.generation = getParameterSurfaceGeneration();
    state.admittedCommandSequence = uiCommandService.lastAdmittedSequence();
    state.parameters.reserve (parameterSlots.size());
    for (const auto& slot : parameterSlots)
    {
        if (! slot.active)
            continue;
        const auto* parameter = parameters.getParameter (slot.slotParameterId);
        state.parameters.push_back ({ slot.descriptor.id.toStdString(),
                                      slot.descriptor.name.toStdString(), parameter->getValue() });
    }
    return state;
}

std::uint32_t DandrumAudioProcessor::getParameterSurfaceGeneration() const noexcept
{
    return parameterSurfaceGeneration.load (std::memory_order_relaxed);
}

InstrumentUiCommandService& DandrumAudioProcessor::uiCommands() noexcept
{
    return uiCommandService;
}

std::uint32_t DandrumAudioProcessor::uiCommandGeneration() const noexcept
{
    return getParameterSurfaceGeneration();
}

InstrumentUiCommandStatus DandrumAudioProcessor::applyUiParameter (
    std::uint32_t generation, const std::string& id, float normalisedValue,
    bool withinGesture)
{
    // Keep generation, public-ID lookup, and host admission in one reload epoch.
    // The recursive lock permits synchronous host listeners to inspect state.
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (generation != getParameterSurfaceGeneration())
        return InstrumentUiCommandStatus::staleGeneration;
    auto* parameter = getParameterForPublicId (juce::String::fromUTF8 (id.data(), static_cast<int> (id.size())));
    if (parameter == nullptr)
        return InstrumentUiCommandStatus::unknownControl;

    if (! withinGesture)
        parameter->beginChangeGesture();
    parameter->setValueNotifyingHost (normalisedValue);
    if (! withinGesture)
        parameter->endChangeGesture();
    return InstrumentUiCommandStatus::accepted;
}

InstrumentUiGestureAdmission DandrumAudioProcessor::beginUiGesture (
    std::uint32_t generation, const std::string& id)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (generation != getParameterSurfaceGeneration())
        return { InstrumentUiCommandStatus::staleGeneration };
    const auto requestedId = juce::String::fromUTF8 (id.data(), static_cast<int> (id.size()));
    for (std::size_t index = 0; index < parameterSlots.size(); ++index)
    {
        const auto& slot = parameterSlots[index];
        if (! slot.active || slot.descriptor.id != requestedId)
            continue;
        parameters.getParameter (slot.slotParameterId)->beginChangeGesture();
        return { InstrumentUiCommandStatus::accepted, index };
    }
    return { InstrumentUiCommandStatus::unknownControl };
}

void DandrumAudioProcessor::endUiGesture (std::size_t hostSlot)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (hostSlot < parameterSlots.size())
        parameters.getParameter (parameterSlots[hostSlot].slotParameterId)->endChangeGesture();
}

const juce::File& DandrumAudioProcessor::currentInstrumentFile() const noexcept
{
    return loadedInstrument.sourceFile;
}

bool DandrumAudioProcessor::isSoundLabInstrumentCompatible() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return instrumentLoaded
           && configuration.soundLabFixturePath.has_value()
           && configuration.matchSourcePath.has_value()
           && loadedInstrument.sourceFile == juce::File (juce::String (configuration.matchSourcePath->string()))
           && loadedInstrument.instrumentId == juce::String (configuration.instrumentId);
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
        case ReplacementState::Muted: return "muted";
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
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    const auto yamlText = yamlFile.loadFileAsString();
    loadedPreset = {};
    const auto reloaded = replaceActiveEngineFromFile (yamlFile, yamlFile, yamlText, true, false, &lastReloadWarning);
    if (reloaded)
        instrumentFileWatcher.watchFile (yamlFile);

    return reloaded;
}

bool DandrumAudioProcessor::reloadInstrumentFromYaml (const juce::String& yamlText,
                                                      const juce::File& sourceHint)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    const auto parent = sourceHint.getParentDirectory();
    if (! parent.isDirectory())
    {
        lastLoadError = "Instrument source directory does not exist: " + parent.getFullPathName();
        return false;
    }

    const auto stagedFile = parent.getNonexistentChildFile (
        ".dandrum_matched_snapshot_", ".yaml", false);
    const auto* yamlBytes = yamlText.toRawUTF8();
    if (! stagedFile.replaceWithData (yamlBytes,
                                      static_cast<std::size_t> (yamlText.getNumBytesAsUTF8())))
    {
        lastLoadError = "Could not stage the matched instrument snapshot beside: "
                        + sourceHint.getFullPathName();
        return false;
    }

    loadedPreset = {};
    const auto reloaded = replaceActiveEngineFromFile (
        stagedFile, sourceHint, yamlText, true, false, &lastReloadWarning);
    stagedFile.deleteFile();
    if (reloaded)
        instrumentFileWatcher.watchFile (sourceHint);

    return reloaded;
}

void DandrumAudioProcessor::setFileWatchEnabled (bool shouldWatch)
{
    instrumentFileWatcher.setEnabled (shouldWatch);
}

bool DandrumAudioProcessor::isFileWatchEnabled() const noexcept
{
    return instrumentFileWatcher.isEnabled();
}

const juce::File& DandrumAudioProcessor::watchedInstrumentFile() const noexcept
{
    return instrumentFileWatcher.watchedFile();
}

void DandrumAudioProcessor::pollInstrumentFileForChanges()
{
    instrumentFileWatcher.poll();
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

    auto* activeKernel = kernel.load (std::memory_order_relaxed);
    if (activeKernel == nullptr)
    {
        lastPresetError = "Rust kernel is not available";
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
        setSlotNormalisedValue (slotIndex, normalised);
        applySlotToKernel (slot, normalised, activeKernel);
    }

    loadedPreset.sourceFile = presetFile;
    loadedPreset.name = preset.name;
    loadedPreset.yamlContent = preset.yamlContent;
    return true;
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
#if defined(DANDRUM_DEFAULT_SAMPLER_PLUGIN)
    return new DandrumAudioProcessor (InstrumentDemoConfiguration::sampler());
#else
    return new DandrumAudioProcessor();
#endif
}
