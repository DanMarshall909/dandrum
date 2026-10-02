#pragma once

#include <algorithm>
#include <array>
#include <atomic>
#include <cmath>
#include <cstddef>
#include <cstdint>

// One audio producer and one off-audio consumer. Storage belongs to the
// processor, so closing an editor never invalidates the callback's queue.
class InstrumentUiMeterCapture final
{
public:
    static constexpr std::size_t capacity = 64;
    static constexpr std::size_t channelCount = 2;
    enum class Bus : std::uint8_t { master };

    struct Frame
    {
        Bus bus = Bus::master;
        std::uint32_t generation = 0;
        std::uint64_t streamId = 0;
        std::uint64_t sequence = 0;
        std::uint64_t samplePosition = 0;
        std::size_t sampleCount = 0;
        std::array<float, channelCount> peak {};
        std::array<double, channelCount> energy {};
        bool valid = true;
    };

    struct ClipSnapshot
    {
        std::uint32_t generation = 0;
        std::array<std::uint64_t, channelCount> ticket {};
        std::array<bool, channelCount> latched {};
        bool valid = false;
    };

    void setEnabled (bool enabled) noexcept
    {
        captureEnabled.store (enabled, std::memory_order_relaxed);
    }

    // Audio-thread only. Old queued frames retain their old stream ID until
    // the consumer drains or discards them.
    void beginStream() noexcept
    {
        ++streamId;
        sequence = 0;
        samplePosition = 0;
    }

    // Audio-thread only. Full queues reject the new frame; the producer never
    // changes the consumer-owned read position. No heap work or OS wakeup.
    void capture (const float* left, const float* right, std::size_t samples,
                  std::uint32_t generation) noexcept
    {
        const auto currentPosition = samplePosition;
        const auto currentSequence = sequence++;
        samplePosition += samples;
        if (! captureEnabled.load (std::memory_order_relaxed))
            return;

        if (clipGeneration.load (std::memory_order_relaxed) != generation)
        {
            // The audio thread alone changes the generation and baselines.
            // An odd revision lets a reader reject a partial reset.
            clipRevision.fetch_add (1, std::memory_order_acq_rel);
            for (std::size_t channel = 0; channel < channelCount; ++channel)
                clipReset[channel].store (clipLatest[channel].load (std::memory_order_relaxed),
                                          std::memory_order_relaxed);
            clipGeneration.store (generation, std::memory_order_release);
            clipRevision.fetch_add (1, std::memory_order_release);
        }

        const auto write = writeIndex.load (std::memory_order_relaxed);
        const auto read = readIndex.load (std::memory_order_acquire);
        const bool queueFull = write - read >= capacity;

        Frame frame;
        frame.generation = generation;
        frame.streamId = streamId;
        frame.sequence = currentSequence;
        frame.samplePosition = currentPosition;
        frame.sampleCount = samples;
        const std::array<const float*, channelCount> channels { left, right };
        for (std::size_t channel = 0; channel < channelCount; ++channel)
        {
            bool clipped = false;
            for (std::size_t sample = 0; sample < samples; ++sample)
            {
                const auto value = channels[channel][sample];
                if (! std::isfinite (value))
                {
                    frame.valid = false;
                    continue;
                }
                const auto magnitude = std::abs (value);
                if (magnitude >= 1.0f)
                    clipped = true;
                if (! queueFull)
                {
                    frame.peak[channel] = std::max (frame.peak[channel], magnitude);
                    frame.energy[channel] += static_cast<double> (value) * value;
                }
            }
            if (clipped)
                clipLatest[channel].store (++nextClipTicket[channel], std::memory_order_release);
        }

        if (queueFull)
        {
            lost.fetch_add (1, std::memory_order_relaxed);
            return;
        }

        frames[write % capacity] = frame;
        writeIndex.store (write + 1, std::memory_order_release);
    }

    // Off-audio single-consumer call. Copies a value; it never owns DSP data.
    bool pop (Frame& frame) noexcept
    {
        const auto read = readIndex.load (std::memory_order_relaxed);
        if (read == writeIndex.load (std::memory_order_acquire))
            return false;
        frame = frames[read % capacity];
        readIndex.store (read + 1, std::memory_order_release);
        return true;
    }

    std::uint64_t lostFrames() const noexcept
    {
        return lost.load (std::memory_order_relaxed);
    }

    // Off-audio. A partial generation reset is rejected and can be retried
    // on the next UI tick; this call never waits for the audio producer.
    ClipSnapshot clipSnapshot() const noexcept
    {
        ClipSnapshot result;
        const auto before = clipRevision.load (std::memory_order_acquire);
        if ((before & 1U) != 0)
            return result;
        result.generation = clipGeneration.load (std::memory_order_acquire);
        for (std::size_t channel = 0; channel < channelCount; ++channel)
        {
            result.ticket[channel] = clipLatest[channel].load (std::memory_order_acquire);
            const auto reset = clipReset[channel].load (std::memory_order_relaxed);
            const auto acknowledged = clipAcknowledged[channel].load (std::memory_order_acquire);
            result.latched[channel] = result.ticket[channel] > std::max (reset, acknowledged);
        }
        result.valid = before == clipRevision.load (std::memory_order_acquire);
        return result;
    }

    // Off-audio. Acknowledgement names exactly the occurrence observed by the
    // renderer. A new clip or generation cannot be cleared by a stale reply.
    bool acknowledgeClip (std::size_t channel, std::uint32_t generation,
                          std::uint64_t ticket) noexcept
    {
        if (channel >= channelCount || generation != clipGeneration.load (std::memory_order_acquire)
            || ticket == 0 || ticket != clipLatest[channel].load (std::memory_order_acquire)
            || ticket <= clipReset[channel].load (std::memory_order_acquire))
            return false;

        auto previous = clipAcknowledged[channel].load (std::memory_order_relaxed);
        while (previous < ticket
               && ! clipAcknowledged[channel].compare_exchange_weak (
                   previous, ticket, std::memory_order_acq_rel, std::memory_order_relaxed)) {}
        return previous < ticket;
    }

private:
    static_assert ((capacity & (capacity - 1)) == 0);
    static_assert (std::atomic<std::uint64_t>::is_always_lock_free);
    static_assert (std::atomic<std::uint32_t>::is_always_lock_free);
    static_assert (std::atomic<bool>::is_always_lock_free);

    std::array<Frame, capacity> frames {};
    std::atomic<std::uint64_t> writeIndex { 0 };
    std::atomic<std::uint64_t> readIndex { 0 };
    std::atomic<std::uint64_t> lost { 0 };
    std::atomic<bool> captureEnabled { false };
    std::atomic<std::uint64_t> clipRevision { 0 };
    std::atomic<std::uint32_t> clipGeneration { 0 };
    std::array<std::atomic<std::uint64_t>, channelCount> clipLatest {};
    std::array<std::atomic<std::uint64_t>, channelCount> clipReset {};
    std::array<std::atomic<std::uint64_t>, channelCount> clipAcknowledged {};
    std::array<std::uint64_t, channelCount> nextClipTicket {};
    std::uint64_t streamId = 0;
    std::uint64_t sequence = 0;
    std::uint64_t samplePosition = 0;
};
