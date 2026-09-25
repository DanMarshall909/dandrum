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

struct MatchHandleDeleter
{
    void operator() (DandrumSoundMatch* matched) const noexcept
    {
        dandrum_sound_match_destroy (matched);
    }
};

struct ProposalHandleDeleter
{
    void operator() (DandrumGraphProposal* proposal) const noexcept
    {
        dandrum_graph_proposal_destroy (proposal);
    }
};

using RenderHandle = std::unique_ptr<DandrumSoundFixtureRender, RenderHandleDeleter>;
using MatchHandle = std::unique_ptr<DandrumSoundMatch, MatchHandleDeleter>;
using ProposalHandle = std::unique_ptr<DandrumGraphProposal, ProposalHandleDeleter>;
}

bool SoundLabController::isBusy (State state) noexcept
{
    return state == State::rendering || state == State::matching || state == State::proposing;
}

bool SoundLabController::tryBeginWork (State nextState)
{
    auto expected = currentState.load (std::memory_order_acquire);
    do
    {
        if (isBusy (expected))
            return false;
    }
    while (! currentState.compare_exchange_weak (expected,
                                                  nextState,
                                                  std::memory_order_acq_rel));
    return true;
}

void SoundLabController::clearResults()
{
    const std::scoped_lock lock (resultMutex);
    completedData.reset();
    completedMatchData.reset();
    completedProposalData.reset();
    completedMatchHandle.reset();
    lastError.clear();
    matchCompletedEvaluations.store (0, std::memory_order_release);
    matchMaxEvaluations.store (0, std::memory_order_release);
    matchBestScore.store (0.0, std::memory_order_release);
}

void SoundLabController::fail (std::string error)
{
    {
        const std::scoped_lock lock (resultMutex);
        lastError = std::move (error);
    }
    currentState.store (State::error, std::memory_order_release);
    currentGeneration.fetch_add (1, std::memory_order_release);
}

bool SoundLabController::startRender (const std::filesystem::path& fixturePath)
{
    if (! tryBeginWork (State::rendering))
        return false;

    clearResults();
    currentGeneration.fetch_add (1, std::memory_order_release);
    worker = std::jthread (
        [this, fixturePath] (std::stop_token stopToken) {
            renderOnWorker (fixturePath, stopToken);
        });
    return true;
}

bool SoundLabController::startMatch (const std::filesystem::path& fixturePath,
                                     const std::filesystem::path& referencePath)
{
    if (! tryBeginWork (State::matching))
        return false;

    clearResults();
    currentGeneration.fetch_add (1, std::memory_order_release);
    worker = std::jthread (
        [this, fixturePath, referencePath] (std::stop_token stopToken) {
            matchOnWorker (fixturePath, referencePath, stopToken);
        });
    return true;
}

bool SoundLabController::startProposal()
{
    std::shared_ptr<DandrumSoundMatch> matched;
    {
        const std::scoped_lock lock (resultMutex);
        matched = completedMatchHandle;
    }
    if (matched == nullptr || ! tryBeginWork (State::proposing))
        return false;

    {
        const std::scoped_lock lock (resultMutex);
        completedProposalData.reset();
        lastError.clear();
    }
    currentGeneration.fetch_add (1, std::memory_order_release);
    worker = std::jthread (
        [this, matched = std::move (matched)] (std::stop_token stopToken) {
            proposalOnWorker (matched, stopToken);
        });
    return true;
}

bool SoundLabController::discardResults()
{
    if (! tryBeginWork (State::idle))
        return false;

    clearResults();
    currentGeneration.fetch_add (1, std::memory_order_release);
    return true;
}

void SoundLabController::cancelCurrentWork()
{
    const auto state = currentState.load (std::memory_order_acquire);
    if (state == State::matching || state == State::proposing)
        worker.request_stop();
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
        completedMatchData,
        completedProposalData,
        matchCompletedEvaluations.load (std::memory_order_acquire),
        matchMaxEvaluations.load (std::memory_order_acquire),
        matchBestScore.load (std::memory_order_acquire),
        lastError,
    };
}

bool SoundLabController::captureMatchProgress (void* context,
                                               std::size_t completedEvaluations,
                                               std::size_t maxEvaluations,
                                               double bestTotal)
{
    auto& callback = *static_cast<MatchCallbackContext*> (context);
    callback.controller.matchCompletedEvaluations.store (completedEvaluations,
                                                          std::memory_order_release);
    callback.controller.matchMaxEvaluations.store (maxEvaluations, std::memory_order_release);
    callback.controller.matchBestScore.store (bestTotal, std::memory_order_release);
    callback.controller.currentGeneration.fetch_add (1, std::memory_order_release);
    return ! callback.stopToken.stop_requested();
}

bool SoundLabController::continueUnlessStopped (void* context)
{
    return ! static_cast<MatchCallbackContext*> (context)->stopToken.stop_requested();
}

void SoundLabController::renderOnWorker (std::filesystem::path fixturePath,
                                        std::stop_token)
{
    const auto path = fixturePath.string();
    RenderHandle render (dandrum_sound_fixture_render_create (path.c_str()));
    if (render == nullptr)
    {
        fail ("Rust could not create the Sound Lab render handle");
        return;
    }

    if (! dandrum_sound_fixture_render_is_ok (render.get()))
    {
        std::array<char, 2048> error {};
        dandrum_sound_fixture_render_error_message (render.get(), error.data(), error.size());
        fail (error.data());
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
            fail ("Rust returned an incomplete Sound Lab metric frame");
            return;
        }
        data->metrics.push_back (metric);
    }

    data->wavBytes.resize (dandrum_sound_fixture_render_wav_size (render.get()));
    if (! dandrum_sound_fixture_render_copy_wav (
            render.get(), data->wavBytes.data(), data->wavBytes.size()))
    {
        fail ("Rust could not copy the Sound Lab audition WAV");
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

void SoundLabController::matchOnWorker (std::filesystem::path fixturePath,
                                       std::filesystem::path referencePath,
                                       std::stop_token stopToken)
{
    MatchCallbackContext callback { *this, stopToken };
    const auto fixture = fixturePath.string();
    const auto reference = referencePath.string();
    MatchHandle matched (dandrum_sound_match_create (fixture.c_str(),
                                                     reference.c_str(),
                                                     captureMatchProgress,
                                                     &callback));
    if (matched == nullptr)
    {
        fail ("Rust could not create the Sound Lab match handle");
        return;
    }
    if (! dandrum_sound_match_is_ok (matched.get()))
    {
        std::array<char, 4096> error {};
        dandrum_sound_match_error_message (matched.get(), error.data(), error.size());
        fail (error.data());
        return;
    }

    auto data = std::make_shared<MatchData>();
    data->sampleRateHz = dandrum_sound_match_sample_rate_hz (matched.get());
    data->durationFrames = dandrum_sound_match_duration_frames (matched.get());
    const auto manifestSize = dandrum_sound_match_manifest_json_size (matched.get());
    std::vector<char> manifest (manifestSize);
    if (manifest.empty()
        || ! dandrum_sound_match_copy_manifest_json (
            matched.get(), manifest.data(), manifest.size()))
    {
        fail ("Rust could not copy the Sound Lab match manifest");
        return;
    }
    data->manifestJson = manifest.data();

    const auto patchYamlSize = dandrum_sound_match_patch_yaml_size (matched.get());
    std::vector<char> patchYaml (patchYamlSize);
    if (patchYaml.empty()
        || ! dandrum_sound_match_copy_patch_yaml (
            matched.get(), patchYaml.data(), patchYaml.size()))
    {
        fail ("Rust could not copy the Sound Lab matched patch snapshot");
        return;
    }
    data->patchYaml = patchYaml.data();

    const auto parameterCount = dandrum_sound_match_parameter_count (matched.get());
    data->parameters.reserve (parameterCount);
    for (std::size_t index = 0; index < parameterCount; ++index)
    {
        MatchParameter parameter;
        std::array<char, 256> id {};
        if (! dandrum_sound_match_parameter (matched.get(),
                                             index,
                                             id.data(),
                                             id.size(),
                                             &parameter.min,
                                             &parameter.max,
                                             &parameter.initial,
                                             &parameter.best,
                                             &parameter.normalized))
        {
            fail ("Rust returned an incomplete Sound Lab match parameter");
            return;
        }
        parameter.id = id.data();
        data->parameters.push_back (std::move (parameter));
    }

    const auto metricCount = dandrum_sound_match_metric_count (matched.get());
    data->metrics.reserve (metricCount);
    for (std::size_t index = 0; index < metricCount; ++index)
    {
        ComparisonMetric metric;
        double referenceTime = 0.0;
        if (! dandrum_sound_match_metric (matched.get(),
                                          index,
                                          false,
                                          &metric.timeSeconds,
                                          &metric.candidateRms,
                                          &metric.candidatePeak,
                                          &metric.candidateSpectralCentroidHz,
                                          &metric.candidateHasSpectralCentroid)
            || ! dandrum_sound_match_metric (matched.get(),
                                              index,
                                              true,
                                              &referenceTime,
                                              &metric.referenceRms,
                                              &metric.referencePeak,
                                              &metric.referenceSpectralCentroidHz,
                                              &metric.referenceHasSpectralCentroid))
        {
            fail ("Rust returned an incomplete Sound Lab comparison metric");
            return;
        }
        data->metrics.push_back (metric);
    }

    data->candidateWavBytes.resize (dandrum_sound_match_wav_size (matched.get(), false));
    data->referenceWavBytes.resize (dandrum_sound_match_wav_size (matched.get(), true));
    if (! dandrum_sound_match_copy_wav (matched.get(),
                                        false,
                                        data->candidateWavBytes.data(),
                                        data->candidateWavBytes.size())
        || ! dandrum_sound_match_copy_wav (matched.get(),
                                           true,
                                           data->referenceWavBytes.data(),
                                           data->referenceWavBytes.size()))
    {
        fail ("Rust could not copy the Sound Lab comparison WAVs");
        return;
    }

    const auto wasCancelled = dandrum_sound_match_was_cancelled (matched.get());
    auto sharedHandle = std::shared_ptr<DandrumSoundMatch> (matched.release(), MatchHandleDeleter {});
    {
        const std::scoped_lock lock (resultMutex);
        completedMatchData = std::move (data);
        completedMatchHandle = std::move (sharedHandle);
        lastError.clear();
    }
    currentState.store (wasCancelled ? State::cancelled : State::matched,
                        std::memory_order_release);
    currentGeneration.fetch_add (1, std::memory_order_release);
}

void SoundLabController::proposalOnWorker (std::shared_ptr<DandrumSoundMatch> matched,
                                          std::stop_token stopToken)
{
    MatchCallbackContext callback { *this, stopToken };
    ProposalHandle proposal (
        dandrum_graph_proposal_create (matched.get(), continueUnlessStopped, &callback));
    if (proposal == nullptr)
    {
        fail ("Rust could not create the Sound Lab proposal handle");
        return;
    }
    if (! dandrum_graph_proposal_is_ok (proposal.get()))
    {
        std::array<char, 4096> error {};
        dandrum_graph_proposal_error_message (proposal.get(), error.data(), error.size());
        fail (error.data());
        return;
    }

    auto data = std::make_shared<ProposalData>();
    std::array<char, 256> provider {};
    std::array<char, 256> patchName {};
    if (! dandrum_graph_proposal_provider_id (
            proposal.get(), provider.data(), provider.size())
        || ! dandrum_graph_proposal_patch_name (
            proposal.get(), patchName.data(), patchName.size()))
    {
        fail ("Rust returned incomplete graph proposal metadata");
        return;
    }
    data->providerId = provider.data();
    data->patchName = patchName.data();

    const auto explanationSize = dandrum_graph_proposal_explanation_size (proposal.get());
    const auto patchYamlSize = dandrum_graph_proposal_patch_yaml_size (proposal.get());
    std::vector<char> explanation (explanationSize);
    std::vector<char> patchYaml (patchYamlSize);
    if (explanation.empty() || patchYaml.empty()
        || ! dandrum_graph_proposal_copy_explanation (
            proposal.get(), explanation.data(), explanation.size())
        || ! dandrum_graph_proposal_copy_patch_yaml (
            proposal.get(), patchYaml.data(), patchYaml.size()))
    {
        fail ("Rust returned incomplete graph proposal content");
        return;
    }
    data->explanation = explanation.data();
    data->patchYaml = patchYaml.data();

    const auto parameterCount = dandrum_graph_proposal_parameter_count (proposal.get());
    data->suggestedSearchParameters.reserve (parameterCount);
    for (std::size_t index = 0; index < parameterCount; ++index)
    {
        std::array<char, 256> parameter {};
        if (! dandrum_graph_proposal_parameter (
                proposal.get(), index, parameter.data(), parameter.size()))
        {
            fail ("Rust returned an incomplete graph proposal search parameter");
            return;
        }
        data->suggestedSearchParameters.emplace_back (parameter.data());
    }

    {
        const std::scoped_lock lock (resultMutex);
        completedProposalData = std::move (data);
        lastError.clear();
    }
    currentState.store (State::proposalReady, std::memory_order_release);
    currentGeneration.fetch_add (1, std::memory_order_release);
}
