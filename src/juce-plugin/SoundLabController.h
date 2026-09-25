#pragma once

#include <atomic>
#include <cstdint>
#include <filesystem>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#include "DefaultPatch.h"

struct DandrumSoundMatch;

class SoundLabController final
{
public:
    enum class State
    {
        idle,
        rendering,
        ready,
        matching,
        matched,
        cancelled,
        proposing,
        proposalReady,
        error,
    };

    struct Metric
    {
        double timeSeconds = 0.0;
        double rms = 0.0;
        double peak = 0.0;
        double spectralCentroidHz = 0.0;
        bool hasSpectralCentroid = false;
    };

    struct RenderData
    {
        std::uint32_t sampleRateHz = 0;
        std::uint64_t durationFrames = 0;
        std::vector<Metric> metrics;
        std::vector<std::uint8_t> wavBytes;
    };

    struct MatchParameter
    {
        std::string id;
        double min = 0.0;
        double max = 0.0;
        double initial = 0.0;
        double best = 0.0;
        double normalized = 0.0;
    };

    struct ComparisonMetric
    {
        double timeSeconds = 0.0;
        double referenceRms = 0.0;
        double candidateRms = 0.0;
        double referencePeak = 0.0;
        double candidatePeak = 0.0;
        double referenceSpectralCentroidHz = 0.0;
        double candidateSpectralCentroidHz = 0.0;
        bool referenceHasSpectralCentroid = false;
        bool candidateHasSpectralCentroid = false;
    };

    struct MatchData
    {
        std::uint32_t sampleRateHz = 0;
        std::uint64_t durationFrames = 0;
        std::string manifestJson;
        std::string patchYaml;
        std::vector<MatchParameter> parameters;
        std::vector<ComparisonMetric> metrics;
        std::vector<std::uint8_t> candidateWavBytes;
        std::vector<std::uint8_t> referenceWavBytes;
    };

    struct ProposalData
    {
        std::string providerId;
        std::string patchName;
        std::string explanation;
        std::string patchYaml;
        std::vector<std::string> suggestedSearchParameters;
    };

    struct Snapshot
    {
        State state = State::idle;
        std::uint64_t generation = 0;
        std::shared_ptr<const RenderData> data;
        std::shared_ptr<const MatchData> match;
        std::shared_ptr<const ProposalData> proposal;
        std::size_t completedEvaluations = 0;
        std::size_t maxEvaluations = 0;
        double bestScore = 0.0;
        std::string error;
    };

    SoundLabController() = default;
    ~SoundLabController() = default;

    bool startRender (const std::filesystem::path& fixturePath);
    bool startMatch (const std::filesystem::path& fixturePath,
                     const std::filesystem::path& referencePath);
    bool startProposal();
    bool discardResults();
    void cancelCurrentWork();
    State state() const noexcept;
    std::uint64_t generation() const noexcept;
    Snapshot snapshot() const;

private:
    struct MatchCallbackContext
    {
        SoundLabController& controller;
        std::stop_token stopToken;
    };

    static bool captureMatchProgress (void* context,
                                      std::size_t completedEvaluations,
                                      std::size_t maxEvaluations,
                                      double bestTotal);
    static bool continueUnlessStopped (void* context);
    bool tryBeginWork (State state);
    static bool isBusy (State state) noexcept;
    void clearResults();
    void fail (std::string error);
    void renderOnWorker (std::filesystem::path fixturePath, std::stop_token stopToken);
    void matchOnWorker (std::filesystem::path fixturePath,
                        std::filesystem::path referencePath,
                        std::stop_token stopToken);
    void proposalOnWorker (std::shared_ptr<DandrumSoundMatch> matched,
                           std::stop_token stopToken);

    std::atomic<State> currentState { State::idle };
    std::atomic<std::uint64_t> currentGeneration { 0 };
    std::atomic<std::size_t> matchCompletedEvaluations { 0 };
    std::atomic<std::size_t> matchMaxEvaluations { 0 };
    std::atomic<double> matchBestScore { 0.0 };
    mutable std::mutex resultMutex;
    std::shared_ptr<const RenderData> completedData;
    std::shared_ptr<const MatchData> completedMatchData;
    std::shared_ptr<const ProposalData> completedProposalData;
    std::shared_ptr<DandrumSoundMatch> completedMatchHandle;
    std::string lastError;
    // Declared last so its destructor joins before the state it writes is torn down.
    std::jthread worker;
};
