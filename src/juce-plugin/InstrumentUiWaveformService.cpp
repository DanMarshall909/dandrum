#include "InstrumentUiWaveformService.h"

#include <algorithm>
#include <cassert>
#include <utility>

InstrumentUiWaveformService::InstrumentUiWaveformService (Reducer reduction)
    : reducer (reduction ? std::move (reduction)
                         : Reducer { &dandrum_kernel_waveform_reduce }),
      worker ([this] (std::stop_token stop) { run (stop); })
{
}

InstrumentUiWaveformService::~InstrumentUiWaveformService()
{
    worker.request_stop();
    wake.notify_all();
}

void InstrumentUiWaveformService::setGeneration (std::uint32_t generation)
{
    const std::scoped_lock lock (mutex);
    if (generation == currentGeneration)
        return;
    currentGeneration = generation;
    for (auto& [id, snapshot] : statuses)
        if (snapshot.generation != generation && snapshot.state != State::cancelled)
        {
            snapshot.state = State::stale;
            snapshot.result.reset();
        }
    std::erase_if (pending, [generation] (const Job& job)
    {
        return job.generation != generation;
    });
}

void InstrumentUiWaveformService::makeStatusRoom()
{
    if (statuses.size() < maxHistory)
        return;
    // The worker and bounded queue hold at most maxPending + 1 running jobs.
    static_assert (maxHistory > maxPending + 1);
    const auto oldestCompleted = std::find_if (statusOrder.begin(), statusOrder.end(),
                                               [this] (std::uint64_t id)
    {
        return statuses.at (id).state != State::running;
    });
    assert (oldestCompleted != statusOrder.end());
    statuses.erase (*oldestCompleted);
    statusOrder.erase (oldestCompleted);
}

std::optional<std::uint64_t> InstrumentUiWaveformService::request (
    std::uint32_t generation, Source source, std::string regionId,
    std::uint16_t channel, std::uint64_t startFrame, std::uint64_t endFrame,
    std::size_t bucketCount, std::uint64_t sessionId)
{
    DandrumKernelWaveformSourceInfo info {};
    if (! source || ! dandrum_kernel_waveform_source_info (source.get(), &info)
        || info.sourceId.size == 0 || regionId.empty()
        || channel >= info.channelCount || startFrame >= endFrame
        || endFrame > info.frameCount || bucketCount == 0
        || bucketCount > maxBuckets || bucketCount > endFrame - startFrame)
        return std::nullopt;

    Key key;
    key.sourceId.assign (info.sourceId.data, info.sourceId.size);
    key.regionId = std::move (regionId);
    std::copy_n (info.contentRevision, key.contentRevision.size(), key.contentRevision.begin());
    key.channel = channel;
    key.startFrame = startFrame;
    key.endFrame = endFrame;
    key.bucketCount = bucketCount;

    std::unique_lock lock (mutex);
    if (generation != currentGeneration)
        return std::nullopt;
    const auto cached = cache.find (key);
    if (cached == cache.end() && pending.size() >= maxPending)
        return std::nullopt;
    makeStatusRoom();

    const auto id = nextJobId++;
    Snapshot snapshot;
    snapshot.jobId = id;
    snapshot.sessionId = sessionId;
    snapshot.generation = generation;
    if (cached != cache.end())
    {
        snapshot.state = State::ready;
        snapshot.result = cached->second;
    }
    statuses.emplace (id, std::move (snapshot));
    statusOrder.push_back (id);
    if (cached == cache.end())
        pending.push_back ({ id, generation, std::move (key), info.sampleRateHz,
                             std::move (source) });
    lock.unlock();
    wake.notify_one();
    return id;
}

bool InstrumentUiWaveformService::cancel (std::uint64_t jobId)
{
    const std::scoped_lock lock (mutex);
    const auto found = statuses.find (jobId);
    if (found == statuses.end() || found->second.state != State::running)
        return false;
    found->second.state = State::cancelled;
    std::erase_if (pending, [jobId] (const Job& job) { return job.id == jobId; });
    return true;
}

void InstrumentUiWaveformService::cancelSession (std::uint64_t sessionId)
{
    if (sessionId == 0)
        return;
    const std::scoped_lock lock (mutex);
    for (auto& [id, snapshot] : statuses)
        if (snapshot.sessionId == sessionId && snapshot.state == State::running)
            snapshot.state = State::cancelled;
    std::erase_if (pending, [this, sessionId] (const Job& job)
    {
        return statuses.at (job.id).sessionId == sessionId;
    });
}

std::optional<InstrumentUiWaveformService::Snapshot>
InstrumentUiWaveformService::status (
    std::uint64_t jobId, std::optional<std::uint32_t> visibleGeneration) const
{
    const std::scoped_lock lock (mutex);
    if (const auto found = statuses.find (jobId); found != statuses.end())
    {
        auto snapshot = found->second;
        if (visibleGeneration && snapshot.generation != *visibleGeneration
            && snapshot.state != State::cancelled)
        {
            snapshot.state = State::stale;
            snapshot.result.reset();
        }
        return snapshot;
    }
    return std::nullopt;
}

void InstrumentUiWaveformService::run (std::stop_token stop)
{
    while (! stop.stop_requested())
    {
        std::unique_lock lock (mutex);
        if (! wake.wait (lock, stop, [this] { return ! pending.empty(); }))
            return;
        auto job = std::move (pending.front());
        pending.pop_front();
        // Cancellation and generation changes remove queued jobs under this mutex.
        assert (job.generation == currentGeneration);
        auto& state = statuses.at (job.id);
        assert (state.state == State::running);
        if (const auto cached = cache.find (job.key); cached != cache.end())
        {
            state.state = State::ready;
            state.result = cached->second;
            continue;
        }
        lock.unlock();

        std::vector<DandrumKernelWaveformBucket> buckets (job.key.bucketCount);
        bool succeeded = false;
        try
        {
            succeeded = reducer (job.source.get(), job.key.channel,
                                 job.key.startFrame, job.key.endFrame,
                                 buckets.data(), buckets.size());
        }
        catch (...)
        {
            succeeded = false;
        }

        lock.lock();
        const auto finished = statuses.find (job.id);
        if (finished == statuses.end() || finished->second.state != State::running
            || job.generation != currentGeneration)
            continue;
        if (! succeeded)
        {
            finished->second.state = State::failed;
            finished->second.error = "Prepared waveform reduction failed";
            continue;
        }

        auto result = std::make_shared<Result>();
        result->sourceId = job.key.sourceId;
        result->regionId = job.key.regionId;
        result->sampleRateHz = job.sampleRateHz;
        result->channel = job.key.channel;
        result->startFrame = job.key.startFrame;
        result->endFrame = job.key.endFrame;
        result->contentRevision = job.key.contentRevision;
        result->buckets = std::move (buckets);
        finished->second.state = State::ready;
        finished->second.result = result;
        if (cache.size() == maxCacheEntries)
        {
            cache.erase (cacheOrder.front());
            cacheOrder.pop_front();
        }
        cacheOrder.push_back (job.key);
        cache.emplace (std::move (job.key), std::move (result));
    }
}
