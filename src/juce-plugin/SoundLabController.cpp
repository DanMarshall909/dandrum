#include "SoundLabController.h"

#include "RustEngineBindings.h"

#include <array>
#include <memory>
#include <utility>

namespace
{
struct RenderHandleDeleter
{
    void operator() (DandrumSoundFixtureRender* render) const noexcept
    {
        dandrum_sound_fixture_render_destroy (render);
    }
};

using RenderHandle = std::unique_ptr<DandrumSoundFixtureRender, RenderHandleDeleter>;
}

bool SoundLabController::startRender (const std::filesystem::path& fixturePath)
{
    auto expected = currentState.load (std::memory_order_acquire);
    do
    {
        if (expected == State::rendering)
            return false;
    }
    while (! currentState.compare_exchange_weak (expected,
                                                  State::rendering,
                                                  std::memory_order_acq_rel));

    {
        const std::scoped_lock lock (resultMutex);
        completedData.reset();
        lastError.clear();
    }
    currentGeneration.fetch_add (1, std::memory_order_release);
    worker = std::jthread ([this, fixturePath] { renderOnWorker (fixturePath); });
    return true;
}

SoundLabController::State SoundLabController::state() const noexcept
{
    return currentState.load (std::memory_order_acquire);
}

std::uint64_t SoundLabController::generation() const noexcept
{
    return currentGeneration.load (std::memory_order_acquire);
}

SoundLabController::Snapshot SoundLabController::snapshot() const
{
    const std::scoped_lock lock (resultMutex);
    return {
        currentState.load (std::memory_order_acquire),
        currentGeneration.load (std::memory_order_acquire),
        completedData,
        lastError,
    };
}

void SoundLabController::renderOnWorker (std::filesystem::path fixturePath)
{
    const auto path = fixturePath.string();
    RenderHandle render (dandrum_sound_fixture_render_create (path.c_str()));
    if (render == nullptr)
    {
        const std::scoped_lock lock (resultMutex);
        lastError = "Rust could not create the Sound Lab render handle";
        currentState.store (State::error, std::memory_order_release);
        currentGeneration.fetch_add (1, std::memory_order_release);
        return;
    }

    if (! dandrum_sound_fixture_render_is_ok (render.get()))
    {
        std::array<char, 2048> error {};
        dandrum_sound_fixture_render_error_message (render.get(), error.data(), error.size());
        const std::scoped_lock lock (resultMutex);
        lastError = error.data();
        currentState.store (State::error, std::memory_order_release);
        currentGeneration.fetch_add (1, std::memory_order_release);
        return;
    }

    auto data = std::make_shared<RenderData>();
    data->sampleRateHz = dandrum_sound_fixture_render_sample_rate_hz (render.get());
    data->durationFrames = dandrum_sound_fixture_render_duration_frames (render.get());

    const auto metricCount = dandrum_sound_fixture_render_metric_count (render.get());
    data->metrics.reserve (metricCount);
    for (std::size_t index = 0; index < metricCount; ++index)
    {
        Metric metric;
        if (! dandrum_sound_fixture_render_metric (render.get(),
                                                    index,
                                                    &metric.timeSeconds,
                                                    &metric.rms,
                                                    &metric.peak,
                                                    &metric.spectralCentroidHz,
                                                    &metric.hasSpectralCentroid))
        {
            const std::scoped_lock lock (resultMutex);
            lastError = "Rust returned an incomplete Sound Lab metric frame";
            currentState.store (State::error, std::memory_order_release);
            currentGeneration.fetch_add (1, std::memory_order_release);
            return;
        }
        data->metrics.push_back (metric);
    }

    data->wavBytes.resize (dandrum_sound_fixture_render_wav_size (render.get()));
    if (! dandrum_sound_fixture_render_copy_wav (
            render.get(), data->wavBytes.data(), data->wavBytes.size()))
    {
        const std::scoped_lock lock (resultMutex);
        lastError = "Rust could not copy the Sound Lab audition WAV";
        currentState.store (State::error, std::memory_order_release);
        currentGeneration.fetch_add (1, std::memory_order_release);
        return;
    }

    {
        const std::scoped_lock lock (resultMutex);
        completedData = std::move (data);
        lastError.clear();
    }
    currentState.store (State::ready, std::memory_order_release);
    currentGeneration.fetch_add (1, std::memory_order_release);
}
