#include "InstrumentUiSpectralService.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <future>
#include <iostream>
#include <limits>
#include <numbers>
#include <stdexcept>
#include <tuple>

namespace
{
using Service = InstrumentUiSpectralService;
using Engine = std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)>;
void require (bool condition, const char* message)
{
    if (! condition) throw std::runtime_error (message);
}

// Independent PCM16 fixture: 128-frame lead, 2048-frame bin-64 left/bin-96
// right sine, silence, DC, and Nyquist regions. No production WAV encoder.
struct Fixture
{
    Fixture()
    {
        directory = std::filesystem::temp_directory_path()
            / ("dandrum-spectral-" + std::to_string (
                std::chrono::steady_clock::now().time_since_epoch().count()));
        std::filesystem::create_directories (directory);
        write();
        std::ofstream (directory / "instrument.yaml")
            << "metadata: { name: spectral-test }\nassets:\n  sample_sources:\n"
               "    - id: tones\n      path: source.wav\n      regions:\n"
               "        - { id: tone, start_frame: 128, end_frame: 2176 }\n"
               "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: osc.audio }\n"
               "modules:\n  - { id: osc, type: oscillator }\n";
    }
    ~Fixture() { std::filesystem::remove_all (directory); }
    void write (double amplitude = 0.5, std::size_t frameCount = 5248) const
    {
        std::ofstream wav (directory / "source.wav", std::ios::binary);
        auto u16 = [&] (std::uint16_t value)
        { wav.put (static_cast<char> (value)); wav.put (static_cast<char> (value >> 8)); };
        auto u32 = [&] (std::uint32_t value)
        { u16 (static_cast<std::uint16_t> (value)); u16 (static_cast<std::uint16_t> (value >> 16)); };
        wav.write ("RIFF", 4); u32 (36 + static_cast<std::uint32_t> (frameCount * 4));
        wav.write ("WAVEfmt ", 8); u32 (16); u16 (1); u16 (2); u32 (48000);
        u32 (192000); u16 (4); u16 (16); wav.write ("data", 4); u32 (static_cast<std::uint32_t> (frameCount * 4));
        for (std::size_t i = 0; i < frameCount; ++i)
        {
            const auto n = static_cast<double> (i) - 128.0;
            const auto left = i < 128 ? 0.75 : i < 2176
                ? amplitude * std::sin (std::numbers::pi * 2.0 * 64.0 * n / 1024.0)
                : i < 3200 ? 0.0 : i < 4224 ? 0.25 : (i % 2 == 0 ? 0.25 : -0.25);
            const auto right = 0.25 * std::sin (std::numbers::pi * 2.0 * 96.0 * n / 1024.0);
            u16 (static_cast<std::uint16_t> (static_cast<std::int16_t> (std::lround (left * 32768.0))));
            u16 (static_cast<std::uint16_t> (static_cast<std::int16_t> (std::lround (right * 32768.0))));
        }
        require (wav.good(), "PCM fixture write failed");
    }
    Engine prepare() const
    {
        const DandrumKernelBusDeclaration bus { "master", 2, 1 };
        // Deliberately different host rate: spectral analysis uses source rate.
        return { dandrum_kernel_prepare_file ((directory / "instrument.yaml").string().c_str(),
                                              44100, 64, &bus, 1), &dandrum_kernel_destroy };
    }
    std::filesystem::path directory;
};
Service::Source sourceFrom (const Engine& engine)
{ return Service::Source (dandrum_kernel_waveform_source_create (engine.get(), 0)); }

Service::Snapshot terminal (const Service& service, std::uint64_t id)
{
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds (2);
    while (std::chrono::steady_clock::now() < deadline)
    {
        auto state = service.status (id);
        if (state && state->state != Service::State::running) return *state;
        std::this_thread::sleep_for (std::chrono::milliseconds (1));
    }
    throw std::runtime_error ("Spectral job did not finish");
}
Service::Snapshot analyze (Service& service, const Engine& engine, std::string region,
                           std::uint16_t channel, std::uint64_t start, std::uint64_t end)
{
    const auto id = service.request (1, sourceFrom (engine), std::move (region), channel, start, end);
    require (id.has_value(), "Prepared spectral request was rejected");
    auto state = terminal (service, *id);
    require (state.state == Service::State::ready && state.result,
             "Prepared spectral job did not produce numeric results");
    return state;
}
void near (double actual, double expected, double tolerance, const char* message)
{ require (std::isfinite (actual) && std::abs (actual - expected) <= tolerance, message); }
std::size_t dominant (const Service::Column& column)
{
    return static_cast<std::size_t> (std::distance (column.magnitudeDbFS.begin(),
        std::max_element (column.magnitudeDbFS.begin(), column.magnitudeDbFS.end())));
}

void measurement()
{
    Fixture fixture;
    auto engine = fixture.prepare();
    require (engine != nullptr, "Spectral fixture did not prepare");
    const auto caller = std::this_thread::get_id();
    std::atomic<int> reads { 0 };
    Service service ([&] (auto source, auto channel, auto start, auto output, auto count)
    {
        require (std::this_thread::get_id() != caller, "PCM read ran on the requesting thread");
        ++reads;
        return dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
    });
    service.setGeneration (1);
    service.setGeneration (1);
    const auto ready = analyze (service, engine, "tone", 0, 128, 2176);
    const auto& result = *ready.result;
    require (result.sourceId == "tones" && result.regionId == "tone"
        && result.sampleRateHz == 48000 && result.channel == 0
        && result.startFrame == 128 && result.endFrame == 2176
        && result.settings.fftSize == 1024 && result.settings.hopFrames == 256
        && result.settings.floorDbFS == -120.0f
        && result.settings.window == Service::Window::periodicHann
        && result.settings.scaling == Service::Scaling::oneSidedPeakDbFS
        && result.settings.channelPolicy == Service::ChannelPolicy::selectedChannel
        && result.columns.size() == 8, "Spectral measurement metadata differs from its contract");
    for (std::size_t k = 0; k < 513; ++k)
        near (result.frequencyHz[k], static_cast<double> (k) * 46.875, 0.0,
              "Frequency-bin source-rate coordinates differ");
    for (std::size_t c = 0; c < 8; ++c)
    {
        require (result.columns[c].startFrame == 128 + c * 256
            && result.columns[c].endFrame == std::min<std::uint64_t> (2176, 128 + c * 256 + 1024),
            "Spectral columns lost source-frame or padded-tail bounds");
        for (const auto value : result.columns[c].magnitudeDbFS)
            require (std::isfinite (value) && value >= -120.0f, "Spectrum has nonfinite or below-floor data");
        if (c <= 4)
        {
            require (dominant (result.columns[c]) == 64, "Bin-centred sine has wrong dominant band");
            near (result.columns[c].magnitudeDbFS[64], -6.020599913, 0.002,
                  "Half-amplitude sine normalization differs from -6.0206 dBFS");
            near (result.columns[c].magnitudeDbFS[63], -12.041199826, 0.004,
                  "Periodic Hann adjacent-band amplitude differs");
        }
    }
    const auto readCount = reads.load();
    const auto cache = analyze (service, engine, "tone", 0, 128, 2176);
    require (cache.result == ready.result && reads == readCount, "Numeric cache did not reuse analysis");
    fixture.write (0.25);
    auto replacement = fixture.prepare();
    require (replacement != nullptr, "Replacement spectral source did not prepare");
    const auto changed = analyze (service, replacement, "tone", 0, 128, 2176);
    require (changed.result->contentRevision != result.contentRevision,
             "Same-path source content revision did not change");
    near (changed.result->columns[0].magnitudeDbFS[64], -12.041199826, 0.002,
          "Same-path replacement reused old spectral magnitudes");
    const auto right = analyze (service, engine, "tone", 1, 128, 2176);
    require (dominant (right.result->columns[0]) == 96 && reads > readCount,
             "Spectral cache or PCM copy lost selected-channel identity");
    near (right.result->columns[0].magnitudeDbFS[96], -12.041199826, 0.002,
          "Right-channel quarter-amplitude scaling differs");
    const auto silence = analyze (service, engine, "silent", 0, 2176, 3200);
    for (const auto& column : silence.result->columns)
        for (auto value : column.magnitudeDbFS)
            require (value == -120.0f, "Silence did not produce the exact finite floor");
    const auto dc = analyze (service, engine, "dc", 0, 3200, 4224);
    near (dc.result->columns[0].magnitudeDbFS[0], -12.041199826, 0.0001,
          "DC endpoint was doubled as an interior one-sided bin");
    const auto nyquist = analyze (service, engine, "nyquist", 0, 4224, 5248);
    near (nyquist.result->columns[0].magnitudeDbFS[512], -12.041199826, 0.0001,
          "Nyquist endpoint was doubled as an interior bin");
    near (dc.result->columns[3].magnitudeDbFS[0], -32.921937327, 0.0001,
          "Partial tail was not zero padded with the declared full-window scaling");

    Service repeat;
    repeat.setGeneration (1);
    require (analyze (repeat, engine, "tone", 0, 128, 2176).result->columns == result.columns,
             "Repeated numeric analysis is nondeterministic");
    require (service.status (ready.jobId, 2)->state == Service::State::stale
        && ! service.status (ready.jobId, 2)->result, "Visible generation exposed an old spectral result");
    require (! service.cancel (changed.jobId) && ! service.cancel (999999)
        && ! service.status (999999), "Terminal or unknown spectral jobs behaved as running");

    for (auto [channel, start, end] : std::vector<std::tuple<std::uint16_t, std::uint64_t, std::uint64_t>>
         { { 2, 128, 2176 }, { 0, 128, 128 }, { 0, 128, 5249 } })
        require (! service.request (1, sourceFrom (engine), "tone", channel, start, end),
                 "Invalid spectral channel or bounds were admitted");
    require (! service.request (1, {}, "tone", 0, 128, 2176)
        && ! service.request (1, sourceFrom (engine), "", 0, 128, 2176)
        && ! service.request (0, sourceFrom (engine), "tone", 0, 128, 2176),
        "Invalid spectral source, region or generation was admitted");
}

struct Release
{
    std::promise<void>& promise;
    bool released = false;
    void open() { if (! released) { promise.set_value(); released = true; } }
    ~Release() { open(); }
};

void lifetimesAndAdmission()
{
    Fixture fixture;
    auto engine = fixture.prepare();
    require (engine != nullptr, "Lifetime fixture did not prepare");
    std::promise<void> entered, resume;
    auto gate = resume.get_future().share();
    std::atomic<int> reads { 0 };
    std::atomic<bool> retainedSample { false };
    Service service ([&] (auto source, auto channel, auto start, auto output, auto count)
    {
        const auto first = ++reads == 1;
        if (first) { entered.set_value(); gate.wait(); }
        auto copied = dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
        if (first) retainedSample = copied && output[0] == 0.25f;
        return copied;
    });
    Release release { resume };
    service.setGeneration (1);
    const auto active = service.request (1, sourceFrom (engine), "active", 0, 3200, 4224, 17);
    require (active && entered.get_future().wait_for (std::chrono::seconds (2)) == std::future_status::ready,
             "Lifetime worker did not enter PCM copy");
    const auto owner = service.request (1, sourceFrom (engine), "owner-queued", 0, 3200, 4224, 17);
    const auto other = service.request (1, sourceFrom (engine), "other", 0, 3200, 4224, 18);
    const auto duplicate = service.request (1, sourceFrom (engine), "other", 0, 3200, 4224, 18);
    const auto cancel = service.request (1, sourceFrom (engine), "cancel", 0, 3200, 4224);
    require (owner && other && duplicate && cancel, "Queue rejected a within-budget spectral job");
    require (! service.request (1, sourceFrom (engine), "overflow", 0, 3200, 4224),
             "Spectral queue admitted work past its bound");
    service.cancelSession (0);
    require (service.status (*active)->state == Service::State::running,
             "Anonymous session cancellation affected owned jobs");
    service.cancelSession (17);
    require (service.status (*active)->state == Service::State::cancelled
        && service.status (*owner)->state == Service::State::cancelled
        && service.status (*other)->state == Service::State::running
        && service.status (*active)->sessionId == 17,
        "Editor cancellation blocked or affected another editor");
    require (service.cancel (*cancel) && ! service.cancel (*cancel)
        && service.status (*cancel, 2)->state == Service::State::cancelled,
        "Individual queued cancellation was not immediate and terminal");
    engine.reset();
    std::filesystem::remove (fixture.directory / "source.wav");
    release.open();
    const auto otherReady = terminal (service, *other);
    const auto duplicateReady = terminal (service, *duplicate);
    require (otherReady.state == Service::State::ready && duplicateReady.result == otherReady.result
        && reads == 8 && retainedSample
        && service.status (*active)->state == Service::State::cancelled,
        "Retained jobs lost PCM or duplicate/cache/cancelled publication semantics");
    near (otherReady.result->columns[0].magnitudeDbFS[0], -12.041199826, 0.0001,
          "Retained result changed after engine/file destruction");

    // History eviction must retire completed records, never a running record.
    fixture.write();
    engine = fixture.prepare();
    for (std::size_t i = 0; i < Service::maxHistory; ++i)
        analyze (service, engine, "other", 0, 3200, 4224);
    require (! service.status (*active), "Bounded history retained every completed record");
}

void generationAndFailures()
{
    Fixture fixture;
    auto engine = fixture.prepare();
    std::promise<void> entered, resume;
    auto gate = resume.get_future().share();
    std::atomic<int> reads { 0 };
    Service service ([&] (auto source, auto channel, auto start, auto output, auto count)
    {
        if (++reads == 1) { entered.set_value(); gate.wait(); }
        return dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
    });
    Release release { resume };
    service.setGeneration (7);
    const auto active = service.request (7, sourceFrom (engine), "old", 0, 3200, 4224);
    require (active && entered.get_future().wait_for (std::chrono::seconds (2)) == std::future_status::ready,
             "Generation worker did not enter analysis");
    const auto queued = service.request (7, sourceFrom (engine), "queued", 0, 3200, 4224);
    const auto cancelled = service.request (7, sourceFrom (engine), "cancelled", 0, 3200, 4224);
    require (queued && cancelled && service.cancel (*cancelled), "Generation fixture admission failed");
    service.setGeneration (8);
    require (service.status (*active)->state == Service::State::stale
        && ! service.status (*active)->result
        && service.status (*queued)->state == Service::State::stale
        && service.status (*cancelled)->state == Service::State::cancelled,
        "Reload did not retire queued/active jobs or preserved cancellation incorrectly");
    const auto current = service.request (8, sourceFrom (engine), "current", 0, 3200, 4224);
    require (current.has_value(), "New generation was rejected");
    engine.reset();
    release.open();
    const auto ready = terminal (service, *current);
    require (ready.state == Service::State::ready && ready.generation == 8
        && service.status (*active)->state == Service::State::stale
        && ! service.status (*active)->result && reads == 8,
        "Old worker published into the replacement generation");

    engine = fixture.prepare();
    std::atomic<int> mode { 1 };
    Service failures ([&] (auto source, auto channel, auto start, auto output, auto count)
    {
        if (mode == 1) return false;
        if (mode == 2) throw std::runtime_error ("read failed");
        auto copied = dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
        if (mode == 3) output[0] = std::numeric_limits<float>::quiet_NaN();
        return copied;
    });
    failures.setGeneration (1);
    for (int failure : { 1, 2, 3 })
    {
        mode = failure;
        const auto job = failures.request (1, sourceFrom (engine), "retry", 0, 3200, 4224);
        require (job.has_value(), "Failure fixture was rejected before analysis");
        const auto failed = terminal (failures, *job);
        require (failed.state == Service::State::failed && ! failed.result && ! failed.error.empty(),
                 "Read failure, exception or nonfinite PCM escaped terminal failure status");
    }
    mode = 0;
    near (analyze (failures, engine, "retry", 0, 3200, 4224).result->columns[0].magnitudeDbFS[0],
          -12.041199826, 0.0001, "Failed analysis could not be retried");
}

void longRegion()
{
    Fixture fixture;
    fixture.write (0.5, 262145);
    auto engine = fixture.prepare();
    Service service;
    service.setGeneration (1);
    const auto ready = analyze (service, engine, "long", 0, 0, 262145);
    require (ready.result->settings.hopFrames == 512 && ready.result->columns.size() == 513,
             "Long-region analysis did not declare its bounded sparse hop");
    for (std::size_t c = 0; c < 513; ++c)
        require (ready.result->columns[c].startFrame == c * 512
            && ready.result->columns[c].endFrame == std::min<std::uint64_t> (262145, c * 512 + 1024),
            "Sparse spectral columns did not retain contiguous source-window bounds");
    near (ready.result->columns[10].magnitudeDbFS[512], -12.041199826, 0.0001,
          "Long-source hop incorrectly decimated PCM instead of selecting contiguous windows");
}

void runningHistory()
{
    Fixture fixture;
    auto engine = fixture.prepare();
    std::promise<void> entered, resume;
    auto gate = resume.get_future().share();
    std::atomic<int> reads { 0 };
    Service service ([&] (auto source, auto channel, auto start, auto output, auto count)
    {
        if (++reads == 5) { entered.set_value(); gate.wait(); }
        return dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
    });
    Release release { resume };
    service.setGeneration (1);
    analyze (service, engine, "seed", 0, 3200, 4224);
    const auto active = service.request (1, sourceFrom (engine), "active", 0, 3200, 4224);
    require (active && entered.get_future().wait_for (std::chrono::seconds (2)) == std::future_status::ready,
             "History worker did not enter analysis");
    for (std::size_t i = 0; i < Service::maxHistory + 1; ++i)
        analyze (service, engine, "seed", 0, 3200, 4224);
    require (service.status (*active) && service.status (*active)->state == Service::State::running,
             "History pressure retired a running spectral job");
    release.open();
    require (terminal (service, *active).state == Service::State::ready,
             "History pressure lost the active job result");
}
}

int main()
{
    try { measurement(); lifetimesAndAdmission(); generationAndFailures(); longRegion(); runningHistory(); }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
    return 0;
}
