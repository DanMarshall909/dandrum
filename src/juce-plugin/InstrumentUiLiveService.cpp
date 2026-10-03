#include "InstrumentUiLiveService.h"

#include <chrono>
#include <utility>

InstrumentUiLiveService::InstrumentUiLiveService (BeforeBatch observe, BeforeBatch publish)
    : beforeBatch (std::move (observe)), beforePublication (std::move (publish)),
      worker ([this] (std::stop_token stop) { run (stop); }) {}
InstrumentUiLiveService::~InstrumentUiLiveService()
{
    worker.request_stop();
    wake.notify_all();
}
InstrumentUiLiveService::Session* InstrumentUiLiveService::find (std::uint64_t id)
{
    for (auto& session : sessions)
        if (session.active && session.id == id) return &session;
    return nullptr;
}
void InstrumentUiLiveService::updateDemand()
{
    std::uint8_t channels = 0;
    for (const auto& session : sessions)
        if (session.active && session.visible) channels |= session.channels;
    captureQueue.setChannels (channels);
    ++demandRevision;
    wake.notify_all();
}
void InstrumentUiLiveService::setGeneration (std::uint32_t generation)
{
    const std::scoped_lock lock (mutex);
    if (generation == currentGeneration) return;
    currentGeneration = generation;
    sessions = {};
    updateDemand();
}
bool InstrumentUiLiveService::subscribe (std::uint64_t id, std::uint32_t generation, std::uint8_t channels)
{
    const std::scoped_lock lock (mutex);
    if (generation == 0 || generation != currentGeneration || channels == 0
        || channels >= (1U << InstrumentUiLiveCapture::channelCount) || find (id)) return false;
    for (auto& session : sessions)
        if (! session.active)
        {
            session = {}; session.id = id; session.channels = channels;
            session.active = true; session.visible = true;
            updateDemand(); return true;
        }
    return false;
}
bool InstrumentUiLiveService::unsubscribe (std::uint64_t id)
{
    const std::scoped_lock lock (mutex);
    const auto session = find (id);
    if (! session) return false;
    *session = {}; updateDemand(); return true;
}
bool InstrumentUiLiveService::setVisible (std::uint64_t id, bool visible)
{
    const std::scoped_lock lock (mutex);
    const auto session = find (id);
    if (! session) return false;
    if (session->visible != visible)
    {
        session->visible = visible;
        session->latest.reset(); session->awaiting = false;
        updateDemand();
    }
    return true;
}
std::uint8_t InstrumentUiLiveService::selectedChannels() const noexcept
{ return static_cast<std::uint8_t> (captureQueue.selectionId()); }
void InstrumentUiLiveService::beginStream() noexcept { captureQueue.beginStream(); }
void InstrumentUiLiveService::capture (const float* left, const float* right, std::size_t count,
                                      std::uint32_t generation, std::uint32_t rate) noexcept
{ captureQueue.capture (left, right, count, generation, rate); }
std::optional<InstrumentUiLiveService::Packet> InstrumentUiLiveService::take (std::uint64_t id)
{
    const std::scoped_lock lock (mutex);
    const auto session = find (id);
    if (! session || ! session->visible || session->awaiting || ! session->latest) return {};
    session->awaiting = true; session->outstanding = session->latest->sequence;
    auto result = session->latest; session->latest.reset(); return result;
}
bool InstrumentUiLiveService::acknowledge (std::uint64_t id, std::uint32_t generation, std::uint64_t sequence)
{
    const std::scoped_lock lock (mutex);
    const auto session = find (id);
    if (! session || ! session->visible || generation != currentGeneration
        || ! session->awaiting || session->outstanding != sequence) return false;
    session->awaiting = false; return true;
}
std::size_t InstrumentUiLiveService::pendingPayloads() const
{
    const std::scoped_lock lock (mutex);
    std::size_t count = 0;
    for (const auto& session : sessions)
        if (session.active) count += session.awaiting + session.latest.has_value();
    return count;
}
InstrumentUiLiveService::Statistics InstrumentUiLiveService::statistics() const
{
    const std::scoped_lock lock (mutex);
    auto result = counters;
    result.droppedCaptureChunks = captureQueue.lostFrames();
    return result;
}
void InstrumentUiLiveService::run (std::stop_token stop)
{
    static_assert (maxBacklogFrames >= InstrumentUiSpectrumAnalysis::fftSize);
    std::uint64_t seenRevision = 0, seenLoss = 0;
    std::array<InstrumentUiLiveCapture::Frame, InstrumentUiLiveCapture::capacity> batch;
    while (! stop.stop_requested())
    {
        std::unique_lock lock (mutex);
        const auto changed = [this, seenRevision] { return demandRevision != seenRevision; };
        if (selectedChannels() == 0) wake.wait (lock, stop, changed);
        else wake.wait_for (lock, stop, std::chrono::milliseconds (10), changed);
        if (stop.stop_requested()) return;
        const bool active = selectedChannels() != 0;
        lock.unlock();
        if (beforeBatch && active) beforeBatch (stop);
        if (stop.stop_requested()) return;
        lock.lock();
        // Read the admitted epoch after a potential stall. The accumulator's
        // own identity/sequence checks reset any earlier partial window.
        const auto generation = currentGeneration;
        const auto selection = captureQueue.selectionId();
        seenRevision = demandRevision;
        lock.unlock();
        InstrumentUiLiveAnalysis::Result result;
        std::optional<Packet> latest;
        std::uint64_t consumed = 0, discarded = 0, windows = 0;
        std::size_t count = 0, frames = 0, first = 0;
        while (consumed < InstrumentUiLiveCapture::capacity && captureQueue.pop (batch[count]))
        {
            ++consumed;
            const auto& frame = batch[count];
            if (frame.selectionId != selection || frame.generation != generation)
            { analysis.reset(); ++discarded; continue; }
            frames += frame.sampleCount; ++count;
        }
        const auto loss = captureQueue.lostFrames();
        if (loss != seenLoss)
        {
            // Admitted history predates overflow. Wait for current, explicitly
            // gapped capture instead of presenting its stale queued tail.
            seenLoss = loss; discarded += count; count = 0; frames = 0;
            analysis.reset();
        }
        while (frames > maxBacklogFrames)
        { frames -= batch[first++].sampleCount; ++discarded; }
        if (first != 0) { analysis.reset(); batch[first].gap = true; }
        bool gap = false;
        for (auto n = first; n < count; ++n)
            if (analysis.consume (batch[n], result))
            {
                ++windows; gap |= result.gap;
                latest = Packet { ++nextSequence, result };
            }
        if (latest) latest->analysis.gap = gap;
        if (latest && beforePublication) beforePublication (stop);
        if (stop.stop_requested()) return;
        lock.lock();
        counters.consumedChunks += consumed;
        counters.discardedChunks += discarded;
        counters.analyzedWindows += windows;
        if (demandRevision != seenRevision || ! latest) continue;
        for (auto& session : sessions)
            if (session.active && session.visible)
            {
                const bool pendingGap = session.latest && session.latest->analysis.gap;
                session.latest = latest;
                session.latest->analysis.gap |= pendingGap;
            }
    }
}
