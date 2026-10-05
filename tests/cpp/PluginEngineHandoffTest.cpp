#include "PluginProcessor.h"

#include <bit>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <functional>
#include <future>
#include <iostream>
#include <limits>
#include <new>
#include <stdexcept>
#include <thread>
#include <utility>
#include <pthread.h>

struct PluginConstructionTestProbe
{
    static juce::ValueTree savedState (DandrumAudioProcessor& processor)
    {
        juce::MemoryBlock bytes;
        processor.getStateInformation (bytes);
        const auto xml = DandrumAudioProcessor::getXmlFromBinary (bytes.getData(), static_cast<int> (bytes.getSize()));
        if (!xml) throw std::runtime_error ("Reload recovery state was not valid XML");
        return juce::ValueTree::fromXml (*xml);
    }
    static float rawValue (DandrumAudioProcessor& processor, const juce::String& id = "dandrum.slot.00")
    { return processor.parameters.getRawParameterValue (id)->load(); }
};

namespace
{
using namespace std::chrono_literals;
thread_local bool measuredAudio = false;
std::atomic<unsigned> callbackAllocation { 0 }, callbackLock { 0 }, callbackLifecycle { 0 };
std::atomic<unsigned> renderCalls { 0 };
std::atomic<bool> failThreadStartup { false };
std::atomic<bool> failPreparation { false };
thread_local bool closingProcessor = false;
thread_local bool insideActivationNotification = false;
std::atomic<bool> shutdownJoinObserved { false };
std::atomic<DandrumAudioProcessor*> shuttingDownProcessor { nullptr };
void require (bool condition, const char* message)
{ if (! condition) throw std::runtime_error (message); }
bool sameFloat (float a, float b)
{ return std::bit_cast<std::uint32_t> (a) == std::bit_cast<std::uint32_t> (b); }
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
struct PreparationBarrier
{
    std::atomic<bool> entered { false }, released { false }, beganMuted { false };
    DandrumAudioProcessor& processor;
    std::atomic<bool> armed { true };
};
std::atomic<PreparationBarrier*> preparationBarrier { nullptr };
struct HoldPreparation
{
    PreparationBarrier& barrier;
    explicit HoldPreparation (PreparationBarrier& value) : barrier (value) { preparationBarrier = &value; }
    ~HoldPreparation()
    {
        barrier.released = true;
        preparationBarrier = nullptr;
    }
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
void render (DandrumAudioProcessor& processor, float expected, int frames = 64, bool measure = true,
             float expectedRight = 0.0f)
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
        {
            if (std::bit_cast<std::uint32_t> (buffer.getSample (c, n))
                != std::bit_cast<std::uint32_t> (c == 0 ? expected : c == 1 ? expectedRight : 0.0f))
                std::cerr << "Signed render channel=" << c << " frame=" << n
                          << " actual=" << buffer.getSample (c, n)
                          << " expected=" << (c == 0 ? expected : c == 1 ? expectedRight : 0.0f) << '\n';
            require (std::bit_cast<std::uint32_t> (buffer.getSample (c, n))
                == std::bit_cast<std::uint32_t> (c == 0 ? expected : c == 1 ? expectedRight : 0.0f),
                "Handoff changed known signed output or failed to clear every lane");
        }
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
void jobBaseline()
{
    Fixture fixture;
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor); render (processor, 0.25f);
    const auto generation = processor.getParameterSurfaceGeneration();
    PreparationBarrier barrier { {}, {}, {}, processor };
    HoldPreparation hold (barrier);
    const auto id = processor.requestInstrumentReloadJob (fixture.edited, generation);
    require (id.has_value(), "Baseline reload job was rejected");
    await ([&] { return barrier.entered.load(); }, "Baseline preparation barrier was not entered");
    const auto running = processor.getInstrumentUiJobStatus (*id);
    require (running && running->state == DandrumAudioProcessor::UiJobState::running
        && !processor.requestInstrumentReloadJob (fixture.original, generation), "Baseline allowed overlapping jobs");
    barrier.released = true;
    std::optional<DandrumAudioProcessor::UiJobStatus> status;
    await ([&] {
        status = processor.getInstrumentUiJobStatus (*id);
        return status && status->state != DandrumAudioProcessor::UiJobState::running;
    }, "Baseline reload job never completed");
    require (status->state == DandrumAudioProcessor::UiJobState::completed,
             "Baseline reload job failed");
    render (processor, -0.5f);
    preparationBarrier = nullptr;
    const auto workingGeneration = processor.getParameterSurfaceGeneration();
    failThreadStartup = true;
    const auto startup = processor.requestInstrumentReloadJob (fixture.original, workingGeneration);
    const auto startupStatus = startup ? processor.getInstrumentUiJobStatus (*startup) : std::nullopt;
    require (!failThreadStartup && startupStatus
        && startupStatus->state == DandrumAudioProcessor::UiJobState::failed && startupStatus->error.isNotEmpty(),
        "Baseline thread-start failure lost queryable terminal state");
    render (processor, -0.5f);
    const auto missing = processor.requestInstrumentReloadJob (fixture.file ("missing.yaml"), workingGeneration);
    require (missing.has_value(), "Baseline missing-file job was rejected");
    await ([&] {
        status = processor.getInstrumentUiJobStatus (*missing);
        return status && status->state != DandrumAudioProcessor::UiJobState::running;
    }, "Baseline failed reload job never completed");
    require (status->state == DandrumAudioProcessor::UiJobState::failed && status->error.isNotEmpty()
        && processor.getParameterSurfaceGeneration() == workingGeneration, "Baseline failure changed working state");
    render (processor, -0.5f);
    const auto invalid = fixture.file ("invalid-baseline.yaml");
    require (invalid.replaceWithText ("modules: [ broken yaml"), "Could not write baseline invalid fixture");
    const auto rejected = processor.requestInstrumentReloadJob (invalid, workingGeneration);
    require (rejected.has_value(), "Baseline invalid job was rejected at admission");
    await ([&] {
        status = processor.getInstrumentUiJobStatus (*rejected);
        return status && status->state != DandrumAudioProcessor::UiJobState::running;
    }, "Baseline invalid job never completed");
    require (status->state == DandrumAudioProcessor::UiJobState::failed && status->error.isNotEmpty(),
             "Baseline invalid configuration did not report failure");
    failPreparation = true;
    const auto exceptional = processor.requestInstrumentReloadJob (fixture.original, workingGeneration);
    require (exceptional.has_value(), "Baseline throwing worker was rejected at admission");
    await ([&] {
        status = processor.getInstrumentUiJobStatus (*exceptional);
        return status && status->state != DandrumAudioProcessor::UiJobState::running;
    }, "Baseline worker exception never completed");
    require (!failPreparation && status->state == DandrumAudioProcessor::UiJobState::failed && status->error.isNotEmpty(),
             "Baseline worker exception lost terminal failure");
    PreparationBarrier staleBarrier { {}, {}, {}, processor };
    HoldPreparation staleHold (staleBarrier);
    const auto stale = processor.requestInstrumentReloadJob (fixture.original, workingGeneration);
    require (stale.has_value(), "Baseline stale candidate was rejected");
    await ([&] { return staleBarrier.entered.load(); }, "Baseline stale worker did not enter preparation");
    require (processor.reloadInstrumentFromFile (fixture.edited), "Baseline newer replacement failed");
    staleBarrier.released = true;
    await ([&] {
        status = processor.getInstrumentUiJobStatus (*stale);
        return status && status->state != DandrumAudioProcessor::UiJobState::running;
    }, "Baseline stale worker never completed");
    require (status->state == DandrumAudioProcessor::UiJobState::stale,
             "Baseline stale candidate replaced a newer engine");
    preparationBarrier = nullptr;
    for (int job = 0; job < 17; ++job)
    {
        const auto next = processor.requestInstrumentReloadJob (fixture.edited, processor.getParameterSurfaceGeneration());
        require (next.has_value(), "Baseline history job was rejected");
        await ([&] {
            status = processor.getInstrumentUiJobStatus (*next);
            return status && status->state != DandrumAudioProcessor::UiJobState::running;
        }, "Baseline history job never completed");
        require (status->state == DandrumAudioProcessor::UiJobState::completed, "Baseline history job failed");
    }
    require (!processor.getInstrumentUiJobStatus (*id) && !processor.getInstrumentUiJobStatus (999999),
             "Baseline terminal history was unbounded or invented an unknown job");
    render (processor, -0.5f);
    std::cout << "RELOAD_JOB_BASELINE signed_success=-0.5 signed_recovery=-0.5 PASS\n";
}
void automaticJob()
{
    Fixture fixture;
    DandrumAudioProcessor* owner = nullptr;
    std::atomic<bool> preflightMuted { false };
    DandrumAudioProcessor processor (fixture.configuration (fixture.original), {}, [&]
        { preflightMuted = owner->isMuted(); });
    owner = &processor;
    DandrumAudioProcessor other (fixture.configuration (fixture.original));
    prepare (processor); prepare (other); render (processor, 0.25f);
    auto* parameter = processor.getParameterForPublicId ("fixture.level");
    const auto count = processor.getParameters().size();
    const auto generation = processor.getParameterSurfaceGeneration();
    const auto document = processor.getPreparedUiDocument();
    require (!processor.requestInstrumentReloadJob (fixture.edited, generation - 1)
        && !processor.isMuted(), "Stale reload admission changed working audio");
    render (processor, 0.25f);
    PreparationBarrier barrier { {}, {}, {}, processor };
    // This guard releases the real validator before processor destruction on failure.
    HoldPreparation hold (barrier);
    const auto id = processor.requestInstrumentReloadJob (fixture.edited, generation);
    require (id.has_value(), "Automatic reload did not return asynchronous job acceptance");
    require (processor.isMuted(), "Reload admission did not immediately mute before validation");
    await ([&] { return barrier.entered.load(); }, "Reload did not enter actual held preparation");
    require (preflightMuted && barrier.beganMuted, "Actual configuration validation began before admission mute");
    const auto status = processor.getInstrumentUiJobStatus (*id);
    require (status && status->state == DandrumAudioProcessor::UiJobState::running
        && !processor.requestInstrumentReloadJob (fixture.original, generation),
        "Pending reload did not expose running state or reject a concurrent job");
    const auto entries = renderCalls.load();
    for (int frames : { 1, 64, 512, 2048 }) render (processor, 0, frames);
    require (renderCalls == entries && callbackAllocation == 0 && callbackLock == 0 && callbackLifecycle == 0,
        "Stalled preparation entered an engine or performed forbidden callback work");
    render (other, 0.25f);
    // Values stay on the ordinary host path while preparation is stalled.
    parameter->setValueNotifyingHost (0.25f);
    parameter->setValueNotifyingHost (0.75f);
    barrier.released = true;
    // No callback, editor, job-status query or message loop drives completion.
    await ([&] { return !processor.isMuted(); }, "Reload required an editor/status poll to resume");
    const auto completed = processor.getInstrumentUiJobStatus (*id);
    require (completed && completed->state == DandrumAudioProcessor::UiJobState::completed
        && processor.getParameterSurfaceGeneration() == generation + 1
        && processor.getParameterForPublicId ("fixture.level") == parameter
        && processor.getParameters().size() == count, "Automatic activation lost generation or host identity");
    render (processor, -1.0f);
    require (document.has_value(), "Automatic reload baseline had no retained document");
    preparationBarrier = nullptr;

    const auto workingGeneration = processor.getParameterSurfaceGeneration();
    const auto invalid = fixture.file ("invalid.yaml");
    require (invalid.replaceWithText ("modules: [ broken yaml"), "Could not write invalid reload fixture");
    PreparationBarrier failure { {}, {}, {}, processor };
    HoldPreparation failedHold (failure);
    const auto failedId = processor.requestInstrumentReloadJob (invalid, workingGeneration);
    require (failedId && processor.isMuted(), "Failed-candidate admission did not immediately mute");
    await ([&] { return failure.entered.load(); }, "Invalid reload did not enter real validation");
    require (failure.beganMuted, "Failed validation began before mute");
    render (processor, 0, 512);
    parameter->setValueNotifyingHost (0.25f);
    failure.released = true;
    await ([&] { return !processor.isMuted(); }, "Failed reload did not automatically recover");
    const auto failed = processor.getInstrumentUiJobStatus (*failedId);
    require (failed && failed->state == DandrumAudioProcessor::UiJobState::failed && failed->error.isNotEmpty()
        && processor.getParameterSurfaceGeneration() == workingGeneration
        && processor.getParameterForPublicId ("fixture.level") == parameter,
        "Failed reload changed working generation or lost queryable error");
    // A negative live gain gives a literal negative IEEE zero on the silent
    // right channel. Muted callbacks above still require positive clear-zero.
    render (processor, 1.0f, 64, true, -0.0f);
    render (other, 0.25f);
    preparationBarrier = nullptr;
    failThreadStartup = true;
    const auto startup = processor.requestInstrumentReloadJob (fixture.edited, workingGeneration);
    const auto startupStatus = startup ? processor.getInstrumentUiJobStatus (*startup) : std::nullopt;
    require (!failThreadStartup && !processor.isMuted() && startupStatus
        && startupStatus->state == DandrumAudioProcessor::UiJobState::failed && startupStatus->error.isNotEmpty()
        && processor.getParameterSurfaceGeneration() == workingGeneration,
        "Worker startup failure stranded mute or lost working state");
    render (processor, 1.0f, 64, true, -0.0f);
    std::cout << "AUTOMATIC_RELOAD admission_mute=PASS silent_blocks=1/64/512/2048"
              << " signed_activation=-1 signed_recovery=1 no_editor_poll=PASS\n";
}
void startupFailureWithReader()
{
    Fixture fixture;
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor);
    Barrier reader;
    Record record (reader);
    std::future<void> audio;
    Release release { reader };
    reader.armed = true;
    audio = std::async (std::launch::async, [&] { render (processor, 0.25f); });
    await ([&] { return reader.entered.load(); }, "Startup failure reader did not enter actual render");
    failThreadStartup = true;
    const auto id = processor.requestInstrumentReloadJob (fixture.edited, processor.getParameterSurfaceGeneration());
    const auto status = id ? processor.getInstrumentUiJobStatus (*id) : std::nullopt;
    require (status && status->state == DandrumAudioProcessor::UiJobState::failed && !processor.isMuted(),
             "Held-reader startup failure did not automatically recover");
    reader.released = true;
    audio.get();
    try { render (processor, 0.25f); }
    catch (const std::exception& error)
    {
        // A lost reader count also strands destructor quiescence. Fail at the
        // observed wrong resumed PCM, rather than turning that defect into a timeout.
        std::cerr << "Startup failure lost an existing callback reader acknowledgement: " << error.what() << '\n';
        std::_Exit (1);
    }
    require (reader.watchedDestroy == 0, "Failed startup retired its last working engine");
    std::cout << "HELD_STARTUP_FAILURE signed_resumption=0.25 PASS\n";
}
void automaticJobWithReader()
{
    Fixture fixture;
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor);
    Barrier reader;
    Record record (reader);
    PreparationBarrier preparation { {}, {}, {}, processor };
    std::future<void> audio;
    std::future<std::optional<std::uint64_t>> admission;
    HoldPreparation hold (preparation);
    Release release { reader };
    reader.armed = true;
    audio = std::async (std::launch::async, [&] { render (processor, 0.25f); });
    await ([&] { return reader.entered.load(); }, "Automatic job reader did not enter actual render");
    const auto generation = processor.getParameterSurfaceGeneration();
    admission = std::async (std::launch::async, [&]
        { return processor.requestInstrumentReloadJob (fixture.edited, generation); });
    require (admission.wait_for (3s) == std::future_status::ready,
             "Reload admission waited for an existing audio reader");
    const auto id = admission.get();
    require (id && processor.isMuted(), "Held-reader job did not accept and mute asynchronously");
    // Keep the actual reader owned beyond the old retirement delay. Neither
    // this observation interval nor a future callback acknowledges ownership.
    require (audio.wait_for (50ms) != std::future_status::ready
        && !preparation.entered && reader.watchedDestroy == 0,
        "Reload validation or retirement began before its existing reader acknowledged release");
    const auto entries = renderCalls.load();
    render (processor, 0, 2048);
    require (renderCalls == entries, "Later callback entered the old engine during automatic quiescence");
    reader.released = true;
    audio.get();
    await ([&] { return preparation.entered.load(); }, "Acknowledged job never entered preparation");
    preparation.released = true;
    await ([&] { return !processor.isMuted(); }, "Acknowledged job did not autonomously resume");
    const auto status = processor.getInstrumentUiJobStatus (*id);
    require (status && status->state == DandrumAudioProcessor::UiJobState::completed
        && reader.watchedDestroy == 1 && reader.prematureDestroy == 0,
        "Automatic job did not retire its engine exactly once after reader release");
    render (processor, -0.5f);
    std::cout << "AUTOMATIC_READER validation_after_ack=PASS signed_resumption=-0.5 PASS\n";
}
void obsoleteHostPreparation()
{
    Fixture fixture;
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor);
    const auto generation = processor.getParameterSurfaceGeneration();
    PreparationBarrier preparation { {}, {}, {}, processor };
    HoldPreparation hold (preparation);
    const auto id = processor.requestInstrumentReloadJob (fixture.edited, generation);
    require (id.has_value(), "Obsolete-host job was rejected");
    await ([&] { return preparation.entered.load(); }, "Obsolete-host worker did not enter preparation");
    processor.setRateAndBufferSizeDetails (96000, 128);
    processor.prepareToPlay (96000, 128);
    require (processor.isMuted(), "Host reprepare reopened an outer job's muted gate");
    render (processor, 0, 128);
    preparation.released = true;
    await ([&] { return !processor.isMuted(); }, "Obsolete candidate did not automatically recover");
    const auto status = processor.getInstrumentUiJobStatus (*id);
    require (status && status->state == DandrumAudioProcessor::UiJobState::stale && status->error.isNotEmpty()
        && processor.getParameterSurfaceGeneration() == generation,
        "Obsolete host settings activated a stale candidate or changed working generation");
    render (processor, 0.25f, 128);
    std::cout << "OBSOLETE_RELOAD host_rate=96000 block=128 signed_recovery=0.25 PASS\n";
}
enum class ActivationException { standard, integer, custom };
void activationFailure (ActivationException exceptionKind = ActivationException::standard)
{
    Fixture fixture;
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor);
    render (processor, 0.25f);
    auto* parameter = processor.getParameterForPublicId ("fixture.level");
    const auto generation = processor.getParameterSurfaceGeneration();
    struct ThrowOnce final : juce::AudioProcessorParameter::Listener
    {
        juce::AudioProcessorParameter& parameter;
        ActivationException exceptionKind;
        std::atomic<bool> armed { true };
        ThrowOnce (juce::AudioProcessorParameter& value, ActivationException kind) : parameter (value), exceptionKind (kind)
        { parameter.addListener (this); }
        ~ThrowOnce() override { parameter.removeListener (this); }
        void parameterValueChanged (int, float) override
        {
            if (!armed.exchange (false)) return;
            if (exceptionKind == ActivationException::integer) throw 42;
            struct UnexpectedActivationException {};
            if (exceptionKind == ActivationException::custom) throw UnexpectedActivationException {};
            throw std::runtime_error ("Injected activation notification failure");
        }
        void parameterGestureChanged (int, bool) override {}
    } listener (*parameter, exceptionKind);
    const auto id = processor.requestInstrumentReloadJob (fixture.edited, generation);
    require (id.has_value(), "Activation-failure job was rejected");
    try { await ([&] { return !processor.isMuted(); }, "Activation failure stranded mute"); }
    catch (const std::exception&)
    {
        // A broken worker may retain a non-standard exception in its future.
        // Report the observable failure before noexcept teardown can terminate.
        std::cerr << "Activation exception stranded plugin mute\n";
        std::_Exit (1);
    }
    const auto status = processor.getInstrumentUiJobStatus (*id);
    require (!listener.armed && status && status->state == DandrumAudioProcessor::UiJobState::failed
        && status->error.isNotEmpty() && processor.getParameterSurfaceGeneration() == generation
        && processor.getParameterForPublicId ("fixture.level") == parameter,
        "Activation failure lost terminal error, generation or host identity");
    render (processor, 0.25f);
    require (processor.replacementTransactionState() == "failed",
        "Recovered activation failure retained a muted replacement phase");
    const auto recovered = processor.requestInstrumentReloadJob (fixture.edited, generation);
    require (recovered.has_value(), "Activation failure retained a hidden pending edit");
    await ([&] { return !processor.isMuted(); }, "Valid edit after activation failure did not resume");
    const auto recoveredStatus = processor.getInstrumentUiJobStatus (*recovered);
    require (recoveredStatus && recoveredStatus->state == DandrumAudioProcessor::UiJobState::completed
        && recoveredStatus->generation > generation,
        "Valid edit after activation failure lost its completed state");
    require (processor.replacementTransactionState() == "running",
        "Valid edit after activation failure did not restore a running replacement phase");
    render (processor, -0.5f);
    std::cout << "ACTIVATION_FAILURE signed_recovery=0.25 next_valid=-0.5 PASS\n";
}
void nonStandardActivationFailures()
{
    activationFailure (ActivationException::integer);
    activationFailure (ActivationException::custom);
    // Each real processor has been destroyed, including its joined future.
    std::cout << "NONSTANDARD_ACTIVATION int/custom terminal_error/readmission/teardown signed_PCM=PASS\n";
}
void hostParameterContract()
{
    Fixture fixture;
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor);
    auto* parameter = processor.getParameterForPublicId ("fixture.level");
    require (parameter && processor.getParameters().size() == 64,
             "Fixed host surface lost its parameter count or public binding");
    const auto& range = parameter->getNormalisableRange();
    require (sameFloat (range.start, 0.0f) && sameFloat (range.end, 1.0f) && sameFloat (range.interval, 0.0f)
        && sameFloat (parameter->getDefaultValue(), 0.0f)
        && parameter->getNumSteps() == std::numeric_limits<int>::max()
        && parameter->getText (0.125f, 0) == "0.1250000"
        && parameter->getText (0.125f, 4) == "0.12"
        && sameFloat (parameter->getValueForText ("0.375"), 0.375f)
        && sameFloat (parameter->getValueForText ("2"), 1.0f)
        && sameFloat (parameter->getValueForText ("-1"), 0.0f),
        "Fixed host surface changed normalized range, defaults, resolution or text conversion");
    for (const auto& [normalised, actual] : { std::pair { -1.0f, -1.0f }, { 2.0f, 1.0f }, { 0.625f, 0.25f } })
    {
        parameter->setValueNotifyingHost (normalised);
        require (sameFloat (parameter->getValue(), juce::jlimit (0.0f, 1.0f, normalised))
            && sameFloat (PluginConstructionTestProbe::rawValue (processor), parameter->getValue()),
            "Fixed host value and real APVTS storage diverged");
        render (processor, actual, 64, true, actual < 0.0f ? -0.0f : 0.0f);
    }
    std::cout << "HOST_PARAMETER normalized_range/text/default/resolution signed_PCM=PASS\n";
}
void activationHostRollback()
{
    // The unchanged one-control fixture cannot distinguish partial host writes.
    // A second real gain allows failure after the first range has been remapped.
    for (int hostWrite : { 0, 1, 2, 3, 4 })
    {
        Fixture fixture;
        auto original = fixture.original.loadFileAsString();
        original = original.replace ("  - { name: master,", "  - { name: extra, direction: input, signal: control, channels: 1, default: 1, min: 0, max: 1, maps_to: tail.gain }\n  - { name: master,");
        original = original.replace ("maps_from: amp.audio_out", "maps_from: tail.audio_out");
        original = original.replace ("    - { name: fixture.level, maps_to: level }", "    - { name: fixture.level, maps_to: level }\n    - { name: fixture.extra, maps_to: extra }");
        original = original.replace ("connections:", "  - { id: tail, type: gain, static: { channels: 2 } }\nconnections:");
        original += "  - { from: amp.audio_out, to: tail.audio_in }\n";
        require (fixture.original.replaceWithText (original)
            && fixture.edited.replaceWithText (original.replace ("min: -1, max: 1", "min: 0, max: 2")),
            "Could not stage the two-control range-change fixture");
        original = fixture.original.loadFileAsString(); // replaceWithText normalizes file line endings
        DandrumAudioProcessor processor (fixture.configuration (fixture.original));
        prepare (processor); render (processor, 0.25f);
        auto* level = processor.getParameterForPublicId ("fixture.level");
        auto* extra = processor.getParameterForPublicId ("fixture.extra");
        require (level && extra && sameFloat (level->getValue(), 0.625f), "Two-control baseline lost its actual host range");
        const auto generation = processor.getParameterSurfaceGeneration();
        std::atomic<bool> held { false }, released { false };
        std::future<void> automation;
        struct ThrowLater final : juce::AudioProcessorParameter::Listener
        {
            juce::AudioProcessorParameter& parameter;
            juce::AudioProcessorParameter& level;
            DandrumAudioProcessor& processor;
            juce::ValueTree retainedState;
            int hostWrite;
            std::atomic<bool>& held;
            std::atomic<bool>& released;
            bool armed = true;
            bool skippedHostWrite = false;
            ThrowLater (juce::AudioProcessorParameter& value, juce::AudioProcessorParameter& first,
                        DandrumAudioProcessor& p, int mode, std::atomic<bool>& entered, std::atomic<bool>& release)
                : parameter (value), level (first), processor (p), hostWrite (mode), held (entered), released (release)
            { parameter.addListener (this); }
            ~ThrowLater() override { parameter.removeListener (this); }
            void parameterValueChanged (int, float) override
            {
                if (hostWrite == 4 && !std::exchange (skippedHostWrite, true)) return;
                if (!std::exchange (armed, false)) return;
                if (hostWrite == 1) level.setValueNotifyingHost (0.75f);
                if (hostWrite == 2) level.setValueNotifyingHost (0.125f); // identical to the candidate write
                if (hostWrite == 3)
                {
                    held = true;
                    while (!released.load()) std::this_thread::sleep_for (1ms);
                }
                retainedState = PluginConstructionTestProbe::savedState (processor);
                throw std::runtime_error ("Injected later-slot activation failure");
            }
            void parameterGestureChanged (int, bool) override {}
        } listener (*extra, *level, processor, hostWrite, held, released);
        struct AutomateSecond final : juce::AudioProcessorParameter::Listener
        {
            juce::AudioProcessorParameter& first;
            juce::AudioProcessorParameter& second;
            bool armed;
            AutomateSecond (juce::AudioProcessorParameter& a, juce::AudioProcessorParameter& b, bool enabled)
                : first (a), second (b), armed (enabled) { first.addListener (this); }
            ~AutomateSecond() override { first.removeListener (this); }
            void parameterValueChanged (int, float) override
            { if (std::exchange (armed, false)) second.setValueNotifyingHost (0.25f); }
            void parameterGestureChanged (int, bool) override {}
        } earlyAutomation (*level, *extra, hostWrite == 4);
        struct ReleaseHeld
        {
            std::atomic<bool>& released;
            ~ReleaseHeld() { released = true; }
        } release { released };
        const auto id = processor.requestInstrumentReloadJob (fixture.edited, generation);
        require (id.has_value(), "Range-change activation job was rejected");
        if (hostWrite == 3)
        {
            await ([&] { return held.load(); }, "Later-slot activation notification did not stall");
            automation = std::async (std::launch::async, [&]
            { level->setValueNotifyingHost (0.25f); level->setValueNotifyingHost (0.75f); });
            require (automation.wait_for (3s) == std::future_status::ready,
                     "Host value write waited for structural activation");
            automation.get();
            released = true;
        }
        await ([&] { return !processor.isMuted(); }, "Partial host-value activation failure stranded mute");
        const auto status = processor.getInstrumentUiJobStatus (*id);
        const float expected = hostWrite == 0 || hostWrite == 4 ? 0.625f : hostWrite == 2 ? 0.125f : 0.75f;
        const float expectedSecond = hostWrite == 4 ? 0.25f : 1.0f;
        require (status && status->state == DandrumAudioProcessor::UiJobState::failed
            && status->error.isNotEmpty() && processor.getParameterSurfaceGeneration() == generation
            && processor.getParameterForPublicId ("fixture.level") == level
            && processor.getParameterForPublicId ("fixture.extra") == extra,
            "Range-change activation failure lost working host bindings or generation");
        require (sameFloat (level->getValue(), expected) && sameFloat (PluginConstructionTestProbe::rawValue (processor), expected)
            && sameFloat (extra->getValue(), expectedSecond)
            && sameFloat (PluginConstructionTestProbe::rawValue (processor, "dandrum.slot.01"), expectedSecond),
                 "Failed range-change activation changed working host values or erased latest automation");
        const auto state = PluginConstructionTestProbe::savedState (processor);
        require (listener.retainedState["instrument_yaml"].toString() == original
            && sameFloat (static_cast<float> (listener.retainedState.getChildWithProperty ("id", "dandrum.slot.00")["value"]), expected)
            && state["instrument_yaml"].toString() == original
            && sameFloat (static_cast<float> (state.getChildWithProperty ("id", "dandrum.slot.00")["value"]), expected)
            && sameFloat (static_cast<float> (state.getChildWithProperty ("id", "dandrum.slot.01")["value"]), expectedSecond),
            "Failed range-change activation saved incoherent working parameter state");
        const float signedOutput = hostWrite == 0 ? 0.25f : hostWrite == 2 ? -0.75f : hostWrite == 4 ? 0.0625f : 0.5f;
        // The second gain sums into its zeroed input: +0 + -0 is +0.
        render (processor, signedOutput);
        const auto next = processor.requestInstrumentReloadJob (fixture.original, generation);
        require (next.has_value(), "Partial host-value failure retained a pending edit");
        await ([&] { return !processor.isMuted(); }, "Valid job after partial host-value failure did not resume");
        const auto nextStatus = processor.getInstrumentUiJobStatus (*next);
        require (nextStatus && nextStatus->state == DandrumAudioProcessor::UiJobState::completed,
                 "Valid job after partial host-value failure did not complete");
        render (processor, signedOutput);
    }
    std::cout << "ACTIVATION_HOST_STATE rollback=0.625 automation=0.75/same-0.125/concurrent-0.75 signed_PCM=PASS\n";
}
void shutdownDuringPreparation()
{
    Fixture fixture;
    auto processor = std::make_unique<DandrumAudioProcessor> (fixture.configuration (fixture.original));
    prepare (*processor);
    PreparationBarrier barrier { {}, {}, {}, *processor };
    std::future<void> shutdown;
    HoldPreparation hold (barrier);
    const auto id = processor->requestInstrumentReloadJob (fixture.edited, processor->getParameterSurfaceGeneration());
    require (id.has_value(), "Shutdown job was rejected");
    await ([&] { return barrier.entered.load(); }, "Shutdown worker did not enter held preparation");
    shutdownJoinObserved = false;
    shutdown = std::async (std::launch::async, [&]
    {
        closingProcessor = true;
        shuttingDownProcessor = processor.get();
        processor.reset();
        shuttingDownProcessor = nullptr;
        closingProcessor = false;
    });
    // Witness the destructor's actual off-audio worker join, not an elapsed
    // guess about whether destruction has started.
    await ([&] { return shutdownJoinObserved.load(); }, "Processor shutdown did not join its pending worker off audio");
    barrier.released = true;
    require (shutdown.wait_for (3s) == std::future_status::ready, "Processor shutdown stranded its reload worker");
    shutdown.get();
    std::cout << "RELOAD_SHUTDOWN pending_worker_join=PASS closed_gate_through_cleanup=PASS\n";
}
void activationMetadataConsistency()
{
    Fixture fixture;
    DandrumAudioProcessor processor (fixture.configuration (fixture.original));
    prepare (processor);
    require (!processor.loadPresetFromFile (fixture.file ("missing-preset.yaml")),
             "Missing preset unexpectedly changed working configuration");
    const auto workingPresetError = processor.getLastPresetError();
    require (workingPresetError.isNotEmpty(), "Missing preset lost its diagnostic");
    const auto generation = processor.getParameterSurfaceGeneration();
    auto* parameter = processor.getParameterForPublicId ("fixture.level");
    struct HeldNotification final : juce::AudioProcessorParameter::Listener
    {
        juce::AudioProcessorParameter& parameter;
        std::atomic<bool> entered { false }, released { false };
        explicit HeldNotification (juce::AudioProcessorParameter& value) : parameter (value)
        { parameter.addListener (this); }
        ~HeldNotification() override { parameter.removeListener (this); }
        void parameterValueChanged (int, float) override
        {
            entered = true;
            while (!released.load()) std::this_thread::sleep_for (1ms);
            throw std::runtime_error ("Injected held activation failure");
        }
        void parameterGestureChanged (int, bool) override {}
    } notification (*parameter);
    std::future<juce::String> observation;
    struct ReleaseNotification
    {
        HeldNotification& notification;
        ~ReleaseNotification() { notification.released = true; }
    } release { notification };
    const auto id = processor.requestInstrumentReloadJob (fixture.edited, generation);
    require (id.has_value(), "Metadata-consistency job was rejected");
    await ([&] { return notification.entered.load(); }, "Activation notification was not held");
    observation = std::async (std::launch::async, [&]
    {
        const auto yaml = processor.currentInstrumentYaml();
        const auto file = processor.currentInstrumentFile();
        const auto ids = processor.getActivePublicParameterIds();
        const auto name = processor.getPublicParameterDisplayName ("fixture.level");
        const auto identity = processor.getParameterForPublicId ("fixture.level");
        const auto loaded = processor.isInstrumentLoaded();
        const auto error = processor.getLastLoadError();
        const auto presetError = processor.getLastPresetError();
        const auto warning = processor.getLastReloadWarning();
        static_cast<void> (processor.currentPresetName());
        static_cast<void> (processor.currentPresetYaml());
        static_cast<void> (processor.getProgramName (0));
        static_cast<void> (processor.isSoundLabInstrumentCompatible());
        static_cast<void> (processor.watchedInstrumentFile());
        juce::MemoryBlock saved;
        processor.getStateInformation (saved);
        require (file == fixture.original && ids.contains ("fixture.level") && name.isNotEmpty()
            && identity == parameter && loaded && error.isEmpty() && presetError == workingPresetError && warning.isEmpty(),
            "Failed activation exposed candidate identity to a configuration reader");
        return yaml;
    });
    // A notification holds an unfinished activation. Observers must not publish
    // that candidate as working configuration; failure restores the old snapshot.
    const auto premature = observation.wait_for (50ms) == std::future_status::ready;
    notification.released = true;
    await ([&] { return !processor.isMuted(); }, "Held activation failure did not recover");
    require (!premature, "Configuration reader observed an uncommitted activation");
    require (observation.get() == fixture.original.loadFileAsString(),
             "Recovered configuration reader returned candidate YAML");
    const auto status = processor.getInstrumentUiJobStatus (*id);
    require (status && status->state == DandrumAudioProcessor::UiJobState::failed
        && processor.getParameterSurfaceGeneration() == generation,
        "Metadata reader changed failed activation state");
    render (processor, 0.25f);
    std::cout << "ACTIVATION_METADATA coherent_working_configuration=PASS signed_recovery=0.25 PASS\n";
}
void reentrantActivationReaders()
{
    for (bool automate : { false, true })
    {
        Fixture fixture;
        const auto original = fixture.original.loadFileAsString();
        require (fixture.edited.replaceWithText (original.replace ("min: -1, max: 1", "min: 0, max: 2")),
                 "Could not stage reentrant range-change fixture");
        const auto preset = fixture.file ("notification-preset.yaml");
        require (preset.replaceWithText ("name: Notification Preset\ninstrument:\n  id: dandrum.plugin-ui-knob-fixture\n  preset_schema_version: 1\nvalues:\n  fixture.level: 0.75\n"),
                 "Could not stage the real notification preset");
        DandrumAudioProcessor processor (fixture.configuration (fixture.original));
        prepare (processor);
        require (processor.loadPresetFromFile (preset), "Reentrant preset fixture was invalid outside activation");
        render (processor, 0.75f);
        auto* parameter = processor.getParameterForPublicId ("fixture.level");
        parameter->setValueNotifyingHost (0.625f);
        render (processor, 0.25f);
        const auto generation = processor.getParameterSurfaceGeneration();
        juce::MemoryBlock workingState;
        processor.getStateInformation (workingState);
        struct Reader final : juce::AudioProcessorParameter::Listener
        {
            DandrumAudioProcessor& processor;
            juce::AudioProcessorParameter& parameter;
            Fixture& fixture;
            const juce::MemoryBlock& workingState;
            bool automate, armed = true, coherent = false, ownershipRejected = false;
            juce::String yaml;
            juce::File file;
            std::optional<InstrumentUiDocument> document;
            juce::ValueTree state;
            std::vector<DandrumAudioProcessor::PublicParameterSnapshotEntry> values;
            Reader (DandrumAudioProcessor& p, juce::AudioProcessorParameter& value, Fixture& f,
                    const juce::MemoryBlock& saved, bool write)
                : processor (p), parameter (value), fixture (f), workingState (saved), automate (write)
            { parameter.addListener (this); }
            ~Reader() override { parameter.removeListener (this); }
            void parameterValueChanged (int, float) override
            {
                if (!std::exchange (armed, false)) return;
                if (automate) parameter.setValueNotifyingHost (0.75f);
                yaml = processor.currentInstrumentYaml();
                file = processor.currentInstrumentFile();
                document = processor.getPreparedUiDocument();
                state = PluginConstructionTestProbe::savedState (processor);
                values = processor.getPublicParameterSnapshot();
                const auto expected = automate ? 0.75f : 0.625f;
                coherent = yaml == fixture.original.loadFileAsString() && file == fixture.original
                    && document && document->generation == processor.getParameterSurfaceGeneration()
                    && document->parameters.size() == 1 && document->parameters[0].id == "fixture.level"
                    && sameFloat (document->parameters[0].minValue, -1.0f)
                    && sameFloat (document->parameters[0].maxValue, 1.0f)
                    && sameFloat (document->parameters[0].normalisedValue, expected)
                    && values.size() == 1 && sameFloat (values[0].normalisedValue, expected)
                    && state["instrument_yaml"].toString() == yaml
                    && state["instrument_path"].toString() == file.getFullPathName()
                    && sameFloat (static_cast<float> (state.getChildWithProperty ("id", "dandrum.slot.00")["value"]), expected);
                if (coherent)
                {
                    struct ObserveOwnership
                    {
                        ObserveOwnership() { insideActivationNotification = true; }
                        ~ObserveOwnership() { insideActivationNotification = false; }
                    } observe;
                    ownershipRejected = !processor.reloadInstrumentFromFile (fixture.edited)
                        && !processor.reloadInstrumentFromYaml (fixture.edited.loadFileAsString(), fixture.edited)
                        && !processor.loadPresetFromFile (fixture.file ("notification-preset.yaml"));
                    processor.setStateInformation (workingState.getData(), static_cast<int> (workingState.getSize()));
                    processor.prepareToPlay (48000, 64);
                }
                throw std::runtime_error ("Injected synchronous reader activation failure");
            }
            void parameterGestureChanged (int, bool) override {}
        } reader (processor, *parameter, fixture, workingState, automate);
        const auto id = processor.requestInstrumentReloadJob (fixture.edited, generation);
        require (id.has_value(), "Reentrant reader job was rejected");
        await ([&] { return !processor.isMuted(); }, "Reentrant reader failure stranded mute");
        const auto status = processor.getInstrumentUiJobStatus (*id);
        require (reader.coherent, "Synchronous configuration reader retained an uncommitted candidate");
        require (reader.ownershipRejected, "Synchronous activation allowed a recursive ownership change");
        const auto recoveredDocument = processor.getPreparedUiDocument();
        require (status && status->state == DandrumAudioProcessor::UiJobState::failed
            && status->error.isNotEmpty() && processor.getParameterSurfaceGeneration() == generation
            && reader.document->generation == generation && processor.currentInstrumentYaml() == reader.yaml
            && processor.currentInstrumentFile() == reader.file
            && recoveredDocument && recoveredDocument->parameters.size() == 1
            && sameFloat (recoveredDocument->parameters[0].minValue, reader.document->parameters[0].minValue),
            "Retained synchronous working metadata changed after failed activation");
        render (processor, automate ? 0.5f : 0.25f);
        const auto next = processor.requestInstrumentReloadJob (fixture.edited, generation);
        require (next.has_value(), "Reentrant failure stranded future reload admission");
        await ([&] { return !processor.isMuted(); }, "Valid reload after reentrant read did not resume");
        const auto nextStatus = processor.getInstrumentUiJobStatus (*next);
        const auto committed = processor.getPreparedUiDocument();
        require (nextStatus && nextStatus->state == DandrumAudioProcessor::UiJobState::completed
            && committed && committed->generation > generation && committed->parameters.size() == 1
            && sameFloat (committed->parameters[0].minValue, 0.0f) && sameFloat (committed->parameters[0].maxValue, 2.0f)
            && processor.currentInstrumentFile() == fixture.edited
            && processor.currentInstrumentYaml() == fixture.edited.loadFileAsString()
            && sameFloat (reader.document->parameters[0].minValue, -1.0f),
                 "Valid reload after reentrant read did not complete");
        render (processor, automate ? 0.5f : 0.25f);
    }
    std::cout << "REENTRANT_READERS retained_working_YAML/file/document/state automation signed_PCM=PASS\n";
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
extern "C" void __real__ZNSt6thread4joinEv (std::thread*);
// std::call_once invokes &std::thread::join as a member-function pointer.
// Its ABI reserves the low address bit for virtual dispatch; a C link wrapper
// must preserve even alignment, including unoptimized/ASan builds.
extern "C" __attribute__((aligned(16))) void __wrap__ZNSt6thread4joinEv (std::thread* thread)
{
    if (measuredAudio) ++callbackLifecycle;
    if (closingProcessor) shutdownJoinObserved = true;
    __real__ZNSt6thread4joinEv (thread);
}
// GNU/Linux link seam at the actual std::async thread-start boundary.
extern "C" void __real__ZNSt6thread15_M_start_threadESt10unique_ptrINS_6_StateESt14default_deleteIS1_EEPFvvE (
    std::thread*, std::unique_ptr<std::thread::_State>, void (*)());
extern "C" void __wrap__ZNSt6thread15_M_start_threadESt10unique_ptrINS_6_StateESt14default_deleteIS1_EEPFvvE (
    std::thread* thread, std::unique_ptr<std::thread::_State> state, void (*dependency)())
{
    if (failThreadStartup.exchange (false))
        throw std::system_error (std::make_error_code (std::errc::resource_unavailable_try_again));
    __real__ZNSt6thread15_M_start_threadESt10unique_ptrINS_6_StateESt14default_deleteIS1_EEPFvvE (
        thread, std::move (state), dependency);
}
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
    if (insideActivationNotification && engine != nullptr)
    { std::cerr << "Activation notification recursively retired a working engine\n"; std::_Exit (1); }
    if (measuredAudio) ++callbackLifecycle;
    if (auto* closing = shuttingDownProcessor.load(); closing && !closing->isMuted())
    {
        std::cerr << "Reload worker reopened audio during processor shutdown/engine cleanup\n";
        std::_Exit (1);
    }
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
    if (insideActivationNotification)
    { std::cerr << "Activation notification recursively prepared an engine\n"; std::_Exit (1); }
    if (measuredAudio) ++callbackLifecycle;
    auto* barrier = preparationBarrier.load();
    if (barrier && barrier->armed.exchange (false))
    {
        barrier->beganMuted = barrier->processor.isMuted();
        barrier->entered = true;
        while (!barrier->released.load()) std::this_thread::sleep_for (1ms);
    }
    if (failPreparation.exchange (false)) throw std::runtime_error ("Injected real preparation worker exception");
    return __real_dandrum_kernel_prepare_file (path, rate, block, buses, count);
}
int main (int argc, char** argv)
{
    juce::ScopedJuceInitialiser_GUI gui;
    const bool live = argc == 2 && std::string (argv[1]) == "--live";
    const bool hold = live || (argc == 2 && std::string (argv[1]) == "--hold");
    try
    {
        if (argc == 2 && std::string (argv[1]) == "--job-baseline") jobBaseline();
        else if (argc == 2 && std::string (argv[1]) == "--host-parameter-contract") hostParameterContract();
        else if (argc == 2 && std::string (argv[1]) == "--activation-host-state") activationHostRollback();
        else if (argc == 2 && std::string (argv[1]) == "--reentrant-readers") reentrantActivationReaders();
        else if (argc == 2 && std::string (argv[1]) == "--nonstandard-activation") nonStandardActivationFailures();
        else if (argc == 2 && std::string (argv[1]) == "--automatic-job")
        { automaticJob(); automaticJobWithReader(); startupFailureWithReader();
          shutdownDuringPreparation(); obsoleteHostPreparation(); activationFailure(); activationMetadataConsistency(); hostParameterContract(); activationHostRollback(); reentrantActivationReaders(); nonStandardActivationFailures(); }
        else for (auto operation : { Operation::reload, Operation::prepare, Operation::restore }) exercise (operation, hold, live);
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
