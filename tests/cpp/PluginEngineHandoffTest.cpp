#include "PluginProcessor.h"

#include <bit>
#include <chrono>
#include <cstdlib>
#include <filesystem>
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
enum class Operation { reload, prepare, restore };
void exercise (Operation operation, bool hold)
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
    if (operation == Operation::prepare)
        require (fixture.original.replaceWithText (fixture.edited.loadFileAsString()), "Reprepare fixture failed");
    auto replace = [&]
    {
        if (operation == Operation::reload) return processor.reloadInstrumentFromFile (fixture.edited);
        if (operation == Operation::prepare) processor.prepareToPlay (48000, 64);
        else processor.setStateInformation (saved.getData(), static_cast<int> (saved.getSize()));
        return processor.isInstrumentLoaded();
    };
    if (hold)
    {
        Barrier barrier;
        Record record (barrier);
        std::future<void> audio;
        std::future<bool> controller;
        // Release before future destructors wait, including an assertion failure.
        Release release { barrier };
        barrier.armed = true;
        audio = std::async (std::launch::async, [&] { render (processor, 0.25f); });
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
    }
    else require (replace(), "Quiescent engine replacement failed");
    require (! processor.isMuted() && processor.getParameterForPublicId ("fixture.level") == identity
        && processor.getParameters().size() == count, "Handoff stranded mute or changed host identity");
    render (processor, -0.5f);
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
    const bool hold = argc == 2 && std::string (argv[1]) == "--hold";
    try
    {
        for (auto operation : { Operation::reload, Operation::prepare, Operation::restore }) exercise (operation, hold);
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
    std::cout << "ENGINE_HANDOFF " << (hold ? "held reader" : "quiescent baseline") << " PASS\n";
}
