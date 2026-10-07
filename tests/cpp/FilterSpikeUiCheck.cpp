#include "FilterProcessor.h"
#include "FilterViewModel.h"
#if defined(FILTER_SPIKE_SLINT)
#include "SlintChecks.h"
#elif defined(FILTER_SPIKE_JIVE)
#include "JiveChecks.h"
#else
#error Select a filter spike backend
#endif
#include <algorithm>
#include <atomic>
#include <chrono>
#include <cmath>
#include <iostream>
#include <memory>
#include <set>
#include <stdexcept>
#include <thread>
#include <vector>

namespace {
using namespace filter_spike;
constexpr int frames = 64;
constexpr double sampleRate = 48000;
#if defined(FILTER_SPIKE_SLINT)
constexpr auto backend = "slint";
constexpr auto checkInteractions = checkSlintInteractions;
#else
constexpr auto backend = "jive";
constexpr auto checkInteractions = checkJiveInteractions;
#endif
void require(bool condition, const juce::String& reason) {
    if (!condition) throw std::runtime_error(reason.toStdString());
}
void pump(int milliseconds) {
    const auto end = std::chrono::steady_clock::now() + std::chrono::milliseconds(milliseconds);
    while (std::chrono::steady_clock::now() < end)
        juce::MessageManager::getInstance()->runDispatchLoopUntil(10);
}
struct Gestures final : juce::AudioProcessorListener {
    void audioProcessorParameterChanged(juce::AudioProcessor*, int, float) override { ++changes; }
    void audioProcessorChanged(juce::AudioProcessor*, const ChangeDetails&) override {}
    void audioProcessorParameterChangeGestureBegin(juce::AudioProcessor*, int index) override {
        ++begins; if (index >= 0 && index < parameterCount) ++open[size_t(index)]; else valid = false;
    }
    void audioProcessorParameterChangeGestureEnd(juce::AudioProcessor*, int index) override {
        ++ends; if (index >= 0 && index < parameterCount && open[size_t(index)] > 0) --open[size_t(index)]; else valid = false;
    }
    bool balanced() const { return valid && begins == ends && std::all_of(open.begin(), open.end(), [](int n) { return n == 0; }); }
    int begins = 0, ends = 0, changes = 0;
    bool valid = true;
    std::array<int, parameterCount> open {};
};
struct Instance {
    Instance() { processor.setPlayConfigDetails(2, 2, sampleRate, frames); processor.prepareToPlay(sampleRate, frames); processor.addListener(&gestures); open(); }
    ~Instance() { editor.reset(); processor.removeListener(&gestures); processor.releaseResources(); }
    void open() {
        editor.reset(processor.createEditor()); require(editor != nullptr, "Editor factory returned null");
        editor->setSize(900, 700); editor->addToDesktop(juce::ComponentPeer::windowIsTemporary); editor->setVisible(true);
    }
    std::array<float, parameterCount> state() const {
        std::array<float, parameterCount> result {};
        for (int i = 0; i < parameterCount; ++i) result[size_t(i)] = processor.normalized(i);
        return result;
    }
    FilterProcessor processor;
    Gestures gestures;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
};
class AudioRun {
public:
    AudioRun(std::vector<std::unique_ptr<Instance>>& owners, bool active) : worker([this, &owners, active] {
        try {
            juce::AudioBuffer<float> buffer(2, frames); juce::MidiBuffer midi;
            auto next = std::chrono::steady_clock::now(); uint64_t position = 0; uint32_t noise = 0x12345678;
            const auto period = std::chrono::duration_cast<std::chrono::steady_clock::duration>(std::chrono::duration<double>(frames / sampleRate));
            while (running.load()) {
                std::array<float, frames> left {}, right {};
                for (int i = 0; i < frames; ++i, ++position) {
                    noise ^= noise << 13; noise ^= noise >> 17; noise ^= noise << 5;
                    const double phase = double(position) / sampleRate;
                    const float random = .03f * (float(noise) / float(UINT32_MAX) * 2 - 1);
                    if (active) { left[size_t(i)] = .12f * float(std::sin(phase * 440 * juce::MathConstants<double>::twoPi)) + random;
                        right[size_t(i)] = .09f * float(std::sin(phase * 880 * juce::MathConstants<double>::twoPi)) - random; }
                }
                for (auto& owner : owners) {
                    buffer.copyFrom(0, 0, left.data(), frames); buffer.copyFrom(1, 0, right.data(), frames);
                    owner->processor.processBlock(buffer, midi);
                    for (int channel = 0; channel < 2; ++channel) for (int i = 0; i < frames; ++i) {
                        const auto value = buffer.getSample(channel, i);
                        if (!std::isfinite(value)) healthy.store(false);
                        peak.store(std::max(peak.load(), std::abs(value)));
                    }
                }
                ++blocks; next += period;
                if (std::chrono::steady_clock::now() > next + period) { ++overruns; next = std::chrono::steady_clock::now(); }
                std::this_thread::sleep_until(next);
            }
        } catch (...) { healthy.store(false); }
    }) {}
    ~AudioRun() { stop(); }
    void stop() { running.store(false); if (worker.joinable()) worker.join(); }
    std::atomic<uint64_t> blocks {0}, overruns {0};
    std::atomic<float> peak {0};
    std::atomic<bool> healthy {true};
private:
    std::atomic<bool> running {true};
    std::thread worker;
};
void interact(Instance& instance) {
    instance.processor.setNormalized(bellFrequency, .35f); instance.processor.setNormalized(bellGain, .5f); pump(70);
    const auto starts = instance.gestures.begins, changes = instance.gestures.changes;
    juce::String failure; require(checkInteractions(instance.processor, *instance.editor, failure), failure);
    require(instance.gestures.begins > starts && instance.gestures.changes > changes && instance.gestures.balanced(), "Backend interaction did not balance host gestures/change notifications");
    require(std::abs(instance.processor.normalized(bellFrequency) - .35f) > .001f, "Backend did not change host frequency");
}
juce::var snapshot(Instance& instance, const juce::File& directory, int index, int width, int height) {
    instance.editor->setSize(width, height); pump(90);
    const auto image = instance.editor->createComponentSnapshot(instance.editor->getLocalBounds());
    require(image.isValid() && image.getWidth() == width && image.getHeight() == height, "Invalid snapshot size");
    std::set<juce::uint32> colours;
    for (int y = 0; y < height; y += 7) for (int x = 0; x < width; x += 7) colours.insert(image.getPixelAt(x, y).getARGB());
    require(colours.size() > 16, "Snapshot is blank or lacks rendered content");
    const auto file = directory.getChildFile(juce::String(backend) + "-instance-" + juce::String(index + 1) + "-" + juce::String(width) + "x" + juce::String(height) + ".png");
    auto stream = file.createOutputStream(); juce::PNGImageFormat png;
    require(stream != nullptr && stream->setPosition(0) && stream->truncate().wasOk() && png.writeImageToStream(image, *stream), "Cannot write PNG snapshot");
    auto* result = new juce::DynamicObject(); result->setProperty("file", file.getFileName());
    result->setProperty("width", width); result->setProperty("height", height); result->setProperty("sampledColours", int(colours.size())); return result;
}
juce::var rasterCost(Instance& instance) {
    instance.editor->setSize(900, 700); pump(90);
    std::vector<double> elapsed; juce::Array<juce::var> samples;
    // No message pump between samples: measure full raster cost, not timer work or screen latency.
    for (int i = 0; i < 200; ++i) {
        const auto start = std::chrono::steady_clock::now();
        const auto image = instance.editor->createComponentSnapshot(instance.editor->getLocalBounds());
        const auto ms = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start).count();
        require(image.isValid(), "Raster measurement produced an invalid image"); elapsed.push_back(ms); samples.add(ms);
    }
    std::sort(elapsed.begin(), elapsed.end()); auto* cost = new juce::DynamicObject();
    cost->setProperty("scope", "Offscreen full createComponentSnapshot raster; timers not pumped between samples; no screen latency claim");
    cost->setProperty("width", 900); cost->setProperty("height", 700); cost->setProperty("samples", samples);
    cost->setProperty("medianMs", (elapsed[99] + elapsed[100]) / 2); cost->setProperty("p95Ms", elapsed[189]); cost->setProperty("maxMs", elapsed.back()); return cost;
}
}

int main(int argc, char** argv) {
    auto* result = new juce::DynamicObject(); juce::var metrics(result);
    result->setProperty("schemaVersion", 1); result->setProperty("backend", backend);
    juce::File directory; int status = 1;
    try {
        require(argc == 2 || argc == 5, "Usage: check <output-directory> [--measure active|idle <editor-count>]");
        const bool measure = argc == 5; const juce::String mode(measure ? argv[3] : "active");
        require(!measure || (juce::String(argv[2]) == "--measure" && (mode == "active" || mode == "idle")), "Invalid measurement mode");
        require(!measure || juce::String(argv[4]).containsOnly("0123456789"), "Editor count must be an integer");
        const int count = measure ? juce::String(argv[4]).getIntValue() : 2;
        require(count >= 1 && count <= 8, "Editor count must be between 1 and 8");
        directory = juce::File::getCurrentWorkingDirectory().getChildFile(argv[1]); require(directory.createDirectory().wasOk(), "Cannot create output directory");
        juce::ScopedJuceInitialiser_GUI gui;
        juce::Component nativeHost; nativeHost.setSize(1, 1); nativeHost.addToDesktop(juce::ComponentPeer::windowIsTemporary);
        nativeHost.setVisible(true); pump(20); nativeHost.setVisible(false);
        std::vector<std::unique_ptr<Instance>> owners;
        for (int i = 0; i < count; ++i) owners.push_back(std::make_unique<Instance>());
        AudioRun audio(owners, mode == "active"); pump(350);
        if (!measure) {
            auto other = owners[1]->state(); interact(*owners[0]); require(owners[1]->state() == other, "First editor changed second processor state");
            const auto first = owners[0]->state(); interact(*owners[1]); require(owners[0]->state() == first, "Second editor changed first processor state");
            other = owners[1]->state(); const auto preserved = owners[0]->state(); owners[0]->editor.reset(); pump(30); owners[0]->open(); pump(90);
            require(owners[0]->state() == preserved, "Reopened editor reset processor state"); interact(*owners[0]);
            require(owners[1]->state() == other, "Reopened editor changed independent processor state");
        }
        result->setProperty("mode", mode); result->setProperty("editorCount", count); result->setProperty("sampleRate", sampleRate); result->setProperty("blockFrames", frames);
        if (measure) {
            result->setProperty("status", "measuring"); result->setProperty("measurementSeconds", 20);
            std::cout << juce::JSON::toString(metrics, true) << std::endl;
            const auto start = std::chrono::steady_clock::now(); pump(20000);
            result->setProperty("measurementElapsedSeconds", std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count());
        }
        juce::Array<juce::var> images, gestures, raster;
        for (int i = 0; i < count; ++i) {
            raster.add(rasterCost(*owners[size_t(i)]));
            images.add(snapshot(*owners[size_t(i)], directory, i, 900, 700)); images.add(snapshot(*owners[size_t(i)], directory, i, 1080, 760));
            const auto& listener = owners[size_t(i)]->gestures; require(listener.balanced(), "Editor retained an open host gesture");
            auto* record = new juce::DynamicObject(); record->setProperty("begins", listener.begins); record->setProperty("ends", listener.ends); record->setProperty("changes", listener.changes); gestures.add(record);
        }
        audio.stop(); require(audio.healthy.load() && audio.blocks.load() >= 16, "Audio worker failed or rendered fewer than 16 blocks");
        require(mode == "idle" || audio.peak.load() > .00001f, "Real processor rendered silence for active input");
        result->setProperty("snapshots", images); result->setProperty("gestures", gestures); result->setProperty("rasterCosts", raster); result->setProperty("blocksPerProcessor", juce::int64(audio.blocks.load()));
        result->setProperty("audioPeak", audio.peak.load()); result->setProperty("audioFinite", audio.healthy.load()); result->setProperty("audioOverruns", juce::int64(audio.overruns.load()));
        result->setProperty("scope", "Native harness rendering and backend interactions; no DAW deployment or production readiness claim");
        result->setProperty("status", "pass"); status = 0;
    } catch (const std::exception& error) { result->setProperty("status", "fail"); result->setProperty("error", error.what()); }
    auto json = juce::JSON::toString(metrics, true);
    if (directory.isDirectory() && !directory.getChildFile("metrics.json").replaceWithText(json)) {
        result->setProperty("status", "fail"); result->setProperty("error", "Cannot write metrics.json"); status = 1; json = juce::JSON::toString(metrics, true);
    }
    std::cout << json << std::endl;
    return status;
}
