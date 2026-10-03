#pragma once

#include <algorithm>
#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>

// One audio producer and one analysis-worker consumer. The processor owns
// storage; callbacks copy PCM, never allocating or retaining a host pointer.
class InstrumentUiLiveCapture final
{
public:
    static constexpr std::size_t maxTaps = 1;
    static constexpr std::size_t channelCount = 2;
    static constexpr std::size_t chunkFrames = 256;
    static constexpr std::size_t capacity = 64;
    enum class Bus : std::uint8_t { master };
    struct Frame
    {
        Bus bus = Bus::master;
        std::uint8_t channels = 0;
        std::uint32_t generation = 0;
        std::uint32_t sampleRateHz = 0;
        std::uint64_t streamId = 0;
        std::uint64_t selectionId = 0;
        std::uint64_t sequence = 0;
        std::uint64_t samplePosition = 0;
        std::size_t sampleCount = 0;
        bool gap = false;
        std::array<std::array<float, chunkFrames>, channelCount> pcm {};
    };
    // One off-audio control writer. Zero disables capture. The revision lets
    // the worker discard earlier queued data even when hide/show happens
    // between callbacks and returns to the same channel mask.
    bool setChannels (std::uint8_t channels) noexcept
    {
        if (channels >= (1U << channelCount)) return false;
        const auto previous = selection.load (std::memory_order_relaxed);
        if (static_cast<std::uint8_t> (previous) != channels)
            selection.store ((((previous >> 8) + 1) << 8) | channels, std::memory_order_release);
        return true;
    }
    std::uint64_t selectionId() const noexcept
    { return selection.load (std::memory_order_acquire); }

    // Audio only, after host preparation or an engine replacement.
    void beginStream() noexcept
    {
        ++streamId; sequence = 0; samplePosition = 0;
        previousSelection = selectionId(); nextGap = true;
    }
    void capture (const float* left, const float* right, std::size_t samples,
                  std::uint32_t generation, std::uint32_t sampleRateHz) noexcept
    {
        if (samples == 0) return;
        const auto selected = selectionId();
        if (selected != previousSelection)
        {
            previousSelection = selected;
            ++streamId; sequence = 0; nextGap = true;
        }
        const auto firstSample = samplePosition;
        samplePosition += samples;
        const auto mask = static_cast<std::uint8_t> (selected);
        if (mask == 0) return;
        const std::array<const float*, channelCount> input { left, right };
        for (std::size_t offset = 0; offset < samples;)
        {
            const auto count = std::min (chunkFrames, samples - offset);
            const auto currentSequence = sequence++;
            const auto write = writeIndex.load (std::memory_order_relaxed);
            if (write - readIndex.load (std::memory_order_acquire) >= capacity)
            {
                lost.fetch_add (1, std::memory_order_relaxed);
                nextGap = true;
            }
            else
            {
                Frame frame;
                frame.channels = mask; frame.generation = generation;
                frame.sampleRateHz = sampleRateHz; frame.streamId = streamId;
                frame.selectionId = selected; frame.sequence = currentSequence;
                frame.samplePosition = firstSample + offset; frame.sampleCount = count;
                frame.gap = nextGap;
                for (std::size_t channel = 0; channel < channelCount; ++channel)
                    if ((mask & (1U << channel)) != 0)
                        std::copy_n (input[channel] + offset, count, frame.pcm[channel].begin());
                frames[write % capacity] = frame;
                writeIndex.store (write + 1, std::memory_order_release);
                nextGap = false;
            }
            offset += count;
        }
    }
    // Analysis-worker only. No editor reads or resets this queue.
    bool pop (Frame& frame) noexcept
    {
        const auto read = readIndex.load (std::memory_order_relaxed);
        if (read == writeIndex.load (std::memory_order_acquire)) return false;
        frame = frames[read % capacity];
        readIndex.store (read + 1, std::memory_order_release);
        return true;
    }
    std::uint64_t lostFrames() const noexcept
    { return lost.load (std::memory_order_relaxed); }
private:
    static_assert (std::atomic<std::uint64_t>::is_always_lock_free);
    std::array<Frame, capacity> frames {};
    std::atomic<std::uint64_t> selection { 0 }, writeIndex { 0 }, readIndex { 0 }, lost { 0 };
    std::uint64_t previousSelection = 0, streamId = 0, sequence = 0, samplePosition = 0;
    bool nextGap = true;
};
