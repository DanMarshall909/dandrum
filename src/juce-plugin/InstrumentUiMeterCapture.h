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

        const auto write = writeIndex.load (std::memory_order_relaxed);
        const auto read = readIndex.load (std::memory_order_acquire);
        if (write - read >= capacity)
        {
            lost.fetch_add (1, std::memory_order_relaxed);
            return;
        }

        Frame frame;
        frame.generation = generation;
        frame.streamId = streamId;
        frame.sequence = currentSequence;
        frame.samplePosition = currentPosition;
        frame.sampleCount = samples;
        const std::array<const float*, channelCount> channels { left, right };
        for (std::size_t channel = 0; channel < channelCount; ++channel)
            for (std::size_t sample = 0; sample < samples; ++sample)
            {
                const auto value = channels[channel][sample];
                frame.peak[channel] = std::max (frame.peak[channel], std::abs (value));
                frame.energy[channel] += static_cast<double> (value) * value;
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

private:
    static_assert ((capacity & (capacity - 1)) == 0);
    static_assert (std::atomic<std::uint64_t>::is_always_lock_free);
    static_assert (std::atomic<bool>::is_always_lock_free);

    std::array<Frame, capacity> frames {};
    std::atomic<std::uint64_t> writeIndex { 0 };
    std::atomic<std::uint64_t> readIndex { 0 };
    std::atomic<std::uint64_t> lost { 0 };
    std::atomic<bool> captureEnabled { false };
    std::uint64_t streamId = 0;
    std::uint64_t sequence = 0;
    std::uint64_t samplePosition = 0;
};
