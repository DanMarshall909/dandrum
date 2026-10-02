#include "PluginProcessor.h"

#include <atomic>
#include <bit>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <future>
#include <iostream>
#include <stdexcept>

extern "C" bool __real_dandrum_kernel_prepared_source_copy_channel (
    const DandrumKernelWaveformSource*, std::uint16_t, std::uint64_t, float*, std::size_t);
extern "C" void __real_dandrum_kernel_waveform_source_destroy (DandrumKernelWaveformSource*);

namespace
{
using Service = InstrumentUiSpectralService;
using namespace std::chrono_literals;
thread_local bool inAudio = false;
void require (bool value, const char* message) { if (! value) throw std::runtime_error (message); }
bool sameBits (float a, float b) { return std::bit_cast<std::uint32_t> (a) == std::bit_cast<std::uint32_t> (b); }
struct Barrier
{
    Barrier() : gate (release.get_future().share()) {}
    void open() { if (! opened.exchange (true)) release.set_value(); }
    std::atomic<bool> armed { false }, opened { false };
    std::atomic<const DandrumKernelWaveformSource*> source { nullptr };
    std::atomic<unsigned> callbackOperations { 0 };
    std::promise<void> entered, release, destroyed;
    std::shared_future<void> gate;
    std::thread::id messageThread = std::this_thread::get_id();
    bool retainedRead = false, readOffMessage = false, destroyedOffMessage = false;
};
std::atomic<Barrier*> recording { nullptr };
struct Record
{
    explicit Record (Barrier& value) { recording = &value; }
    ~Record() { recording = nullptr; }
};
struct Release
{
    Barrier& barrier;
    ~Release() { barrier.open(); }
};

struct Fixture
{
    Fixture()
    {
        directory = std::filesystem::temp_directory_path()
            / ("dandrum-spectral-job-" + std::to_string (
                std::chrono::steady_clock::now().time_since_epoch().count()));
        std::filesystem::create_directories (directory);
        write (false);
    }
    ~Fixture() { std::filesystem::remove_all (directory); }
    void write (bool replacement)
    {
        std::ofstream wav (directory / "source.wav", std::ios::binary);
        auto u16 = [&] (std::uint16_t v) { wav.put (static_cast<char> (v)); wav.put (static_cast<char> (v >> 8)); };
        auto u32 = [&] (std::uint32_t v) { u16 (static_cast<std::uint16_t> (v)); u16 (static_cast<std::uint16_t> (v >> 16)); };
        wav.write ("RIFF", 4); u32 (36 + 1056 * 2); wav.write ("WAVEfmt ", 8);
        u32 (16); u16 (1); u16 (1); u32 (48000); u32 (96000); u16 (2); u16 (16);
        wav.write ("data", 4); u32 (1056 * 2);
        for (std::size_t i = 0; i < 1056; ++i)
            u16 (i < 16 || i >= 1040 ? 24576 : replacement ? 16384 : 8192);
        wav.close();
        require (wav.good(), "Spectral job WAV fixture failed");
        if (! replacement) std::filesystem::copy_file (directory / "source.wav", directory / "other.wav");
        std::ofstream yaml (directory / "instrument.yaml");
        yaml << "metadata: { name: spectral-job }\n"
                "instrument: { id: dandrum.spectral-job, preset_schema_version: 1 }\n"
                "assets:\n  sample_sources:\n    - id: retained\n      path: source.wav\n"
                "      regions:\n        - { id: full, start_frame: 16, end_frame: 1040 }\n"
                "    - id: other\n      path: other.wav\n"
                "      regions:\n        - { id: full, start_frame: 16, end_frame: 1040 }\n"
                "ports:\n  - { name: master, direction: output, signal: audio, channels: 2, maps_from: tone.out }\n"
                "modules:\n  - { id: tone, type: control_to_audio, static: { channels: 2 }, defaults: { in: "
             << (replacement ? "-0.25" : "0.125") << " } }\nconnections: []\n";
        require (yaml.good(), "Spectral job YAML fixture failed");
    }
    juce::File patch() const { return juce::File (juce::String ((directory / "instrument.yaml").string())); }
    std::filesystem::path directory;
};
void render (DandrumAudioProcessor& processor, float left)
{
    juce::AudioBuffer<float> buffer (2, 64);
    juce::MidiBuffer midi;
    inAudio = true;
    processor.processBlock (buffer, midi);
    inAudio = false;
    for (int i = 0; i < 64; ++i)
        require (sameBits (buffer.getSample (0, i), left) && sameBits (buffer.getSample (1, i), 0.0f),
                 "Analysis changed known signed audio output");
}
Service::Snapshot ready (DandrumAudioProcessor& processor, std::uint64_t job)
{
    const auto deadline = std::chrono::steady_clock::now() + 2s;
    while (std::chrono::steady_clock::now() < deadline)
    {
        auto state = processor.getPreparedSpectrumJobStatus (job);
        if (state && state->state == Service::State::ready && state->result) return *state;
        std::this_thread::sleep_for (1ms);
    }
    throw std::runtime_error ("Processor spectral job did not become ready");
}
}

extern "C" bool __wrap_dandrum_kernel_prepared_source_copy_channel (
    const DandrumKernelWaveformSource* source, std::uint16_t channel,
    std::uint64_t start, float* output, std::size_t count)
{
    auto* record = recording.load();
    if (record && inAudio) ++record->callbackOperations;
    const bool held = record && record->armed.exchange (false);
    if (held)
    {
        record->source = source;
        record->readOffMessage = std::this_thread::get_id() != record->messageThread;
        record->entered.set_value();
        record->gate.wait();
    }
    const auto copied = __real_dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
    if (held)
        record->retainedRead = copied && start == 16 && count == 1024 && sameBits (output[0], 0.25f)
            && sameBits (output[1023], 0.25f);
    return copied;
}
extern "C" void __wrap_dandrum_kernel_waveform_source_destroy (DandrumKernelWaveformSource* source)
{
    auto* record = recording.load();
    if (record && inAudio) ++record->callbackOperations;
    const bool watched = record && source == record->source.load();
    if (watched) record->source = nullptr;
    __real_dandrum_kernel_waveform_source_destroy (source);
    if (watched)
    {
        record->destroyedOffMessage = std::this_thread::get_id() != record->messageThread;
        record->destroyed.set_value();
    }
}
int main()
{
    juce::ScopedJuceInitialiser_GUI gui;
    try
    {
        Fixture fixture;
        auto missingConfiguration = InstrumentDemoConfiguration::sampler();
        missingConfiguration.instrumentPath = fixture.directory / "missing.yaml";
        {
            DandrumAudioProcessor missing (missingConfiguration);
            require (! missing.isInstrumentLoaded()
                && ! missing.requestPreparedSpectrum (missing.getParameterSurfaceGeneration(), "retained", "full", 0),
                "Processor without prepared metadata admitted spectral analysis");
        }
        Barrier barrier;
        Record record (barrier);
        std::unique_ptr<juce::AudioProcessor> original (createPluginFilter());
        auto* processor = dynamic_cast<DandrumAudioProcessor*> (original.get());
        Release release { barrier };
        require (processor != nullptr, "Original sampler factory did not return its processor");
        require (! processor->requestPreparedSpectrum (processor->getParameterSurfaceGeneration(), "retained", "full", 0),
                 "Default instrument admitted an unknown spectral source");
        processor->setRateAndBufferSizeDetails (44100, 64);
        processor->prepareToPlay (44100, 64);
        processor->setFileWatchEnabled (false);
        if (! processor->reloadInstrumentFromFile (fixture.patch()))
            throw std::runtime_error ("Spectral job fixture did not load: " + processor->getLastLoadError().toStdString());
        const auto generation = processor->getParameterSurfaceGeneration();
        require (! processor->requestPreparedSpectrum (generation - 1, "retained", "full", 0)
            && ! processor->requestPreparedSpectrum (generation, "missing", "full", 0)
            && ! processor->requestPreparedSpectrum (generation, "retained", "missing", 0)
            && ! processor->requestPreparedSpectrum (generation, "retained", "full", 1),
            "Processor accepted stale or invalid prepared spectral requests");
        barrier.armed = true;
        const auto active = processor->requestPreparedSpectrum (generation, "retained", "full", 0, 17);
        require (active && barrier.entered.get_future().wait_for (2s) == std::future_status::ready,
                 "Processor did not admit its prepared spectral worker");
        require (processor->getPreparedSpectrumJobStatus (*active)->sessionId == 17,
                 "Processor lost the spectral editor session");
        // Callback returns while the real PCM reader is held on a separate thread.
        for (int i = 0; i < 8; ++i) render (*processor, 0.125f);
        require (! barrier.opened && barrier.callbackOperations == 0,
                 "Audio waited for analysis or performed analysis/resource cleanup");
        processor->cancelPreparedSpectrumSession (17);
        require (processor->getPreparedSpectrumJobStatus (*active)->state == Service::State::cancelled
            && ! processor->cancelPreparedSpectrumJob (*active)
            && ! processor->getPreparedSpectrumJobStatus (999999), "Processor cancellation did not remain terminal");
        const auto queued = processor->requestPreparedSpectrum (generation, "retained", "full", 0, 18);
        const auto cancelled = processor->requestPreparedSpectrum (generation, "retained", "full", 0, 19);
        require (queued && cancelled && processor->cancelPreparedSpectrumJob (*cancelled),
                 "Processor could not cancel an independent queued spectral job");
        fixture.write (true);
        require (processor->reloadInstrumentFromFile (fixture.patch()), "Replacement fixture did not load");
        const auto currentGeneration = processor->getParameterSurfaceGeneration();
        require (currentGeneration != generation
            && processor->getPreparedSpectrumJobStatus (*queued)->state == Service::State::stale
            && ! processor->getPreparedSpectrumJobStatus (*queued)->result,
            "Processor reload did not retire old spectral work");
        const auto current = processor->requestPreparedSpectrum (currentGeneration, "retained", "full", 0, 20);
        require (current.has_value(), "Current prepared spectrum was rejected");
        std::filesystem::remove (fixture.directory / "source.wav");
        for (int i = 0; i < 8; ++i) render (*processor, -0.25f);
        barrier.open();
        const auto currentReady = ready (*processor, *current);
        require (barrier.destroyed.get_future().wait_for (2s) == std::future_status::ready
            && barrier.retainedRead && barrier.readOffMessage && barrier.destroyedOffMessage
            && barrier.callbackOperations == 0, "Retained source read/destruction did not stay outside callbacks/UI");
        require (currentReady.generation == currentGeneration && currentReady.result->sampleRateHz == 48000
            && currentReady.result->startFrame == 16 && currentReady.result->endFrame == 1040
            && std::abs (currentReady.result->columns[0].magnitudeDbFS[0] + 6.020599913) < 0.0001,
            "Processor published wrong current-source magnitude, rate or frame coordinates");
        require (processor->getPreparedSpectrumJobStatus (*active)->state == Service::State::cancelled
            && processor->getPreparedSpectrumJobStatus (*queued)->state == Service::State::stale,
            "Late spectral result overwrote cancelled/stale status");
        const auto other = processor->requestPreparedSpectrum (currentGeneration, "other", "full", 0, 21);
        require (other.has_value(), "Second prepared source was rejected");
        const auto otherReady = ready (*processor, *other);
        require (otherReady.result->sourceId == "other"
            && std::abs (otherReady.result->columns[0].magnitudeDbFS[0] + 12.041199826) < 0.0001,
            "Processor retained the wrong prepared source index/content");
        std::cout << "Original sampler spectral jobs: retained PCM, off-thread work, exact audio, cancellation/reload PASS\n";
    }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
    return 0;
}
