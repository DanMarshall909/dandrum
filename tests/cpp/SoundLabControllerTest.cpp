#include "SoundLabController.h"

#include <chrono>
#include <cmath>
#include <iostream>
#include <thread>

int main()
{
    SoundLabController controller;
    const auto fixture = dandrum::soundDesignFixturePath();

    if (! controller.startRender (fixture))
    {
        std::cerr << "first Sound Lab render request was rejected\n";
        return 1;
    }

    if (controller.state() != SoundLabController::State::rendering)
    {
        std::cerr << "Sound Lab did not enter rendering state immediately\n";
        return 1;
    }

    if (controller.startRender (fixture))
    {
        std::cerr << "Sound Lab accepted a duplicate render request\n";
        return 1;
    }

    for (int attempt = 0;
         attempt < 200 && controller.state() == SoundLabController::State::rendering;
         ++attempt)
        std::this_thread::sleep_for (std::chrono::milliseconds (25));

    const auto completed = controller.snapshot();
    if (completed.state != SoundLabController::State::ready || completed.data == nullptr)
    {
        std::cerr << "Sound Lab render failed: " << completed.error << '\n';
        return 1;
    }

    if (completed.data->sampleRateHz != 48000
        || completed.data->durationFrames != 528000
        || completed.data->metrics.size() <= 4000
        || completed.data->wavBytes.size() <= 44)
    {
        std::cerr << "Sound Lab completed artifact is missing expected audio or metrics\n";
        return 1;
    }

    const auto& metric = completed.data->metrics[completed.data->metrics.size() / 2];
    if (! std::isfinite (metric.timeSeconds)
        || ! std::isfinite (metric.rms)
        || ! std::isfinite (metric.peak)
        || (metric.hasSpectralCentroid && ! std::isfinite (metric.spectralCentroidHz)))
    {
        std::cerr << "Sound Lab completed artifact contains invalid metrics\n";
        return 1;
    }

    SoundLabController failing;
    if (! failing.startRender ("/definitely/missing/dandrum-sound-fixture.yaml"))
    {
        std::cerr << "Sound Lab rejected a retryable invalid-path request before starting it\n";
        return 1;
    }
    for (int attempt = 0;
         attempt < 100 && failing.state() == SoundLabController::State::rendering;
         ++attempt)
        std::this_thread::sleep_for (std::chrono::milliseconds (10));
    const auto failed = failing.snapshot();
    if (failed.state != SoundLabController::State::error || failed.error.empty())
    {
        std::cerr << "Sound Lab did not preserve a render diagnostic\n";
        return 1;
    }
    if (! failing.startRender (fixture))
    {
        std::cerr << "Sound Lab error state was not retryable\n";
        return 1;
    }

    return 0;
}
