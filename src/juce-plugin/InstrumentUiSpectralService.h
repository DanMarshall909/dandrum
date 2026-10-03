#pragma once

#include "InstrumentUiWaveformService.h"
#include "InstrumentUiSpectrumAnalysis.h"

// Prepared spectral analysis belongs to the UI service layer, never the audio
// render dependency chain. Jobs own an independently retained source.
class InstrumentUiSpectralService
{
public:
    using Source = InstrumentUiWaveformService::Source;
    using Reader = std::function<bool (const DandrumKernelWaveformSource*, std::uint16_t,
                                       std::uint64_t, float*, std::size_t)>;
    static constexpr std::size_t fftSize = InstrumentUiSpectrumAnalysis::fftSize;
    static constexpr std::size_t binCount = InstrumentUiSpectrumAnalysis::binCount;
    static constexpr std::uint64_t baseHopFrames = InstrumentUiSpectrumAnalysis::hopFrames;
    static constexpr std::size_t maxColumns = 1024;
    static constexpr std::size_t maxPending = 4;
    static constexpr std::size_t maxHistory = 16;
    static constexpr std::size_t maxCacheEntries = 4;
    static constexpr float floorDbFS = InstrumentUiSpectrumAnalysis::floorDbFS;

    using Window = InstrumentUiSpectrumAnalysis::Window;
    using Scaling = InstrumentUiSpectrumAnalysis::Scaling;
    using ChannelPolicy = InstrumentUiSpectrumAnalysis::ChannelPolicy;
    enum class State { running, ready, failed, cancelled, stale };

    using Settings = InstrumentUiSpectrumAnalysis::Settings;

    struct Column
    {
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0; // Actual input end; the rest of the window is zero padded.
        std::array<float, binCount> magnitudeDbFS {};
        bool operator== (const Column&) const = default;
    };

    struct Result
    {
        std::string sourceId;
        std::string regionId;
        std::uint32_t sampleRateHz = 0;
        std::uint16_t channel = 0;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
        std::array<std::uint8_t, 32> contentRevision {};
        Settings settings;
        std::array<double, binCount> frequencyHz {};
        std::vector<Column> columns;
    };

    struct Snapshot
    {
        std::uint64_t jobId = 0;
        std::uint64_t sessionId = 0;
        std::uint32_t generation = 0;
        State state = State::running;
        std::shared_ptr<const Result> result;
        std::string error;
    };

    explicit InstrumentUiSpectralService (Reader reader = {});
    ~InstrumentUiSpectralService();
    void setGeneration (std::uint32_t generation);
    std::optional<std::uint64_t> request (std::uint32_t generation, Source source,
                                          std::string regionId, std::uint16_t channel,
                                          std::uint64_t startFrame, std::uint64_t endFrame,
                                          std::uint64_t sessionId = 0);
    std::optional<Snapshot> status (
        std::uint64_t jobId, std::optional<std::uint32_t> visibleGeneration = std::nullopt) const;
    bool cancel (std::uint64_t jobId);
    void cancelSession (std::uint64_t sessionId);

private:
    struct Key
    {
        std::string sourceId;
        std::string regionId;
        std::array<std::uint8_t, 32> contentRevision {};
        std::uint32_t sampleRateHz = 0;
        std::uint16_t channel = 0;
        std::uint64_t startFrame = 0;
        std::uint64_t endFrame = 0;
        Settings settings;
        auto operator<=> (const Key&) const = default;
    };
    struct Job
    {
        std::uint64_t id = 0;
        std::uint32_t generation = 0;
        Key key;
        Source source;
    };
    std::shared_ptr<Result> analyze (const Job& job) const;
    void makeStatusRoom();
    void run (std::stop_token stop);

    Reader reader;
    mutable std::mutex mutex;
    std::condition_variable_any wake;
    std::uint32_t currentGeneration = 0;
    std::uint64_t nextJobId = 1;
    std::deque<Job> pending;
    std::map<std::uint64_t, Snapshot> statuses;
    std::deque<std::uint64_t> statusOrder;
    std::map<Key, std::shared_ptr<const Result>> cache;
    std::deque<Key> cacheOrder;
    // Joined before state/source storage is destroyed; teardown is off audio.
    std::jthread worker;
};
