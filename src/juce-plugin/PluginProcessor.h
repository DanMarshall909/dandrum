#pragma once

#include <atomic>
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
    bool hasPublicParameter (juce::StringRef parameterId) const;

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

    /// The currently loaded instrument's source file, if loaded from one.
    const juce::File& currentInstrumentFile() const noexcept;

    /// The currently loaded instrument's YAML content, captured at load time
    /// so it can be embedded in plugin state without depending on the source
    /// file still existing at the original path.
    const juce::String& currentInstrumentYaml() const noexcept;

    /// Non-empty after a reload that left one or more previously-live public
    /// parameters without a resolvable target in the new instrument (the
    /// fixed APVTS surface keeps exposing them, but they no longer reach any
    /// engine parameter). Empty after construction and after any reload that
    /// dropped nothing.
    const juce::String& getLastReloadWarning() const noexcept;

private:
    /// Sentinel used throughout the engine-slot resolution path for "this
    /// target has no live engine slot" (unresolved, unknown, or dropped by a
    /// reload).
    static constexpr std::intptr_t kNoEngineSlot = -1;

    struct ParameterSlot
    {
        const std::atomic<float>* rawValue = nullptr;
        // One resolved slot per internal target a public parameter fans out
        // to (composite bindings can map a single public parameter to more
        // than one internal parameter); kNoEngineSlot marks an unresolved
        // target. Built once off the audio thread in
        // preparePublicParameterSlots() and only read on the audio thread.
        std::vector<std::intptr_t> engineSlotIndices;
        float lastAppliedValue = 0.0f;
    };

    /// The plugin's explicit concept of "the currently loaded immutable
    /// instrument definition" — distinct from the mutable public parameter
    /// values held in `parameters` (the APVTS).
    struct LoadedInstrument
    {
        juce::File sourceFile;   // restore hint only, per design.md
        juce::String yamlContent;
    };

    struct PublicParameterDescriptor
    {
        juce::String id;
        juce::String name;
        float defaultValue = 0.0f;
        float minValue = 0.0f;
        float maxValue = 1.0f;
    };

    static std::vector<PublicParameterDescriptor> loadPublicParameterDescriptors (const std::string& patchPath);
    static juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout (
        const std::vector<PublicParameterDescriptor>& descriptors);

    bool loadDefaultInstrument();
    void renderSilence (juce::AudioBuffer<float>& buffer) const;
    /// Rebuilds parameterSlots for instrumentFile, parsing its public
    /// parameter descriptors first. When droppedParametersWarning is
    /// non-null, it is cleared and then set to describe any previously-live
    /// public parameter that instrumentFile no longer defines a target for.
    void preparePublicParameterSlots (const juce::File& instrumentFile,
                                       juce::String* droppedParametersWarning = nullptr);
    /// Same as above, but from already-parsed descriptors — used at
    /// construction to avoid parsing the default patch's public parameters
    /// twice (once for the APVTS layout, once for slot resolution).
    void preparePublicParameterSlots (const std::vector<PublicParameterDescriptor>& descriptors,
                                       juce::String* droppedParametersWarning = nullptr);
    void applyChangedParameters (DandrumEngine* activeEngine) noexcept;

    std::vector<PublicParameterDescriptor> defaultParameterDescriptors;
    juce::AudioProcessorValueTreeState parameters;
    std::atomic<DandrumEngine*> engine { nullptr };
    bool instrumentLoaded = false;
    juce::String lastLoadError;
    juce::String lastReloadWarning;
    LoadedInstrument loadedInstrument;
    std::vector<ParameterSlot> parameterSlots;
    std::atomic<bool> muted { false };
    // Serializes reloadInstrumentFromFile() against itself: without this, two
    // overlapping reloads can each capture the other's just-published engine
    // as their own "previous" and destroy it while it is still in use.
    std::mutex reloadMutex;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (DandrumAudioProcessor)
};