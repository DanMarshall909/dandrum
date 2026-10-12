#include "ui/advanced-sampler/engine/Model.h"
#include <algorithm>
#include <chrono>
#include <bit>
#include <condition_variable>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <mutex>
#include <stdexcept>
#include <thread>

using namespace dandrum::sampler;
namespace {
int checks = 0;
void expect(bool condition, const char* message) {
    ++checks;
    if (!condition) throw std::runtime_error(message);
}
template<class Predicate> void until(Model& model, Predicate ready) {
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(3);
    while (!ready() && std::chrono::steady_clock::now() < deadline) {
        model.tick(.01);
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    expect(ready(), "The asynchronous asset operation completes within the test deadline");
}
struct Files {
    std::filesystem::path directory = std::filesystem::temp_directory_path() /
            ("dandrum-async-assets-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    Files() { std::filesystem::create_directories(directory); }
    ~Files() { std::error_code ignored; std::filesystem::remove_all(directory, ignored); }
    std::filesystem::path wav(std::string name = "signed.wav") {
        auto path = directory / name;
        std::ofstream file(path, std::ios::binary);
        const auto u16 = [&](unsigned n) { file.put(n & 255); file.put((n >> 8) & 255); };
        const auto u32 = [&](unsigned n) { u16(n & 65535); u16(n >> 16); };
        file.write("RIFF", 4); u32(44); file.write("WAVEfmt ", 8); u32(16);
        u16(1); u16(1); u32(4); u32(8); u16(2); u16(16);
        file.write("data", 4); u32(8); u16(0); u16(16384); u16(32768); u16(32767);
        return path;
    }
};

// The decoder gate controls completion without depending on disk speed. The
// production worker still executes it and the normal UI tick publishes it.
struct DecoderGate {
    struct Call { std::string path; std::stop_token stop; std::thread::id thread; bool released{}; float minimum{-0.75f}; std::string error; std::vector<Peak> peaks; };
    std::mutex mutex;
    std::condition_variable_any changed;
    std::vector<std::shared_ptr<Call>> calls;
    unsigned completed{};
    bool ignoreStop{};
    DecodedAsset decode(std::string_view path, std::stop_token stop) {
        std::unique_lock lock(mutex);
        auto call = std::make_shared<Call>(Call{std::string(path), stop, std::this_thread::get_id()});
        calls.push_back(call); changed.notify_all();
        bool released;
        if (ignoreStop) released = changed.wait_for(lock, std::chrono::seconds(3), [&] { return call->released; });
        else released = changed.wait_for(lock, stop, std::chrono::seconds(3), [&] { return call->released; });
        ++completed; changed.notify_all();
        if (!released) {
            if (stop.stop_requested()) return {{}, {}, "cancelled"};
            throw std::runtime_error("Decoder gate was not released by the test");
        }
        return {Record{"", {{"frames", "4"}, {"channels", "1"}, {"sample-rate", "4"},
                    {"duration", "1"}, {"metadata", "Controlled signed waveform"}, {"zero-crossings", "0.25"},
                    {"path", "must-not-replace.wav"}, {"name", "Must not rename"}, {"region-start", "0.99"}}},
                call->peaks.empty() ? std::vector<Peak>{{call->minimum, .5f}} : call->peaks, call->error};
    }
    AssetDecoder decoder() { return [this](auto path, auto stop) { return decode(path, stop); }; }
    std::shared_ptr<Call> waitFor(unsigned count) {
        std::unique_lock lock(mutex);
        expect(changed.wait_for(lock, std::chrono::seconds(3), [&] { return calls.size() >= count; }),
               "Background decoder starts the expected request");
        return calls[count - 1];
    }
    void release(unsigned index, float minimum = -.75f, std::string error = {}, std::vector<Peak> peaks = {}) {
        std::lock_guard lock(mutex);
        calls.at(index - 1)->minimum = minimum;
        calls.at(index - 1)->error = std::move(error);
        calls.at(index - 1)->peaks = std::move(peaks);
        calls.at(index - 1)->released = true; changed.notify_all();
    }
    void waitCompleted(unsigned count) {
        std::unique_lock lock(mutex);
        expect(changed.wait_for(lock, std::chrono::seconds(3), [&] { return completed >= count; }),
               "Cancelled decoder returns without waiting for the UI thread");
    }
};

void dependentJobsWaitForTheirSource() {
    for (const auto action : {"sample.history-collapse", "autosampler.start", "freeze.start", "analysis.start"}) {
        DecoderGate gate;
        Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
        model.command("patch.load", "kit");
        model.command("autosampler.configure", "", "hi", 36);
        model.command("autosampler.configure", "", "layers", 1);
        expect(model.command("asset.import", "", "dependent.wav"), "Queue the dependent source import");
        gate.waitFor(1);
        const auto source = model.text("selected-source");
        const bool analysis = std::string_view(action) == "analysis.start";
        const bool collapse = std::string_view(action) == "sample.history-collapse";
        const auto state = analysis ? "analysis-state" : collapse ? "collapse-state" : "freeze-state";
        expect(model.command(action, analysis ? "transients" : source), "Accept a job while its source is loading");
        const auto sourceCount = model.records("source").size();
        const auto undoCount = model.undoCount();
        model.tick(1);
        expect(model.text(state) == "running" && model.records("source").size() == sourceCount &&
               model.records("analysis").empty() && model.undoCount() == undoCount &&
               model.find("source", source)->number("frozen") == 0,
               "Source-dependent work remains queued until its decoder publishes a result");
        expect(model.command("asset.select", "keys-source"), "Select an independent source while work is pending");
        const auto independentPeak = model.peaks().front();
        gate.release(1, -.875f, {}, {{0, .1f}, {-.875f, .25f}, {-.125f, .125f}, {-.5f, .75f}});
        until(model, [&] { return model.text(state) == "done"; });
        expect(model.text("selected-source") == "keys-source" &&
               model.peaks().front().minimum == independentPeak.minimum && model.peaks().front().maximum == independentPeak.maximum,
               "Completion preserves an independent source selection and its waveform");
        if (analysis) {
            const auto& candidates = model.records("analysis");
            expect(candidates.size() == 3 && candidates[0].number("end") == .25 &&
                   candidates[1].number("end") == .75 && candidates[2].number("end") == 1,
                   "Transient candidates use the captured source's signed waveform after decoding");
            expect(std::all_of(candidates.begin(), candidates.end(), [&](const auto& row) { return row.text("source") == source; }),
                   "Transient candidates retain the source that requested analysis");
        } else {
            std::string result = source;
            if (std::string_view(action) != "freeze.start") {
                const auto& sources = model.records("source");
                const auto row = std::find_if(sources.begin(), sources.end(), [&](const auto& row) {
                    return row.text(collapse ? "original-source" : "origin") == source;
                });
                expect(row != sources.end() && sources.size() == sourceCount + 1, "Exactly one derived source is published");
                result = row->id;
            } else expect(model.find("source", source)->number("frozen") == 1, "Freeze runs after source decoding completes");
            expect(model.command("asset.select", result) && model.peaks().size() == 4 &&
                   model.peaks()[1].minimum == -.875f && model.peaks()[3].maximum == .75f,
                   "Completed derived work retains the exact signed source waveform");
        }
        expect(model.undoCount() == undoCount + (analysis ? 0 : 1), "Only successful derived work creates a history transaction");
    }
}

void dependentJobsFailWithoutSource() {
    for (const auto action : {"sample.history-collapse", "autosampler.start", "freeze.start", "analysis.start"}) {
        for (const auto failure : {"missing", "unsupported", "removed"}) {
            DecoderGate gate;
            Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
            model.command("asset.import", "", "unavailable.wav"); gate.waitFor(1);
            const auto source = model.text("selected-source");
            const bool analysis = std::string_view(action) == "analysis.start";
            const auto state = analysis ? "analysis-state" : std::string_view(action) == "sample.history-collapse" ? "collapse-state" : "freeze-state";
            expect(model.command(action, analysis ? "transients" : source), "Accept work awaiting a source that may fail");
            model.tick(1);
            if (std::string_view(failure) == "removed") model.command("asset.remove", source);
            else gate.release(1, -.75f, failure);
            const auto sourceCount = model.records("source").size();
            const auto undoCount = model.undoCount();
            const auto historyCount = model.records("history").size();
            until(model, [&] { return model.text(state) == "failed"; });
            expect(model.records("source").size() == sourceCount && model.undoCount() == undoCount &&
                   model.records("history").size() == historyCount && model.records("analysis").empty(),
                   "Unavailable dependencies fail without cached assets, candidates or history mutations");
            expect(!model.text("error").empty(), "Dependency failure reports an actionable failure state");
        }
    }
}

void dependentJobCancellation() {
    for (const auto action : {"analysis.start", "freeze.start", "autosampler.start"}) {
        DecoderGate gate;
        Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
        model.command("asset.import", "", "cancel-dependent.wav"); gate.waitFor(1);
        const bool analysis = std::string_view(action) == "analysis.start";
        const auto state = analysis ? "analysis-state" : "freeze-state";
        model.command(action, analysis ? "transients" : model.text("selected-source"));
        model.tick(1);
        expect(model.command(analysis ? "analysis.cancel" : "freeze.cancel"), "A dependent job remains cancellable while waiting");
        const auto undoCount = model.undoCount();
        gate.release(1);
        until(model, [&] { return model.text("asset-state") == "loaded"; });
        model.tick(1);
        expect(model.text(state) == "idle" && model.records("source").size() == 1 &&
               model.records("analysis").empty() && model.undoCount() == undoCount &&
               model.records("source").front().number("frozen") == 0,
               "Cancelling dependent work does not cancel its source import or resume after loading");
    }
}

void responsiveDecodePreservesEdits() {
    DecoderGate gate;
    Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
    expect(model.command("asset.import", "", "first.wav"), "Accept the import before decoding finishes");
    const auto call = gate.waitFor(1);
    expect(call->thread != std::this_thread::get_id(), "WAV decoding runs on a worker thread");
    const auto source = model.text("selected-source");
    model.tick(10);
    expect(model.text("asset-state") == "loading" && model.peaks().empty(),
           "A UI tick returns while decoding is blocked without inventing a result");
    expect(model.command("sample.set", "region-start", "", .25), "Editing remains available during decoding");
    expect(model.gesture("region-end", 0, 1) && model.gesture("region-end", 1, .75),
           "An editor gesture can remain active while the worker completes");
    expect(model.noteOn(60, 100, "responsive-test"), "The UI can process a note while decoding is blocked");
    model.noteOff(60, "responsive-test");
    const auto history = model.undoCount();
    gate.release(1);
    until(model, [&] { return model.text("asset-state") == "loaded"; });
    expect(model.find("source", source)->number("region-start") == .25 && model.number("region-start") == .25,
           "Decoded metadata preserves source edits made while the worker was running");
    expect(model.find("source", source)->text("path") == "first.wav" && model.find("source", source)->text("name") == "first.wav",
           "Decoder output cannot change source identity, path or authored controls");
    expect(model.gesture("region-end", 3, 0) && model.find("source", source)->text("state") == "loaded" &&
           model.number("region-end") == 1, "Cancelling an in-flight gesture preserves the completed waveform metadata");
    expect(model.peaks().size() == 1 && model.peaks()[0].minimum == -.75f && model.peaks()[0].maximum == .5f,
           "UI publication preserves the decoder's known signed values");
    expect(model.undoCount() == history, "Decoder completion does not create another Undo entry");
    expect(model.command("undo") && model.number("region-start") == 0 && model.text("asset-state") == "loaded",
           "Undoing an intervening edit retains completed sample metadata");
    expect(model.command("redo") && model.number("region-start") == .25 && model.text("asset-state") == "loaded",
           "Redoing an intervening edit retains completed sample metadata");
    expect(model.command("undo") && model.command("undo") && model.records("source").empty(),
           "The whole completed import remains one reversible transaction");
    expect(model.command("redo") && model.text("asset-state") == "loaded" && model.peaks()[0].minimum == -.75f,
           "Redo restores the completed import and cached waveform without another decode");
}

void cancelledUndoCanRedo() {
    DecoderGate gate;
    Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
    expect(model.command("asset.import", "", "undo.wav"), "Queue a cancellable import");
    const auto first = gate.waitFor(1);
    expect(model.command("undo") && model.records("source").empty() && first->stop.stop_requested(),
           "Undo removes the pending import and requests cooperative cancellation");
    gate.waitCompleted(1);
    model.tick(1);
    expect(model.records("source").empty() && model.undoCount() == 0,
           "Cancelled work cannot resurrect an undone source");
    expect(model.command("redo"), "Redo restores the pending import transaction");
    gate.waitFor(2); gate.release(2);
    until(model, [&] { return model.text("asset-state") == "loaded"; });
    expect(model.undoCount() == 1 && model.peaks()[0].minimum == -.75f,
           "Redo queues fresh decoding and completion joins the original import");
}

void replacementHistoryKeepsWaveforms() {
    DecoderGate gate;
    Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
    expect(model.command("asset.import", "", "before.wav"), "Queue the original waveform");
    gate.waitFor(1); gate.release(1, -.25f);
    until(model, [&] { return model.text("asset-state") == "loaded"; });
    const auto source = model.text("selected-source");
    expect(model.command("asset.replace", source, "after.wav"), "Queue its replacement waveform");
    gate.waitFor(2); gate.release(2, -.875f);
    until(model, [&] { return model.text("asset-state") == "loaded"; });
    expect(model.command("undo") && model.find("source", source)->text("path") == "before.wav" &&
           model.peaks()[0].minimum == -.25f, "Undo restores the matching completed waveform, not the replacement cache");
    expect(model.command("redo") && model.find("source", source)->text("path") == "after.wav" &&
           model.peaks()[0].minimum == -.875f, "Redo restores the replacement waveform without rereading its file");
    expect(model.definition().find("_waveform") == std::string::npos, "Private waveform cache revisions are never serialized");
}

void supersededResultCannotPublish() {
    DecoderGate gate; gate.ignoreStop = true;
    Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
    expect(model.command("asset.import", "", "same.wav"), "Queue the original same-path request");
    auto first = gate.waitFor(1);
    const auto source = model.text("selected-source");
    expect(model.command("asset.replace", source, "same.wav") && first->stop.stop_requested(),
           "Replacing even the same path cancels the earlier request identity");
    gate.release(1, -.25f); gate.waitFor(2);
    model.tick(10);
    expect(model.text("asset-state") == "loading" && model.peaks().empty(),
           "An obsolete completion cannot satisfy a later request for the same path");
    gate.release(2, -.875f);
    until(model, [&] { return model.text("asset-state") == "loaded"; });
    expect(model.peaks()[0].minimum == -.875f, "Only the latest request's waveform is published");
    expect(model.command("asset.replace", source, "old-generation.wav"), "Start a request in the old patch");
    auto third = gate.waitFor(3);
    expect(model.command("patch.new") && third->stop.stop_requested(), "A patch change cancels its asset jobs");
    const auto definition = model.definition();
    gate.release(3); gate.waitCompleted(3); model.tick(10);
    expect(model.definition() == definition && model.records("source").empty() && model.undoCount() == 0,
           "An old generation's late completion cannot change the replacement patch");
}

void removalAndQueueCancellation() {
    DecoderGate gate;
    Model model(std::make_shared<SilentMockAdapter>(), gate.decoder());
    std::string paths;
    for (int i = 0; i < 128; ++i) paths += "queued-" + std::to_string(i) + ".wav\n";
    expect(model.command("asset.import-many", "stack", paths), "Accept the existing bounded multi-file import");
    auto first = gate.waitFor(1);
    expect(model.command("patch.new") && first->stop.stop_requested(), "Replacing a patch cancels the active batch");
    expect(model.command("asset.import", "", "replacement.wav"), "A replacement patch can queue fresh work");
    auto fresh = gate.waitFor(2);
    expect(fresh->path == "replacement.wav", "Cancelled queued files are skipped before decoding");
    const auto source = model.text("selected-source");
    expect(model.command("asset.remove", source) && fresh->stop.stop_requested(),
           "Removing a source cancels its pending decode immediately");
    expect(model.command("undo"), "Undo source removal restores its pending import");
    gate.waitFor(3); gate.release(3);
    until(model, [&] { return model.text("asset-state") == "loaded"; });
    expect(model.find("source", source) && model.undoCount() == 1,
           "Restored source loading finishes without an extra history transaction");
}

void shutdownAndDecoderFailure() {
    DecoderGate gate;
    {
        auto model = std::make_unique<Model>(std::make_shared<SilentMockAdapter>(), gate.decoder());
        expect(model->command("asset.import-many", "stack", "active.wav\nqueued.wav"), "Queue active and pending shutdown work");
        auto call = gate.waitFor(1);
        model.reset();
        expect(call->stop.stop_requested() && gate.completed == 1 && gate.calls.size() == 1,
               "Model destruction cancels, joins the worker, and skips queued files");
    }
    Model failed(std::make_shared<SilentMockAdapter>(), [](auto, auto) -> DecodedAsset {
        throw std::runtime_error("Decoder failure");
    });
    expect(failed.command("asset.import", "", "throws.wav"), "Accept an import whose decoder will fail");
    until(failed, [&] { return failed.text("asset-state") == "unsupported"; });
    expect(failed.undoCount() == 1 && !failed.text("error").empty(),
           "A worker exception becomes observable failure without terminating the process or adding history");
}
void definitionQueuesWav() {
    Files files;
    const auto path = files.wav();
    Model source;
    expect(source.command("asset.import", "", path.string()), "Accept a real signed WAV import");
    until(source, [&] { return source.text("asset-state") == "loaded"; });
    Model restored;
    expect(restored.importDefinition(source.definition()), "Restore the serialized sample instrument");
    expect(restored.text("asset-state") == "loading" && restored.peaks().empty(),
           "Definition import queues WAV decoding before returning to its caller");
    expect(restored.undoCount() == 0, "Restoring a definition creates no import Undo entry");
    until(restored, [&] { return restored.text("asset-state") == "loaded"; });
    expect(restored.peaks().size() == 4 && restored.peaks()[2].minimum == -1.f &&
           restored.peaks()[3].maximum == 32767.f / 32768.f,
           "Background restoration publishes the exact signed waveform on a UI tick");
    expect(restored.undoCount() == 0, "Background restoration completion creates no extra Undo entry");
}

void realDecoderFormatsAndFailures() {
    Files files;
    const auto write = [&](std::string name, unsigned format, unsigned bits, unsigned channels,
                           std::vector<unsigned char> data) {
        auto path = files.directory / name;
        std::ofstream out(path, std::ios::binary);
        const auto u16 = [&](unsigned n) { out.put(n & 255); out.put((n >> 8) & 255); };
        const auto u32 = [&](unsigned n) { u16(n & 65535); u16(n >> 16); };
        out.write("RIFF", 4); u32(36 + data.size()); out.write("WAVEfmt ", 8); u32(16);
        u16(format); u16(channels); u32(4); u32(4 * channels * bits / 8); u16(channels * bits / 8); u16(bits);
        out.write("data", 4); u32(data.size());
        out.write(reinterpret_cast<const char*>(data.data()), data.size());
        return path;
    };
    const auto pcm8 = Model::decodeWav(write("eight.wav", 1, 8, 1, {0, 128, 192, 255}).string());
    expect(pcm8.error.empty() && pcm8.peaks.size() == 4 && pcm8.peaks[0].minimum == -1.f &&
           pcm8.peaks[2].maximum == .5f && pcm8.peaks[3].maximum == 127.f / 128.f,
           "Unsigned eight-bit WAV preserves exact signed amplitudes");
    for (const auto bits : {24u, 32u}) {
        std::vector<unsigned char> bytes(bits / 8 * 2);
        bytes[bits / 8 - 1] = 0x80; bytes.back() = 0x40;
        auto decoded = Model::decodeWav(write(std::to_string(bits) + ".wav", 1, bits, 1, bytes).string());
        expect(decoded.error.empty() && decoded.peaks[0].minimum == -1.f && decoded.peaks[1].maximum == .5f,
               "Wide signed PCM WAV preserves negative full scale and positive half scale");
    }
    std::vector<unsigned char> floats;
    for (const auto value : {-.75f, .5f, std::numeric_limits<float>::quiet_NaN(), 2.f}) {
        auto bits = std::bit_cast<std::uint32_t>(value);
        for (unsigned byte = 0; byte < 4; ++byte) floats.push_back((bits >> (byte * 8)) & 255);
    }
    auto floating = Model::decodeWav(write("float.wav", 3, 32, 1, floats).string());
    expect(floating.error.empty() && floating.peaks[0].minimum == -.75f && floating.peaks[1].maximum == .5f &&
           floating.peaks[2].maximum == 0 && floating.peaks[3].maximum == 1.f,
           "Float WAV preserves signed samples, silences NaN and bounds oversized amplitudes");
    auto stereo = Model::decodeWav(write("stereo.wav", 1, 16, 2, {0, 128, 0, 64}).string());
    expect(stereo.error.empty() && stereo.metadata.number("channels") == 2 && stereo.metadata.number("duration") == .25 &&
           stereo.peaks[0].minimum == -1.f && stereo.peaks[0].maximum == .5f,
           "Stereo metadata and waveform include both channels' signed extrema");
    expect(Model::decodeWav((files.directory / "absent.wav").string()).error == "missing", "Missing files retain their distinct failure state");
    expect(Model::decodeWav(files.directory.string()).error == "unsupported", "Nonregular inputs are rejected before opening a stream");
    const auto large = files.wav("too-large.wav"); std::filesystem::resize_file(large, 512u * 1024 * 1024 + 1);
    expect(Model::decodeWav(large.string()).error == "unsupported", "The existing 512 MiB WAV limit remains enforced");
    const auto shortFile = files.directory / "short.wav";
    { std::ofstream(shortFile) << "RIFF"; }
    expect(Model::decodeWav(shortFile.string()).error == "unsupported", "Truncated WAV input fails safely");
    std::stop_source cancelled; cancelled.request_stop();
    expect(Model::decodeWav(files.wav().string(), cancelled.get_token()).error == "cancelled",
           "An already-cancelled real decoder returns without file processing");
    Model model;
    expect(model.command("telemetry.fail", "", "", 1), "Enable the existing simulated engine failure fixture");
    expect(model.command("asset.import", "", files.wav().string()), "Load a real waveform before a failed replacement");
    until(model, [&] { return model.text("asset-state") == "loaded"; });
    expect(model.command("asset.replace", model.text("selected-source"), shortFile.string()), "Accept a replacement before reading it");
    until(model, [&] { return model.text("asset-state") == "unsupported"; });
    expect(model.peaks().size() == 4 && model.peaks()[2].minimum == -1.f,
           "A failed asynchronous replacement retains the previous signed waveform cache");
}
}
int main() {
    try {
        definitionQueuesWav();
        realDecoderFormatsAndFailures();
        dependentJobsWaitForTheirSource();
        dependentJobsFailWithoutSource();
        dependentJobCancellation();
        responsiveDecodePreservesEdits();
        cancelledUndoCanRedo();
        replacementHistoryKeepsWaveforms();
        supersededResultCannotPublish();
        removalAndQueueCancellation();
        shutdownAndDecoderFailure();
        std::cout << "PASS: " << checks << " asynchronous sampler asset checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
