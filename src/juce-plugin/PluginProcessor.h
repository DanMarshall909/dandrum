#pragma once

#include <atomic>
#include <cstdint>
#include <map>
#include <mutex>
#include <string>
#include <vector>

#include <juce_audio_processors/juce_audio_processors.h>

#include "RustEngineBindings.h"

class DandrumAudioProcessor final : public juce::AudioProcessor
{
public:
    DandrumAudioProcessor();
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
    bool hasPublicParameter (juce::StringRef parameterId) const;
    juce::RangedAudioParameter* getParameterForPublicId (juce::StringRef parameterId) const;
    juce::String getPublicParameterDisplayName (juce::StringRef parameterId) const;
    juce::StringArray getActivePublicParameterIds() const;
    std::uint32_t getParameterSurfaceGeneration() const noexcept;

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

    /// Applies a compatible preset as mutable public-parameter state for the
    /// currently loaded immutable instrument. This never reparses, rewrites, or
    /// replaces the loaded instrument YAML.
    bool loadPresetFromFile (const juce::File& presetFile);

    /// The currently loaded instrument's source file, if loaded from one. This
    /// is only a restore hint; plugin state embeds the YAML content too.
    const juce::File& currentInstrumentFile() const noexcept;

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

    /// Diagnostic counter placeholder for the plugin surface. The bounded MIDI
    /// queue currently lives inside the Rust runtime; v1 exposes the status
    /// surface now and can wire a richer Rust counter without UI churn later.
    std::size_t getDroppedMidiEventCount() const noexcept;

private:
    static constexpr std::intptr_t kNoEngineSlot = -1;
    static constexpr int kPublicParameterSlotCount = 64;
    static constexpr int kPluginStateSchemaVersion = 1;

    enum class ReplacementState : int
    {
        Running = 0,
        Validating,
        Muted,
        Compiling,
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
        std::vector<std::intptr_t> engineSlotIndices;
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
    static juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout();
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
    void applyChangedParameters (DandrumEngine* activeEngine) noexcept;
    void applySlotToEngine (ParameterSlot& slot, float normalisedValue, DandrumEngine* activeEngine) noexcept;
    void setSlotNormalisedValue (int slotIndex, float normalisedValue, bool notifyHost);

    juce::AudioProcessorValueTreeState parameters;
    std::atomic<DandrumEngine*> engine { nullptr };
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
    // Serializes reloadInstrumentFromFile()/setStateInformation() replacement
    // work against itself: without this, overlapping reloads can each capture
    // the other's just-published engine as their own "previous" and destroy it
    // while it is still in use.
    std::mutex reloadMutex;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessor)
};
