#include "PluginEditor.h"
#include "Tb303WebUi.h"

#include <cstring>
#include <string>
#include <vector>

namespace
{
std::vector<std::byte> toBytes (const char* text)
{
    const auto length = std::char_traits<char>::length (text);
    std::vector<std::byte> bytes (length);
    std::memcpy (bytes.data(), text, length);
    return bytes;
}

constexpr auto nativeFunctionBootstrap = R"JS(
(() => {
  const backend = window.__JUCE__.backend;
  let nextPromiseId = 0;
  const pending = new Map();
  backend.addEventListener('__juce__complete', ({ promiseId, result }) => {
    const entry = pending.get(promiseId);
    if (!entry) return;
    pending.delete(promiseId);
    entry.resolve(result);
  });
  backend.getNativeFunction = name => (...params) => {
    const resultId = nextPromiseId++;
    const promise = new Promise((resolve, reject) => pending.set(resultId, { resolve, reject }));
    backend.emitEvent('__juce__invoke', { name, params, resultId });
    return promise;
  };
})();
)JS";
}

DandrumAudioProcessorEditor::DandrumAudioProcessorEditor (DandrumAudioProcessor& processorToUse)
    : juce::AudioProcessorEditor (&processorToUse),
      processor (processorToUse),
      browser (createBrowserOptions())
{
    addAndMakeVisible (browser);
    setResizable (true, true);
    setResizeLimits (760, 560, 1500, 1100);
    setSize (1180, 860);
    browser.goToURL (juce::WebBrowserComponent::getResourceProviderRoot());
    lastSeenParameterSurfaceGeneration = processor.getParameterSurfaceGeneration();
    lastSeenSoundLabGeneration = soundLabController.generation();
    startTimerHz (12);
}

DandrumAudioProcessorEditor::~DandrumAudioProcessorEditor() = default;

void DandrumAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (juce::Colour (0xff111111));
}

void DandrumAudioProcessorEditor::resized()
{
    browser.setBounds (getLocalBounds());
}

juce::WebBrowserComponent::Options DandrumAudioProcessorEditor::createBrowserOptions()
{
    using Options = juce::WebBrowserComponent::Options;

    auto options = Options{}
        .withNativeIntegrationEnabled()
        .withKeepPageLoadedWhenBrowserIsHidden()
        .withUserScript (nativeFunctionBootstrap)
        .withNativeFunction (
            "setParameter",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                setParameterFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "getParameters",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                getParametersForWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "noteOn",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                noteOnFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "noteOff",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                noteOffFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "renderSoundLab",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                renderSoundLabFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "getSoundLabAnalysis",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                getSoundLabAnalysisForWeb (arguments, std::move (completion));
            });

   #if JUCE_WINDOWS
    options = options
        .withBackend (Options::Backend::webview2)
        .withWinWebView2Options (
            Options::WinWebView2{}
                .withUserDataFolder (juce::File::getSpecialLocation (juce::File::tempDirectory)
                                         .getChildFile ("dandrum-webview2"))
                .withStatusBarDisabled()
                .withBuiltInErrorPageDisabled()
                .withBackgroundColour (juce::Colour (0xff111111)));
   #endif

   #if JUCE_WEB_BROWSER_RESOURCE_PROVIDER_AVAILABLE
    options = options.withResourceProvider (
        [this] (const juce::String& path)
        {
            return provideResource (path);
        });
   #endif

    return options;
}

std::optional<juce::WebBrowserComponent::Resource>
DandrumAudioProcessorEditor::provideResource (const juce::String& path) const
{
    if (path == "/" || path == "/index.html")
        return juce::WebBrowserComponent::Resource { toBytes (Tb303WebUi::indexHtml), "text/html" };

    if (path.startsWith ("/sound-lab.wav"))
    {
        const auto snapshot = soundLabController.snapshot();
        if (snapshot.state != SoundLabController::State::ready || snapshot.data == nullptr)
            return std::nullopt;

        std::vector<std::byte> bytes (snapshot.data->wavBytes.size());
        std::memcpy (bytes.data(), snapshot.data->wavBytes.data(), snapshot.data->wavBytes.size());
        return juce::WebBrowserComponent::Resource { std::move (bytes), "audio/wav" };
    }

    return std::nullopt;
}

void DandrumAudioProcessorEditor::setParameterFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.size() < 2)
    {
        completion (juce::var ("setParameter expects parameter id and normalised value"));
        return;
    }

    const auto publicId = arguments[0].toString();
    auto* parameter = processor.getParameterForPublicId (publicId);
    if (parameter == nullptr)
    {
        completion (juce::var ("Unknown public parameter: " + publicId));
        return;
    }

    const auto normalised = juce::jlimit (0.0f, 1.0f, static_cast<float> (arguments[1]));
    parameter->beginChangeGesture();
    parameter->setValueNotifyingHost (normalised);
    parameter->endChangeGesture();
    completion (juce::var());
}

void DandrumAudioProcessorEditor::getParametersForWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    completion (parameterSnapshotForWeb());
}

void DandrumAudioProcessorEditor::noteOnFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.size() < 2)
    {
        completion (juce::var ("noteOn expects MIDI note and normalised velocity"));
        return;
    }

    if (! processor.enqueueEditorNoteOn (static_cast<int> (arguments[0]),
                                         static_cast<float> (arguments[1])))
    {
        completion (juce::var ("Editor MIDI queue is full; note-on was dropped"));
        return;
    }

    completion (juce::var());
}

void DandrumAudioProcessorEditor::noteOffFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.isEmpty())
    {
        completion (juce::var ("noteOff expects a MIDI note"));
        return;
    }

    if (! processor.enqueueEditorNoteOff (static_cast<int> (arguments[0])))
    {
        completion (juce::var ("Editor MIDI queue is full; note-off was dropped"));
        return;
    }

    completion (juce::var());
}

void DandrumAudioProcessorEditor::renderSoundLabFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (! soundLabController.startRender (dandrum::soundDesignFixturePath()))
    {
        completion (juce::var ("Sound Lab is already rendering"));
        return;
    }

    completion (juce::var());
}

void DandrumAudioProcessorEditor::getSoundLabAnalysisForWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    completion (soundLabSnapshotForWeb());
}

juce::var DandrumAudioProcessorEditor::parameterSnapshotForWeb() const
{
    juce::Array<juce::var> result;

    for (const auto& publicId : processor.getActivePublicParameterIds())
    {
        auto* parameter = processor.getParameterForPublicId (publicId);
        if (parameter == nullptr)
            continue;

        auto object = std::make_unique<juce::DynamicObject>();
        object->setProperty ("id", publicId);
        object->setProperty ("name", processor.getPublicParameterDisplayName (publicId));
        object->setProperty ("value", parameter->getValue());
        result.add (juce::var (object.release()));
    }

    return juce::var (result);
}

juce::var DandrumAudioProcessorEditor::soundLabSnapshotForWeb() const
{
    const auto snapshot = soundLabController.snapshot();
    auto report = std::make_unique<juce::DynamicObject>();
    report->setProperty ("generation", static_cast<juce::int64> (snapshot.generation));

    switch (snapshot.state)
    {
        case SoundLabController::State::idle: report->setProperty ("state", "idle"); break;
        case SoundLabController::State::rendering: report->setProperty ("state", "rendering"); break;
        case SoundLabController::State::ready: report->setProperty ("state", "ready"); break;
        case SoundLabController::State::error: report->setProperty ("state", "error"); break;
    }

    if (! snapshot.error.empty())
        report->setProperty ("error", juce::String (snapshot.error));

    if (snapshot.data != nullptr)
    {
        report->setProperty ("sample_rate_hz", static_cast<int> (snapshot.data->sampleRateHz));
        report->setProperty (
            "duration_seconds",
            static_cast<double> (snapshot.data->durationFrames) / snapshot.data->sampleRateHz);
        report->setProperty (
            "audio_url",
            "/sound-lab.wav?generation=" + juce::String (static_cast<juce::int64> (snapshot.generation)));

        juce::Array<juce::var> metrics;
        metrics.ensureStorageAllocated (static_cast<int> (snapshot.data->metrics.size()));
        for (const auto& metric : snapshot.data->metrics)
        {
            auto frame = std::make_unique<juce::DynamicObject>();
            frame->setProperty ("time_seconds", metric.timeSeconds);
            frame->setProperty ("rms", metric.rms);
            frame->setProperty ("peak", metric.peak);
            frame->setProperty (
                "spectral_centroid_hz",
                metric.hasSpectralCentroid ? juce::var (metric.spectralCentroidHz) : juce::var());
            metrics.add (juce::var (frame.release()));
        }
        report->setProperty ("metrics", juce::var (metrics));
    }

    return juce::var (report.release());
}

void DandrumAudioProcessorEditor::timerCallback()
{
    const auto generation = processor.getParameterSurfaceGeneration();
    if (generation != lastSeenParameterSurfaceGeneration)
    {
        lastSeenParameterSurfaceGeneration = generation;
        browser.refresh();
        return;
    }

    browser.emitEventIfBrowserIsVisible ("parameterValuesChanged", parameterSnapshotForWeb());

    const auto soundLabGeneration = soundLabController.generation();
    if (soundLabGeneration != lastSeenSoundLabGeneration)
    {
        lastSeenSoundLabGeneration = soundLabGeneration;
        browser.emitEventIfBrowserIsVisible ("soundLabAnalysisChanged", soundLabSnapshotForWeb());
    }
}
