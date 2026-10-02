#include "RustEngineBindings.h"
#include "DefaultPatch.h"

#include <cmath>
#include <cstddef>
#include <array>
#include <atomic>
#include <filesystem>
#include <fstream>
#include <future>
#include <iostream>
#include <memory>
#include <string>
#include <thread>

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

bool preparedMetadataSmoke()
{
    const auto root = std::filesystem::path (__FILE__).parent_path().parent_path().parent_path();
    const auto patch = root / "examples/patches/advanced-drum-kit.yaml";
    const DandrumKernelBusDeclaration master { "master", 2, 2 };
    std::unique_ptr<DandrumKernelInstrument, decltype (&dandrum_kernel_destroy)> engine (
        dandrum_kernel_prepare_file (patch.string().c_str(), 96000, 8, &master, 1),
        &dandrum_kernel_destroy);
    if (! engine)
        return false;
    std::unique_ptr<DandrumKernelUiSnapshot, decltype (&dandrum_kernel_ui_snapshot_destroy)> snapshot (
        dandrum_kernel_ui_snapshot_create (engine.get()), &dandrum_kernel_ui_snapshot_destroy);
    std::unique_ptr<DandrumKernelUiSnapshot, decltype (&dandrum_kernel_ui_snapshot_destroy)> stalledSnapshot (
        dandrum_kernel_ui_snapshot_create (engine.get()), &dandrum_kernel_ui_snapshot_destroy);
    std::unique_ptr<DandrumKernelWaveformSource, decltype (&dandrum_kernel_waveform_source_destroy)> waveform (
        dandrum_kernel_waveform_source_create (engine.get(), 0),
        &dandrum_kernel_waveform_source_destroy);
    if (! waveform || dandrum_kernel_waveform_source_create (engine.get(), 1) != nullptr)
        return false;
    std::promise<void> resumeReader;
    auto resume = resumeReader.get_future();
    std::atomic<bool> retainedValueWasValid { false };
    std::thread reader ([owned = std::move (stalledSnapshot),
                         resume = std::move (resume), &retainedValueWasValid] () mutable
    {
        resume.wait();
        DandrumKernelUiRegion region {};
        retainedValueWasValid.store (owned
            && dandrum_kernel_ui_region (owned.get(), 0, 2, &region)
            && std::string (region.id.data, region.id.size) == "snare_hard_a",
            std::memory_order_relaxed);
    });
    engine.reset();
    resumeReader.set_value();
    reader.join();
    if (! retainedValueWasValid.load (std::memory_order_relaxed)
        || ! snapshot || dandrum_kernel_ui_source_count (snapshot.get()) != 1)
        return false;
    DandrumKernelWaveformSourceInfo waveformInfo {};
    DandrumKernelWaveformBucket waveformBucket {};
    if (! dandrum_kernel_waveform_source_info (waveform.get(), &waveformInfo)
        || std::string (waveformInfo.sourceId.data, waveformInfo.sourceId.size) != "drums"
        || waveformInfo.sampleRateHz != 48000 || waveformInfo.channelCount != 1
        || waveformInfo.frameCount != 51000
        || ! dandrum_kernel_waveform_reduce (waveform.get(), 0, 0, 1, &waveformBucket, 1)
        || waveformBucket.startFrame != 0 || waveformBucket.endFrame != 1
        || waveformBucket.minimum != -0.5f || waveformBucket.maximum != -0.5f)
        return false;
    DandrumKernelUiSource source {};
    DandrumKernelUiMap map {};
    DandrumKernelUiZone soft {}, hard {};
    std::int32_t sharedGroup = -1, snareGroup = -1;
    const auto text = [] (DandrumKernelStringView value) { return std::string (value.data, value.size); };
    return dandrum_kernel_ui_source (snapshot.get(), 0, &source)
        && text (source.id) == "drums" && source.sampleRateHz == 48000
        && source.frameCount == 51000
        && dandrum_kernel_ui_map (snapshot.get(), 0, &map)
        && text (map.selectionMode) == "round_robin"
        && dandrum_kernel_ui_zone (snapshot.get(), 0, 1, &soft)
        && dandrum_kernel_ui_zone (snapshot.get(), 0, 2, &hard)
        && soft.velocityHigh == 63 && hard.velocityLow == 64
        && hard.regionIndex == 2 && hard.controlGroup == 2
        && dandrum_kernel_ui_public_control_group (snapshot.get(), 0, &sharedGroup)
        && dandrum_kernel_ui_public_control_group (snapshot.get(), 9, &snareGroup)
        && sharedGroup == 0 && snareGroup == 2;
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

    if (! preparedMetadataSmoke())
    {
        std::cerr << "prepared metadata FFI smoke failed\n";
        return 1;
    }

    return 0;
}
