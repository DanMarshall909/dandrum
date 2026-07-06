#include "RustEngineBindings.h"
#include "DefaultPatch.h"

#include <cmath>
#include <cstddef>
#include <iostream>

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
    DandrumEngine* engine = dandrum_engine_create();
    if (engine == nullptr)
    {
        std::cerr << "dandrum_engine_create returned null\n";
        return 1;
    }

    const auto patchPath = dandrum::defaultPatchPath().string();
    if (! dandrum_engine_load_patch (engine, patchPath.c_str()))
    {
        std::cerr << "failed to load default patch: " << patchPath << '\n';
        return 1;
    }

    constexpr std::size_t numSamples = 64;
    constexpr std::size_t noteOnOffset = 20;
    constexpr std::size_t noteOffOffset = 50;

    dandrum_engine_prepare_realtime (engine, 48000.0f, numSamples);
    dandrum_engine_note_on_at (engine, 60, 110, noteOnOffset);
    dandrum_engine_note_off_at (engine, 60, noteOffOffset);

    float left[numSamples] {};
    float right[numSamples] {};
    const auto rendered = dandrum_engine_render (engine, left, right, numSamples);

    dandrum_engine_destroy (engine);

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
