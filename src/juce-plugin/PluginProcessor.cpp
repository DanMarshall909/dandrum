#include "PluginProcessor.h"
#include "InstrumentUiOutputBuses.h"
#include "SoundLabController.h"

#include <algorithm>
#include <array>
#include <bit>
#include <cctype>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace
{
bool sameBitPattern (float a, float b) noexcept
{
    return std::bit_cast<std::uint32_t> (a) == std::bit_cast<std::uint32_t> (b);
}

// Fixed normalized host slots retain their value and host-write revision in one
// atomic word. Notifications cannot identify writes: a listener may throw
// before APVTS sees them, or recursively write the very same candidate value.
class PublicSlotParameter final : public juce::RangedAudioParameter
{
public:
    using RangedAudioParameter::RangedAudioParameter;
    const juce::NormalisableRange<float>& getNormalisableRange() const override { return range; }
    float getValue() const override { return valueOf (state.load (std::memory_order_acquire)); }
    float getDefaultValue() const override { return 0.0f; }
    int getNumSteps() const override { return AudioProcessorParameterWithID::getNumSteps(); }
    juce::String getText (float value, int length) const override
    {
        const juce::String text (convertFrom0to1 (value), 7);
        return length > 0 ? text.substring (0, length) : text;
    }
    float getValueForText (const juce::String& text) const override
    { return convertTo0to1 (text.getFloatValue()); }

    std::uint64_t snapshot() const noexcept { return state.load (std::memory_order_acquire); }
    float workingValue (std::uint64_t captured) const noexcept
    {
        const auto current = snapshot();
        return valueOf ((current >> 32) == (captured >> 32) ? captured : current);
    }
    void setValue (float value) override
    {
        // Consume the token before notifications; recursive host writes are
        // ordinary writes even when they occur on the rebuilding thread.
        const auto ownWrite = reloadWrite.parameter == this;
        const auto revision = reloadWrite.revision;
        if (ownWrite) reloadWrite = {};
        const auto bits = std::bit_cast<std::uint32_t> (convertTo0to1 (convertFrom0to1 (value)));
        auto previous = state.load (std::memory_order_acquire);
        do
        {
            if (ownWrite && (previous >> 32) != revision) return;
        }
        while (!state.compare_exchange_weak (previous,
                    ((ownWrite ? revision : (previous >> 32) + 1) << 32) | bits,
                    std::memory_order_acq_rel, std::memory_order_acquire));
    }
    void setReloadValue (float value, std::uint64_t captured)
    {
        const auto previous = std::exchange (reloadWrite, ReloadWrite { this, captured >> 32 });
        struct RestoreToken
        {
            ReloadWrite previous;
            ~RestoreToken() { reloadWrite = previous; }
        } restore { previous };
        setValue (value);
        sendValueChangedMessageToListeners (getValue());
    }
    void restore (std::uint64_t captured, std::atomic<float>& raw) noexcept
    {
        auto current = state.load (std::memory_order_acquire);
        while ((current >> 32) == (captured >> 32)
            && !state.compare_exchange_weak (current, captured,
                    std::memory_order_acq_rel, std::memory_order_acquire)) {}
        // Recovery must not call the listener which just failed. Mirror the
        // authoritative value, retrying if automation races this repair.
        do
        {
            current = state.load (std::memory_order_acquire);
            raw.store (valueOf (current), std::memory_order_release);
        }
        while (state.load (std::memory_order_acquire) != current);
    }

private:
    struct ReloadWrite { PublicSlotParameter* parameter = nullptr; std::uint64_t revision = 0; };
    static thread_local ReloadWrite reloadWrite;
    static float valueOf (std::uint64_t value) noexcept
    { return std::bit_cast<float> (static_cast<std::uint32_t> (value)); }
    const juce::NormalisableRange<float> range { 0.0f, 1.0f };
    std::atomic<std::uint64_t> state { 0 };
    static_assert (std::atomic<std::uint64_t>::is_always_lock_free);
};
thread_local PublicSlotParameter::ReloadWrite PublicSlotParameter::reloadWrite;

class ReloadHostValues
{
public:
    ReloadHostValues (juce::AudioProcessorValueTreeState& parameters,
                      const juce::Array<juce::AudioProcessorParameter*>& hostParameters)
    {
        values.reserve (static_cast<std::size_t> (hostParameters.size()));
        for (auto* host : hostParameters)
        {
            auto& parameter = static_cast<PublicSlotParameter&> (*host);
            values.push_back ({ &parameter, parameter.snapshot(), parameters.getRawParameterValue (parameter.paramID) });
        }
        previous = std::exchange (active, this);
    }
    ReloadHostValues (const ReloadHostValues&) = delete;
    ReloadHostValues& operator= (const ReloadHostValues&) = delete;
    ~ReloadHostValues()
    {
        if (!committed)
            for (const auto& value : values) value.parameter->restore (value.captured, *value.raw);
        active = previous;
    }
    void commit() noexcept { committed = true; }
    static float read (const PublicSlotParameter& parameter) noexcept
    {
        if (active != nullptr)
            for (const auto& captured : active->values)
                if (captured.parameter == &parameter)
                    return parameter.workingValue (captured.captured);
        return parameter.getValue();
    }
    static void write (PublicSlotParameter& parameter, float value)
    {
        if (active != nullptr)
            for (const auto& captured : active->values)
                if (captured.parameter == &parameter)
                {
                    parameter.setReloadValue (value, captured.captured);
                    return;
                }
        parameter.setValueNotifyingHost (value);
    }

private:
    struct Value { PublicSlotParameter* parameter; std::uint64_t captured; std::atomic<float>* raw; };
    std::vector<Value> values;
    ReloadHostValues* previous = nullptr;
    bool committed = false;
    static thread_local ReloadHostValues* active;
};
thread_local ReloadHostValues* ReloadHostValues::active = nullptr;

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
        layout.add (std::make_unique<PublicSlotParameter> (
            juce::ParameterID { publicSlotParameterId (slotIndex), 1 },
            name));
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
    : DandrumAudioProcessor (std::move (demo), {}) {}

DandrumAudioProcessor::DandrumAudioProcessor (
    InstrumentDemoConfiguration demo, InstrumentUiLiveService::BeforeBatch observeLiveWorker,
    std::function<void()> beforeReloadPreparation)
    : juce::AudioProcessor (BusesProperties()
                                 .withInput ("Input", juce::AudioChannelSet::stereo(), true)
                                 .withOutput ("Output", juce::AudioChannelSet::stereo(), true)),
      configuration (std::move (demo)),
      hostOutputBuses (captureInstrumentUiOutputBuses (*this, "master")),
      parameters (*this, nullptr, "DandrumState", createParameterLayout (configuration)),
      liveService (std::move (observeLiveWorker)),
      beforeUiReloadPreparation (std::move (beforeReloadPreparation))
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

    if (configuration.soundLabFixturePath)
        soundLabController = std::make_unique<SoundLabController>();

    instrumentFileWatcher.onReload ([this] (const juce::File& changedFile) { reloadInstrumentFromFile (changedFile); });
    if (instrumentLoaded && loadedInstrument.sourceFile.existsAsFile())
        instrumentFileWatcher.watchFile (loadedInstrument.sourceFile);
}

DandrumAudioProcessor::EngineReaderGuard::EngineReaderGuard (std::atomic<std::uint32_t>& state)
    : access (state)
{
    auto observed = access.load (std::memory_order_relaxed);
    acquired = (observed & engineAccessClosed) == 0
        && access.compare_exchange_strong (observed, observed + 1,
                                           std::memory_order_acquire, std::memory_order_relaxed);
}
DandrumAudioProcessor::EngineReaderGuard::~EngineReaderGuard()
{
    if (acquired) access.fetch_sub (1, std::memory_order_release);
}
void DandrumAudioProcessor::waitForEngineReaders() const
{
    // The closed bit prevents new acquisitions. The count, rather than a
    // elapsed delay or another callback, acknowledges every previous reader.
    while (engineAccess.load (std::memory_order_acquire) != engineAccessClosed)
        std::this_thread::sleep_for (std::chrono::milliseconds (1));
}
DandrumAudioProcessor::EngineAccessPause::EngineAccessPause (DandrumAudioProcessor& processor)
    : owner (processor)
{
    ownsGate = (owner.engineAccess.fetch_or (engineAccessClosed, std::memory_order_acq_rel)
                & engineAccessClosed) == 0;
    if (ownsGate)
    {
        previousMute = owner.isMuted();
        owner.setMuted (true);
    }
    // An outer asynchronous reload may already own the gate. A synchronous
    // replacement nested under it still must acknowledge existing readers.
    owner.waitForEngineReaders();
}
DandrumAudioProcessor::EngineAccessPause::~EngineAccessPause()
{
    if (ownsGate)
    {
        owner.engineAccess.store (0, std::memory_order_release);
        owner.setMuted (previousMute);
    }
}

DandrumAudioProcessor::~DandrumAudioProcessor()
{
    instrumentFileWatcher.stopWatching();
    {
        const std::lock_guard<std::recursive_mutex> lock (reloadMutex);
        uiReloadShuttingDown = true;
        engineAccess.fetch_or (engineAccessClosed, std::memory_order_acq_rel);
        setMuted (true);
    }
    waitForEngineReaders();
    soundLabController.reset();
    if (pendingUiReload.valid())
    {
        // The worker observes shutdown under reloadMutex and cannot reopen the
        // gate. Joining and final engine cleanup remain off the audio callback.
        // Teardown needs completion, not the result already recorded by the
        // coordinator. wait() never rethrows a stored worker exception.
        pendingUiReload.wait();
    }
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
    if (engineActivationInProgress || ! instrumentLoaded)
        return;

    const EngineAccessPause handoff (*this);
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

    const auto descriptors = loadPublicParameterDescriptors (path);
    installPreparedEngine (candidateKernel, sourceHint, yamlText, descriptors,
                           preferCurrentSlotValues, reloadWarning);
    return true;
}

void DandrumAudioProcessor::installPreparedEngine (
    DandrumKernelInstrument* candidateKernel, const juce::File& sourceHint,
    const juce::String& yamlText,
    const std::vector<PublicParameterDescriptor>& descriptors,
    bool preferCurrentSlotValues, juce::String* reloadWarning)
{
    juce::String instrumentId;
    int schemaVersion = 0;
    readInstrumentIdentity (yamlText, instrumentId, schemaVersion);

    replacementState.store (static_cast<int> (ReplacementState::Muted), std::memory_order_relaxed);
    const EngineAccessPause handoff (*this);
    const juce::ScopedValueSetter<bool> activating (engineActivationInProgress, true);
    LoadedInstrument candidateInstrument { sourceHint, yamlText, instrumentId, schemaVersion };
    CandidateParameterBindings bindings { candidateKernel, parameterSlots };
    juce::String candidateWarning;
    // Notifications may re-enter readers or fail. Only the candidate's bindings
    // and DSP receive preparation writes; committed metadata stays available.
    preparePublicParameterSlots (descriptors, &candidateWarning, preferCurrentSlotValues, &bindings);

    auto* previousKernel = kernel.exchange (candidateKernel, std::memory_order_acq_rel);
    std::swap (loadedInstrument, candidateInstrument);
    parameterSlots.swap (bindings.slots);
    instrumentLoaded = true;
    lastLoadError.clear();
    if (reloadWarning != nullptr) *reloadWarning = std::move (candidateWarning);
    publishParameterSurface();

    dandrum_kernel_destroy (previousKernel);
    replacementState.store (static_cast<int> (ReplacementState::Running), std::memory_order_relaxed);
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
                                                          bool preferCurrentSlotValues,
                                                          CandidateParameterBindings* candidate)
{
    if (droppedParametersWarning != nullptr)
        droppedParametersWarning->clear();

    auto* activeKernel = candidate != nullptr ? candidate->kernel : kernel.load (std::memory_order_relaxed);
    if (activeKernel == nullptr || (candidate == nullptr && ! instrumentLoaded))
        return;
    auto& slots = candidate != nullptr ? candidate->slots : parameterSlots;

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

    for (auto& slot : slots)
    {
        slot.active = false;
        slot.descriptor = {};
        slot.kernelSlotIndex = kNoEngineSlot;
        slot.lastAppliedNormalisedValue = slot.rawValue != nullptr ? slot.rawValue->load (std::memory_order_relaxed) : 0.0f;
    }

    const auto activeCount = juce::jmin (static_cast<int> (descriptors.size()), kPublicParameterSlotCount);
    for (int slotIndex = 0; slotIndex < activeCount; ++slotIndex)
    {
        auto& slot = slots[slotIndex];
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
        const auto latestValue = parameters.getParameter (slot.slotParameterId)->getValue();
        applySlotToKernel (slot, latestValue, activeKernel);
        slot.lastAppliedNormalisedValue = latestValue;
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

    if (candidate == nullptr) publishParameterSurface();
}

void DandrumAudioProcessor::publishParameterSurface()
{
    clearEditorNoteIntentForReload();
    const auto generation = parameterSurfaceGeneration.fetch_add (1, std::memory_order_relaxed) + 1;
    waveformService.setGeneration (generation);
    spectralService.setGeneration (generation);
    liveService.setGeneration (generation);
}

void DandrumAudioProcessor::setSlotNormalisedValue (int slotIndex, float normalisedValue)
{
    if (! juce::isPositiveAndBelow (slotIndex, kPublicParameterSlotCount))
        return;

    auto* parameter = parameters.getParameter (publicSlotParameterId (slotIndex));
    if (parameter == nullptr)
        return;

    const auto value = juce::jlimit (0.0f, 1.0f, normalisedValue);
    ReloadHostValues::write (static_cast<PublicSlotParameter&> (*parameter), value);
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

void DandrumAudioProcessor::publishEditorNoteIntent (std::uint8_t note, std::uint8_t velocity) noexcept
{
    auto& state = editorNoteIntent[note];
    const auto previous = state.load (std::memory_order_relaxed);
    if (velocity == 0)
        state.store (previous & ~std::uint64_t (0xff), std::memory_order_release);
    else
        state.store (((previous >> 16) + 1) << 16
                         | (std::uint64_t (velocity) << 8) | velocity,
                     std::memory_order_release);
}

bool DandrumAudioProcessor::enqueueEditorNoteOn (
    int noteNumber, float velocity, std::uint64_t sessionId) noexcept
{
    const std::lock_guard<std::mutex> noteLock (editorNoteCommandMutex);
    const auto note = static_cast<std::uint8_t> (juce::jlimit (0, 127, noteNumber));
    const auto midiVelocity = static_cast<std::uint8_t> (
        juce::roundToInt (juce::jlimit (1.0f / 127.0f, 1.0f, velocity) * 127.0f));
    auto& owner = editorNoteOwner[note];
    if (owner && *owner != sessionId)
    {
        droppedMidiEventCount.fetch_add (1, std::memory_order_relaxed);
        return false;
    }
    owner = sessionId;
    publishEditorNoteIntent (note, midiVelocity);
    return true;
}

bool DandrumAudioProcessor::enqueueEditorNoteOff (
    int noteNumber, std::uint64_t sessionId) noexcept
{
    const std::lock_guard<std::mutex> noteLock (editorNoteCommandMutex);
    const auto note = static_cast<std::uint8_t> (juce::jlimit (0, 127, noteNumber));
    auto& owner = editorNoteOwner[note];
    if (! owner)
        return true;
    if (*owner != sessionId)
        return false;
    owner.reset();
    publishEditorNoteIntent (note, 0);
    return true;
}

void DandrumAudioProcessor::closeEditorNoteSession (std::uint64_t sessionId) noexcept
{
    const std::lock_guard<std::mutex> noteLock (editorNoteCommandMutex);
    for (std::size_t note = 0; note < editorNoteOwner.size(); ++note)
    {
        if (editorNoteOwner[note] == sessionId)
        {
            editorNoteOwner[note].reset();
            auto& state = editorNoteIntent[note];
            const auto previous = state.load (std::memory_order_relaxed);
            // Disconnect cancels an onset that audio has not seen yet. An
            // ordinary note-off preserves it for short drum-pad taps.
            state.store (previous & ~std::uint64_t (0xffff), std::memory_order_release);
        }
    }
}

void DandrumAudioProcessor::clearEditorNoteIntentForReload() noexcept
{
    const std::lock_guard<std::mutex> noteLock (editorNoteCommandMutex);
    for (std::size_t note = 0; note < editorNoteOwner.size(); ++note)
    {
        editorNoteOwner[note].reset();
        editorNoteIntent[note].store (0, std::memory_order_release);
    }
}

void DandrumAudioProcessor::deliverEditorNoteIntent (
    DandrumKernelInstrument* activeKernel,
    const std::array<bool, 128>& hostTouchedNotes) noexcept
{
    for (std::size_t note = 0; note < editorNoteIntent.size(); ++note)
    {
        const auto intent = editorNoteIntent[note].load (std::memory_order_acquire);
        const auto midiNote = static_cast<std::uint8_t> (note);
        const auto onSequence = intent >> 16;
        const auto onVelocity = static_cast<std::uint8_t> ((intent >> 8) & 0xff);
        const auto desiredVelocity = static_cast<std::uint8_t> (intent & 0xff);

        if (audioEditorTransientRelease[note])
        {
            if (audioEditorNoteHeld[note])
            {
                if (! dandrum_kernel_note_off_at (activeKernel, midiNote, 0))
                    continue;
                audioEditorNoteHeld[note] = false;
            }
            audioEditorTransientRelease[note] = false;
        }

        if (onSequence != lastAudioEditorOnSequence[note])
        {
            if (audioEditorNoteHeld[note])
            {
                if (! dandrum_kernel_note_off_at (activeKernel, midiNote, 0))
                    continue;
                audioEditorNoteHeld[note] = false;
            }
            if (onVelocity == 0 || hostTouchedNotes[note] || audioHostNoteHeld[note])
            {
                lastAudioEditorOnSequence[note] = onSequence;
                continue;
            }
            if (dandrum_kernel_note_on_at (activeKernel, midiNote, onVelocity, 0))
            {
                lastAudioEditorOnSequence[note] = onSequence;
                audioEditorNoteHeld[note] = true;
                audioEditorTransientRelease[note] = desiredVelocity == 0;
            }
        }
        else if (desiredVelocity == 0 && audioEditorNoteHeld[note])
        {
            if (dandrum_kernel_note_off_at (activeKernel, midiNote, 0))
                audioEditorNoteHeld[note] = false;
        }
        else if (desiredVelocity != 0 && ! audioEditorNoteHeld[note]
                 && ! audioHostNoteHeld[note] && ! hostTouchedNotes[note])
        {
            if (dandrum_kernel_note_on_at (activeKernel, midiNote, desiredVelocity, 0))
                audioEditorNoteHeld[note] = true;
        }
    }
}

void DandrumAudioProcessor::processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    juce::ScopedNoDenormals noDenormals;
    const EngineReaderGuard reader (engineAccess);
    if (! reader.acquired)
    {
        renderSilence (buffer);
        return;
    }

    const auto numSamples = buffer.getNumSamples();
    const auto liveGeneration = parameterSurfaceGeneration.load (std::memory_order_relaxed);
    const auto liveRate = static_cast<std::uint32_t> (juce::jmax (1.0, getSampleRate()));

    for (auto channel = 2; channel < buffer.getNumChannels(); ++channel)
        buffer.clear (channel, 0, numSamples);

    auto* activeKernel = kernel.load (std::memory_order_acquire);
    if (activeKernel != lastMeterKernel)
    {
        lastMeterKernel = activeKernel;
        meterCapture.beginStream();
        liveService.beginStream();
    }

    if (activeKernel == nullptr || ! instrumentLoaded || isMuted()
        || numSamples <= 0 || buffer.getNumChannels() <= 0)
    {
        renderSilence (buffer);
        if (numSamples > 0 && buffer.getNumChannels() >= 2)
        {
            meterCapture.capture (buffer.getReadPointer (0), buffer.getReadPointer (1),
                                  static_cast<std::size_t> (numSamples),
                                  parameterSurfaceGeneration.load (std::memory_order_relaxed));
            liveService.capture (buffer.getReadPointer (0), buffer.getReadPointer (1),
                                 static_cast<std::size_t> (numSamples), liveGeneration, liveRate);
        }
        return;
    }

    buffer.clear();
    if (buffer.getNumChannels() < 2)
        return;

    if (activeKernel != lastAudioKernel)
    {
        lastAudioKernel = activeKernel;
        lastAudioEditorOnSequence.fill (0);
        audioEditorNoteHeld.fill (false);
        audioEditorTransientRelease.fill (false);
        audioHostNoteHeld.fill (false);
        audioHostReleasePending.fill (false);
    }
    applyChangedParameters (activeKernel);
    const auto preparedBlockSize = static_cast<std::size_t> (juce::jmax (1, getBlockSize()));
    for (std::size_t blockStart = 0; blockStart < static_cast<std::size_t> (numSamples);)
    {
        const auto frames = std::min (preparedBlockSize,
                                      static_cast<std::size_t> (numSamples) - blockStart);
        std::array<bool, 128> hostTouchedNotes {};
        for (std::size_t note = 0; note < audioHostReleasePending.size(); ++note)
        {
            if (audioHostReleasePending[note]
                && dandrum_kernel_note_off_at (
                    activeKernel, static_cast<unsigned char> (note), 0))
            {
                audioHostReleasePending[note] = false;
                audioHostNoteHeld[note] = false;
            }
        }
        for (const auto metadata : midiMessages)
        {
            const auto message = metadata.getMessage();
            const auto frameOffset = static_cast<std::size_t> (
                juce::jlimit (0, numSamples - 1, metadata.samplePosition));
            if (frameOffset < blockStart || frameOffset >= blockStart + frames)
                continue;

            const auto localOffset = frameOffset - blockStart;
            if (message.isNoteOn())
            {
                const auto note = static_cast<std::size_t> (message.getNoteNumber());
                hostTouchedNotes[note] = true;
                if (audioEditorNoteHeld[note])
                {
                    if (dandrum_kernel_note_off_at (
                            activeKernel, static_cast<unsigned char> (note), localOffset))
                        audioEditorNoteHeld[note] = false;
                }
                if (dandrum_kernel_note_on_at (activeKernel,
                        static_cast<unsigned char> (note), message.getVelocity(), localOffset))
                    audioHostNoteHeld[note] = true;
            }
            else if (message.isNoteOff())
            {
                const auto note = static_cast<std::size_t> (message.getNoteNumber());
                hostTouchedNotes[note] = true;
                if (audioHostNoteHeld[note])
                {
                    if (dandrum_kernel_note_off_at (
                            activeKernel, static_cast<unsigned char> (note), localOffset))
                        audioHostNoteHeld[note] = false;
                    else
                        audioHostReleasePending[note] = true;
                }
                else if (! audioEditorNoteHeld[note])
                    dandrum_kernel_note_off_at (
                        activeKernel, static_cast<unsigned char> (note), localOffset);
            }
        }

        if (blockStart == 0)
            deliverEditorNoteIntent (activeKernel, hostTouchedNotes);

        float* channels[] { buffer.getWritePointer (0, static_cast<int> (blockStart)),
                            buffer.getWritePointer (1, static_cast<int> (blockStart)) };
        const DandrumKernelOutputBusView master { "master", channels, 2, frames };
        if (dandrum_kernel_render (activeKernel, nullptr, 0, &master, 1, frames) != frames)
            break;
        blockStart += frames;
    }
    meterCapture.capture (buffer.getReadPointer (0), buffer.getReadPointer (1),
                          static_cast<std::size_t> (numSamples),
                          parameterSurfaceGeneration.load (std::memory_order_relaxed));
    liveService.capture (buffer.getReadPointer (0), buffer.getReadPointer (1),
                         static_cast<std::size_t> (numSamples), liveGeneration, liveRate);
}

bool DandrumAudioProcessor::subscribeLiveAnalysis (std::uint64_t session, std::uint32_t generation, std::uint8_t channels)
{ return liveService.subscribe (session, generation, channels); }
bool DandrumAudioProcessor::unsubscribeLiveAnalysis (std::uint64_t session)
{ return liveService.unsubscribe (session); }
bool DandrumAudioProcessor::setLiveAnalysisVisible (std::uint64_t session, bool visible)
{ return liveService.setVisible (session, visible); }
std::optional<InstrumentUiLiveService::Packet> DandrumAudioProcessor::takeLiveAnalysisPacket (std::uint64_t session)
{ return liveService.take (session); }
bool DandrumAudioProcessor::acknowledgeLiveAnalysisPacket (std::uint64_t session, std::uint32_t generation, std::uint64_t sequence)
{ return liveService.acknowledge (session, generation, sequence); }
InstrumentUiLiveService::Statistics DandrumAudioProcessor::getLiveAnalysisStatistics() const
{ return liveService.statistics(); }

void DandrumAudioProcessor::setMeterCaptureEnabled (bool enabled) noexcept
{
    meterCapture.setEnabled (enabled);
}

bool DandrumAudioProcessor::popMeterFrame (InstrumentUiMeterCapture::Frame& frame) noexcept
{
    if (meterDelivery.visibleCount() != 0)
        return false;
    return meterCapture.pop (frame);
}

std::uint64_t DandrumAudioProcessor::getDroppedMeterFrameCount() const noexcept
{
    return meterCapture.lostFrames();
}

InstrumentUiMeterCapture::ClipSnapshot DandrumAudioProcessor::getMeterClipSnapshot() const noexcept
{
    return meterCapture.clipSnapshot();
}

bool DandrumAudioProcessor::acknowledgeMeterClip (
    std::size_t channel, std::uint32_t generation, std::uint64_t ticket) noexcept
{
    return meterCapture.acknowledgeClip (channel, generation, ticket);
}

void DandrumAudioProcessor::refreshMeterCaptureSubscription (bool wasInactive) noexcept
{
    const bool active = meterDelivery.visibleCount() != 0;
    if (wasInactive && active)
    {
        InstrumentUiMeterCapture::Frame stale;
        for (std::size_t n = 0; n < InstrumentUiMeterCapture::capacity
                              && meterCapture.pop (stale); ++n) {}
        meterAggregation.observeLostFrames (meterCapture.lostFrames());
        meterAggregation.resetWindow();
    }
    if (! active)
        meterDisplay.reset();
    meterCapture.setEnabled (active);
}

bool DandrumAudioProcessor::subscribeMeter (
    std::uint64_t sessionId, std::uint32_t generation) noexcept
{
    const auto currentGeneration = getParameterSurfaceGeneration();
    meterDelivery.retireOtherGenerations (currentGeneration);
    if (generation != currentGeneration)
        return false;
    const bool wasInactive = meterDelivery.visibleCount() == 0;
    const bool accepted = meterDelivery.subscribe (sessionId, generation);
    refreshMeterCaptureSubscription (wasInactive);
    return accepted;
}

bool DandrumAudioProcessor::unsubscribeMeter (std::uint64_t sessionId) noexcept
{
    const bool removed = meterDelivery.unsubscribe (sessionId);
    refreshMeterCaptureSubscription (false);
    return removed;
}

bool DandrumAudioProcessor::setMeterSessionVisible (
    std::uint64_t sessionId, bool visible) noexcept
{
    const bool wasInactive = meterDelivery.visibleCount() == 0;
    const bool found = meterDelivery.setVisible (sessionId, visible);
    refreshMeterCaptureSubscription (wasInactive);
    return found;
}

void DandrumAudioProcessor::pollMeterDelivery() noexcept
{
    meterDelivery.retireOtherGenerations (getParameterSurfaceGeneration());
    refreshMeterCaptureSubscription (false);
    meterAggregation.resetWindow();
    InstrumentUiMeterCapture::Frame frame;
    bool observed = false;
    for (std::size_t n = 0; n < InstrumentUiMeterCapture::capacity
                          && meterCapture.pop (frame); ++n)
    {
        if (meterDelivery.visibleCount() != 0)
        {
            meterAggregation.append (frame);
            observed = true;
        }
    }
    meterAggregation.observeLostFrames (meterCapture.lostFrames());
    if (! observed && ! meterDisplay.snapshot().valid)
        return;
    InstrumentUiMeterDelivery::Packet packet;
    packet.meter = meterAggregation.snapshot();
    packet.clip = meterCapture.clipSnapshot();
    if (! meterDisplay.ingest (packet.meter, packet.clip,
                               juce::Time::getMillisecondCounterHiRes()))
        return;
    packet.display = meterDisplay.snapshot();
    meterDelivery.publish (packet);
}

std::optional<InstrumentUiMeterDelivery::Packet>
DandrumAudioProcessor::takeMeterPacket (std::uint64_t sessionId) noexcept
{
    return meterDelivery.take (sessionId);
}

bool DandrumAudioProcessor::acknowledgeMeterPacket (
    std::uint64_t sessionId, std::uint32_t generation, std::uint64_t sequence) noexcept
{
    return meterDelivery.acknowledge (sessionId, generation, sequence);
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
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return loadedPreset.name;
}

void DandrumAudioProcessor::changeProgramName (int, const juce::String&) {}

void DandrumAudioProcessor::getStateInformation (juce::MemoryBlock& destData)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    auto state = parameters.copyState();
    // A throwing host listener can prevent APVTS from marking its ValueTree
    // dirty. Serialize each fixed slot's authority, not that stale mirror.
    for (auto* host : getParameters())
    {
        auto& parameter = static_cast<PublicSlotParameter&> (*host);
        state.getChildWithProperty ("id", parameter.paramID).setProperty ("value", ReloadHostValues::read (parameter), nullptr);
    }
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
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (engineActivationInProgress) return;
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

    const EngineAccessPause handoff (*this);
    parameters.replaceState (state);

    if (candidateKernel != nullptr)
    {
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

        dandrum_kernel_destroy (previousKernel);
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

bool DandrumAudioProcessor::isInstrumentLoaded() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return instrumentLoaded;
}

juce::String DandrumAudioProcessor::getLastLoadError() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return lastLoadError;
}

juce::String DandrumAudioProcessor::getLastPresetError() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
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
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    for (const auto& slot : parameterSlots)
    {
        if (slot.active && slot.descriptor.id == parameterId)
            return parameters.getParameter (slot.slotParameterId);
    }

    return nullptr;
}

juce::String DandrumAudioProcessor::getPublicParameterDisplayName (juce::StringRef parameterId) const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    for (const auto& slot : parameterSlots)
        if (slot.active && slot.descriptor.id == parameterId)
            return slot.descriptor.name.isNotEmpty() ? slot.descriptor.name : slot.descriptor.id;

    return {};
}

juce::StringArray DandrumAudioProcessor::getActivePublicParameterIds() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
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
        values.push_back ({ slot.descriptor.id, slot.descriptor.name,
            ReloadHostValues::read (static_cast<const PublicSlotParameter&> (*parameter)) });
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
        // processBlock maps the prepared master bus to the stereo main output.
        document.outputBuses = hostOutputBuses;
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
            value.normalisedValue = ReloadHostValues::read (static_cast<const PublicSlotParameter&> (*parameter));
            value.normalisedDefaultValue = normalisePublicValue (
                slot.descriptor, slot.descriptor.defaultValue);
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
    document.capabilities.preparedWaveform = ! document.sources.empty();
    return document;
}

std::optional<std::uint64_t> DandrumAudioProcessor::requestPreparedWaveform (
    std::uint32_t expectedGeneration, const std::string& sourceId,
    const std::string& regionId, std::uint16_t channel, std::size_t bucketCount,
    std::uint64_t sessionId)
{
    InstrumentUiWaveformService::Source retainedSource;
    std::uint64_t startFrame = 0;
    std::uint64_t endFrame = 0;
    {
        const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
        if (expectedGeneration != getParameterSurfaceGeneration())
            return std::nullopt;
        const auto document = getPreparedUiDocument();
        if (! document)
            return std::nullopt;
        const auto source = std::find_if (document->sources.begin(), document->sources.end(),
            [&sourceId] (const auto& value) { return value.id == sourceId; });
        if (source == document->sources.end())
            return std::nullopt;
        const auto region = std::find_if (source->regions.begin(), source->regions.end(),
            [&regionId] (const auto& value) { return value.id == regionId; });
        if (region == source->regions.end())
            return std::nullopt;
        startFrame = region->startFrame;
        endFrame = region->endFrame;
        const auto sourceIndex = static_cast<std::size_t> (
            std::distance (document->sources.begin(), source));
        retainedSource.reset (dandrum_kernel_waveform_source_create (
            kernel.load (std::memory_order_acquire), sourceIndex));
    }
    return waveformService.request (expectedGeneration, std::move (retainedSource),
                                    regionId, channel, startFrame, endFrame, bucketCount,
                                    sessionId);
}

std::optional<InstrumentUiWaveformService::Snapshot>
DandrumAudioProcessor::getPreparedWaveformJobStatus (std::uint64_t jobId) const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return waveformService.status (jobId, getParameterSurfaceGeneration());
}

bool DandrumAudioProcessor::cancelPreparedWaveformJob (std::uint64_t jobId)
{
    return waveformService.cancel (jobId);
}

void DandrumAudioProcessor::cancelPreparedWaveformSession (std::uint64_t sessionId)
{
    waveformService.cancelSession (sessionId);
}

std::optional<std::uint64_t> DandrumAudioProcessor::requestPreparedSpectrum (
    std::uint32_t expectedGeneration, const std::string& sourceId,
    const std::string& regionId, std::uint16_t channel, std::uint64_t sessionId)
{
    InstrumentUiSpectralService::Source retainedSource;
    std::uint64_t startFrame = 0;
    std::uint64_t endFrame = 0;
    {
        const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
        if (expectedGeneration != getParameterSurfaceGeneration())
            return std::nullopt;
        const auto document = getPreparedUiDocument();
        if (! document)
            return std::nullopt;
        const auto source = std::find_if (document->sources.begin(), document->sources.end(),
            [&sourceId] (const auto& value) { return value.id == sourceId; });
        if (source == document->sources.end())
            return std::nullopt;
        const auto region = std::find_if (source->regions.begin(), source->regions.end(),
            [&regionId] (const auto& value) { return value.id == regionId; });
        if (region == source->regions.end())
            return std::nullopt;
        startFrame = region->startFrame;
        endFrame = region->endFrame;
        const auto sourceIndex = static_cast<std::size_t> (
            std::distance (document->sources.begin(), source));
        retainedSource.reset (dandrum_kernel_waveform_source_create (
            kernel.load (std::memory_order_acquire), sourceIndex));
    }
    return spectralService.request (expectedGeneration, std::move (retainedSource),
                                    regionId, channel, startFrame, endFrame, sessionId);
}
std::optional<InstrumentUiSpectralService::Snapshot>
DandrumAudioProcessor::getPreparedSpectrumJobStatus (std::uint64_t jobId) const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return spectralService.status (jobId, getParameterSurfaceGeneration());
}
bool DandrumAudioProcessor::cancelPreparedSpectrumJob (std::uint64_t jobId)
{
    return spectralService.cancel (jobId);
}
void DandrumAudioProcessor::cancelPreparedSpectrumSession (std::uint64_t sessionId)
{
    spectralService.cancelSession (sessionId);
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

SoundLabController* DandrumAudioProcessor::getSoundLabController() noexcept
{
    return soundLabController.get();
}

juce::File& DandrumAudioProcessor::getSoundLabReferenceFile() noexcept
{
    return soundLabReferenceFile;
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

juce::File DandrumAudioProcessor::currentInstrumentFile() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
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

juce::String DandrumAudioProcessor::currentInstrumentYaml() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return loadedInstrument.yamlContent;
}

juce::String DandrumAudioProcessor::currentPresetName() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return loadedPreset.name;
}

juce::String DandrumAudioProcessor::currentPresetYaml() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return loadedPreset.yamlContent;
}

juce::String DandrumAudioProcessor::getLastReloadWarning() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
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
    if (engineActivationInProgress) return false;
    const auto yamlText = yamlFile.loadFileAsString();
    loadedPreset = {};
    const auto reloaded = replaceActiveEngineFromFile (yamlFile, yamlFile, yamlText, true, false, &lastReloadWarning);
    if (reloaded)
        instrumentFileWatcher.watchFile (yamlFile);

    return reloaded;
}

std::optional<std::uint64_t> DandrumAudioProcessor::requestInstrumentReloadJob (
    const juce::File& yamlFile, std::uint32_t expectedGeneration)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (expectedGeneration != getParameterSurfaceGeneration()
        || getSampleRate() <= 0.0 || uiReloadActive || uiReloadShuttingDown)
        return std::nullopt;

    // A terminal worker has no further processor locks to acquire. Reap it
    // before replacing the future; never join a running preparation on admission.
    if (pendingUiReload.valid()) pendingUiReload.get();
    const auto sampleRate = static_cast<std::uint32_t> (juce::jmax (1.0, getSampleRate()));
    const auto preparedBlockSize = static_cast<std::size_t> (juce::jmax (1, getBlockSize()));
    const auto jobId = nextUiJobId++;
    uiJobHistory.push_back ({ jobId, UiJobState::running, expectedGeneration, {} });
    while (uiJobHistory.size() > 16) uiJobHistory.pop_front();

    const auto previousMute = isMuted();
    uiReloadActive = true;
    engineAccess.fetch_or (engineAccessClosed, std::memory_order_acq_rel);
    setMuted (true);
    try
    {
        pendingUiReload = std::async (std::launch::async,
            [this, jobId, yamlFile, expectedGeneration, sampleRate, preparedBlockSize, previousMute]
            {
                runInstrumentReloadJob (jobId, yamlFile, expectedGeneration,
                                        sampleRate, preparedBlockSize, previousMute);
            });
    }
    catch (const std::exception& error)
    {
        finishInstrumentReloadJob ({ jobId, UiJobState::failed, expectedGeneration,
                                     juce::String (error.what()) }, previousMute);
    }
    return jobId;
}

void DandrumAudioProcessor::runInstrumentReloadJob (
    std::uint64_t jobId, const juce::File& yamlFile, std::uint32_t expectedGeneration,
    std::uint32_t sampleRate, std::size_t preparedBlockSize, bool previousMute)
{
    UiJobStatus outcome { jobId, UiJobState::failed, expectedGeneration, {} };
    PreparedUiReload prepared;
    try
    {
        // Admission has closed new readers. The worker, never the caller or
        // audio callback, acknowledges any already-entered reader before work.
        waitForEngineReaders();
        if (beforeUiReloadPreparation) beforeUiReloadPreparation();
        if (!yamlFile.existsAsFile())
            prepared.error = "Instrument file does not exist: " + yamlFile.getFullPathName();
        else
        {
            prepared.yaml = yamlFile.loadFileAsString();
            const auto path = yamlFile.getFullPathName().toStdString();
            prepared.candidate.reset (prepareKernelWithPublicControls (path, sampleRate, preparedBlockSize));
            if (prepared.candidate == nullptr)
                prepared.error = "Failed to load instrument: " + yamlFile.getFullPathName();
            else
                prepared.descriptors = loadPublicParameterDescriptors (path);
        }

        const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
        if (uiReloadShuttingDown)
            outcome.error = "Processor closed while reload was preparing";
        else if (expectedGeneration != getParameterSurfaceGeneration()
            || static_cast<std::uint32_t> (juce::jmax (1.0, getSampleRate())) != sampleRate
            || static_cast<std::size_t> (juce::jmax (1, getBlockSize())) != preparedBlockSize)
        {
            outcome.state = UiJobState::stale;
            outcome.error = "Instrument changed while reload was preparing";
        }
        else if (prepared.candidate == nullptr)
            outcome.error = prepared.error;
        else
        {
            // Keep the working engine and metadata until the activation call
            // commits. Host notifications can throw while slots are updated.
            auto workingInstrument = loadedInstrument;
            auto workingSlots = parameterSlots;
            const auto workingError = lastLoadError;
            const auto workingWarning = lastReloadWarning;
            const auto workingLoaded = instrumentLoaded;
            auto* workingKernel = kernel.load (std::memory_order_relaxed);
            ReloadHostValues hostValues (parameters, getParameters());
            try
            {
                installPreparedEngine (prepared.candidate.get(), yamlFile,
                                       prepared.yaml, prepared.descriptors, false, &lastReloadWarning);
            }
            catch (...)
            {
                kernel.store (workingKernel, std::memory_order_release);
                loadedInstrument = std::move (workingInstrument);
                parameterSlots.swap (workingSlots);
                instrumentLoaded = workingLoaded;
                lastLoadError = workingError;
                lastReloadWarning = workingWarning;
                parameterSurfaceGeneration.store (expectedGeneration, std::memory_order_relaxed);
                waveformService.setGeneration (expectedGeneration);
                spectralService.setGeneration (expectedGeneration);
                liveService.setGeneration (expectedGeneration);
                replacementState.store (static_cast<int> (ReplacementState::Failed), std::memory_order_relaxed);
                throw;
            }
            static_cast<void> (prepared.candidate.release());
            hostValues.commit();
            loadedPreset = {};
            outcome.state = UiJobState::completed;
            outcome.generation = getParameterSurfaceGeneration();
            // Watching is ancillary to an already committed activation. An
            // observation error remains queryable without misreporting rollback.
            instrumentFileWatcher.watchFile (yamlFile);
        }
    }
    catch (const std::exception& error)
    {
        outcome.error = juce::String (error.what());
    }
    catch (...)
    {
        outcome.error = "Unexpected exception while reloading instrument";
    }
    // Failed/stale candidates and their assets are retired on this worker
    // before unmuting or admitting another job.
    prepared.candidate.reset();
    finishInstrumentReloadJob (std::move (outcome), previousMute);
}

void DandrumAudioProcessor::finishInstrumentReloadJob (UiJobStatus outcome, bool previousMute)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    // One active job and bounded terminal history: no later job can append
    // until this job publishes its terminal record under the same lock.
    uiJobHistory.back() = std::move (outcome);
    uiReloadActive = false;
    if (!uiReloadShuttingDown)
    {
        // Startup can fail before the worker acknowledges existing readers.
        // Reopen admission without erasing their outstanding ownership count.
        engineAccess.fetch_and (~engineAccessClosed, std::memory_order_release);
        setMuted (previousMute);
    }
}

std::optional<DandrumAudioProcessor::UiJobStatus>
DandrumAudioProcessor::getInstrumentUiJobStatus (std::uint64_t jobId)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    const auto job = std::find_if (uiJobHistory.begin(), uiJobHistory.end(),
        [jobId] (const UiJobStatus& item) { return item.id == jobId; });
    return job != uiJobHistory.end() ? std::optional<UiJobStatus> (*job) : std::nullopt;
}

bool DandrumAudioProcessor::reloadInstrumentFromYaml (const juce::String& yamlText,
                                                      const juce::File& sourceHint)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (engineActivationInProgress) return false;
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

juce::File DandrumAudioProcessor::watchedInstrumentFile() const
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    return instrumentFileWatcher.watchedFile();
}

void DandrumAudioProcessor::pollInstrumentFileForChanges()
{
    instrumentFileWatcher.poll();
}

bool DandrumAudioProcessor::loadPresetFromFile (const juce::File& presetFile)
{
    const std::lock_guard<std::recursive_mutex> reloadLock (reloadMutex);
    if (engineActivationInProgress) return false;
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
