#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

// Copied prepared facts for either editor. No member points into a reloadable
// engine, sample buffer, JUCE component, browser object, or source YAML.
struct InstrumentUiDocument
{
    enum class ControlScope { instrument, sampleGroup };

    struct Parameter
    {
        std::string id;
        std::string name;
        float normalisedValue = 0.0f;
        // Loaded instrument default, independently of the fixed host slot default.
        float normalisedDefaultValue = 0.0f;
        float minValue = 0.0f;
        float maxValue = 1.0f;
        ControlScope scope = ControlScope::instrument;
        std::optional<int> controlGroup;
    };

    struct RegionLoop
    {
        std::string mode;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
        double crossfadeMs = 0.0;
    };

    struct Region
    {
        std::string id;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
        std::optional<int> rootNote;
        std::optional<double> gainDb;
        std::optional<double> pan;
        bool reverse = false;
        double fadeInMs = 0.0;
        double fadeOutMs = 0.0;
        std::optional<RegionLoop> loop;
    };

    struct Slice
    {
        std::string id;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
    };

    struct Source
    {
        std::string id;
        std::uint32_t sampleRateHz = 0;
        std::uint16_t channelCount = 0;
        std::uint64_t frameCount = 0;
        std::vector<Region> regions;
        std::vector<Slice> slices;
    };

    struct Zone
    {
        std::string id;
        std::size_t sourceIndex = 0;
        std::size_t regionIndex = 0;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
        std::uint8_t keyLow = 0;
        std::uint8_t keyHigh = 0;
        std::uint8_t velocityLow = 0;
        std::uint8_t velocityHigh = 0;
        std::string roundRobinGroup;
        std::string chokeGroup;
        std::optional<int> controlGroup;
        std::uint32_t weight = 1;
        std::optional<double> gainDb;
        std::optional<double> pan;
        std::optional<double> pitchSemitones;
    };

    struct Map
    {
        std::string id;
        std::string selectionMode;
        std::uint64_t selectionSeed = 0;
        std::vector<Zone> zones;
    };

    struct Capabilities
    {
        bool sampleKeyMap = false;
        bool preparedWaveform = false;
        bool synthLayer = false;
        bool nestedPatchLayer = false;
        bool moduleChain = false;
        bool patternSequencer = false;
        bool hostTransport = false;
    };

    std::uint32_t generation = 0;
    std::string instrumentId;
    std::vector<Parameter> parameters;
    std::vector<Source> sources;
    std::vector<Map> maps;
    Capabilities capabilities;
};
