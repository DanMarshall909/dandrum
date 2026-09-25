#pragma once

#include <atomic>
#include <cstdint>
#include <filesystem>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#include "DefaultPatch.h"

class SoundLabController final
{
public:
    enum class State
    {
        idle,
        rendering,
        ready,
        error,
    };

    struct Metric
    {
        double timeSeconds = 0.0;
        double rms = 0.0;
        double peak = 0.0;
        double spectralCentroidHz = 0.0;
        bool hasSpectralCentroid = false;
    };

    struct RenderData
    {
        std::uint32_t sampleRateHz = 0;
        std::uint64_t durationFrames = 0;
        std::vector<Metric> metrics;
        std::vector<std::uint8_t> wavBytes;
    };

    struct Snapshot
    {
        State state = State::idle;
        std::uint64_t generation = 0;
        std::shared_ptr<const RenderData> data;
        std::string error;
    };

    SoundLabController() = default;
    ~SoundLabController() = default;

    bool startRender (const std::filesystem::path& fixturePath);
    State state() const noexcept;
    std::uint64_t generation() const noexcept;
    Snapshot snapshot() const;

private:
    void renderOnWorker (std::filesystem::path fixturePath);

    std::atomic<State> currentState { State::idle };
    std::atomic<std::uint64_t> currentGeneration { 0 };
    mutable std::mutex resultMutex;
    std::shared_ptr<const RenderData> completedData;
    std::string lastError;
    // Declared last so its destructor joins before the state it writes is torn down.
    std::jthread worker;
};
