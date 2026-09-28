#include "RustEngineBindings.h"
#include "DefaultPatch.h"

#include <cmath>
#include <cstddef>
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
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

bool kernelBusSmoke()
{
    const auto path = std::filesystem::temp_directory_path() / "dandrum-kernel-ffi-smoke.yaml";
    {
        std::ofstream file (path);
        file << "metadata: { name: kernel-ffi-smoke }\n"
                "ports:\n"
                "  - { name: input, direction: input, signal: audio, channels: 2, maps_to: amp.audio_in }\n"
                "  - { name: master, direction: output, signal: audio, channels: 2, maps_from: amp.audio_out }\n"
                "modules:\n"
                "  - { id: amp, type: gain, static: { channels: 2 } }\n"
                "connections: []\n";
    }

    const DandrumKernelBusDeclaration buses[] {
        { "input", 1, 2 },
        { "master", 2, 2 },
    };
    std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)> instrument (
        dandrum_kernel_prepare_file (path.string().c_str(), 48000, 8, buses, 2),
        &dandrum_kernel_destroy);
    std::filesystem::remove (path);
    if (! instrument || dandrum_kernel_root_port_count (instrument.get()) != 2)
        return false;

    char portName[32] {};
    std::uint32_t direction = 0, signalType = 0;
    std::size_t channels = 0;
    if (! dandrum_kernel_root_port (instrument.get(), 1, portName, sizeof portName,
                                   &direction, &signalType, &channels)
        || std::string (portName) != "master"
        || direction != 2 || signalType != 1 || channels != 2
        || dandrum_kernel_total_latency_samples (instrument.get()) != 0)
        return false;

    std::array<float, 8> leftIn, rightIn, leftOut {}, rightOut {};
    leftIn.fill (-0.5f);
    rightIn.fill (0.25f);
    const float* sourceChannels[] { leftIn.data(), rightIn.data() };
    float* destinationChannels[] { leftOut.data(), rightOut.data() };
    const DandrumKernelInputBusView input { "input", sourceChannels, 2, 8 };
    const DandrumKernelOutputBusView output { "master", destinationChannels, 2, 8 };
    if (dandrum_kernel_render (instrument.get(), &input, 1, &output, 1, 8) != 8)
        return false;
    if (! dandrum_kernel_reset (instrument.get()))
        return false;
    leftOut.fill (0.0f);
    rightOut.fill (0.0f);
    return dandrum_kernel_render (instrument.get(), &input, 1, &output, 1, 8) == 8
        && leftOut == leftIn && rightOut == rightIn;
}
} // namespace

int main()
{
    const auto patchPath = dandrum::defaultPatchPath().string();
    const DandrumKernelBusDeclaration master { "master", 2, 2 };
    std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)> engine (
        dandrum_kernel_prepare_file (patchPath.c_str(), 48000, 64, &master, 1),
        &dandrum_kernel_destroy);
    if (! engine)
    {
        std::cerr << "failed to load default patch: " << patchPath << '\n';
        return 1;
    }

    dandrum_kernel_note_on_at (engine.get(), 60, 110, 0);

    float left[64] {};
    float right[64] {};
    float* channels[] { left, right };
    const DandrumKernelOutputBusView output { "master", channels, 2, 64 };
    const auto rendered = dandrum_kernel_render (engine.get(), nullptr, 0, &output, 1, 64);

    const auto reset = dandrum_kernel_reset (engine.get());
    float silentLeft[64] {};
    float silentRight[64] {};
    float* silentChannels[] { silentLeft, silentRight };
    const DandrumKernelOutputBusView silentOutput { "master", silentChannels, 2, 64 };
    const auto silentFrames = dandrum_kernel_render (engine.get(), nullptr, 0, &silentOutput, 1, 64);
    dandrum_kernel_note_on_at (engine.get(), 61, 110, 0);
    float restartedLeft[64] {};
    float restartedRight[64] {};
    float* restartedChannels[] { restartedLeft, restartedRight };
    const DandrumKernelOutputBusView restartedOutput { "master", restartedChannels, 2, 64 };
    const auto restartedFrames = dandrum_kernel_render (engine.get(), nullptr, 0, &restartedOutput, 1, 64);

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

    if (! reset || silentFrames != 64 || restartedFrames != 64)
    {
        std::cerr << "host reset failed to preserve a usable engine\n";
        return 1;
    }
    bool restartedHasSignal = false;
    for (std::size_t i = 0; i < 64; ++i)
    {
        if (silentLeft[i] != 0.0f || silentRight[i] != 0.0f)
        {
            std::cerr << "host reset left audio running\n";
            return 1;
        }
        restartedHasSignal = restartedHasSignal || restartedLeft[i] != 0.0f || restartedRight[i] != 0.0f;
    }
    if (! restartedHasSignal || ! bufferIsFinite (restartedLeft, 64) || ! bufferIsFinite (restartedRight, 64))
    {
        std::cerr << "render after host reset produced non-finite samples\n";
        return 1;
    }

    if (! kernelBusSmoke())
    {
        std::cerr << "kernel named-bus FFI smoke failed\n";
        return 1;
    }

    return 0;
}
