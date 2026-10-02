#include "PluginProcessor.h"

#include <array>
#include <atomic>
#include <bit>
#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <future>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <thread>

extern "C" bool __real_dandrum_kernel_waveform_reduce (
    const DandrumKernelWaveformSource*, std::uint16_t, std::uint64_t, std::uint64_t,
    DandrumKernelWaveformBucket*, std::size_t);
extern "C" void __real_dandrum_kernel_waveform_source_destroy (DandrumKernelWaveformSource*);
extern "C" void __real_dandrum_kernel_destroy (DandrumKernelInstrument*);

namespace
{
using Service = InstrumentUiWaveformService;
using namespace std::chrono_literals;
thread_local bool inAudioCallback = false;

void require (bool condition, const char* message)
{
    if (! condition)
        throw std::runtime_error (message);
}

struct ReductionBarrier
{
    ReductionBarrier() : resume (release.get_future().share()) {}
    void open()
    {
        if (! opened.exchange (true))
            release.set_value();
    }
    std::atomic<bool> armed { false }, claimed { false }, opened { false };
    std::atomic<const DandrumKernelWaveformSource*> watchedSource { nullptr };
    std::atomic<unsigned> audioResourceOperations { 0 }, engineDestructions { 0 };
    std::promise<void> entered, release, sourceDestroyed;
    std::shared_future<void> resume;
    bool retainedReadCorrect = false;
    bool reducedOffMessageThread = false;
    bool sourceDestroyedOffMessageThread = false;
    std::thread::id messageThread = std::this_thread::get_id();
};

std::atomic<ReductionBarrier*> recording { nullptr };

struct RecordingScope
{
    explicit RecordingScope (ReductionBarrier& barrier) { recording = &barrier; }
    ~RecordingScope() { recording = nullptr; }
};

// Declared after the processor: errors release the barrier before its worker
// is joined, while the recorder remains alive until after processor teardown.
struct ReleaseOnExit
{
    explicit ReleaseOnExit (ReductionBarrier& value) : barrier (value) {}
    ~ReleaseOnExit() { barrier.open(); }
    ReductionBarrier& barrier;
};

constexpr std::array<float, 4> oldSamples { -0.75f, 0.5f, 0.25f, -0.125f };
constexpr std::array<float, 4> newSamples { 0.125f, -0.5f, 0.0f, 0.75f };

bool sameBits (float actual, float expected)
{
    return std::bit_cast<std::uint32_t> (actual) == std::bit_cast<std::uint32_t> (expected);
}

bool exactBuckets (const DandrumKernelWaveformBucket* buckets, std::size_t count,
                   const std::array<float, 4>& samples)
{
    if (count != samples.size())
        return false;
    for (std::size_t i = 0; i < count; ++i)
        if (buckets[i].startFrame != i || buckets[i].endFrame != i + 1
            || ! sameBits (buckets[i].minimum, samples[i])
            || ! sameBits (buckets[i].maximum, samples[i]))
            return false;
    return true;
}

struct Fixture
{
    Fixture()
    {
        directory = std::filesystem::temp_directory_path()
            / ("dandrum-native-waveform-" + std::to_string (
                std::chrono::steady_clock::now().time_since_epoch().count()));
        std::filesystem::create_directories (directory);
        write (false);
    }
    ~Fixture() { std::filesystem::remove_all (directory); }

    void write (bool replacement) const
    {
        std::ofstream wav (directory / "source.wav", std::ios::binary | std::ios::trunc);
        const auto u16 = [&wav] (std::uint16_t value)
        {
            wav.put (static_cast<char> (value & 255));
            wav.put (static_cast<char> (value >> 8));
        };
        const auto u32 = [&u16] (std::uint32_t value)
        {
            u16 (static_cast<std::uint16_t> (value));
            u16 (static_cast<std::uint16_t> (value >> 16));
        };
        wav.write ("RIFF", 4); u32 (44); wav.write ("WAVEfmt ", 8); u32 (16);
        u16 (1); u16 (1); u32 (48000); u32 (96000); u16 (2); u16 (16);
        wav.write ("data", 4); u32 (8);
        for (const auto sample : replacement ? newSamples : oldSamples)
            u16 (static_cast<std::uint16_t> (static_cast<std::int16_t> (sample * 32768.0f)));
        require (wav.good(), "could not write native waveform PCM fixture");
        std::ofstream yaml (directory / "instrument.yaml");
        yaml << "metadata: { name: Native Waveform Lifetime }\n"
                "instrument: { id: dandrum.waveform-lifetime, preset_schema_version: 1 }\n"
                "assets:\n"
                "  sample_sources:\n"
                "    - id: retained\n"
                "      path: source.wav\n"
                "      regions:\n"
                "        - { id: full, start_frame: 0, end_frame: 4 }\n"
                "ports:\n"
                "  - { name: master, direction: output, signal: audio, channels: 2, maps_from: tone.out }\n"
                "modules:\n"
                "  - { id: tone, type: control_to_audio, static: { channels: 2 }, defaults: { in: "
             << (replacement ? "-0.5" : "0.25") << " } }\nconnections: []\n";
        require (yaml.good(), "could not write native waveform patch fixture");
    }
    juce::File patch() const { return juce::File (juce::String ((directory / "instrument.yaml").string())); }
    std::filesystem::path directory;
};

juce::Component& panel (juce::AudioProcessorEditor& editor)
{
    auto* component = editor.findChildWithID ("prepared-waveform");
    require (component != nullptr, "original native editor has no waveform panel");
    return *component;
}

Service::Snapshot findRunningJob (DandrumAudioProcessor& processor,
                                  std::uint32_t generation, std::uint64_t after = 0)
{
    const auto deadline = std::chrono::steady_clock::now() + 2s;
    while (std::chrono::steady_clock::now() < deadline)
    {
        juce::Timer::callPendingTimersSynchronously();
        for (std::uint64_t id = after + 1; id <= Service::maxHistory; ++id)
            if (const auto status = processor.getPreparedWaveformJobStatus (id))
                if (status->generation == generation && status->state == Service::State::running)
                    return *status;
        std::this_thread::sleep_for (1ms);
    }
    throw std::runtime_error ("original native editor did not admit an owned waveform job");
}

void waitForDisplay (juce::AudioProcessorEditor& editor)
{
    const auto deadline = std::chrono::steady_clock::now() + 2s;
    while (panel (editor).getName() != "Prepared waveform: retained.full"
           && std::chrono::steady_clock::now() < deadline)
    {
        juce::Timer::callPendingTimersSynchronously();
        std::this_thread::sleep_for (1ms);
    }
    require (panel (editor).getName() == "Prepared waveform: retained.full",
             "surviving native editor did not display current analysis");
}

Service::Snapshot waitReady (DandrumAudioProcessor& processor, std::uint64_t id)
{
    const auto deadline = std::chrono::steady_clock::now() + 2s;
    while (std::chrono::steady_clock::now() < deadline)
    {
        if (const auto status = processor.getPreparedWaveformJobStatus (id))
            if (status->state == Service::State::ready)
                return *status;
        std::this_thread::sleep_for (1ms);
    }
    throw std::runtime_error ("current native waveform job did not finish");
}

void renderWhileStalled (DandrumAudioProcessor& processor, ReductionBarrier& barrier, float expected)
{
    juce::AudioBuffer<float> buffer (2, 64);
    for (int channel = 0; channel < 2; ++channel)
        std::fill_n (buffer.getWritePointer (channel), 64, 0.777f);
    juce::MidiBuffer midi;
    std::promise<void> returned;
    auto finished = returned.get_future();
    std::jthread audio ([&]
    {
        inAudioCallback = true;
        processor.processBlock (buffer, midi);
        inAudioCallback = false;
        returned.set_value();
    });
    const auto independent = finished.wait_for (2s) == std::future_status::ready;
    // Permit safe failure cleanup if a regression makes processing wait.
    if (! independent)
        barrier.open();
    audio.join();
    require (independent && ! barrier.opened,
             "audio waited for stalled waveform analysis or worker shutdown");
    for (int channel = 0; channel < 2; ++channel)
        for (int frame = 0; frame < 64; ++frame)
            if (! sameBits (buffer.getSample (channel, frame), channel == 0 ? expected : 0.0f))
                throw std::runtime_error ("stalled native waveform analysis changed known signed PCM at channel "
                    + std::to_string (channel) + " frame " + std::to_string (frame));
    require (barrier.audioResourceOperations == 0,
             "audio performed waveform reduction, source cleanup or engine cleanup");
}

void runScenario (bool reload)
{
    Fixture fixture;
    ReductionBarrier barrier;
    RecordingScope recordingScope (barrier);
    std::unique_ptr<juce::AudioProcessor> plugin (createPluginFilter());
    auto* processor = dynamic_cast<DandrumAudioProcessor*> (plugin.get());
    require (processor != nullptr, "sampler factory returned the wrong processor");
    ReleaseOnExit releaseGuard (barrier);
    processor->setPlayConfigDetails (0, 2, 48000.0, 64);
    processor->prepareToPlay (48000.0, 64);
    require (processor->reloadInstrumentFromFile (fixture.patch()), "native waveform fixture did not load");
    const auto generation = processor->getParameterSurfaceGeneration();
    barrier.armed = true;
    std::unique_ptr<juce::AudioProcessorEditor> editor (processor->createEditor());
    require (editor != nullptr && panel (*editor).getName() == "PREPARING WAVEFORM",
             "native editor failed to display pending analysis");
    require (barrier.entered.get_future().wait_for (2s) == std::future_status::ready,
             "native waveform request did not reach the real reduction barrier");
    const auto oldJob = findRunningJob (*processor, generation);
    require (oldJob.sessionId != 0, "native waveform job lost its editor session");
    renderWhileStalled (*processor, barrier, 0.25f);

    std::unique_ptr<juce::AudioProcessorEditor> other;
    std::uint64_t currentJobId = 0;
    if (reload)
    {
        const auto before = barrier.engineDestructions.load();
        fixture.write (true);
        require (processor->reloadInstrumentFromFile (fixture.patch()), "replacement native waveform fixture did not load");
        require (barrier.engineDestructions > before,
                 "reload did not retire the old engine while analysis retained its source");
        juce::Timer::callPendingTimersSynchronously();
        require (panel (*editor).getName() == "PREPARING WAVEFORM",
                 "native reload presented previous analysis as current data");
        const auto current = findRunningJob (*processor, processor->getParameterSurfaceGeneration(), oldJob.jobId);
        require (current.sessionId == oldJob.sessionId && current.generation != generation,
                 "native reload lost editor ownership or instrument generation");
        currentJobId = current.jobId;
        require (processor->getPreparedWaveformJobStatus (oldJob.jobId)->state == Service::State::stale,
                 "native reload left the previous waveform job current");
        renderWhileStalled (*processor, barrier, -0.5f);
    }
    else
    {
        other.reset (processor->createEditor());
        require (other != nullptr, "second native editor did not open");
        const auto independent = findRunningJob (*processor, generation, oldJob.jobId);
        require (independent.sessionId != 0 && independent.sessionId != oldJob.sessionId,
                 "native editors shared a waveform owner");
        currentJobId = independent.jobId;
        editor.reset();
        require (! barrier.opened
                     && processor->getPreparedWaveformJobStatus (oldJob.jobId)->state == Service::State::cancelled
                     && processor->getPreparedWaveformJobStatus (currentJobId)->state == Service::State::running,
                 "native editor teardown failed to cancel only its active analysis");
        renderWhileStalled (*processor, barrier, 0.25f);
    }

    auto sourceDestroyed = barrier.sourceDestroyed.get_future();
    barrier.open();
    require (sourceDestroyed.wait_for (2s) == std::future_status::ready
                 && barrier.retainedReadCorrect && barrier.reducedOffMessageThread
                 && barrier.sourceDestroyedOffMessageThread,
             "retained native waveform source failed its signed read or off-thread cleanup");
    const auto ready = waitReady (*processor, currentJobId);
    require (ready.result != nullptr && ready.result->sourceId == "retained"
                 && ready.result->regionId == "full" && ready.result->sampleRateHz == 48000
                 && exactBuckets (ready.result->buckets.data(), ready.result->buckets.size(), reload ? newSamples : oldSamples),
             "current native waveform lost independent signed samples or source coordinates");
    const auto terminal = processor->getPreparedWaveformJobStatus (oldJob.jobId);
    require (terminal && terminal->state == (reload ? Service::State::stale : Service::State::cancelled)
                 && terminal->result == nullptr,
             "late native waveform completion overwrote cancelled or stale terminal data");
    waitForDisplay (reload ? *editor : *other);
    if (! reload)
    {
        editor.reset (processor->createEditor());
        require (editor != nullptr && panel (*editor).getName() == "Prepared waveform: retained.full",
                 "reopened native editor could not reuse current waveform data");
    }
    require (barrier.audioResourceOperations == 0, "waveform lifecycle reclaimed a resource on audio");
    std::cout << (reload ? "reload" : "close")
              << ": oldJob=" << oldJob.jobId << " owner=" << oldJob.sessionId
              << " currentJob=" << currentJobId << " generation=" << ready.generation
              << " firstSample=" << ready.result->buckets[0].minimum
              << " audioResourceOperations=" << barrier.audioResourceOperations
              << "; owned cancellation, retained signed PCM, independent callback and current display PASS\n";
}
}

extern "C" bool __wrap_dandrum_kernel_waveform_reduce (
    const DandrumKernelWaveformSource* source, std::uint16_t channel,
    std::uint64_t start, std::uint64_t end, DandrumKernelWaveformBucket* buckets, std::size_t count)
{
    auto* barrier = recording.load();
    if (barrier && inAudioCallback)
        ++barrier->audioResourceOperations;
    const auto held = barrier && barrier->armed && ! barrier->claimed.exchange (true);
    if (held)
    {
        barrier->watchedSource = source;
        barrier->reducedOffMessageThread = std::this_thread::get_id() != barrier->messageThread;
        barrier->entered.set_value();
        barrier->resume.wait();
    }
    const auto success = __real_dandrum_kernel_waveform_reduce (source, channel, start, end, buckets, count);
    if (held)
        barrier->retainedReadCorrect = success && channel == 0 && start == 0 && end == 4
            && exactBuckets (buckets, count, oldSamples);
    return success;
}

extern "C" void __wrap_dandrum_kernel_waveform_source_destroy (DandrumKernelWaveformSource* source)
{
    auto* barrier = recording.load();
    if (barrier && inAudioCallback)
        ++barrier->audioResourceOperations;
    __real_dandrum_kernel_waveform_source_destroy (source);
    const DandrumKernelWaveformSource* expected = source;
    if (barrier && source != nullptr && barrier->watchedSource.compare_exchange_strong (expected, nullptr))
    {
        barrier->sourceDestroyedOffMessageThread = std::this_thread::get_id() != barrier->messageThread;
        barrier->sourceDestroyed.set_value();
    }
}

extern "C" void __wrap_dandrum_kernel_destroy (DandrumKernelInstrument* engine)
{
    if (auto* barrier = recording.load())
    {
        ++barrier->engineDestructions;
        if (inAudioCallback)
            ++barrier->audioResourceOperations;
    }
    __real_dandrum_kernel_destroy (engine);
}

int main()
{
    if (std::getenv ("DISPLAY") == nullptr)
    {
        std::cerr << "native waveform lifetime check requires a display\n";
        return 77;
    }
    juce::ScopedJuceInitialiser_GUI gui;
    try
    {
        runScenario (false);
        runScenario (true);
    }
    catch (const std::exception& error)
    {
        std::cerr << error.what() << '\n';
        return 1;
    }
    return 0;
}
