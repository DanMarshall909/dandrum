#include "RustEngineBindings.h"
#include "DefaultPatch.h"

#include <cmath>
#include <cstddef>
#include <iostream>
#include <string>

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

    dandrum_engine_prepare (engine, 48000.0f);
    dandrum_engine_note_on (engine, 60, 110);

    float left[64] {};
    float right[64] {};
    const auto rendered = dandrum_engine_render (engine, left, right, 64);

    dandrum_engine_note_off (engine, 60);
    dandrum_engine_destroy (engine);

    if (rendered != 64)
    {
        std::cerr << "expected 64 rendered samples, got " << rendered << '\n';
        return 1;
    }

    if (! bufferIsFinite (left, 64) || ! bufferIsFinite (right, 64))
    {
        std::cerr << "render produced non-finite samples\n";
        return 1;
    }

    return 0;
}
