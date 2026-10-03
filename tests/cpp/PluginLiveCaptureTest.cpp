#include "InstrumentUiLiveCapture.h"

#include <array>
#include <atomic>
#include <bit>
#include <cstdlib>
#include <iostream>
#include <new>

namespace
{
std::atomic<bool> counting { false };
std::atomic<unsigned> allocations { 0 };
void require (bool condition, const char* message)
{
    if (! condition) { std::cerr << message << '\n'; std::exit (1); }
}
using Capture = InstrumentUiLiveCapture;
using Frame = Capture::Frame;
}

void* operator new (std::size_t size)
{
    if (counting.load (std::memory_order_relaxed)) ++allocations;
    if (auto* memory = std::malloc (size)) return memory;
    throw std::bad_alloc();
}
void operator delete (void* memory) noexcept { std::free (memory); }
void operator delete (void* memory, std::size_t) noexcept { std::free (memory); }

int main()
{
    static_assert (Capture::maxTaps == 1 && Capture::channelCount == 2);
    static_assert (sizeof (Capture) <= 140 * 1024);
    Capture capture;
    Frame frame;
    std::array<float, 513> left {}, right {};
    left.fill (0.125f); right.fill (-0.25f);
    left[255] = -0.75f; left[256] = 0.5f; left[512] = -0.5f;
    right[0] = 0.625f; right[511] = -0.625f;

    capture.beginStream();
    capture.capture (nullptr, nullptr, 13, 7, 48000);
    require (! capture.pop (frame), "Disabled live capture published PCM");
    require (capture.setChannels (1), "Left live channel was not admitted");
    counting = true;
    capture.capture (left.data(), right.data(), left.size(), 7, 48000);
    counting = false;
    require (allocations == 0, "Live callback capture allocated");
    const auto original = left;
    left.fill (0.875f); // Host buffer reuse must not change queued PCM.
    std::uint64_t firstStream = 0;
    for (std::size_t chunk = 0; chunk < 3; ++chunk)
    {
        require (capture.pop (frame), "Subscribed live capture lost contiguous PCM");
        if (chunk == 0) firstStream = frame.streamId;
        const auto count = chunk == 2 ? 1U : 256U;
        require (frame.bus == Capture::Bus::master && frame.channels == 1
                 && frame.generation == 7 && frame.sampleRateHz == 48000
                 && frame.streamId == firstStream && firstStream != 0
                 && frame.sequence == chunk && frame.samplePosition == 13 + chunk * 256
                 && frame.sampleCount == count && frame.gap == (chunk == 0),
                 "Live capture lost source time, channel, rate or stream identity");
        for (std::size_t n = 0; n < frame.sampleCount; ++n)
            require (frame.pcm[0][n] == original[chunk * 256 + n]
                     && frame.pcm[1][n] == 0,
                     "Live capture changed signed PCM or published an unrequested channel");
    }
    require (! capture.pop (frame), "Live capture grew an extra chunk");

    require (! capture.setChannels (4) && ! capture.setChannels (255),
             "Unsupported live channels were accepted");
    capture.capture (original.data(), nullptr, 1, 7, 48000);
    require (capture.pop (frame) && frame.channels == 1 && frame.pcm[0][0] == 0.125f,
             "Invalid channel request changed the working capture selection");

    require (capture.setChannels (3), "Stereo live selection was rejected");
    capture.capture (original.data(), right.data(), 513, 7, 48000);
    const auto stereoStream = firstStream;
    require (capture.pop (frame) && frame.channels == 3 && frame.gap
             && frame.streamId != stereoStream && frame.sequence == 0
             && frame.samplePosition == 527 && frame.pcm[0][255] == -0.75f
             && frame.pcm[1][0] == 0.625f && frame.pcm[1][255] == -0.25f,
             "Live stereo selection lost signed samples or its discontinuity");
    while (capture.pop (frame)) {}

    capture.beginStream();
    counting = true;
    for (std::size_t n = 0; n < Capture::capacity + 1000; ++n)
        capture.capture (original.data(), right.data(), 256, 8, 44100);
    counting = false;
    require (allocations == 0 && capture.lostFrames() == 1000,
             "Stalled live consumer lost its memory/allocation/drop bound");
    for (std::size_t n = 0; n < Capture::capacity; ++n)
    {
        require (capture.pop (frame) && frame.sequence == n
                 && frame.samplePosition == n * 256 && frame.generation == 8
                 && frame.sampleRateHz == 44100 && frame.pcm[0][255] == -0.75f
                 && frame.pcm[1][0] == 0.625f && frame.gap == (n == 0),
                 "Overflow overwrote admitted live history or changed the consumer position");
    }
    require (! capture.pop (frame), "Live queue exceeded its fixed capacity");
    capture.capture (original.data(), right.data(), 256, 8, 44100);
    require (capture.pop (frame) && frame.gap
             && frame.sequence == Capture::capacity + 1000
             && frame.samplePosition == (Capture::capacity + 1000) * 256,
             "Live overflow hid the missing sample interval");
    capture.capture (original.data(), right.data(), 256, 8, 44100);
    require (capture.pop (frame) && ! frame.gap,
             "A live gap persisted after contiguous publication recovered");
    const auto oldStream = frame.streamId;
    const auto nextPosition = frame.samplePosition + frame.sampleCount;
    require (capture.setChannels (0), "Live capture could not unsubscribe");
    capture.capture (nullptr, nullptr, 1003, 8, 44100);
    require (! capture.pop (frame), "Hidden live view continued publishing");
    require (capture.setChannels (2), "Right live channel was rejected");
    capture.capture (nullptr, right.data(), 1, 8, 44100);
    require (capture.pop (frame) && frame.channels == 2 && frame.gap
             && frame.sequence == 0 && frame.streamId != oldStream
             && frame.samplePosition == nextPosition + 1003
             && frame.pcm[0][0] == 0 && frame.pcm[1][0] == 0.625f,
             "Resumed live capture reused stale coordinates or unrequested data");

    capture.capture (nullptr, nullptr, 0, 8, 44100);
    require (! capture.pop (frame), "Zero-length callback published live data");
    capture.beginStream();
    capture.capture (nullptr, right.data(), 1, 9, 96000);
    require (capture.pop (frame) && frame.generation == 9 && frame.sampleRateHz == 96000
             && frame.samplePosition == 0 && frame.sequence == 0 && frame.gap,
             "Prepared live stream retained an old generation or time origin");
    const auto preparedStream = frame.streamId;
    const auto oldSelection = frame.selectionId;
    require (capture.setChannels (2), "Unchanged live selection was rejected");
    capture.capture (nullptr, right.data(), 256, 9, 96000);
    // A hide/show entirely between callbacks must still retire queued PCM.
    require (capture.setChannels (0) && capture.setChannels (2),
             "Rapid live hide/show was rejected");
    const auto newSelection = capture.selectionId();
    require (newSelection != oldSelection && capture.pop (frame)
             && frame.selectionId == oldSelection && frame.selectionId != newSelection
             && frame.streamId == preparedStream && frame.sequence == 1 && ! frame.gap,
             "Rapid live hide/show made old queued PCM appear current");
    capture.capture (nullptr, right.data(), 1, 9, 96000);
    require (capture.pop (frame) && frame.selectionId == newSelection && frame.gap
             && frame.streamId != preparedStream && frame.sequence == 0
             && frame.samplePosition == 257 && frame.pcm[1][0] == 0.625f,
             "Live selection revision lost its current sample coordinates");
    std::cout << "LIVE_CAPTURE exact signed PCM, bounded loss and current resume PASS\n";
}
