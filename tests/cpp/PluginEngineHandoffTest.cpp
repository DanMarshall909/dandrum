#include "PluginProcessor.h"

#include <bit>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <functional>
#include <future>
#include <iostream>
#include <new>
#include <stdexcept>
#include <thread>
#include <pthread.h>

namespace
{
using namespace std::chrono_literals;
thread_local bool measuredAudio = false;
std::atomic<unsigned> callbackAllocation { 0 }, callbackLock { 0 }, callbackLifecycle { 0 };
std::atomic<unsigned> renderCalls { 0 };
void require (bool condition, const char* message)
{ if (! condition) throw std::runtime_error (message); }
template<class Predicate> void await (Predicate predicate, const char* message)
{
    const auto deadline = std::chrono::steady_clock::now() + 3s;
    while (! predicate())
    {
        require (std::chrono::steady_clock::now() < deadline, message);
        std::this_thread::sleep_for (1ms);
    }
}
struct Barrier
{
    std::atomic<bool> armed { false }, entered { false }, released { false };
    std::atomic<DandrumKernelInstrument*> engine { nullptr };
    std::atomic<unsigned> prematureDestroy { 0 }, watchedDestroy { 0 };
    std::function<void()> beforeRetirement; // off audio, while the gate remains closed
};
std::atomic<Barrier*> recording { nullptr };
struct Record
{
    explicit Record (Barrier& barrier) { recording = &barrier; }
    ~Record() { recording = nullptr; }
};
struct Release
{
    Barrier& barrier;
    ~Release() { barrier.released = true; }
};
struct Fixture
{
    Fixture()
    {
        directory = std::filesystem::temp_directory_path()
            / ("dandrum-engine-handoff-" + std::to_string (
                std::chrono::steady_clock::now().time_since_epoch().count()));
        std::filesystem::create_directories (directory);
        original = file ("original.yaml"); edited = file ("edited.yaml");
        auto yaml = juce::File (juce::String (DANDRUM_SOURCE_ROOT) + "/tests/fixtures/plugin-ui-knob.yaml").loadFileAsString();
        require (yaml.contains ("default: 0, min: -1"), "Handoff fixture default changed");
        yaml = yaml.replace ("default: 0, min: -1", "default: 0.25, min: -1");
        require (yaml.contains ("in: 1"), "Handoff fixture source changed");
        require (original.replaceWithText (yaml)
            && edited.replaceWithText (yaml.replace ("in: 1", "in: -2")), "Handoff fixtures could not be written");
    }
    ~Fixture() { std::filesystem::remove_all (directory); }
    juce::File file (const char* name) const
    { return juce::File (juce::String ((directory / name).string())); }
    InstrumentDemoConfiguration configuration (const juce::File& source) const
    {
        auto value = InstrumentDemoConfiguration::sampler();
        value.instrumentPath = source.getFullPathName().toStdString(); return value;
    }
    std::filesystem::path directory;
    juce::File original, edited;
};
void prepare (DandrumAudioProcessor& processor)
{
    require (processor.isInstrumentLoaded(), "Handoff fixture did not load");
    processor.setFileWatchEnabled (false);
    processor.setRateAndBufferSizeDetails (48000, 64);
    processor.prepareToPlay (48000, 64);
}
void render (DandrumAudioProcessor& processor, float expected, int frames = 64, bool measure = true)
{
    juce::AudioBuffer<float> buffer (4, frames);
    juce::MidiBuffer midi;
    for (int c = 0; c < 4; ++c)
        for (int n = 0; n < frames; ++n) buffer.setSample (c, n, 0.93f);
    measuredAudio = measure;
    processor.processBlock (buffer, midi);
    measuredAudio = false;
    for (int c = 0; c < 4; ++c)
        for (int n = 0; n < frames; ++n)
            require (std::bit_cast<std::uint32_t> (buffer.getSample (c, n))
                == std::bit_cast<std::uint32_t> (c == 0 ? expected : 0.0f),
                "Handoff changed known signed output or failed to clear every lane");
}
InstrumentUiLiveService::Packet packet (DandrumAudioProcessor& processor)
{
    std::optional<InstrumentUiLiveService::Packet> result;
    await ([&] { result = processor.takeLiveAnalysisPacket (71); return result.has_value(); },
        "Handoff did not publish a complete live window");
    return *result;
}
void assertLiveSignal (const InstrumentUiLiveService::Packet& value, float expected)
{
    for (const auto& bucket : value.analysis.channel[0].scope)
        require (std::bit_cast<std::uint32_t> (bucket.minimum) == std::bit_cast<std::uint32_t> (expected)
            && std::bit_cast<std::uint32_t> (bucket.maximum) == std::bit_cast<std::uint32_t> (expected),
            "Handoff live window mixed old and replacement signed samples");
    const auto expectedDb = 20.0 * std::log10 (std::abs (expected));
    require (std::abs (value.analysis.channel[0].magnitudeDbFS[0] - expectedDb) < 0.0002
        && std::abs (value.analysis.channel[1].magnitudeDbFS[0] + 120) < 0.0002,
        "Handoff live spectrum lost literal DC amplitude or silent right channel");
}
enum class Operation { reload, prepare, restore };
void exercise (Operation operation, bool hold, bool live = false)
{
    Fixture fixture;
    juce::MemoryBlock saved;
    if (operation == Operation::restore)
    {
        DandrumAudioProcessor donor (fixture.configuration (fixture.edited));
        prepare (donor); render (donor, -0.5f); donor.getStateInformation (saved);
    }
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor); render (processor, 0.25f);
    auto* identity = processor.getParameterForPublicId ("fixture.level");
    require (identity != nullptr, "Handoff lost public host control");
    const auto count = processor.getParameters().size();
    const auto generation = processor.getParameterSurfaceGeneration();
    std::optional<InstrumentUiLiveService::Packet> retained;
    if (live)
    {
        require (processor.subscribeLiveAnalysis (71, generation, 3), "Handoff rejected current live subscription");
        render (processor, 0.25f, 1024);
        retained = packet (processor);
        require (retained->analysis.generation == generation && retained->analysis.sampleRateHz == 48000
            && retained->analysis.startFrame == 64 && retained->analysis.endFrame == 1088,
            "Old handoff packet lost original generation/rate/coordinates");
        assertLiveSignal (*retained, 0.25f);
        require (processor.acknowledgeLiveAnalysisPacket (71, generation, retained->sequence),
            "Handoff rejected original live acknowledgement");
        // Start the held old half-window without the baseline FFT's 768-frame
        // overlap; otherwise its valid later old windows can remain pending.
        require (processor.setLiveAnalysisVisible (71, false) && processor.setLiveAnalysisVisible (71, true),
            "Handoff could not reset baseline live demand");
    }
    if (operation == Operation::prepare)
        require (fixture.original.replaceWithText (fixture.edited.loadFileAsString()), "Reprepare fixture failed");
    auto replace = [&]
    {
        if (operation == Operation::reload) return processor.reloadInstrumentFromFile (fixture.edited);
        if (operation == Operation::prepare)
        {
            const auto rate = live ? 96000.0 : 48000.0;
            processor.setRateAndBufferSizeDetails (rate, 64);
            processor.prepareToPlay (rate, 64);
        }
        else processor.setStateInformation (saved.getData(), static_cast<int> (saved.getSize()));
        return processor.isInstrumentLoaded();
    };
    if (hold)
    {
        Barrier barrier;
        std::optional<InstrumentUiLiveService::Packet> oldCompleted;
        if (live && operation == Operation::prepare)
            barrier.beforeRetirement = [&]
            {
                // Read the real old window before reprepare reopens audio.
                // Host details already say 96kHz, but these samples were
                // rendered by the held 48kHz engine.
                oldCompleted = packet (processor);
                require (oldCompleted->analysis.generation == generation
                    && oldCompleted->analysis.sampleRateHz == 48000
                    && oldCompleted->analysis.startFrame == 1088 && oldCompleted->analysis.endFrame == 2112,
                    "Held old live window was relabelled with replacement rate or coordinates");
                assertLiveSignal (*oldCompleted, 0.25f);
                require (processor.acknowledgeLiveAnalysisPacket (71, generation, oldCompleted->sequence),
                    "Handoff rejected held old live acknowledgement");
            };
        Record record (barrier);
        std::future<void> audio;
        std::future<bool> controller;
        // Release before future destructors wait, including an assertion failure.
        Release release { barrier };
        barrier.armed = true;
        const auto heldFrames = live ? (operation == Operation::prepare ? 1024 : 512) : 64;
        audio = std::async (std::launch::async, [&] { render (processor, 0.25f, heldFrames); });
        await ([&] { return barrier.entered.load(); }, "Actual callback never reached held completed render");
        controller = std::async (std::launch::async, replace);
        await ([&] { return processor.isMuted() || barrier.watchedDestroy != 0
            || controller.wait_for (0ms) == std::future_status::ready; }, "Replacement never closed audio access");
        // Keep a real reader held beyond the old five-millisecond assumption.
        // This timeout only observes a premature completion; ownership must be
        // released explicitly below, never inferred from elapsed time.
        const auto completion = controller.wait_for (50ms);
        require (barrier.prematureDestroy == 0 && completion != std::future_status::ready,
            "Replacement retired an engine while its actual callback reader was held");
        require (processor.isMuted(), "Replacement failed to mute while its reader was held");
        const auto entries = renderCalls.load();
        for (int frames : { 1, 64, 512, 2048 }) render (processor, 0, frames, true);
        require (renderCalls == entries && callbackAllocation == 0 && callbackLock == 0 && callbackLifecycle == 0,
            "Closed engine access entered an engine or performed forbidden callback work");
        barrier.released = true;
        audio.get();
        require (controller.wait_for (3s) == std::future_status::ready && controller.get(),
            "Acknowledged handoff did not finish replacement");
        require (barrier.prematureDestroy == 0 && barrier.watchedDestroy == 1,
            "Engine retirement was not exactly once after reader release");
        require (! live || operation != Operation::prepare || oldCompleted.has_value(),
            "Handoff failed to observe the held old window before reopening audio");
    }
    else require (replace(), "Quiescent engine replacement failed");
    require (! processor.isMuted() && processor.getParameterForPublicId ("fixture.level") == identity
        && processor.getParameters().size() == count, "Handoff stranded mute or changed host identity");
    if (! live) { render (processor, -0.5f); return; }

    const auto resumedGeneration = processor.getParameterSurfaceGeneration();
    if (operation == Operation::prepare)
        require (resumedGeneration == generation, "Host reprepare changed the public control generation");
    else
    {
        require (resumedGeneration == generation + 1 && ! processor.takeLiveAnalysisPacket (71)
            && ! processor.acknowledgeLiveAnalysisPacket (71, generation, retained->sequence)
            && ! processor.subscribeLiveAnalysis (71, generation, 3)
            && processor.subscribeLiveAnalysis (71, resumedGeneration, 3),
            "Replacement retained obsolete live session generation");
    }
    render (processor, -0.5f, 512);
    // Four baseline chunks, held old chunks, then two replacement chunks.
    // Wait for the sole real worker, not an elapsed guess about accumulation.
    const auto consumed = operation == Operation::prepare ? 10U : 8U;
    await ([&] { return processor.getLiveAnalysisStatistics().consumedChunks >= consumed; },
        "Handoff worker did not consume the first replacement half-window");
    require (! processor.takeLiveAnalysisPacket (71), "Handoff spliced an old partial window into new capture");
    render (processor, -0.5f, 512);
    const auto resumed = packet (processor);
    const auto rate = operation == Operation::prepare ? 96000U : 48000U;
    require (resumed.analysis.generation == resumedGeneration && resumed.analysis.sampleRateHz == rate
        && resumed.analysis.streamId != retained->analysis.streamId
        && resumed.analysis.startFrame == 0 && resumed.analysis.endFrame == 1024 && resumed.analysis.gap,
        "Replacement live window lost current generation/rate/stream/frame origin");
    require (std::bit_cast<std::uint64_t> (resumed.analysis.frequencyHz[64])
        == std::bit_cast<std::uint64_t> (rate / 16.0),
        "Replacement live frequency coordinates used the previous engine rate");
    assertLiveSignal (resumed, -0.5f);
    require (retained->analysis.generation == generation && retained->analysis.sampleRateHz == 48000
        && retained->analysis.startFrame == 64 && retained->analysis.endFrame == 1088,
        "Replacement mutated retained old live packet identity");
    assertLiveSignal (*retained, 0.25f);
    require (processor.acknowledgeLiveAnalysisPacket (71, resumedGeneration, resumed.sequence)
        && processor.unsubscribeLiveAnalysis (71), "Handoff stranded current live delivery");
    std::cout << "LIVE_HANDOFF operation=" << static_cast<int> (operation)
              << " generation=" << resumedGeneration << " rate=" << rate << " PASS\n";
}
}

void* operator new (std::size_t size)
{
    if (measuredAudio) ++callbackAllocation;
    if (auto* value = std::malloc (size ? size : 1)) return value;
    throw std::bad_alloc();
}
void* operator new[] (std::size_t size) { return ::operator new (size); }
void operator delete (void* value) noexcept { std::free (value); }
void operator delete (void* value, std::size_t) noexcept { std::free (value); }
void operator delete[] (void* value) noexcept { std::free (value); }
void operator delete[] (void* value, std::size_t) noexcept { std::free (value); }
extern "C" int __real_pthread_mutex_lock (pthread_mutex_t*);
extern "C" int __wrap_pthread_mutex_lock (pthread_mutex_t* value)
{ if (measuredAudio) ++callbackLock; return __real_pthread_mutex_lock (value); }
extern "C" std::size_t __real_dandrum_kernel_render (DandrumKernelInstrument*,
    const DandrumKernelInputBusView*, std::size_t, const DandrumKernelOutputBusView*, std::size_t, std::size_t);
extern "C" std::size_t __wrap_dandrum_kernel_render (DandrumKernelInstrument* engine,
    const DandrumKernelInputBusView* inputs, std::size_t inputCount,
    const DandrumKernelOutputBusView* outputs, std::size_t outputCount, std::size_t frames)
{
    ++renderCalls;
    const auto result = __real_dandrum_kernel_render (engine, inputs, inputCount, outputs, outputCount, frames);
    auto* barrier = recording.load();
    if (barrier && barrier->armed.exchange (false))
    {
        barrier->engine = engine; barrier->entered = true;
        while (! barrier->released.load()) std::this_thread::sleep_for (1ms);
    }
    return result;
}
extern "C" void __real_dandrum_kernel_destroy (DandrumKernelInstrument*);
extern "C" void __wrap_dandrum_kernel_destroy (DandrumKernelInstrument* engine)
{
    if (measuredAudio) ++callbackLifecycle;
    auto* barrier = recording.load();
    if (barrier && engine != nullptr && engine == barrier->engine.load())
    {
        if (! barrier->released.load())
        {
            ++barrier->prematureDestroy;
            // Broken counter/ownership mutations may also strand normal
            // unwinding. Fail at the actual unsafe FFI call, before freeing
            // borrowed storage, rather than converting it into a timeout.
            std::cerr << "Replacement requested engine destruction while its actual callback reader was held\n";
            std::_Exit (1);
        }
        if (barrier->beforeRetirement) barrier->beforeRetirement();
        ++barrier->watchedDestroy;
    }
    __real_dandrum_kernel_destroy (engine);
}
extern "C" DandrumKernelInstrument* __real_dandrum_kernel_prepare_file (
    const char*, std::uint32_t, std::size_t, const DandrumKernelBusDeclaration*, std::size_t);
extern "C" DandrumKernelInstrument* __wrap_dandrum_kernel_prepare_file (const char* path,
    std::uint32_t rate, std::size_t block, const DandrumKernelBusDeclaration* buses, std::size_t count)
{
    if (measuredAudio) ++callbackLifecycle;
    return __real_dandrum_kernel_prepare_file (path, rate, block, buses, count);
}
int main (int argc, char** argv)
{
    juce::ScopedJuceInitialiser_GUI gui;
    const bool live = argc == 2 && std::string (argv[1]) == "--live";
    const bool hold = live || (argc == 2 && std::string (argv[1]) == "--hold");
    try
    {
        for (auto operation : { Operation::reload, Operation::prepare, Operation::restore }) exercise (operation, hold, live);
        std::cout << "HANDOFF_CALLBACK cpp_new=" << callbackAllocation << " locks=" << callbackLock
                  << " engine_lifecycle=" << callbackLifecycle << '\n';
        require (callbackAllocation == 0 && callbackLock == 0 && callbackLifecycle == 0,
            "Engine handoff performed forbidden operations on an actual audio callback");
    }
    catch (const std::exception& error)
    {
        std::cerr << "HANDOFF_CALLBACK cpp_new=" << callbackAllocation << " locks=" << callbackLock
                  << " engine_lifecycle=" << callbackLifecycle << '\n' << error.what() << '\n';
        return 1;
    }
    std::cout << "ENGINE_HANDOFF " << (live ? "live identity" : hold ? "held reader" : "quiescent baseline") << " PASS\n";
}
