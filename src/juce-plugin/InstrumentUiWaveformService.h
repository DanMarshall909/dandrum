#pragma once

#include "RustEngineBindings.h"

#include <array>
#include <compare>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

// Owns bounded prepared-waveform jobs and numeric cache outside the audio
// callback. A retained Rust source is moved into each admitted job; no worker
// holds a pointer to a reloadable instrument or to a renderer.
class InstrumentUiWaveformService final
{
public:
    struct SourceDeleter
    {
        void operator() (DandrumKernelWaveformSource* source) const noexcept
        {
            dandrum_kernel_waveform_source_destroy (source);
        }
    };
    using Source = std::unique_ptr<DandrumKernelWaveformSource, SourceDeleter>;
    using Reducer = std::function<bool (const DandrumKernelWaveformSource*, std::uint16_t,
                                        std::uint64_t, std::uint64_t,
                                        DandrumKernelWaveformBucket*, std::size_t)>;

    static constexpr std::size_t maxPending = 8;
    static constexpr std::size_t maxHistory = 64;
    static constexpr std::size_t maxCacheEntries = 8;
    static constexpr std::size_t maxBuckets = 4096;

    enum class State { running, ready, failed, cancelled, stale };

    struct Result
    {
        std::string sourceId;
        std::string regionId;
        std::uint32_t sampleRateHz = 0;
        std::uint16_t channel = 0;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
        std::array<std::uint8_t, 32> contentRevision {};
        std::vector<DandrumKernelWaveformBucket> buckets;
    };

    struct Snapshot
    {
        std::uint64_t jobId = 0;
        std::uint32_t generation = 0;
        State state = State::running;
        std::shared_ptr<const Result> result;
        std::string error;
    };

    explicit InstrumentUiWaveformService (Reducer reducer = {});
    ~InstrumentUiWaveformService();

    void setGeneration (std::uint32_t generation);
    std::optional<std::uint64_t> request (std::uint32_t generation, Source source,
                                          std::string regionId, std::uint16_t channel,
                                          std::uint64_t startFrame, std::uint64_t endFrame,
                                          std::size_t bucketCount);
    bool cancel (std::uint64_t jobId);
    std::optional<Snapshot> status (
        std::uint64_t jobId, std::optional<std::uint32_t> visibleGeneration = std::nullopt) const;

private:
    struct Key
    {
        std::string sourceId;
        std::string regionId;
        std::array<std::uint8_t, 32> contentRevision {};
        std::uint16_t channel = 0;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
        std::size_t bucketCount = 0;

        auto operator<=> (const Key&) const = default;
    };

    struct Job
    {
        std::uint64_t id = 0;
        std::uint32_t generation = 0;
        Key key;
        std::uint32_t sampleRateHz = 0;
        Source source;
    };

    void makeStatusRoom();
    void run (std::stop_token stop);

    Reducer reducer;
    mutable std::mutex mutex;
    std::condition_variable_any wake;
    std::uint32_t currentGeneration = 0;
    std::uint64_t nextJobId = 1;
    std::deque<Job> pending;
    std::map<std::uint64_t, Snapshot> statuses;
    std::deque<std::uint64_t> statusOrder;
    std::map<Key, std::shared_ptr<const Result>> cache;
    std::deque<Key> cacheOrder;
    // Destroyed first: its destructor joins before any state it reads is freed.
    std::jthread worker;
};
