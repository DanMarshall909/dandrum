#include "SoundLabController.h"

#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <thread>

namespace
{
void replaceFirst (std::string& text, const std::string& from, const std::string& to)
{
    const auto position = text.find (from);
    if (position == std::string::npos)
        throw std::runtime_error ("Sound Lab test fixture token was missing: " + from);
    text.replace (position, from.size(), to);
}

void replaceAll (std::string& text, const std::string& from, const std::string& to)
{
    for (auto position = text.find (from); position != std::string::npos;
         position = text.find (from, position + to.size()))
        text.replace (position, from.size(), to);
}

std::filesystem::path makeShortFixture (const std::filesystem::path& sourceFixture,
                                        const std::filesystem::path& directory)
{
    std::ifstream sourceStream (sourceFixture);
    std::ostringstream sourceText;
    sourceText << sourceStream.rdbuf();
    auto yaml = sourceText.str();
    const auto patch = std::filesystem::weakly_canonical (
        sourceFixture.parent_path().parent_path() / "patches" / "tb303-acid.yaml");
    replaceFirst (yaml,
                  "patch: ../patches/tb303-acid.yaml",
                  "patch: " + patch.string());
    replaceAll (yaml, "duration_frames: 528000", "duration_frames: 96000");
    replaceAll (yaml, "max_evaluations: 16", "max_evaluations: 2");
    replaceAll (yaml, "start_frame: 48000", "start_frame: 0");
    replaceFirst (yaml, "length_frames: 96000", "length_frames: 12000");
    replaceAll (yaml, "repetitions: 4", "repetitions: 1");
    const auto result = directory / "short-acid.yaml";
    std::ofstream (result) << yaml;
    return result;
}

template <typename Predicate>
bool waitUntil (Predicate predicate)
{
    for (int attempt = 0; attempt < 400; ++attempt)
    {
        if (predicate())
            return true;
        std::this_thread::sleep_for (std::chrono::milliseconds (25));
    }
    return predicate();
}
}

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

    const auto temporaryDirectory = std::filesystem::temp_directory_path()
        / ("dandrum-sound-lab-controller-"
           + std::to_string (
               std::chrono::steady_clock::now().time_since_epoch().count()));
    std::filesystem::create_directories (temporaryDirectory);
    const auto shortFixture = makeShortFixture (fixture, temporaryDirectory);
    SoundLabController referenceRenderer;
    if (! referenceRenderer.startRender (shortFixture)
        || ! waitUntil ([&] { return referenceRenderer.state() != SoundLabController::State::rendering; }))
    {
        std::cerr << "Sound Lab could not render the short matching reference\n";
        return 1;
    }
    const auto referenceRender = referenceRenderer.snapshot();
    if (referenceRender.state != SoundLabController::State::ready || referenceRender.data == nullptr)
    {
        std::cerr << "Sound Lab short reference failed: " << referenceRender.error << '\n';
        return 1;
    }
    const auto referencePath = temporaryDirectory / "reference.wav";
    std::ofstream referenceFile (referencePath, std::ios::binary);
    referenceFile.write (reinterpret_cast<const char*> (referenceRender.data->wavBytes.data()),
                         static_cast<std::streamsize> (referenceRender.data->wavBytes.size()));
    referenceFile.close();

    SoundLabController matcher;
    if (! matcher.startMatch (shortFixture, referencePath)
        || matcher.state() != SoundLabController::State::matching)
    {
        std::cerr << "Sound Lab did not enter matching state\n";
        return 1;
    }
    if (matcher.startRender (shortFixture) || matcher.startMatch (shortFixture, referencePath))
    {
        std::cerr << "Sound Lab accepted conflicting work during matching\n";
        return 1;
    }
    if (! waitUntil ([&] { return matcher.state() != SoundLabController::State::matching; }))
    {
        std::cerr << "Sound Lab match did not finish\n";
        return 1;
    }
    const auto match = matcher.snapshot();
    if (match.state != SoundLabController::State::matched || match.match == nullptr
        || match.completedEvaluations != 2 || match.maxEvaluations != 2
        || ! std::isfinite (match.bestScore) || match.match->parameters.size() != 4
        || match.match->metrics.empty() || match.match->candidateWavBytes.size() <= 44
        || match.match->referenceWavBytes.size() <= 44 || match.match->manifestJson.empty()
        || match.match->patchYaml.empty())
    {
        std::cerr << "Sound Lab completed match is missing progress or comparison artifacts: "
                  << match.error << '\n';
        return 1;
    }
    const auto& comparison = match.match->metrics[match.match->metrics.size() / 2];
    if (! std::isfinite (comparison.referencePeak)
        || ! std::isfinite (comparison.candidatePeak))
    {
        std::cerr << "Sound Lab completed match is missing finite peak trajectories\n";
        return 1;
    }
    if (! matcher.discardResults())
    {
        std::cerr << "Sound Lab could not discard a completed match for a new reference\n";
        return 1;
    }
    const auto discarded = matcher.snapshot();
    if (discarded.state != SoundLabController::State::idle || discarded.match != nullptr
        || discarded.proposal != nullptr || discarded.completedEvaluations != 0)
    {
        std::cerr << "Sound Lab retained mixed-reference results after discard\n";
        return 1;
    }

    SoundLabController cancelledMatcher;
    if (! cancelledMatcher.startMatch (shortFixture, referencePath))
    {
        std::cerr << "Sound Lab rejected cancellable match\n";
        return 1;
    }
    cancelledMatcher.cancelCurrentWork();
    if (! waitUntil ([&] {
            return cancelledMatcher.state() != SoundLabController::State::matching;
        }))
    {
        std::cerr << "Sound Lab cancelled match did not stop\n";
        return 1;
    }
    const auto cancelled = cancelledMatcher.snapshot();
    if (cancelled.state != SoundLabController::State::cancelled || cancelled.match == nullptr
        || cancelled.completedEvaluations == 0)
    {
        std::cerr << "Sound Lab cancellation did not preserve the best completed candidate\n";
        return 1;
    }

    SoundLabController retryableMatcher;
    if (! retryableMatcher.startMatch (shortFixture, "/definitely/missing/reference.wav")
        || ! waitUntil ([&] {
            return retryableMatcher.state() != SoundLabController::State::matching;
        }))
    {
        std::cerr << "Sound Lab did not run invalid matching request\n";
        return 1;
    }
    const auto failedMatch = retryableMatcher.snapshot();
    if (failedMatch.state != SoundLabController::State::error || failedMatch.error.empty()
        || ! retryableMatcher.startMatch (shortFixture, referencePath))
    {
        std::cerr << "Sound Lab matching error was not diagnosed and retryable\n";
        return 1;
    }
    if (! waitUntil ([&] {
            return retryableMatcher.state() != SoundLabController::State::matching;
        })
        || retryableMatcher.state() != SoundLabController::State::matched)
    {
        std::cerr << "Sound Lab matching retry did not complete\n";
        return 1;
    }

    std::filesystem::remove_all (temporaryDirectory);

    return 0;
}
