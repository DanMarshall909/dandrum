#include "InstrumentUiLiveService.h"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <condition_variable>
#include <iostream>
#include <numbers>
#include <stdexcept>

namespace
{
using Service = InstrumentUiLiveService;
using namespace std::chrono_literals;
void require (bool value, const char* message)
{ if (! value) throw std::runtime_error (message); }
void near (double actual, double expected, double tolerance, const char* message)
{ require (std::isfinite (actual) && std::abs (actual - expected) <= tolerance, message); }
template<class Predicate> void await (Predicate predicate, const char* message)
{
    const auto deadline = std::chrono::steady_clock::now() + 3s;
    while (! predicate())
    {
        require (std::chrono::steady_clock::now() < deadline, message);
        std::this_thread::sleep_for (1ms);
    }
}
std::array<float, 1024> tone (unsigned bin, double amplitude)
{
    std::array<float, 1024> samples;
    for (std::size_t n = 0; n < samples.size(); ++n)
        samples[n] = static_cast<float> (amplitude * std::sin (2 * std::numbers::pi * bin * n / 1024));
    return samples;
}
class BatchGate
{
public:
    void hold (std::stop_token stop)
    {
        std::unique_lock lock (mutex);
        if (entered) return;
        entered = true; wake.notify_all();
        wake.wait (lock, stop, [&] { return released; });
    }
    void awaitEntry()
    {
        std::unique_lock lock (mutex);
        require (wake.wait_for (lock, 3s, [&] { return entered; }), "Actual worker did not reach its barrier");
    }
    void release() { const std::scoped_lock lock (mutex); released = true; wake.notify_all(); }
private:
    std::mutex mutex;
    std::condition_variable_any wake;
    bool entered = false, released = false;
};

void workerAndDelivery()
{
    static_assert (Service::maxSessions == 4);
    static_assert (sizeof (Service) <= 320 * 1024);
    const auto caller = std::this_thread::get_id();
    std::atomic<bool> workerObserved { false }, wrongThread { false };
    Service service ([&] (std::stop_token)
    {
        if (std::this_thread::get_id() == caller) wrongThread = true;
        workerObserved = true;
    });
    require (! service.subscribe (1, 0, 1), "Unprepared live worker admitted a subscriber");
    service.setGeneration (7);
    service.setGeneration (7);
    require (! service.subscribe (1, 6, 1) && ! service.subscribe (1, 7, 0)
        && ! service.subscribe (1, 7, 4), "Live worker admitted stale or invalid demand");
    require (service.subscribe (11, 7, 1), "Live worker rejected current subscriber");
    require (! service.subscribe (11, 7, 2), "Live worker admitted duplicate session");
    require (service.subscribe (22, 7, 2) && service.selectedChannels() == 3,
        "Live demand union lost independently subscribed channels");
    require (service.subscribe (33, 7, 1)
        && service.subscribe (44, 7, 3) && ! service.subscribe (55, 7, 1),
        "Live subscription table lost its fixed capacity");
    require (service.selectedChannels() == 3 && ! service.take (999),
        "Live demand union or unknown-session delivery is wrong");
    auto left = tone (64, 0.5), right = tone (96, 0.25);
    service.beginStream();
    service.capture (left.data(), right.data(), 1024, 7, 48000);
    std::optional<Service::Packet> first;
    await ([&] { first = service.take (11); return first.has_value(); }, "Actual live worker failed to publish");
    require (workerObserved && ! wrongThread && first->analysis.generation == 7
        && first->analysis.channels == 3 && first->analysis.startFrame == 0
        && first->analysis.endFrame == 1024 && first->analysis.gap,
        "Live analysis ran on the caller or lost stream identity");
    near (first->analysis.channel[0].scope[1].minimum, -0.5, 0, "Worker lost signed scope minimum");
    near (first->analysis.channel[0].magnitudeDbFS[64], -6.020599913, 0.0002, "Worker changed left spectral amplitude");
    near (first->analysis.channel[1].magnitudeDbFS[96], -12.041199826, 0.0002, "Worker changed right spectral amplitude");
    near (first->analysis.frequencyHz[64], 3000, 0, "Worker lost actual frequency coordinates");
    require (! service.take (11), "Unacknowledged live session received another packet");
    const auto other = service.take (22);
    require (other && other->sequence == first->sequence
        && other->analysis.endFrame == 1024, "Sessions did not receive the same owned numeric result");
    for (unsigned block = 0; block < 12; ++block)
        service.capture (left.data(), right.data(), 1024, 7, 48000);
    await ([&] { return service.statistics().consumedChunks >= 52; }, "Worker did not drain later capture");
    require (service.pendingPayloads() <= 8 && ! service.take (11), "Stalled live delivery grew or replayed history");
    require (! service.acknowledge (999, 7, first->sequence)
        && ! service.acknowledge (11, 6, first->sequence)
        && ! service.acknowledge (11, 7, first->sequence + 1)
        && service.acknowledge (11, 7, first->sequence)
        && ! service.acknowledge (11, 7, first->sequence), "Live acknowledgements lost identity or uniqueness");
    const auto latest = service.take (11);
    require (latest && latest->analysis.endFrame == 13312
        && latest->sequence > first->sequence,
        "Resumed live subscriber did not receive current coalesced coordinates");
    near (first->analysis.channel[0].scope[1].minimum, -0.5, 0, "Retained live packet changed after worker reuse");
    require (! service.setVisible (999, false) && service.setVisible (11, true)
        && service.setVisible (11, false) && ! service.take (11)
        && ! service.acknowledge (11, 7, latest->sequence), "Hidden live session retained delivery");
    require (service.unsubscribe (22) && ! service.unsubscribe (22)
        && service.setVisible (33, false) && service.setVisible (44, false)
        && service.selectedChannels() == 0, "Last hidden live session left capture enabled");
    service.capture (nullptr, nullptr, 256, 7, 48000);
    require (service.setVisible (11, true) && service.selectedChannels() == 1
        && ! service.take (11), "Visible live session replayed a hidden packet");
    service.capture (left.data(), right.data(), 1024, 7, 48000);
    std::optional<Service::Packet> resumed;
    await ([&] { resumed = service.take (11); return resumed.has_value(); }, "Reopened live session missed fresh analysis");
    require (resumed->analysis.startFrame == 13568 && resumed->analysis.endFrame == 14592
        && resumed->analysis.gap && resumed->analysis.channels == 1,
        "Reopened live analysis lost current coordinates or selected channel");
    service.setGeneration (8);
    require (service.selectedChannels() == 0 && service.pendingPayloads() == 0
        && ! service.take (11) && ! service.acknowledge (11, 7, resumed->sequence)
        && service.subscribe (11, 8, 2), "Reload did not retire old live sessions");
    service.beginStream();
    const auto beforeStale = service.statistics().consumedChunks;
    service.capture (left.data(), right.data(), 1024, 7, 44100);
    await ([&] { return service.statistics().consumedChunks >= beforeStale + 4; }, "Worker failed to discard obsolete generation");
    require (! service.take (11), "Worker published stale-generation PCM as current");
    service.capture (left.data(), right.data(), 1024, 8, 44100);
    std::optional<Service::Packet> reloaded;
    await ([&] { reloaded = service.take (11); return reloaded.has_value(); }, "Reloaded worker failed to resume");
    require (reloaded->analysis.generation == 8 && reloaded->analysis.startFrame == 1024
        && reloaded->analysis.endFrame == 2048 && reloaded->analysis.gap,
        "Reloaded worker replayed earlier generation/time");
    near (reloaded->analysis.frequencyHz[96], 4134.375, 0, "Reloaded worker lost source rate");
    require (service.unsubscribe (11) && service.selectedChannels() == 0,
        "Last closed live session left capture active");
}

void stoppableTeardown()
{
    std::mutex mutex;
    std::condition_variable_any wake;
    bool entered = false, exited = false;
    {
        Service service ([&] (std::stop_token stop)
        {
            std::unique_lock lock (mutex);
            entered = true; wake.notify_all();
            wake.wait (lock, stop, [] { return false; });
            exited = true;
        });
        service.setGeneration (1);
        require (service.subscribe (1, 1, 1), "Stoppable live worker rejected demand");
        std::unique_lock lock (mutex);
        require (wake.wait_for (lock, 3s, [&] { return entered; }), "Live teardown worker never reached barrier");
    }
    require (exited, "Off-audio live destruction did not stop and join its held worker");
    Service idle;
}

void boundedBacklog()
{
    for (const auto chunk : { 64U, 128U, 256U })
    {
        BatchGate gate;
        Service service ([&] (std::stop_token stop) { gate.hold (stop); });
        service.setGeneration (7);
        require (service.subscribe (11, 7, 1), "Backlog worker rejected subscription");
        service.beginStream(); gate.awaitEntry();
        auto pcm = tone (96, 0.25);
        for (unsigned n = 0; n < 64; ++n)
            service.capture (pcm.data() + n * chunk % 1024, nullptr, chunk, 7, 48000);
        gate.release();
        await ([&] { return service.statistics().consumedChunks == 64; }, "Backlog worker did not finish bounded drain");
        const auto packet = service.take (11);
        require (packet && packet->analysis.startFrame == 64 * chunk - 1024
            && packet->analysis.endFrame == 64 * chunk && packet->analysis.gap,
            "Worker failed to identify discarded backlog gap at current coordinates");
        const auto stats = service.statistics();
        require (stats.discardedChunks == 64 - 2048 / chunk && stats.analyzedWindows == 5
            && stats.analyzedWindows <= Service::maxWindowsPerBatch && stats.droppedCaptureChunks == 0,
            "Worker processed obsolete backlog beyond declared frame/window limits");
        near (packet->analysis.channel[0].magnitudeDbFS[96], -12.041199826, 0.0002,
            "Bounded backlog changed contiguous sine measurement");
    }
}
void overflowRecovery()
{
    BatchGate gate;
    Service service ([&] (std::stop_token stop) { gate.hold (stop); });
    service.setGeneration (7);
    require (service.subscribe (11, 7, 1), "Overflow worker rejected subscription");
    service.beginStream(); gate.awaitEntry();
    auto before = tone (64, 0.5), after = tone (96, 0.25);
    for (unsigned n = 0; n < 17; ++n) service.capture (before.data(), nullptr, 1024, 7, 48000);
    gate.release();
    await ([&] { return service.statistics().consumedChunks == 64; }, "Overflow worker failed to discard old queue");
    require (! service.take (11), "Worker published obsolete history after capture overflow");
    const auto stats = service.statistics();
    require (stats.discardedChunks == 64 && stats.analyzedWindows == 0 && stats.droppedCaptureChunks == 4,
        "Overflow recovery lost bounded loss/work accounting");
    service.capture (after.data(), nullptr, 512, 7, 48000);
    await ([&] { return service.statistics().consumedChunks == 66; }, "Recovery worker missed first half window");
    require (! service.take (11), "Overflow recovery emitted a partial window");
    service.capture (after.data() + 512, nullptr, 512, 7, 48000);
    std::optional<Service::Packet> packet;
    await ([&] { packet = service.take (11); return packet.has_value(); }, "Overflow worker failed fresh contiguous recovery");
    require (packet->analysis.startFrame == 17408 && packet->analysis.endFrame == 18432 && packet->analysis.gap,
        "Overflow recovery replayed old sample coordinates");
    near (packet->analysis.channel[0].magnitudeDbFS[96], -12.041199826, 0.0002, "Recovered spectrum lost post-gap tone");
    require (packet->analysis.channel[0].magnitudeDbFS[64] < -115,
        "Overflow recovery joined PCM from opposite sides of loss");
}
void selectionAndGenerationRaces()
{
    for (const bool reload : { false, true })
    {
        BatchGate gate;
        Service service ([&] (std::stop_token stop) { gate.hold (stop); });
        service.setGeneration (7);
        require (service.subscribe (11, 7, 1), "Race worker rejected subscription");
        service.beginStream(); gate.awaitEntry();
        auto old = tone (64, 0.5), fresh = tone (96, 0.25);
        service.capture (old.data(), nullptr, 1024, 7, 48000);
        const auto generation = reload ? 8U : 7U;
        if (reload)
        {
            service.setGeneration (8);
            require (service.subscribe (11, 8, 1), "Reload race could not reopen subscription");
            service.beginStream();
        }
        else
            require (service.setVisible (11, false) && service.setVisible (11, true),
                "Selection race could not hide/show between callbacks");
        service.capture (fresh.data(), nullptr, 512, generation, 44100);
        gate.release();
        await ([&] { return service.statistics().consumedChunks == 6; }, "Race worker did not consume changed selection");
        require (! service.take (11) && service.statistics().analyzedWindows == 0,
            "Worker analyzed obsolete queued selection or generation");
        service.capture (fresh.data() + 512, nullptr, 512, generation, 44100);
        std::optional<Service::Packet> packet;
        await ([&] { packet = service.take (11); return packet.has_value(); }, "Worker discarded fresh PCM after selection race");
        require (packet->analysis.generation == generation && packet->analysis.gap
            && packet->analysis.startFrame == (reload ? 0U : 1024U)
            && packet->analysis.endFrame == (reload ? 1024U : 2048U),
            "Race recovery lost current stream coordinates");
        near (packet->analysis.channel[0].magnitudeDbFS[96], -12.041199826, 0.0002, "Race recovery lost fresh tone");
    }
}
void completedResultRace()
{
    BatchGate gate;
    Service service ({}, [&] (std::stop_token stop) { gate.hold (stop); });
    service.setGeneration (7);
    require (service.subscribe (11, 7, 1), "Publication race rejected subscription");
    service.beginStream();
    auto pcm = tone (64, 0.5);
    service.capture (pcm.data(), nullptr, 1024, 7, 48000);
    gate.awaitEntry();
    service.setGeneration (8);
    require (service.subscribe (11, 8, 1), "Publication race failed current resubscription");
    gate.release();
    await ([&] { return service.statistics().consumedChunks == 4; }, "Completed analysis did not leave publication barrier");
    require (! service.take (11), "Worker published obsolete completed analysis");
}
void gapSurvivesCoalescing()
{
    BatchGate gate;
    Service service ([&] (std::stop_token stop) { gate.hold (stop); });
    service.setGeneration (7);
    require (service.subscribe (11, 7, 1), "Gap coalescing worker rejected demand");
    service.beginStream(); gate.awaitEntry();
    auto pcm = tone (96, 0.25);
    for (unsigned n = 0; n < 64; ++n)
        service.capture (pcm.data() + n * 128 % 1024, nullptr, 128, 7, 48000);
    gate.release();
    await ([&] { return service.statistics().consumedChunks == 64; }, "Gap worker missed bounded batch");
    service.capture (pcm.data(), nullptr, 1024, 7, 48000);
    await ([&] { return service.statistics().consumedChunks == 68; }, "Gap worker missed contiguous continuation");
    const auto packet = service.take (11);
    require (packet && packet->analysis.endFrame == 9216 && packet->analysis.gap,
        "Live coalescing lost an unobserved gap");
}
void tinyChunks()
{
    Service service;
    service.setGeneration (7);
    require (service.subscribe (11, 7, 1), "Tiny-chunk worker rejected demand");
    service.beginStream();
    auto pcm = tone (64, 0.5);
    for (unsigned batch = 0; batch < 16; ++batch)
    {
        for (unsigned n = batch * 64; n < (batch + 1) * 64; ++n)
            service.capture (pcm.data() + n, nullptr, 1, 7, 48000);
        await ([&] { return service.statistics().consumedChunks == (batch + 1) * 64; },
            "Worker failed to accumulate small contiguous chunks");
        if (batch != 15) require (! service.take (11), "Worker padded a partial tiny-chunk window");
    }
    const auto packet = service.take (11);
    require (packet && packet->analysis.startFrame == 0 && packet->analysis.endFrame == 1024
        && service.statistics().analyzedWindows == 1 && service.statistics().droppedCaptureChunks == 0,
        "Worker backlog policy starved valid tiny-chunk accumulation");
    near (packet->analysis.channel[0].magnitudeDbFS[64], -6.020599913, 0.0002,
        "Tiny-chunk worker corrupted contiguous spectral input");
}
}
int main()
{
    try { workerAndDelivery(); stoppableTeardown(); boundedBacklog(); overflowRecovery();
          selectionAndGenerationRaces(); completedResultRace(); gapSurvivesCoalescing(); tinyChunks(); }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
    std::cout << "LIVE_SERVICE actual worker and bounded delivery PASS\n";
}
