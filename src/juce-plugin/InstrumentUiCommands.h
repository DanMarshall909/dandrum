#pragma once

#include <atomic>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <map>
#include <string>

// The same message-thread command service is used by native components and
// browser transport. No browser or JUCE value type enters this contract.
enum class InstrumentUiCommandStatus {
    accepted, staleGeneration, invalidValue, unknownControl, gestureActive, noGesture
};

struct InstrumentUiSetParameter
{
    std::uint32_t generation;
    std::string id;
    double normalisedValue;
    std::uint64_t sessionId = 0;
};

struct InstrumentUiGestureRequest
{
    std::uint32_t generation;
    std::string id;
    std::uint64_t sessionId;
};

struct InstrumentUiGestureAdmission
{
    InstrumentUiCommandStatus status;
    std::size_t hostSlot = 0;
};

struct InstrumentUiCommandReply
{
    InstrumentUiCommandStatus status;
    std::uint32_t generation;
    std::uint64_t sequence;
};

class InstrumentUiCommandHost
{
public:
    virtual ~InstrumentUiCommandHost() = default;
    virtual std::uint32_t uiCommandGeneration() const noexcept = 0;
    virtual InstrumentUiCommandStatus applyUiParameter (
        std::uint32_t generation, const std::string& id, float normalisedValue,
        bool withinGesture) = 0;
    virtual InstrumentUiGestureAdmission beginUiGesture (
        std::uint32_t generation, const std::string& id) = 0;
    virtual void endUiGesture (std::size_t hostSlot) = 0;
};

class InstrumentUiCommandService final
{
public:
    explicit InstrumentUiCommandService (InstrumentUiCommandHost& hostToUse) : host (hostToUse) {}

    std::uint64_t createSession() noexcept { return ++nextSessionId; }
    std::uint64_t lastAdmittedSequence() const noexcept
    {
        return sequence.load (std::memory_order_relaxed);
    }

    InstrumentUiCommandReply setParameter (const InstrumentUiSetParameter& request)
    {
        const auto generation = host.uiCommandGeneration();
        if (request.generation != generation)
            return { InstrumentUiCommandStatus::staleGeneration, generation, lastAdmittedSequence() };
        if (! std::isfinite (request.normalisedValue)
            || request.normalisedValue < 0.0 || request.normalisedValue > 1.0)
            return { InstrumentUiCommandStatus::invalidValue, generation, lastAdmittedSequence() };

        const auto active = gestures.find (request.sessionId);
        if (active != gestures.end()
            && (active->second.generation != generation || active->second.id != request.id))
            return { InstrumentUiCommandStatus::gestureActive, generation, lastAdmittedSequence() };
        const auto status = host.applyUiParameter (
            generation, request.id, static_cast<float> (request.normalisedValue),
            active != gestures.end());
        if (status == InstrumentUiCommandStatus::accepted)
            ++sequence;
        return { status, host.uiCommandGeneration(), lastAdmittedSequence() };
    }

    InstrumentUiCommandReply beginGesture (const InstrumentUiGestureRequest& request)
    {
        const auto generation = host.uiCommandGeneration();
        if (request.generation != generation)
            return { InstrumentUiCommandStatus::staleGeneration, generation, lastAdmittedSequence() };
        if (request.sessionId == 0 || gestures.contains (request.sessionId))
            return { InstrumentUiCommandStatus::gestureActive, generation, lastAdmittedSequence() };

        const auto admission = host.beginUiGesture (generation, request.id);
        if (admission.status == InstrumentUiCommandStatus::accepted)
        {
            gestures.emplace (request.sessionId,
                              ActiveGesture { generation, request.id, admission.hostSlot });
            ++sequence;
        }
        return { admission.status, host.uiCommandGeneration(), lastAdmittedSequence() };
    }

    InstrumentUiCommandReply endGesture (const InstrumentUiGestureRequest& request)
    {
        const auto active = gestures.find (request.sessionId);
        if (active == gestures.end())
            return { InstrumentUiCommandStatus::noGesture, host.uiCommandGeneration(), lastAdmittedSequence() };
        if (active->second.id != request.id)
            return { InstrumentUiCommandStatus::unknownControl, host.uiCommandGeneration(), lastAdmittedSequence() };

        const auto stale = request.generation != active->second.generation
            || request.generation != host.uiCommandGeneration();
        host.endUiGesture (active->second.hostSlot);
        gestures.erase (active);
        if (stale)
            return { InstrumentUiCommandStatus::staleGeneration, host.uiCommandGeneration(), lastAdmittedSequence() };
        ++sequence;
        return { InstrumentUiCommandStatus::accepted, request.generation, lastAdmittedSequence() };
    }

    void closeSession (std::uint64_t sessionId)
    {
        const auto active = gestures.find (sessionId);
        if (active == gestures.end())
            return;
        host.endUiGesture (active->second.hostSlot);
        gestures.erase (active);
        ++sequence;
    }

private:
    struct ActiveGesture
    {
        std::uint32_t generation;
        std::string id;
        std::size_t hostSlot;
    };

    InstrumentUiCommandHost& host;
    std::atomic<std::uint64_t> sequence { 0 };
    std::uint64_t nextSessionId = 0;
    std::map<std::uint64_t, ActiveGesture> gestures;
};
