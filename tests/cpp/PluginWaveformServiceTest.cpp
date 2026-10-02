#include "InstrumentUiWaveformService.h"

#include <atomic>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <future>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace
{
using Service = InstrumentUiWaveformService;
using Engine = std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)>;

void require (bool condition, const char* message)
{
    if (! condition)
        throw std::runtime_error (message);
}

struct Fixture
{
    Fixture()
    {
        const auto root = std::filesystem::path (__FILE__).parent_path().parent_path().parent_path();
        directory = std::filesystem::temp_directory_path()
            / ("dandrum-waveform-service-" + std::to_string (
                std::chrono::steady_clock::now().time_since_epoch().count()));
        std::filesystem::create_directories (directory);
        std::filesystem::copy_file (root / "examples/patches/assets/advanced-drums.wav",
                                    directory / "source.wav");
        std::ofstream (directory / "instrument.yaml")
            << "metadata: { name: waveform-service }\n"
               "assets:\n"
               "  sample_sources:\n"
               "    - id: drums\n"
               "      path: source.wav\n"
               "      regions:\n"
               "        - { id: full, start_frame: 0, end_frame: 8 }\n"
               "ports:\n"
               "  - { name: master, direction: output, signal: audio, channels: 1, maps_from: osc.audio }\n"
               "modules:\n"
               "  - { id: osc, type: oscillator }\n";
    }

    ~Fixture() { std::filesystem::remove_all (directory); }

    Engine prepare() const
    {
        const DandrumKernelBusDeclaration bus { "master", 2, 1 };
        return { dandrum_kernel_prepare_file ((directory / "instrument.yaml").string().c_str(),
                                              48000, 8, &bus, 1), &dandrum_kernel_destroy };
    }

    void replaceFirstSampleWithPositiveHalf() const
    {
        std::fstream wav (directory / "source.wav", std::ios::in | std::ios::out | std::ios::binary);
        wav.seekp (44);
        const char positiveHalf[] { 0, 0x40 };
        wav.write (positiveHalf, sizeof positiveHalf);
        require (wav.good(), "could not replace source PCM at the same path");
    }

    std::filesystem::path directory;
};

Service::Source sourceFrom (const Engine& engine)
{
    return Service::Source (dandrum_kernel_waveform_source_create (engine.get(), 0));
}

Service::Snapshot waitForReady (const Service& service, std::uint64_t id)
{
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds (2);
    while (std::chrono::steady_clock::now() < deadline)
    {
        const auto snapshot = service.status (id);
        if (snapshot && snapshot->state == Service::State::ready)
            return *snapshot;
        std::this_thread::sleep_for (std::chrono::milliseconds (1));
    }
    throw std::runtime_error ("waveform job did not become ready");
}

struct PromiseRelease
{
    explicit PromiseRelease (std::promise<void>& promiseToRelease) : promise (promiseToRelease) {}
    ~PromiseRelease() { open(); }

    void open()
    {
        if (! released)
        {
            promise.set_value();
            released = true;
        }
    }

    std::promise<void>& promise;
    bool released = false;
};
}

int main()
{
    try
    {
        Fixture fixture;
        auto engine = fixture.prepare();
        require (engine != nullptr, "waveform fixture did not prepare");
        std::atomic<int> reductions { 0 };
        Service service ([&] (const DandrumKernelWaveformSource* source, std::uint16_t channel,
                             std::uint64_t start, std::uint64_t end,
                             DandrumKernelWaveformBucket* buckets, std::size_t count)
        {
            ++reductions;
            return dandrum_kernel_waveform_reduce (source, channel, start, end, buckets, count);
        });
        service.setGeneration (1);
        service.setGeneration (1);
        const auto first = service.request (1, sourceFrom (engine), "full", 0, 0, 1, 1);
        require (first.has_value(), "prepared waveform request was rejected");
        auto ready = waitForReady (service, *first);
        require (ready.result && ready.result->sourceId == "drums"
                     && ready.result->regionId == "full" && ready.result->sampleRateHz == 48000
                     && ready.result->buckets.size() == 1
                     && ready.result->buckets[0].minimum == -0.5f
                     && ready.result->buckets[0].maximum == -0.5f,
                 "worker lost source identity, rate or known signed samples");
        const auto cached = service.request (1, sourceFrom (engine), "full", 0, 0, 1, 1);
        require (cached.has_value() && service.status (*cached)->state == Service::State::ready
                     && reductions == 1,
                 "identical prepared waveform did not reuse cached numeric data");
        const auto otherRegion = service.request (1, sourceFrom (engine), "alternate", 0, 0, 1, 1);
        require (otherRegion.has_value(), "distinct region was rejected");
        waitForReady (service, *otherRegion);
        require (reductions == 2, "region identity was omitted from the cache key");

        fixture.replaceFirstSampleWithPositiveHalf();
        auto replacement = fixture.prepare();
        require (replacement != nullptr, "same-path replacement did not prepare");
        const auto changed = service.request (1, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (changed.has_value(), "changed content was rejected");
        ready = waitForReady (service, *changed);
        require (reductions == 3 && ready.result->buckets[0].minimum == 0.5f
                     && ready.result->buckets[0].maximum == 0.5f,
                 "same-path replacement reused the old waveform");
        require (! service.request (0, sourceFrom (replacement), "full", 0, 0, 1, 1)
                     && ! service.request (1, {}, "full", 0, 0, 1, 1)
                     && ! service.request (1, sourceFrom (replacement), "full", 2, 0, 1, 1)
                     && ! service.request (1, sourceFrom (replacement), "full", 0, 0, 51001, 1),
                 "invalid generation, source, channel or region range was admitted");
        require (! service.cancel (*first) && ! service.cancel (999999)
                     && ! service.status (999999),
                 "finished or unknown jobs were reported as cancellable or present");

        for (int region = 0; region < 64; ++region)
        {
            const auto id = service.request (1, sourceFrom (replacement),
                                             "history-" + std::to_string (region), 0, 0, 1, 1);
            require (id.has_value(), "a completed job did not free queue capacity");
            waitForReady (service, *id);
        }
        require (! service.status (*first), "bounded history retained its oldest completed job");
        const auto reloadedCache = service.request (1, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (reloadedCache.has_value(), "evicted waveform was rejected");
        waitForReady (service, *reloadedCache);
        require (reductions == 68, "bounded cache reused an evicted numeric result");

        Service failed ([] (const DandrumKernelWaveformSource*, std::uint16_t,
                           std::uint64_t, std::uint64_t,
                           DandrumKernelWaveformBucket*, std::size_t) { return false; });
        failed.setGeneration (1);
        const auto failedId = failed.request (1, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (failedId.has_value(), "failed reduction request was rejected before running");
        const auto failureDeadline = std::chrono::steady_clock::now() + std::chrono::seconds (2);
        while (failed.status (*failedId)->state == Service::State::running
               && std::chrono::steady_clock::now() < failureDeadline)
            std::this_thread::sleep_for (std::chrono::milliseconds (1));
        require (failed.status (*failedId)->state == Service::State::failed
                     && ! failed.status (*failedId)->error.empty(),
                 "reducer failure was not reported to its requester");

        Service throwing ([] (const DandrumKernelWaveformSource*, std::uint16_t,
                             std::uint64_t, std::uint64_t,
                             DandrumKernelWaveformBucket*, std::size_t) -> bool
        {
            throw std::runtime_error ("reducer failed");
        });
        throwing.setGeneration (1);
        const auto throwingId = throwing.request (1, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (throwingId.has_value(), "throwing reduction request was rejected before running");
        const auto throwingDeadline = std::chrono::steady_clock::now() + std::chrono::seconds (2);
        while (throwing.status (*throwingId)->state == Service::State::running
               && std::chrono::steady_clock::now() < throwingDeadline)
            std::this_thread::sleep_for (std::chrono::milliseconds (1));
        require (throwing.status (*throwingId)->state == Service::State::failed,
                 "worker did not contain a reducer exception");

        std::promise<void> duplicateEntered;
        std::promise<void> duplicateRelease;
        const auto duplicateResume = duplicateRelease.get_future().share();
        std::atomic<int> duplicateReductions { 0 };
        Service duplicate ([&] (const DandrumKernelWaveformSource* source, std::uint16_t channel,
                                std::uint64_t start, std::uint64_t end,
                                DandrumKernelWaveformBucket* buckets, std::size_t count)
        {
            if (++duplicateReductions == 1)
            {
                duplicateEntered.set_value();
                duplicateResume.wait();
            }
            return dandrum_kernel_waveform_reduce (source, channel, start, end, buckets, count);
        });
        PromiseRelease duplicateGuard (duplicateRelease);
        duplicate.setGeneration (1);
        const auto duplicateFirst = duplicate.request (1, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (duplicateFirst.has_value()
                     && duplicateEntered.get_future().wait_for (std::chrono::seconds (2))
                            == std::future_status::ready,
                 "first duplicate request did not enter the worker");
        const auto duplicateSecond = duplicate.request (1, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (duplicateSecond.has_value(), "queued duplicate request was rejected");
        duplicateGuard.open();
        waitForReady (duplicate, *duplicateFirst);
        waitForReady (duplicate, *duplicateSecond);
        require (duplicateReductions == 1, "queued duplicate repeated the same reduction");

        std::promise<void> historyEntered;
        std::promise<void> historyRelease;
        const auto historyResume = historyRelease.get_future().share();
        std::atomic<int> historyReductions { 0 };
        Service fullHistory ([&] (const DandrumKernelWaveformSource* source, std::uint16_t channel,
                                  std::uint64_t start, std::uint64_t end,
                                  DandrumKernelWaveformBucket* buckets, std::size_t count)
        {
            if (++historyReductions == 2)
            {
                historyEntered.set_value();
                historyResume.wait();
            }
            return dandrum_kernel_waveform_reduce (source, channel, start, end, buckets, count);
        });
        PromiseRelease historyGuard (historyRelease);
        fullHistory.setGeneration (1);
        const auto seed = fullHistory.request (1, sourceFrom (replacement), "seed", 0, 0, 1, 1);
        require (seed.has_value(), "history cache seed was rejected");
        waitForReady (fullHistory, *seed);
        const auto historyActive = fullHistory.request (1, sourceFrom (replacement), "active", 0, 0, 1, 1);
        require (historyActive.has_value()
                     && historyEntered.get_future().wait_for (std::chrono::seconds (2))
                            == std::future_status::ready,
                 "history fixture did not hold an active job");
        for (std::size_t i = 0; i < Service::maxHistory; ++i)
            require (fullHistory.request (1, sourceFrom (replacement), "seed", 0, 0, 1, 1)
                         .has_value(),
                     "full history rejected a cached request while one job was running");
        require (fullHistory.status (*historyActive)->state == Service::State::running,
                 "bounded history evicted an active job");
        historyGuard.open();
        waitForReady (fullHistory, *historyActive);

        std::promise<void> entered;
        std::promise<void> release;
        std::promise<void> reductionFinished;
        const auto resume = release.get_future().share();
        std::atomic<bool> retainedRead { false };
        Service stalled ([&] (const DandrumKernelWaveformSource* source, std::uint16_t channel,
                             std::uint64_t start, std::uint64_t end,
                             DandrumKernelWaveformBucket* buckets, std::size_t count)
        {
            entered.set_value();
            resume.wait();
            const auto reduced = dandrum_kernel_waveform_reduce (
                source, channel, start, end, buckets, count);
            retainedRead = reduced && buckets[0].minimum == 0.5f
                && buckets[0].maximum == 0.5f;
            reductionFinished.set_value();
            return reduced;
        });
        PromiseRelease stalledGuard (release);
        stalled.setGeneration (7);
        const auto active = stalled.request (7, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (active.has_value() && entered.get_future().wait_for (std::chrono::seconds (2))
                     == std::future_status::ready,
                 "stalled worker never began reduction");
        const auto queued = stalled.request (7, sourceFrom (replacement), "full", 0, 0, 1, 1);
        require (queued.has_value() && stalled.cancel (*queued)
                     && stalled.status (*queued)->state == Service::State::cancelled,
                 "queued analysis did not cancel without waiting for the worker");
        const auto staleQueued = stalled.request (7, sourceFrom (replacement), "stale", 0, 0, 1, 1);
        require (staleQueued.has_value(), "queued reload fixture was rejected");
        for (std::size_t i = 1; i < Service::maxPending; ++i)
            require (stalled.request (7, sourceFrom (replacement),
                                      "pending-" + std::to_string (i), 0, 0, 1, 1).has_value(),
                     "bounded queue rejected an admitted request");
        require (! stalled.request (7, sourceFrom (replacement), "overflow", 0, 0, 1, 1),
                 "bounded queue admitted work past its limit");
        replacement.reset();
        stalled.setGeneration (8);
        require (stalled.status (*active)->state == Service::State::stale
                     && stalled.status (*staleQueued)->state == Service::State::stale
                     && stalled.status (*queued)->state == Service::State::cancelled,
                 "reload did not retire the old analysis job");
        stalledGuard.open();
        require (reductionFinished.get_future().wait_for (std::chrono::seconds (2))
                     == std::future_status::ready && retainedRead,
                 "worker lost retained PCM when the instrument was destroyed");
        require (stalled.status (*active)->state == Service::State::stale,
                 "old worker result overwrote the replacement generation");
    }
    catch (const std::exception& error)
    {
        std::cerr << error.what() << '\n';
        return 1;
    }
    return 0;
}
