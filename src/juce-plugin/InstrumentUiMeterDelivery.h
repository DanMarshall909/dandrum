#pragma once

#include "InstrumentUiMeterAggregation.h"
#include "InstrumentUiMeterDisplay.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>

// Message-thread only. One fixed slot per editor session and no queue of
// browser payloads: a stalled consumer keeps one replaceable latest packet.
class InstrumentUiMeterDelivery final
{
public:
    static constexpr std::size_t maxSessions = 4;

    struct Packet
    {
        std::uint64_t sequence = 0;
        InstrumentUiMeterAggregation::Snapshot meter;
        InstrumentUiMeterCapture::ClipSnapshot clip;
        InstrumentUiMeterDisplay::Snapshot display;
    };

    bool subscribe (std::uint64_t id, std::uint32_t generation) noexcept
    {
        if (find (id) != nullptr)
            return false;
        for (auto& session : sessions)
            if (! session.active)
            {
                session = {};
                session.active = true;
                session.visible = true;
                session.id = id;
                session.generation = generation;
                return true;
            }
        return false;
    }

    bool unsubscribe (std::uint64_t id) noexcept
    {
        if (auto* session = find (id))
        {
            *session = {};
            return true;
        }
        return false;
    }

    void retireOtherGenerations (std::uint32_t currentGeneration) noexcept
    {
        for (auto& session : sessions)
            if (session.active && session.generation != currentGeneration)
                session = {};
    }

    bool setVisible (std::uint64_t id, bool visible) noexcept
    {
        auto* session = find (id);
        if (session == nullptr)
            return false;
        if (session->visible != visible)
        {
            session->visible = visible;
            session->latestValid = false;
            session->awaitingAck = false;
        }
        return true;
    }

    std::size_t visibleCount() const noexcept
    {
        std::size_t count = 0;
        for (const auto& session : sessions)
            if (session.active && session.visible)
                ++count;
        return count;
    }

    void publish (Packet packet) noexcept
    {
        if (! packet.clip.valid || packet.clip.generation != packet.meter.generation)
            return;
        packet.sequence = ++nextSequence;
        for (auto& session : sessions)
            if (session.active && session.visible
                && session.generation == packet.meter.generation)
            {
                session.latest = packet;
                session.latestValid = true;
            }
    }

    std::optional<Packet> take (std::uint64_t id) noexcept
    {
        auto* session = find (id);
        if (session == nullptr || ! session->visible || ! session->latestValid
            || session->awaitingAck)
            return std::nullopt;
        session->awaitingAck = true;
        session->outstandingSequence = session->latest.sequence;
        session->latestValid = false;
        return session->latest;
    }

    bool acknowledge (std::uint64_t id, std::uint32_t generation,
                      std::uint64_t sequence) noexcept
    {
        auto* session = find (id);
        if (session == nullptr || ! session->visible
            || session->generation != generation || ! session->awaitingAck
            || session->outstandingSequence != sequence)
            return false;
        session->awaitingAck = false;
        return true;
    }

    std::size_t pendingPayloads() const noexcept
    {
        std::size_t count = 0;
        for (const auto& session : sessions)
            if (session.active)
                count += static_cast<std::size_t> (session.awaitingAck)
                         + static_cast<std::size_t> (session.latestValid);
        return count;
    }

private:
    struct Session
    {
        std::uint64_t id = 0;
        std::uint64_t outstandingSequence = 0;
        std::uint32_t generation = 0;
        Packet latest;
        bool active = false;
        bool visible = false;
        bool latestValid = false;
        bool awaitingAck = false;
    };

    Session* find (std::uint64_t id) noexcept
    {
        for (auto& session : sessions)
            if (session.active && session.id == id)
                return &session;
        return nullptr;
    }

    std::array<Session, maxSessions> sessions {};
    std::uint64_t nextSequence = 0;
};
