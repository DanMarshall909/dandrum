#include "InstrumentUiMeterCapture.h"

#include <atomic>
#include <cmath>
#include <cstdlib>
#include <iostream>
#include <new>

namespace
{
std::atomic<bool> countingAllocations { false };
std::atomic<unsigned> allocationCount { 0 };

bool near (double actual, double expected)
{
    return std::abs (actual - expected) < 0.000001;
}
}

void* operator new (std::size_t size)
{
    if (countingAllocations.load (std::memory_order_relaxed))
        allocationCount.fetch_add (1, std::memory_order_relaxed);
    if (auto* memory = std::malloc (size))
        return memory;
    throw std::bad_alloc();
}

void operator delete (void* memory) noexcept { std::free (memory); }
void operator delete (void* memory, std::size_t) noexcept { std::free (memory); }

int main()
{
    static_assert (sizeof (InstrumentUiMeterCapture) <= 8192);
    InstrumentUiMeterCapture meter;
    InstrumentUiMeterCapture::Frame frame;
    const float left[] { 0.5f, -0.5f };
    const float right[] { 0.25f, -0.25f };

    meter.beginStream();
    meter.capture (left, right, 2, 7);
    if (meter.pop (frame))
    {
        std::cerr << "disabled meter published visual data\n";
        return 1;
    }

    meter.setEnabled (true);
    countingAllocations.store (true, std::memory_order_relaxed);
    meter.capture (left, right, 2, 7);
    countingAllocations.store (false, std::memory_order_relaxed);
    if (allocationCount.load (std::memory_order_relaxed) != 0 || ! meter.pop (frame)
        || frame.generation != 7 || frame.streamId != 1 || frame.sequence != 1
        || frame.samplePosition != 2 || frame.sampleCount != 2
        || frame.bus != InstrumentUiMeterCapture::Bus::master
        || ! near (frame.peak[0], 0.5) || ! near (frame.peak[1], 0.25)
        || ! near (frame.energy[0], 0.5) || ! near (frame.energy[1], 0.125))
    {
        std::cerr << "meter capture lost signed stereo measurements, identity or callback allocation bound\n";
        return 1;
    }

    countingAllocations.store (true, std::memory_order_relaxed);
    for (std::size_t i = 0; i < InstrumentUiMeterCapture::capacity + 1000; ++i)
        meter.capture (left, right, 2, 7);
    countingAllocations.store (false, std::memory_order_relaxed);
    if (allocationCount.load (std::memory_order_relaxed) != 0
        || meter.lostFrames() != 1000)
    {
        std::cerr << "stalled meter consumer did not retain bounded capture and loss accounting\n";
        return 1;
    }

    for (std::size_t i = 0; i < InstrumentUiMeterCapture::capacity; ++i)
    {
        if (! meter.pop (frame) || frame.sequence != i + 2
            || frame.samplePosition != 4 + 2 * i)
        {
            std::cerr << "overflow changed previously published meter history\n";
            return 1;
        }
    }
    if (meter.pop (frame))
    {
        std::cerr << "meter queue grew beyond fixed capacity\n";
        return 1;
    }

    meter.beginStream();
    meter.capture (left, right, 2, 8);
    if (! meter.pop (frame) || frame.generation != 8 || frame.streamId != 2
        || frame.sequence != 0 || frame.samplePosition != 0)
    {
        std::cerr << "new instrument stream reused stale meter identity\n";
        return 1;
    }
    return 0;
}
