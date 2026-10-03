#pragma once
#include "InstrumentUiLiveAnalysis.h"
#include <atomic>
#include <condition_variable>
#include <functional>
#include <mutex>
#include <optional>
#include <thread>

// Processor-owned storage, one audio producer and one analysis worker. All
// session operations are off audio. Locks and wakeups never enter capture.
class InstrumentUiLiveService final
{
public:
    static constexpr std::size_t maxSessions = 4;
    static constexpr std::size_t maxBacklogFrames = 2048;
    static constexpr std::size_t maxWindowsPerBatch = maxBacklogFrames / InstrumentUiSpectrumAnalysis::hopFrames;
    struct Packet { std::uint64_t sequence = 0; InstrumentUiLiveAnalysis::Result analysis; };
    struct Statistics
    {
        std::uint64_t consumedChunks = 0, discardedChunks = 0;
        std::uint64_t analyzedWindows = 0, droppedCaptureChunks = 0;
    };
    using BeforeBatch = std::function<void (std::stop_token)>;
    // Optional off-audio observation/barrier for deterministic instrumentation.
    // It must honor the stop token and must not throw.
    explicit InstrumentUiLiveService (BeforeBatch = {}, BeforeBatch beforePublication = {});
    ~InstrumentUiLiveService();
    void setGeneration (std::uint32_t);
    bool subscribe (std::uint64_t, std::uint32_t, std::uint8_t);
    bool unsubscribe (std::uint64_t);
    bool setVisible (std::uint64_t, bool);
    std::uint8_t selectedChannels() const noexcept;
    // Audio producer only; beginStream is called at a safe preparation boundary.
    void beginStream() noexcept;
    void capture (const float*, const float*, std::size_t, std::uint32_t, std::uint32_t) noexcept;
    std::optional<Packet> take (std::uint64_t);
    bool acknowledge (std::uint64_t, std::uint32_t, std::uint64_t);
    std::size_t pendingPayloads() const;
    Statistics statistics() const;
private:
    struct Session
    {
        std::uint64_t id = 0, outstanding = 0;
        std::uint8_t channels = 0;
        bool active = false, visible = false, awaiting = false;
        std::optional<Packet> latest;
    };
    Session* find (std::uint64_t);
    void updateDemand(); // caller holds mutex; sole selection writer
    void run (std::stop_token);
    InstrumentUiLiveCapture captureQueue;
    InstrumentUiLiveAnalysis analysis;
    BeforeBatch beforeBatch, beforePublication;
    mutable std::mutex mutex;
    std::condition_variable_any wake;
    std::array<Session, maxSessions> sessions {};
    std::uint32_t currentGeneration = 0;
    std::uint64_t demandRevision = 0, nextSequence = 0;
    Statistics counters;
    // Joined first at off-audio destruction, before any queue/session state.
    std::jthread worker;
};
