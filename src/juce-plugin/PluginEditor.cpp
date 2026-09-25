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
            "chooseSoundLabReference",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                chooseSoundLabReferenceFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "matchSoundLab",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                matchSoundLabFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "cancelSoundLab",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                cancelSoundLabFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "acceptSoundLabMatch",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                acceptSoundLabMatchFromWeb (arguments, std::move (completion));
            })
        .withNativeFunction (
            "requestGraphProposal",
            [this] (const juce::Array<juce::var>& arguments,
                    juce::WebBrowserComponent::NativeFunctionCompletion completion)
            {
                requestGraphProposalFromWeb (arguments, std::move (completion));
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

    if (path.startsWith ("/sound-lab-reference.wav")
        || path.startsWith ("/sound-lab-candidate.wav"))
    {
        const auto snapshot = soundLabController.snapshot();
        if (snapshot.match == nullptr)
            return std::nullopt;

        const auto& audio = path.startsWith ("/sound-lab-reference.wav")
            ? snapshot.match->referenceWavBytes
            : snapshot.match->candidateWavBytes;
        std::vector<std::byte> bytes (audio.size());
        std::memcpy (bytes.data(), audio.data(), audio.size());
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

void DandrumAudioProcessorEditor::chooseSoundLabReferenceFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (soundLabFileChooser != nullptr)
    {
        completion (juce::var ("A reference-file chooser is already open"));
        return;
    }

    soundLabFileChooser = std::make_unique<juce::FileChooser> (
        "Choose an aligned 48 kHz PCM WAV reference",
        soundLabReferenceFile.existsAsFile()
            ? soundLabReferenceFile.getParentDirectory()
            : juce::File::getSpecialLocation (juce::File::userHomeDirectory),
        "*.wav");
    soundLabFileChooser->launchAsync (
        juce::FileBrowserComponent::openMode | juce::FileBrowserComponent::canSelectFiles,
        [this, completion = std::move (completion)] (const juce::FileChooser& chooser) mutable
        {
            const auto selected = chooser.getResult();
            if (selected.existsAsFile())
                soundLabReferenceFile = selected;
            soundLabFileChooser.reset();
            browser.emitEventIfBrowserIsVisible ("soundLabAnalysisChanged",
                                                 soundLabSnapshotForWeb());
            completion (selected.existsAsFile() ? juce::var (selected.getFileName())
                                                : juce::var());
        });
}

void DandrumAudioProcessorEditor::matchSoundLabFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (! soundLabReferenceFile.existsAsFile())
    {
        completion (juce::var ("Choose a 48 kHz PCM WAV reference first"));
        return;
    }
    if (! soundLabController.startMatch (dandrum::soundDesignFixturePath(),
                                         soundLabReferenceFile.getFullPathName().toStdString()))
    {
        completion (juce::var ("Sound Lab already has offline work in progress"));
        return;
    }
    completion (juce::var());
}

void DandrumAudioProcessorEditor::cancelSoundLabFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    soundLabController.cancelCurrentWork();
    completion (juce::var());
}

void DandrumAudioProcessorEditor::acceptSoundLabMatchFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    const auto snapshot = soundLabController.snapshot();
    if (snapshot.match == nullptr)
    {
        completion (juce::var (
            "A completed or cancelled Sound Lab match is required before acceptance"));
        return;
    }

    const auto patchPath = dandrum::findRepositoryExample ("examples/patches/tb303-acid.yaml");
    if (! processor.reloadInstrumentFromFile (juce::File (juce::String (patchPath.string()))))
    {
        completion (juce::var (processor.getLastLoadError()));
        return;
    }

    for (const auto& value : snapshot.match->parameters)
    {
        auto* parameter = processor.getParameterForPublicId (juce::String (value.id));
        if (parameter == nullptr)
        {
            completion (juce::var ("Matched public parameter is unavailable: "
                                   + juce::String (value.id)));
            return;
        }
        parameter->beginChangeGesture();
        parameter->setValueNotifyingHost (static_cast<float> (value.normalized));
        parameter->endChangeGesture();
    }
    completion (juce::var());
}

void DandrumAudioProcessorEditor::requestGraphProposalFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (! soundLabController.startProposal())
    {
        completion (juce::var (
            "A completed match is required, and Sound Lab must not already be busy"));
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
        case SoundLabController::State::matching: report->setProperty ("state", "matching"); break;
        case SoundLabController::State::matched: report->setProperty ("state", "matched"); break;
        case SoundLabController::State::cancelled: report->setProperty ("state", "cancelled"); break;
        case SoundLabController::State::proposing: report->setProperty ("state", "proposing"); break;
        case SoundLabController::State::proposalReady:
            report->setProperty ("state", "proposal_ready");
            break;
        case SoundLabController::State::error: report->setProperty ("state", "error"); break;
    }

    report->setProperty ("reference_name",
                         soundLabReferenceFile.existsAsFile()
                             ? soundLabReferenceFile.getFileName()
                             : juce::String());
    report->setProperty ("completed_evaluations",
                         static_cast<juce::int64> (snapshot.completedEvaluations));
    report->setProperty ("max_evaluations",
                         static_cast<juce::int64> (snapshot.maxEvaluations));
    report->setProperty ("best_score", snapshot.bestScore);

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

    if (snapshot.match != nullptr)
    {
        report->setProperty ("sample_rate_hz", static_cast<int> (snapshot.match->sampleRateHz));
        report->setProperty (
            "duration_seconds",
            static_cast<double> (snapshot.match->durationFrames) / snapshot.match->sampleRateHz);
        report->setProperty (
            "candidate_audio_url",
            "/sound-lab-candidate.wav?generation="
                + juce::String (static_cast<juce::int64> (snapshot.generation)));
        report->setProperty (
            "reference_audio_url",
            "/sound-lab-reference.wav?generation="
                + juce::String (static_cast<juce::int64> (snapshot.generation)));
        report->setProperty ("manifest",
                             juce::JSON::parse (juce::String (snapshot.match->manifestJson)));

        juce::Array<juce::var> bestParameters;
        bestParameters.ensureStorageAllocated (
            static_cast<int> (snapshot.match->parameters.size()));
        for (const auto& parameter : snapshot.match->parameters)
        {
            auto value = std::make_unique<juce::DynamicObject>();
            value->setProperty ("id", juce::String (parameter.id));
            value->setProperty ("min", parameter.min);
            value->setProperty ("max", parameter.max);
            value->setProperty ("initial", parameter.initial);
            value->setProperty ("best", parameter.best);
            value->setProperty ("normalized", parameter.normalized);
            bestParameters.add (juce::var (value.release()));
        }
        report->setProperty ("best_parameters", juce::var (bestParameters));

        juce::Array<juce::var> comparisonMetrics;
        comparisonMetrics.ensureStorageAllocated (
            static_cast<int> (snapshot.match->metrics.size()));
        for (const auto& metric : snapshot.match->metrics)
        {
            auto frame = std::make_unique<juce::DynamicObject>();
            frame->setProperty ("time_seconds", metric.timeSeconds);
            frame->setProperty ("reference_rms", metric.referenceRms);
            frame->setProperty ("candidate_rms", metric.candidateRms);
            frame->setProperty (
                "reference_spectral_centroid_hz",
                metric.referenceHasSpectralCentroid
                    ? juce::var (metric.referenceSpectralCentroidHz)
                    : juce::var());
            frame->setProperty (
                "candidate_spectral_centroid_hz",
                metric.candidateHasSpectralCentroid
                    ? juce::var (metric.candidateSpectralCentroidHz)
                    : juce::var());
            comparisonMetrics.add (juce::var (frame.release()));
        }
        report->setProperty ("comparison_metrics", juce::var (comparisonMetrics));
    }

    if (snapshot.proposal != nullptr)
    {
        auto proposal = std::make_unique<juce::DynamicObject>();
        proposal->setProperty ("provider_id", juce::String (snapshot.proposal->providerId));
        proposal->setProperty ("patch_name", juce::String (snapshot.proposal->patchName));
        proposal->setProperty ("explanation", juce::String (snapshot.proposal->explanation));
        proposal->setProperty ("patch_yaml", juce::String (snapshot.proposal->patchYaml));
        juce::Array<juce::var> searchParameters;
        for (const auto& parameter : snapshot.proposal->suggestedSearchParameters)
            searchParameters.add (juce::String (parameter));
        proposal->setProperty ("suggested_search_parameters", juce::var (searchParameters));
        report->setProperty ("proposal", juce::var (proposal.release()));
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
