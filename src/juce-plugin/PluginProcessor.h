#pragma once

#include <array>
#include <atomic>
#include <cstdint>
#include <map>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include <juce_audio_processors/juce_audio_processors.h>

#include "InstrumentFileWatcher.h"
#include "InstrumentDemoConfiguration.h"
#include "InstrumentUiCommands.h"
#include "InstrumentUiDocument.h"
#include "InstrumentUiParameterState.h"
#include "RustEngineBindings.h"

class DandrumAudioProcessor final : public juce::AudioProcessor, private InstrumentUiCommandHost
{
public:
    struct PublicParameterSnapshotEntry
    {
        juce::String id;
        juce::String displayName;
        float normalisedValue = 0.0f;
    };

    explicit DandrumAudioProcessor (
        InstrumentDemoConfiguration configuration = InstrumentDemoConfiguration::tb303());
    ~DandrumAudioProcessor() override;

    using juce::AudioProcessor::processBlock;

    void prepareToPlay (double sampleRate, int samplesPerBlock) override;
    void releaseResources() override;
    bool isBusesLayoutSupported (const BusesLayout& layouts) const override;
    void processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages) override;

    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override;

    const juce::String getName() const override;
    bool acceptsMidi() const override;
    bool producesMidi() const override;
    double getTailLengthSeconds() const override;

    int getNumPrograms() override;
    int getCurrentProgram() override;
    void setCurrentProgram (int index) override;
    const juce::String getProgramName (int index) override;
    void changeProgramName (int index, const juce::String& newName) override;

    void getStateInformation (juce::MemoryBlock& destData) override;
    void setStateInformation (const void* data, int sizeInBytes) override;

    bool isInstrumentLoaded() const noexcept;
    const juce::String& getLastLoadError() const noexcept;
    const juce::String& getLastPresetError() const noexcept;
    const InstrumentDemoConfiguration& demoConfiguration() const noexcept;
    bool hasPublicParameter (juce::StringRef parameterId) const;
    juce::RangedAudioParameter* getParameterForPublicId (juce::StringRef parameterId) const;
    juce::String getPublicParameterDisplayName (juce::StringRef parameterId) const;
    juce::StringArray getActivePublicParameterIds() const;
    std::vector<PublicParameterSnapshotEntry> getPublicParameterSnapshot() const;
    std::optional<InstrumentUiDocument> getPreparedUiDocument() const;
    InstrumentUiParameterState getUiParameterState() const;
    std::uint32_t getParameterSurfaceGeneration() const noexcept;
    InstrumentUiCommandService& uiCommands() noexcept;

    /// Publishes bounded, per-note editor intent for delivery by processBlock.
    /// The message thread never calls the Rust engine directly. Session 0 is
    /// reserved for existing direct callers; each Web editor uses its own ID.
    bool enqueueEditorNoteOn (int noteNumber, float velocity, std::uint64_t sessionId = 0) noexcept;
    bool enqueueEditorNoteOff (int noteNumber, std::uint64_t sessionId = 0) noexcept;
    void closeEditorNoteSession (std::uint64_t sessionId) noexcept;

    /// Silences audio output during an explicit instrument-replacement
    /// transaction. Safe to call from any thread; processBlock reads this
    /// atomically each block.
    void setMuted (bool shouldMute) noexcept;
    bool isMuted() const noexcept;

    /// Loads a replacement instrument from a YAML file as an explicit,
    /// off-audio-thread replacement transaction: a candidate engine is built
    /// and validated first, and the active engine is only swapped once the
    /// candidate is fully prepared. On failure the previously active
    /// instrument keeps running unchanged and getLastLoadError() reports why.
    /// Fails if the host has not yet called prepareToPlay (the sample rate is
    /// not yet known). Safe to call from multiple threads: concurrent calls
    /// are serialized so one reload's engine can never be destroyed while
    /// another is still building on it. Must not be called from processBlock.
    bool reloadInstrumentFromFile (const juce::File& yamlFile);

    /// Loads an owned YAML snapshot while retaining sourceHint as the
    /// instrument's provenance and relative-asset root. The snapshot is staged
    /// beside sourceHint for Rust's path-based loader, then removed after the
    /// replacement transaction. Must not be called from processBlock.
    bool reloadInstrumentFromYaml (const juce::String& yamlText,
                                   const juce::File& sourceHint);

    /// Enables/disables watching the loaded instrument file for external
    /// edits. While disabled, external edits do not trigger a reload and the
    /// running instrument is left unchanged until watching is re-enabled or a
    /// manual reload is requested. Safe to call from the message thread.
    void setFileWatchEnabled (bool shouldWatch);
    bool isFileWatchEnabled() const noexcept;

    /// The instrument file currently being watched for external edits, if any.
    const juce::File& watchedInstrumentFile() const noexcept;

    /// Polls the watched instrument file once for a stable external change,
    /// reloading through the standard replacement transaction if one is found.
    /// The plugin drives this from a low-frequency background timer; it is
    /// exposed so tests can drive polling deterministically. Never called from
    /// processBlock.
    void pollInstrumentFileForChanges();

    /// Applies a compatible preset as mutable public-parameter state for the
    /// currently loaded immutable instrument. This never reparses, rewrites, or
    /// replaces the loaded instrument YAML.
    bool loadPresetFromFile (const juce::File& presetFile);

    /// The currently loaded instrument's source file, if loaded from one. This
    /// is only a restore hint; plugin state embeds the YAML content too.
    const juce::File& currentInstrumentFile() const noexcept;
    bool isSoundLabInstrumentCompatible() const;

    /// The currently loaded instrument's YAML content, captured at load time
    /// so it can be embedded in plugin state without depending on the source
    /// file still existing at the original path.
    const juce::String& currentInstrumentYaml() const noexcept;

    const juce::String& currentPresetName() const noexcept;
    const juce::String& currentPresetYaml() const noexcept;

    /// Non-empty after a reload that dropped previously-live public parameters
    /// or exceeded the fixed host slot budget.
    const juce::String& getLastReloadWarning() const noexcept;

    /// Current explicit replacement transaction phase for the editor/status
    /// surface. The audio callback only reads the atomic mute flag.
    juce::String replacementTransactionState() const;

    /// Number of web-editor MIDI events rejected because the fixed-capacity
    /// message-thread-to-audio-thread queue was full.
    std::size_t getDroppedMidiEventCount() const noexcept;

private:
    std::uint32_t uiCommandGeneration() const noexcept override;
    InstrumentUiCommandStatus applyUiParameter (
        std::uint32_t generation, const std::string& id, float normalisedValue,
        bool withinGesture) override;
    InstrumentUiGestureAdmission beginUiGesture (
        std::uint32_t generation, const std::string& id) override;
    void endUiGesture (std::size_t hostSlot) override;

    static constexpr std::intptr_t kNoEngineSlot = -1;
    static constexpr int kPublicParameterSlotCount = 64;
    static constexpr int kPluginStateSchemaVersion = 1;

    enum class ReplacementState : int
    {
        Running = 0,
        Muted,
        Failed,
    };

    struct PublicParameterDescriptor
    {
        juce::String id;
        juce::String name;
        float defaultValue = 0.0f;
        float minValue = 0.0f;
        float maxValue = 1.0f;
    };

    struct ParameterSlot
    {
        juce::String slotParameterId;
        bool active = false;
        PublicParameterDescriptor descriptor;
        const std::atomic<float>* rawValue = nullptr;
        std::intptr_t kernelSlotIndex = kNoEngineSlot;
        float lastAppliedNormalisedValue = 0.0f;
    };

    /// The plugin's explicit concept of "the currently loaded immutable
    /// instrument definition" — distinct from the mutable public parameter
    /// values held in the fixed APVTS slots.
    struct LoadedInstrument
    {
        juce::File sourceFile;   // restore hint only, per design.md
        juce::String yamlContent;
        juce::String instrumentId;
        int presetSchemaVersion = 0;
    };

    struct LoadedPreset
    {
        juce::File sourceFile;
        juce::String name;
        juce::String yamlContent;
    };

    struct ParsedPreset
    {
        juce::String name;
        juce::String instrumentId;
        int presetSchemaVersion = 0;
        std::map<juce::String, double> values;
        juce::String yamlContent;
        juce::String error;
    };

    static juce::String publicSlotParameterId (int slotIndex);
    static std::vector<PublicParameterDescriptor> loadPublicParameterDescriptors (const std::string& patchPath);
    static juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout (
        const InstrumentDemoConfiguration& initialConfiguration);
    static bool readInstrumentIdentity (const juce::String& yaml, juce::String& instrumentId, int& schemaVersion);
    static ParsedPreset parsePresetFile (const juce::File& presetFile);
    static float clampToDescriptorRange (const PublicParameterDescriptor& descriptor, float value) noexcept;
    static float normalisePublicValue (const PublicParameterDescriptor& descriptor, float value) noexcept;
    static float denormalisePublicValue (const PublicParameterDescriptor& descriptor, float normalisedValue) noexcept;

    bool loadDefaultInstrument();
    bool replaceActiveEngineFromFile (const juce::File& yamlFile,
                                      const juce::File& sourceHint,
                                      const juce::String& yamlText,
                                      bool requirePreparedHost,
                                      bool preferCurrentSlotValues,
                                      juce::String* reloadWarning);
    void renderSilence (juce::AudioBuffer<float>& buffer) const;
    void preparePublicParameterSlots (const juce::File& instrumentFile,
                                      juce::String* droppedParametersWarning,
                                      bool preferCurrentSlotValues);
    void preparePublicParameterSlots (const std::vector<PublicParameterDescriptor>& descriptors,
                                      juce::String* droppedParametersWarning,
                                      bool preferCurrentSlotValues);
    void applyChangedParameters (DandrumKernelInstrument* activeKernel) noexcept;
    void applySlotToKernel (ParameterSlot& slot, float normalisedValue, DandrumKernelInstrument* activeKernel) noexcept;
    void setSlotNormalisedValue (int slotIndex, float normalisedValue);
    void publishEditorNoteIntent (std::uint8_t note, std::uint8_t velocity) noexcept;
    void deliverEditorNoteIntent (
        DandrumKernelInstrument* activeKernel,
        const std::array<bool, 128>& hostTouchedNotes) noexcept;
    void clearEditorNoteIntentForReload() noexcept;

    const InstrumentDemoConfiguration configuration;
    juce::AudioProcessorValueTreeState parameters;
    std::atomic<DandrumKernelInstrument*> kernel { nullptr };
    bool instrumentLoaded = false;
    juce::String lastLoadError;
    juce::String lastPresetError;
    juce::String lastReloadWarning;
    LoadedInstrument loadedInstrument;
    LoadedPreset loadedPreset;
    std::vector<ParameterSlot> parameterSlots;
    std::atomic<bool> muted { false };
    std::atomic<int> replacementState { static_cast<int> (ReplacementState::Running) };
    std::atomic<std::uint32_t> parameterSurfaceGeneration { 0 };
    std::atomic<std::size_t> droppedMidiEventCount { 0 };
    // One admission writer and one audio reader. The upper 48 bits count note
    // onsets, then one byte retains their velocity and the low byte is desired
    // gate velocity. An on/off pair between callbacks still carries an onset.
    static_assert (std::atomic<std::uint64_t>::is_always_lock_free);
    // Reload may run off the message thread; only admission/teardown takes
    // this lock. The audio callback reads atomic intent without locking.
    std::mutex editorNoteCommandMutex;
    std::array<std::atomic<std::uint64_t>, 128> editorNoteIntent {};
    std::array<std::optional<std::uint64_t>, 128> editorNoteOwner {};
    std::array<std::uint64_t, 128> lastAudioEditorOnSequence {};
    std::array<bool, 128> audioEditorNoteHeld {};
    std::array<bool, 128> audioEditorTransientRelease {};
    std::array<bool, 128> audioHostNoteHeld {};
    std::array<bool, 128> audioHostReleasePending {};
    DandrumKernelInstrument* lastAudioKernel = nullptr;
    // Serializes engine replacement, reprepare, and UI metadata snapshots.
    // Readers copy retained metadata before a previous engine is destroyed.
    // Host notifications may re-enter snapshot readers on the same thread.
    mutable std::recursive_mutex reloadMutex;
    InstrumentUiCommandService uiCommandService { *this };
    // Watches the loaded instrument file for external edits and reloads it
    // through the standard replacement transaction. Declared last so it is
    // destroyed (and its timer stopped) before the members its callback uses.
    InstrumentFileWatcher instrumentFileWatcher;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessor)
};
