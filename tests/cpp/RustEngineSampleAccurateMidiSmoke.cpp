#include "RustEngineBindings.h"
#include "DefaultPatch.h"

#include <cmath>
#include <cstddef>
#include <iostream>
#include <memory>

namespace
{
bool bufferIsFinite (const float* samples, std::size_t count)
{
    for (std::size_t i = 0; i < count; ++i)
        if (! std::isfinite (samples[i]))
            return false;

    return true;
}
} // namespace

int main()
{
    const auto patchPath = dandrum::defaultPatchPath().string();
    constexpr std::size_t numSamples = 64;
    const DandrumKernelBusDeclaration master { "master", 2, 2 };
    std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)> engine (
        dandrum_kernel_prepare_file (patchPath.c_str(), 48000, numSamples, &master, 1),
        &dandrum_kernel_destroy);
    if (! engine)
    {
        std::cerr << "failed to load default patch: " << patchPath << '\n';
        return 1;
    }

    constexpr std::size_t noteOnOffset = 20;
    constexpr std::size_t noteOffOffset = 50;

    dandrum_kernel_note_on_at (engine.get(), 60, 110, noteOnOffset);
    dandrum_kernel_note_off_at (engine.get(), 60, noteOffOffset);

    float left[numSamples] {};
    float right[numSamples] {};
    float* channels[] { left, right };
    const DandrumKernelOutputBusView output { "master", channels, 2, numSamples };
    const auto rendered = dandrum_kernel_render (engine.get(), nullptr, 0, &output, 1, numSamples);

    if (rendered != numSamples)
    {
        std::cerr << "expected " << numSamples << " rendered samples, got " << rendered << '\n';
        return 1;
    }

    if (! bufferIsFinite (left, numSamples) || ! bufferIsFinite (right, numSamples))
    {
        std::cerr << "render produced non-finite samples\n";
        return 1;
    }

    for (std::size_t i = 0; i < noteOnOffset; ++i)
    {
        if (left[i] != 0.0f || right[i] != 0.0f)
        {
            std::cerr << "expected silence before note-on frame offset " << noteOnOffset
                       << ", got non-zero sample at index " << i << '\n';
            return 1;
        }
    }

    return 0;
}
