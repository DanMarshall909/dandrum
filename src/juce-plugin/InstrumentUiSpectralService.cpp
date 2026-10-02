#include "InstrumentUiSpectralService.h"

#include <algorithm>
#include <cassert>
#include <utility>
#include <cmath>
#include <bit>
#include <numbers>
#include <numeric>
#include <juce_dsp/juce_dsp.h>

InstrumentUiSpectralService::InstrumentUiSpectralService (Reader copy)
    : reader (copy ? std::move (copy)
                     : Reader { &dandrum_kernel_prepared_source_copy_channel }),
      worker ([this] (std::stop_token stop) { run (stop); })
{
}

InstrumentUiSpectralService::~InstrumentUiSpectralService()
{
    worker.request_stop();
    wake.notify_all();
}

void InstrumentUiSpectralService::setGeneration (std::uint32_t generation)
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

void InstrumentUiSpectralService::makeStatusRoom()
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

std::optional<std::uint64_t> InstrumentUiSpectralService::request (
    std::uint32_t generation, Source source, std::string regionId,
    std::uint16_t channel, std::uint64_t startFrame, std::uint64_t endFrame,
    std::uint64_t sessionId)
{
    DandrumKernelWaveformSourceInfo info {};
    if (! source || ! dandrum_kernel_waveform_source_info (source.get(), &info)
        || info.sourceId.size == 0 || regionId.empty()
        || channel >= info.channelCount || startFrame >= endFrame
        || endFrame > info.frameCount)
        return std::nullopt;

    Key key;
    key.sourceId.assign (info.sourceId.data, info.sourceId.size);
    key.regionId = std::move (regionId);
    std::copy_n (info.contentRevision, key.contentRevision.size(), key.contentRevision.begin());
    key.channel = channel;
    key.startFrame = startFrame;
    key.endFrame = endFrame;
    key.sampleRateHz = info.sampleRateHz;
    // Sparse column starts bound work; each FFT still reads contiguous PCM at
    // the original source rate. No audio downsampling or joined windows.
    key.settings.hopFrames = baseHopFrames
        * (1 + (endFrame - startFrame - 1) / (baseHopFrames * maxColumns));

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
        pending.push_back ({ id, generation, std::move (key), std::move (source) });
    lock.unlock();
    wake.notify_one();
    return id;
}

bool InstrumentUiSpectralService::cancel (std::uint64_t jobId)
{
    const std::scoped_lock lock (mutex);
    const auto found = statuses.find (jobId);
    if (found == statuses.end() || found->second.state != State::running)
        return false;
    found->second.state = State::cancelled;
    std::erase_if (pending, [jobId] (const Job& job) { return job.id == jobId; });
    return true;
}

void InstrumentUiSpectralService::cancelSession (std::uint64_t sessionId)
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

std::optional<InstrumentUiSpectralService::Snapshot>
InstrumentUiSpectralService::status (
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

std::shared_ptr<InstrumentUiSpectralService::Result>
InstrumentUiSpectralService::analyze (const Job& job) const
{
    auto result = std::make_shared<Result>();
    result->sourceId = job.key.sourceId;
    result->regionId = job.key.regionId;
    result->sampleRateHz = job.key.sampleRateHz;
    result->channel = job.key.channel;
    result->startFrame = job.key.startFrame;
    result->endFrame = job.key.endFrame;
    result->contentRevision = job.key.contentRevision;
    result->settings = job.key.settings;
    for (std::size_t bin = 0; bin < binCount; ++bin)
        result->frequencyHz[bin] = static_cast<double> (bin) * job.key.sampleRateHz / fftSize;
    std::array<float, fftSize> window;
    for (std::size_t n = 0; n < fftSize; ++n)
        window[n] = static_cast<float> (0.5 * (1.0 - std::cos (
            2.0 * std::numbers::pi * static_cast<double> (n) / fftSize)));
    const auto windowSum = std::accumulate (window.begin(), window.end(), 0.0);
    juce::dsp::FFT fft (std::countr_zero (fftSize));
    const auto span = job.key.endFrame - job.key.startFrame;
    const auto count = 1 + (span - 1) / job.key.settings.hopFrames;
    result->columns.reserve (static_cast<std::size_t> (count));
    for (std::uint64_t c = 0; c < count; ++c)
    {
        Column column;
        column.startFrame = job.key.startFrame + c * job.key.settings.hopFrames;
        const auto validFrames = static_cast<std::size_t> (
            std::min<std::uint64_t> (fftSize, job.key.endFrame - column.startFrame));
        column.endFrame = column.startFrame + validFrames;
        std::array<float, fftSize * 2> data {};
        if (! reader (job.source.get(), job.key.channel, column.startFrame, data.data(), validFrames))
            return {};
        for (std::size_t n = 0; n < validFrames; ++n)
        {
            if (! std::isfinite (data[n])) return {};
            data[n] *= window[n];
        }
        fft.performFrequencyOnlyForwardTransform (data.data(), true);
        for (std::size_t bin = 0; bin < binCount; ++bin)
        {
            const auto amplitude = data[bin] * (bin == 0 || bin == fftSize / 2 ? 1.0 : 2.0) / windowSum;
            column.magnitudeDbFS[bin] = amplitude > 0.0
                ? static_cast<float> (std::max<double> (floorDbFS, 20.0 * std::log10 (amplitude)))
                : floorDbFS;
        }
        result->columns.push_back (std::move (column));
    }
    return result;
}

void InstrumentUiSpectralService::run (std::stop_token stop)
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

        std::shared_ptr<Result> result;
        try { result = analyze (job); }
        catch (...) { result.reset(); }

        lock.lock();
        const auto finished = statuses.find (job.id);
        if (finished == statuses.end() || finished->second.state != State::running
            || job.generation != currentGeneration)
            continue;
        if (! result)
        {
            finished->second.state = State::failed;
            finished->second.error = "Prepared spectral analysis failed";
            continue;
        }

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
