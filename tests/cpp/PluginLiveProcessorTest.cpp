#include "PluginProcessor.h"

#include <bit>
#include <chrono>
#include <condition_variable>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <future>
#include <iostream>
#include <new>
#include <stdexcept>
#include <thread>
#if defined(__linux__)
 #include <pthread.h>
#endif

namespace
{
using namespace std::chrono_literals;
thread_local bool inAudio = false;
std::atomic<unsigned> allocations { 0 }, locks { 0 }, joins { 0 }, lifecycle { 0 };
std::atomic<unsigned> observedJoins { 0 };
std::atomic<unsigned> ffts { 0 }, observedFfts { 0 };
void require (bool value, const char* message) { if (! value) throw std::runtime_error (message); }
void near (double a, double b, const char* message)
{ require (std::isfinite (a) && std::abs (a - b) < 0.0002, message); }
template<class Predicate> void await (Predicate predicate, const char* message)
{
    const auto deadline = std::chrono::steady_clock::now() + 3s;
    while (! predicate())
    {
        require (std::chrono::steady_clock::now() < deadline, message);
        std::this_thread::sleep_for (1ms);
    }
}
InstrumentDemoConfiguration fixture()
{
    auto configuration = InstrumentDemoConfiguration::sampler();
    configuration.instrumentPath = std::filesystem::path (DANDRUM_SOURCE_ROOT) / "tests/fixtures/plugin-ui-knob.yaml";
    return configuration;
}
void prepare (DandrumAudioProcessor& processor, double rate)
{
    require (processor.isInstrumentLoaded(), "Live processor fixture did not load");
    processor.setFileWatchEnabled (false);
    processor.setRateAndBufferSizeDetails (rate, 64);
    processor.prepareToPlay (rate, 64);
    auto* level = processor.getParameterForPublicId ("fixture.level");
    require (level != nullptr, "Live processor fixture lost its actual public control");
    level->setValueNotifyingHost (0.75f);
}
void render (DandrumAudioProcessor& processor, int frames, float expected)
{
    juce::AudioBuffer<float> buffer (4, frames);
    juce::MidiBuffer midi;
    for (int channel = 0; channel < 4; ++channel)
        for (int n = 0; n < frames; ++n) buffer.setSample (channel, n, 0.93f);
    inAudio = true;
    processor.processBlock (buffer, midi);
    inAudio = false;
    for (int channel = 0; channel < 4; ++channel)
        for (int n = 0; n < frames; ++n)
        {
            // The fixture multiplies its silent right lane by gain: negative
            // gain produces IEEE -0. Extra cleared output lanes remain +0.
            const float silent = channel == 1 && expected < 0 ? -0.0f : 0.0f;
            require (std::bit_cast<std::uint32_t> (buffer.getSample (channel, n))
                == std::bit_cast<std::uint32_t> (channel == 0 ? expected : silent),
                "Live capture changed literal signed processor output or uncleared channels");
        }
}
InstrumentUiLiveService::Packet packet (DandrumAudioProcessor& processor, std::uint64_t session)
{
    std::optional<InstrumentUiLiveService::Packet> result;
    await ([&] { result = processor.takeLiveAnalysisPacket (session); return result.has_value(); },
        "Real processor failed to publish live captured PCM");
    return *result;
}
void normalCapture()
{
    for (const auto rate : { 44100.0, 48000.0, 96000.0 })
    {
        DandrumAudioProcessor processor (fixture());
        prepare (processor, rate);
        const auto generation = processor.getParameterSurfaceGeneration();
        auto* identity = processor.getParameterForPublicId ("fixture.level");
        for (unsigned n = 0; n < 3; ++n) render (processor, 64, 0.5f);
        require (! processor.subscribeLiveAnalysis (11, generation - 1, 3)
            && ! processor.subscribeLiveAnalysis (11, generation, 4)
            && processor.subscribeLiveAnalysis (11, generation, 3),
            "Real processor rejected valid live analysis admission");
        for (unsigned n = 0; n < 16; ++n) render (processor, 64, 0.5f);
        const auto first = packet (processor, 11);
        require (first.analysis.generation == generation && first.analysis.sampleRateHz == static_cast<std::uint32_t> (rate)
            && first.analysis.startFrame == 192 && first.analysis.endFrame == 1216
            && first.analysis.channels == 3 && first.analysis.gap,
            "Processor live capture lost actual generation/rate/sample coordinates");
        near (first.analysis.channel[0].scope[0].minimum, 0.5, "Live processor scope did not measure actual positive audio");
        near (first.analysis.channel[0].magnitudeDbFS[0], -6.020599913,
            "Live processor DC spectrum did not measure actual positive audio");
        near (first.analysis.channel[1].magnitudeDbFS[0], -120,
            "Live processor invented right-channel audio");
        near (first.analysis.frequencyHz[64], rate / 16, "Live processor used an invented analysis rate");
        require (processor.acknowledgeLiveAnalysisPacket (11, generation, first.sequence),
            "Processor rejected current live acknowledgement");
        identity->setValueNotifyingHost (0.25f);
        for (unsigned n = 0; n < 16; ++n) render (processor, 64, -0.5f);
        await ([&] { return processor.getLiveAnalysisStatistics().consumedChunks >= 32; },
            "Processor worker failed later captured audio");
        const auto negative = packet (processor, 11);
        require (negative.analysis.endFrame == 2240
            && std::bit_cast<std::uint32_t> (negative.analysis.channel[0].scope[0].minimum)
                == std::bit_cast<std::uint32_t> (-0.5f)
            && processor.getParameterSurfaceGeneration() == generation
            && processor.getParameterForPublicId ("fixture.level") == identity,
            "Ordinary live automation rebuilt the processor or lost negative PCM");
        require (processor.setLiveAnalysisVisible (11, false)
            && ! processor.takeLiveAnalysisPacket (11), "Hidden processor retained live delivery");
        render (processor, 256, -0.5f);
        require (processor.setLiveAnalysisVisible (11, true), "Processor could not resume live demand");
        for (unsigned n = 0; n < 16; ++n) render (processor, 64, -0.5f);
        const auto resumed = packet (processor, 11);
        require (resumed.analysis.startFrame == 2496 && resumed.analysis.endFrame == 3520
            && resumed.analysis.gap, "Processor resumed obsolete hidden coordinates");
        require (processor.acknowledgeLiveAnalysisPacket (11, generation, resumed.sequence),
            "Processor could not acknowledge resumed data");
        processor.setMuted (true);
        for (unsigned n = 0; n < 16; ++n) render (processor, 64, 0.0f);
        await ([&] { return processor.getLiveAnalysisStatistics().consumedChunks >= 64; },
            "Processor did not capture actual muted output");
        const auto muted = packet (processor, 11);
        require (muted.analysis.startFrame == 3520 && muted.analysis.endFrame == 4544,
            "Muted capture lost continuous sample coordinates");
        near (muted.analysis.channel[0].scope[0].maximum, 0, "Muted scope retained sounding audio");
        near (muted.analysis.channel[0].magnitudeDbFS[0], -120, "Muted spectrum retained sounding audio");
        processor.setMuted (false);
        render (processor, 64, -0.5f);
        require (processor.unsubscribeLiveAnalysis (11), "Processor retained closed live demand");
    }
}
void stalledCapture()
{
    std::mutex mutex;
    std::condition_variable_any wake;
    bool entered = false, released = false;
    std::atomic<unsigned> workerCalls { 0 }, callbackWorkerCalls { 0 };
    DandrumAudioProcessor processor (fixture(), [&] (std::stop_token stop)
    {
        ++workerCalls;
        if (inAudio) ++callbackWorkerCalls;
        std::unique_lock lock (mutex);
        entered = true; wake.notify_all();
        wake.wait (lock, stop, [&] { return released; });
    });
    prepare (processor, 48000);
    const auto generation = processor.getParameterSurfaceGeneration();
    require (processor.subscribeLiveAnalysis (22, generation, 3), "Held processor rejected live admission");
    {
        std::unique_lock lock (mutex);
        require (wake.wait_for (lock, 3s, [&] { return entered; }), "Actual processor worker did not reach hold");
    }
    auto audio = std::async (std::launch::async, [&]
    {
        for (unsigned n = 0; n < 72; ++n) render (processor, 256, 0.5f);
    });
    require (audio.wait_for (3s) == std::future_status::ready,
        "Processor audio waited for its stalled live worker");
    audio.get();
    require (! released && workerCalls != 0 && callbackWorkerCalls == 0
        && processor.getLiveAnalysisStatistics().droppedCaptureChunks == 8,
        "Processor stall did not preserve bounded capture loss or worker isolation");
    {
        const std::scoped_lock lock (mutex); released = true; wake.notify_all();
    }
    await ([&] { return processor.getLiveAnalysisStatistics().consumedChunks == 64; },
        "Processor failed to discard old overflow history");
    require (! processor.takeLiveAnalysisPacket (22), "Processor published pre-overflow history");
    render (processor, 512, 0.5f);
    await ([&] { return processor.getLiveAnalysisStatistics().consumedChunks == 66; },
        "Processor failed to capture first recovery half");
    require (! processor.takeLiveAnalysisPacket (22), "Processor padded partial recovery PCM");
    render (processor, 512, 0.5f);
    const auto recovered = packet (processor, 22);
    require (recovered.analysis.startFrame == 18432 && recovered.analysis.endFrame == 19456
        && recovered.analysis.gap, "Processor live recovery lost current sample coordinates");
    near (recovered.analysis.channel[0].scope[0].minimum, 0.5,
        "Processor recovery changed signed captured PCM");
}
}

void* operator new (std::size_t size)
{
    if (inAudio) ++allocations;
    if (auto* memory = std::malloc (size ? size : 1)) return memory;
    throw std::bad_alloc();
}
void* operator new[] (std::size_t size) { return ::operator new (size); }
void operator delete (void* value) noexcept { std::free (value); }
void operator delete (void* value, std::size_t) noexcept { std::free (value); }
void operator delete[] (void* value) noexcept { std::free (value); }
void operator delete[] (void* value, std::size_t) noexcept { std::free (value); }
#if defined(__linux__)
extern "C" int __real_pthread_mutex_lock (pthread_mutex_t*);
extern "C" int __wrap_pthread_mutex_lock (pthread_mutex_t* mutex)
{ if (inAudio) ++locks; return __real_pthread_mutex_lock (mutex); }
// libstdc++ calls pthread_join inside a shared library, outside linker wrapping.
// Wrap the actual undefined std::thread::join symbol verified with nm instead.
extern "C" void __real__ZNSt6thread4joinEv (std::thread*);
// Member-function pointers reserve the low bit. The free wrapper must retain
// member-function alignment when std::async takes &std::thread::join.
extern "C" __attribute__((aligned(16))) void __wrap__ZNSt6thread4joinEv (std::thread* thread)
{ ++observedJoins; if (inAudio) ++joins; __real__ZNSt6thread4joinEv (thread); }
extern "C" void __real__ZNK4juce3dsp3FFT36performFrequencyOnlyForwardTransformEPfb (
    const juce::dsp::FFT*, float*, bool);
extern "C" __attribute__((aligned(16))) void __wrap__ZNK4juce3dsp3FFT36performFrequencyOnlyForwardTransformEPfb (
    const juce::dsp::FFT* transform, float* pcm, bool positiveOnly)
{
    ++observedFfts; if (inAudio) ++ffts;
    __real__ZNK4juce3dsp3FFT36performFrequencyOnlyForwardTransformEPfb (transform, pcm, positiveOnly);
}
extern "C" void __real_dandrum_kernel_destroy (DandrumKernelInstrument*);
extern "C" void __wrap_dandrum_kernel_destroy (DandrumKernelInstrument* engine)
{ if (inAudio) ++lifecycle; __real_dandrum_kernel_destroy (engine); }
extern "C" DandrumKernelInstrument* __real_dandrum_kernel_prepare_file (
    const char*, std::uint32_t, std::size_t, const DandrumKernelBusDeclaration*, std::size_t);
extern "C" DandrumKernelInstrument* __wrap_dandrum_kernel_prepare_file (
    const char* file, std::uint32_t rate, std::size_t block,
    const DandrumKernelBusDeclaration* buses, std::size_t count)
{ if (inAudio) ++lifecycle; return __real_dandrum_kernel_prepare_file (file, rate, block, buses, count); }
#endif
int main()
{
    juce::ScopedJuceInitialiser_GUI gui;
    try
    {
        normalCapture(); stalledCapture();
#if defined(__linux__)
        require (observedJoins >= 4, "Live processor join interceptor never observed actual worker teardown");
        require (observedFfts >= 8, "Live processor FFT interceptor never observed real analysis");
#endif
        std::cout << "CALLBACK_GUARD cpp_new=" << allocations << " locks=" << locks
                  << " joins=" << joins << " engine_lifecycle=" << lifecycle
                  << " ffts=" << ffts << " observed_total_joins=" << observedJoins
                  << " observed_total_ffts=" << observedFfts << '\n';
        require (allocations == 0 && locks == 0 && joins == 0 && lifecycle == 0 && ffts == 0,
            "Live processor callback allocated, locked, joined, transformed or managed engine resources");
    }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
    std::cout << "LIVE_PROCESSOR signed capture and held-worker independence PASS\n";
}
