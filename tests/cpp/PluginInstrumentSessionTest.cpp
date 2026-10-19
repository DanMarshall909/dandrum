#include "PluginProcessor.h"

#include <atomic>
#include <bit>
#include <chrono>
#include <future>
#include <iostream>
#include <pthread.h>
#include <stdexcept>
#include <string>
#include <thread>

struct PluginInstrumentSessionTestProbe
{
    static void failNextActivation (DandrumAudioProcessor& processor)
    {
        processor.failNextSessionActivationForTest = true;
    }

    static bool leasePublicationInProgress (const DandrumAudioProcessor& processor)
    {
        return processor.instrumentSessionLeasePublishing.load (std::memory_order_acquire)
            && processor.instrumentSessionLeasePublicationThread.load (std::memory_order_relaxed)
                == std::hash<std::thread::id> {} (std::this_thread::get_id());
    }
};

namespace
{
using namespace std::chrono_literals;

void require (bool condition, const char* message)
{
    if (! condition) throw std::runtime_error (message);
}

template<class Predicate>
void await (Predicate predicate, const char* message)
{
    const auto deadline = std::chrono::steady_clock::now() + 5s;
    while (! predicate())
    {
        require (std::chrono::steady_clock::now() < deadline, message);
        std::this_thread::sleep_for (1ms);
    }
}

struct RenderBarrier
{
    std::atomic<bool> armed { false };
    std::atomic<bool> entered { false };
    std::atomic<bool> released { false };
    std::atomic<DandrumKernelInstrument*> engine { nullptr };
    std::atomic<unsigned> prematureDestroy { 0 };
    std::atomic<unsigned> watchedDestroy { 0 };
};

struct PreparationBarrier
{
    std::atomic<bool> armed { false };
    std::atomic<bool> entered { false };
    std::atomic<bool> released { false };
};

std::atomic<RenderBarrier*> recording { nullptr };
std::atomic<PreparationBarrier*> preparationRecording { nullptr };
std::atomic<DandrumAudioProcessor*> publicationProcessor { nullptr };
std::atomic<unsigned> publicationMutexLocks { 0 };

float renderOnset (DandrumAudioProcessor& processor, int note, int frames = 64)
{
    juce::AudioBuffer<float> buffer (2, frames);
    juce::MidiBuffer midi;
    buffer.clear();
    midi.addEvent (juce::MidiMessage::noteOn (1, note, 1.0f), 0);
    processor.processBlock (buffer, midi);
    return buffer.getSample (0, 0);
}

const InstrumentUiDocument::Zone& kickZone (const InstrumentUiDocument& document)
{
    for (const auto& map : document.maps)
        if (map.id == "kit")
            for (const auto& zone : map.zones)
                if (zone.id == "kick") return zone;
    throw std::runtime_error ("Prepared UI document lost native kit/kick identity");
}

void sessionHandoff()
{
    DandrumAudioProcessor processor (InstrumentDemoConfiguration::sampler());
    processor.setFileWatchEnabled (false);
    processor.setRateAndBufferSizeDetails (48000, 64);
    processor.prepareToPlay (48000, 64);
    require (processor.isInstrumentLoaded(), "Sampler fixture did not load");

    const auto initial = processor.getInstrumentSessionSnapshot();
    require (initial && initial->revision == 0 && initial->undoCount == 0 && initial->redoCount == 0,
             "Native host did not expose the initial Rust session snapshot");
    require (processor.currentInstrumentYaml().toStdString() == initial->yaml,
             "Initial host YAML and session YAML disagree");

    RenderBarrier barrier;
    recording = &barrier;
    barrier.armed = true;
    auto audio = std::async (std::launch::async, [&] {
        const auto sample = renderOnset (processor, 36);
        require (std::bit_cast<std::uint32_t> (sample) == std::bit_cast<std::uint32_t> (-0.5f),
                 "Baseline real callback lost the known signed kick sample");
    });
    await ([&] { return barrier.entered.load(); }, "Real callback reader was not held");

    publicationProcessor = &processor;
    const auto request = processor.requestInstrumentSessionMappingJob (
        1, initial->revision, "kit", "kick", 60, 60);
    require (request && *request == 1, "Native host rejected the session mapping request");
    await ([&] { return processor.isMuted(); }, "Session activation never closed real audio admission");
    require (barrier.prematureDestroy == 0 && barrier.watchedDestroy == 0,
             "Session activation retired the engine before reader release");

    barrier.released = true;
    audio.get();
    std::optional<DandrumAudioProcessor::InstrumentSessionJobStatus> completed;
    await ([&] {
        completed = processor.getInstrumentSessionJobStatus (*request);
        return completed && completed->state != DandrumAudioProcessor::UiJobState::running;
    }, "Editor-independent session worker did not complete");
    publicationProcessor = nullptr;
    recording = nullptr;
    require (completed->state == DandrumAudioProcessor::UiJobState::completed
                 && completed->revision == 1 && completed->undoCount == 1 && completed->redoCount == 0,
             "Accepted session job lost coherent revision/history metadata");
    require (barrier.prematureDestroy == 0 && barrier.watchedDestroy == 1,
             "Old callback engine was not retired exactly once after reader release");
    require (publicationMutexLocks == 0,
             "Infallible session lease publication acquired a native mutex");

    const auto accepted = processor.getInstrumentSessionSnapshot();
    const auto document = processor.getPreparedUiDocument();
    require (accepted && document && accepted->revision == 1
                 && accepted->undoCount == 1 && accepted->redoCount == 0
                 && processor.currentInstrumentYaml().toStdString() == accepted->yaml,
             "Active consumer metadata and accepted session snapshot were not published together");
    const auto& mapped = kickZone (*document);
    require (mapped.keyLow == 60 && mapped.keyHigh == 60,
             "Actual consumer UI metadata did not receive the accepted native mapping");
    require (std::bit_cast<std::uint32_t> (renderOnset (processor, 36))
                 == std::bit_cast<std::uint32_t> (0.0f),
             "Accepted native mapping still rendered its old MIDI note");

    PluginInstrumentSessionTestProbe::failNextActivation (processor);
    const auto failedId = processor.requestInstrumentSessionMappingJob (
        2, accepted->revision, "kit", "kick", 61, 61);
    require (failedId && *failedId == 2, "Native host rejected failure-path request admission");
    std::optional<DandrumAudioProcessor::InstrumentSessionJobStatus> failed;
    await ([&] {
        failed = processor.getInstrumentSessionJobStatus (*failedId);
        return failed && failed->state != DandrumAudioProcessor::UiJobState::running;
    }, "Injected activation failure did not finish");
    const auto preserved = processor.getInstrumentSessionSnapshot();
    const auto preservedDocument = processor.getPreparedUiDocument();
    require (failed->state == DandrumAudioProcessor::UiJobState::failed && failed->error.isNotEmpty()
                 && preserved && *preserved == *accepted
                 && processor.currentInstrumentYaml().toStdString() == accepted->yaml
                 && preservedDocument && kickZone (*preservedDocument).keyLow == 60
                 && kickZone (*preservedDocument).keyHigh == 60,
             "Failed concrete activation changed runtime/document/history metadata");
    require (std::bit_cast<std::uint32_t> (renderOnset (processor, 60))
                 == std::bit_cast<std::uint32_t> (-0.5f),
             "Failed activation did not preserve known signed output from the accepted engine");

    processor.setRateAndBufferSizeDetails (48000, 128);
    processor.prepareToPlay (48000, 128);
    const auto reconfigured = processor.getInstrumentSessionSnapshot();
    require (reconfigured && *reconfigured == *accepted
                 && processor.currentInstrumentYaml().toStdString() == accepted->yaml
                 && std::bit_cast<std::uint32_t> (renderOnset (processor, 60))
                        == std::bit_cast<std::uint32_t> (-0.5f),
             "Host render reconfiguration discarded accepted mapping/history or signed output");

    PreparationBarrier preparation;
    preparationRecording = &preparation;
    preparation.armed = true;
    const auto concurrentId = processor.requestInstrumentSessionMappingJob (
        3, reconfigured->revision, "kit", "kick", 62, 62);
    require (concurrentId && *concurrentId == 3,
             "Native host rejected the concurrent lifecycle request");
    await ([&] { return preparation.entered.load(); },
           "Session worker did not reach held preparation");
    std::atomic<bool> reconfigureStarted { false };
    std::atomic<bool> reconfigureReturned { false };
    auto hostPreparation = std::async (std::launch::async, [&] {
        reconfigureStarted = true;
        processor.setRateAndBufferSizeDetails (48000, 256);
        processor.prepareToPlay (48000, 256);
        reconfigureReturned = true;
    });
    await ([&] { return reconfigureStarted.load(); },
           "Concurrent host preparation did not start");
    std::this_thread::sleep_for (30ms);
    const auto waitedForWorker = ! reconfigureReturned.load();
    preparation.released = true;
    hostPreparation.get();
    std::optional<DandrumAudioProcessor::InstrumentSessionJobStatus> concurrent;
    await ([&] {
        concurrent = processor.getInstrumentSessionJobStatus (*concurrentId);
        return concurrent && concurrent->state != DandrumAudioProcessor::UiJobState::running;
    }, "Concurrent lifecycle session job did not complete");
    preparationRecording = nullptr;
    require (waitedForWorker,
             "Host reconfiguration bypassed an admitted session worker");
    require (concurrent->state == DandrumAudioProcessor::UiJobState::completed
                 && concurrent->revision == 2 && concurrent->undoCount == 2
                 && concurrent->redoCount == 0
                 && std::bit_cast<std::uint32_t> (renderOnset (processor, 62, 256))
                        == std::bit_cast<std::uint32_t> (-0.5f),
             "Host reconfiguration did not rebuild the newly accepted mapping at the new block size");

    juce::MemoryBlock editedState;
    processor.getStateInformation (editedState);
    const auto editedYaml = processor.currentInstrumentYaml();

    PreparationBarrier restorePreparation;
    preparationRecording = &restorePreparation;
    restorePreparation.armed = true;
    const auto restoreRaceId = processor.requestInstrumentSessionMappingJob (
        4, concurrent->revision, "kit", "kick", 63, 63);
    require (restoreRaceId && *restoreRaceId == 4,
             "Native host rejected the state-restore lifecycle request");
    await ([&] { return restorePreparation.entered.load(); },
           "State-restore fixture did not hold session preparation");
    std::atomic<bool> restoreStarted { false };
    std::atomic<bool> restoreReturned { false };
    auto stateRestore = std::async (std::launch::async, [&] {
        restoreStarted = true;
        processor.setStateInformation (editedState.getData(),
                                       static_cast<int> (editedState.getSize()));
        restoreReturned = true;
    });
    await ([&] { return restoreStarted.load(); }, "Concurrent state restore did not start");
    std::this_thread::sleep_for (30ms);
    const auto restoreWaitedForWorker = ! restoreReturned.load();
    restorePreparation.released = true;
    stateRestore.get();
    preparationRecording = nullptr;
    const auto restoreRace = processor.getInstrumentSessionJobStatus (*restoreRaceId);
    require (restoreWaitedForWorker && restoreRace
                 && restoreRace->state == DandrumAudioProcessor::UiJobState::completed,
             "Host state restore discarded or bypassed admitted session work");
    require (! processor.getInstrumentSessionSnapshot(),
             "Host state restore retained a stale persistent session");
    processor.setRateAndBufferSizeDetails (48000, 128);
    processor.prepareToPlay (48000, 128);
    const auto restoredDocument = processor.getPreparedUiDocument();
    require (processor.currentInstrumentYaml() == editedYaml,
             "Host preparation replaced the restored embedded YAML");
    require (restoredDocument && kickZone (*restoredDocument).keyLow == 62
                 && kickZone (*restoredDocument).keyHigh == 62,
             "Host preparation replaced the restored embedded UI metadata");
    require (std::bit_cast<std::uint32_t> (renderOnset (processor, 63, 128))
                 == std::bit_cast<std::uint32_t> (0.0f),
             "Host preparation retained the superseded session mapping");
    require (std::bit_cast<std::uint32_t> (renderOnset (processor, 36, 128))
                 == std::bit_cast<std::uint32_t> (0.0f),
             "Host preparation reloaded the original disk mapping");
    require (std::bit_cast<std::uint32_t> (renderOnset (processor, 62, 128))
                 == std::bit_cast<std::uint32_t> (-0.5f),
             "Host preparation did not render the restored embedded mapping");

    std::cout << "INSTRUMENT_SESSION_HANDOFF held_reader=PASS worker=PASS metadata=PASS failure=PASS reconfigure=PASS concurrent_reconfigure=PASS lease_publish=PASS state_restore=PASS signed_pcm=-0.5\n";
}

void admittedUndoSurvivesConnectionClosure()
{
    DandrumAudioProcessor processor (InstrumentDemoConfiguration::sampler());
    processor.setFileWatchEnabled (false);
    processor.setRateAndBufferSizeDetails (48000, 64);
    processor.prepareToPlay (48000, 64);
    const auto initial = processor.getInstrumentSessionSnapshot();
    require (initial.has_value(), "Undo fixture did not create a session");
    const auto map = processor.requestInstrumentSessionMappingJob (
        1, initial->revision, "kit", "kick", 60, 60);
    require (map.has_value(), "Undo fixture mapping was not admitted");
    await ([&] {
        const auto status = processor.getInstrumentSessionJobStatus (*map);
        return status && status->state != DandrumAudioProcessor::UiJobState::running;
    }, "Undo fixture mapping did not complete");
    const auto connection = processor.openInstrumentSessionConnection();
    require (connection && connection->undoCount == 1,
             "Undo fixture connection has no native history");
    PreparationBarrier preparation;
    preparationRecording = &preparation;
    preparation.armed = true;
    const auto undo = processor.requestInstrumentSessionUndoJob (
        connection->instanceId, connection->attachmentId,
        connection->nextRequestId, connection->revision);
    require (undo.has_value(), "Native undo was not admitted before editor closure");
    await ([&] { return preparation.entered.load(); },
           "Admitted native undo did not reach held owned preparation");
    require (processor.closeInstrumentSessionConnection (
                 connection->instanceId, connection->attachmentId),
             "Editor connection did not close while native undo was preparing");
    preparation.released = true;
    std::optional<DandrumAudioProcessor::InstrumentSessionJobStatus> completed;
    await ([&] {
        completed = processor.getInstrumentSessionJobStatus (*undo);
        return completed && completed->state != DandrumAudioProcessor::UiJobState::running;
    }, "Owned native undo did not finish after editor closure");
    preparationRecording = nullptr;
    require (completed->state == DandrumAudioProcessor::UiJobState::completed
                 && completed->undoCount == 0 && completed->redoCount == 1,
             "Accepted native undo still depended on its closed editor connection");
    require (std::bit_cast<std::uint32_t> (renderOnset (processor, 36))
                 == std::bit_cast<std::uint32_t> (-0.5f),
             "Detached native undo did not restore known signed output");
}
}

extern "C" int __real_pthread_mutex_lock (pthread_mutex_t*);
extern "C" int __wrap_pthread_mutex_lock (pthread_mutex_t* mutex)
{
    if (auto* processor = publicationProcessor.load();
        processor != nullptr
            && PluginInstrumentSessionTestProbe::leasePublicationInProgress (*processor))
        ++publicationMutexLocks;
    return __real_pthread_mutex_lock (mutex);
}

extern "C" std::size_t __real_dandrum_kernel_render (
    DandrumKernelInstrument*, const DandrumKernelInputBusView*, std::size_t,
    const DandrumKernelOutputBusView*, std::size_t, std::size_t);
extern "C" std::size_t __wrap_dandrum_kernel_render (
    DandrumKernelInstrument* engine, const DandrumKernelInputBusView* inputs,
    std::size_t inputCount, const DandrumKernelOutputBusView* outputs,
    std::size_t outputCount, std::size_t frames)
{
    const auto result = __real_dandrum_kernel_render (
        engine, inputs, inputCount, outputs, outputCount, frames);
    if (auto* barrier = recording.load(); barrier && barrier->armed.exchange (false))
    {
        barrier->engine = engine;
        barrier->entered = true;
        while (! barrier->released.load()) std::this_thread::sleep_for (1ms);
    }
    return result;
}

extern "C" void __real_dandrum_kernel_destroy (DandrumKernelInstrument*);
extern "C" void __wrap_dandrum_kernel_destroy (DandrumKernelInstrument* engine)
{
    if (auto* barrier = recording.load(); barrier && engine == barrier->engine.load())
    {
        if (! barrier->released.load()) ++barrier->prematureDestroy;
        ++barrier->watchedDestroy;
    }
    __real_dandrum_kernel_destroy (engine);
}

extern "C" bool __real_dandrum_instrument_session_prepare_job (
    DandrumInstrumentSessionJob*);
extern "C" bool __wrap_dandrum_instrument_session_prepare_job (
    DandrumInstrumentSessionJob* job)
{
    if (auto* barrier = preparationRecording.load();
        barrier && barrier->armed.exchange (false))
    {
        barrier->entered = true;
        while (! barrier->released.load()) std::this_thread::sleep_for (1ms);
    }
    return __real_dandrum_instrument_session_prepare_job (job);
}

int main()
{
    juce::ScopedJuceInitialiser_GUI gui;
    try
    {
        sessionHandoff();
        admittedUndoSurvivesConnectionClosure();
    }
    catch (const std::exception& error)
    {
        std::cerr << error.what() << '\n';
        return 1;
    }
    return 0;
}
