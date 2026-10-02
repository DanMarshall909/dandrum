#include "PluginEditor.h"
#include "GenericWebUi.h"
#include "KickWebUi.h"
#include "SamplerWebUi.h"
#include "SoundLabWebUi.h"
#include "Tb303WebUi.h"

#if defined(DANDRUM_EMBED_TB303_REACT)
#include "Tb303ReactBinaryData.h"
#endif

#if defined(DANDRUM_EMBED_SAMPLER_REACT)
#include "SamplerReactBinaryData.h"
#endif

#include <cstring>
#include <cmath>
#include <string>
#include <string_view>
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

#if defined(DANDRUM_EMBED_TB303_REACT)
std::optional<juce::WebBrowserComponent::Resource> tb303ReactResource (const juce::String& path)
{
    const char* name = nullptr;
    const char* mime = nullptr;
    if (path == "/" || path == "/index.html")
    {
        name = "index_html";
        mime = "text/html";
    }
    else if (path == "/app.js")
    {
        name = "app_js";
        mime = "text/javascript";
    }
    else if (path == "/app.css")
    {
        name = "app_css";
        mime = "text/css";
    }
    if (name == nullptr)
        return std::nullopt;

    int size = 0;
    const auto* data = Tb303ReactBinaryData::getNamedResource (name, size);
    if (data == nullptr || size <= 0)
        return std::nullopt;
    std::vector<std::byte> bytes (static_cast<std::size_t> (size));
    std::memcpy (bytes.data(), data, bytes.size());
    return juce::WebBrowserComponent::Resource { std::move (bytes), mime };
}
#endif

#if defined(DANDRUM_EMBED_SAMPLER_REACT)
std::optional<juce::WebBrowserComponent::Resource> samplerReactResource (const juce::String& path)
{
    const char* name = nullptr;
    const char* mime = nullptr;
    if (path == "/" || path == "/index.html")
    {
        name = "index_html";
        mime = "text/html";
    }
    else if (path == "/app.js")
    {
        name = "app_js";
        mime = "text/javascript";
    }
    else if (path == "/app.css")
    {
        name = "app_css";
        mime = "text/css";
    }
    if (name == nullptr)
        return std::nullopt;

    int size = 0;
    const auto* data = SamplerReactBinaryData::getNamedResource (name, size);
    if (data == nullptr || size <= 0)
        return std::nullopt;
    std::vector<std::byte> bytes (static_cast<std::size_t> (size));
    std::memcpy (bytes.data(), data, bytes.size());
    return juce::WebBrowserComponent::Resource { std::move (bytes), mime };
}
#endif

void replaceMarker (std::string& page, std::string_view marker, std::string_view replacement)
{
    const auto position = page.find (marker);
    if (position != std::string::npos)
        page.replace (position, marker.size(), replacement);
}

std::string composePage (std::string page, bool soundLabEnabled)
{
    replaceMarker (page, "<!--sound-lab-panel-->",
                   soundLabEnabled ? SoundLabWebUi::panelHtml : "");
    replaceMarker (page, "/*sound-lab-style*/",
                   soundLabEnabled ? SoundLabWebUi::css : "");
    replaceMarker (page, "<!--sound-lab-script-->",
                   soundLabEnabled ? "<script src=\"/sound-lab-ui.js\"></script>" : "");
    return page;
}

std::string pageForInstrument (const std::string& instrumentId)
{
    if (instrumentId == "dandrum.tb303-acid")
        return Tb303WebUi::indexHtml;
    if (instrumentId == "dandrum.synthetic-808-kick")
        return KickWebUi::indexHtml;
    if (instrumentId == "dandrum.advanced-drum-kit")
        return SamplerWebUi::indexHtml;
    return GenericWebUi::indexHtml;
}

bool hasExpectedSoundLabGeneration (const juce::String& path,
                                    std::uint64_t generation)
{
    const auto marker = juce::String ("?generation=");
    const auto markerIndex = path.indexOf (marker);
    if (markerIndex < 0)
        return false;

    return path.substring (markerIndex + marker.length())
           == juce::String (static_cast<juce::int64> (generation));
}

juce::var acceptedAnalysisJob (const SoundLabController::Snapshot& snapshot)
{
    auto result = std::make_unique<juce::DynamicObject>();
    result->setProperty ("status", "accepted");
    result->setProperty ("job_id", static_cast<juce::int64> (snapshot.jobId));
    result->setProperty ("instrument_generation",
                         static_cast<juce::int64> (snapshot.instrumentGeneration));
    return juce::var (result.release());
}
}

DandrumAudioProcessorEditor::DandrumAudioProcessorEditor (DandrumAudioProcessor& processorToUse)
    : juce::AudioProcessorEditor (&processorToUse),
      processor (processorToUse),
      hostBridge (processorToUse),
      soundLabController (processorToUse.getSoundLabController()),
      soundLabReferenceFile (processorToUse.getSoundLabReferenceFile()),
      browser (createBrowserOptions())
{
    addAndMakeVisible (browser);
    setName (juce::String (processor.demoConfiguration().title));
    setResizable (true, true);
    setResizeLimits (760, 560, 1500, 1100);
    setSize (1180, 860);
    browser.goToURL (juce::WebBrowserComponent::getResourceProviderRoot());
    if (soundLabController != nullptr)
        lastSeenSoundLabGeneration = soundLabController->generation();
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

    auto options = hostBridge.addNativeFunctions (Options{}
        .withNativeIntegrationEnabled()
        .withKeepPageLoadedWhenBrowserIsHidden()
        .withUserScript (InstrumentHostWebBridge::bootstrapScript()));

    if (soundLabController != nullptr)
        options = options.withNativeFunction (
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
#if defined(DANDRUM_EMBED_SAMPLER_REACT)
    if (processor.demoConfiguration().instrumentId == "dandrum.advanced-drum-kit")
        if (auto resource = samplerReactResource (path))
            return resource;
#endif
#if defined(DANDRUM_EMBED_TB303_REACT)
    if (processor.demoConfiguration().instrumentId == "dandrum.tb303-acid")
        if (auto resource = tb303ReactResource (path))
            return resource;
#endif
    const bool soundLabEnabled = soundLabController != nullptr
                                 && processor.isSoundLabInstrumentCompatible();
    if (path == "/sound-lab-ui.js" && soundLabEnabled)
        return juce::WebBrowserComponent::Resource {
            toBytes (SoundLabWebUi::script), "text/javascript" };

    if (auto shared = hostBridge.provideResource (
            path, composePage (pageForInstrument (
                     processor.demoConfiguration().instrumentId), soundLabEnabled)))
        return shared;

    if (! soundLabEnabled)
        return std::nullopt;

    if (path.startsWith ("/sound-lab.wav"))
    {
        const auto snapshot = soundLabController->snapshot();
        if (! hasExpectedSoundLabGeneration (path, snapshot.generation)
            || snapshot.instrumentGeneration != processor.getParameterSurfaceGeneration()
            || snapshot.state != SoundLabController::State::ready || snapshot.data == nullptr)
            return std::nullopt;

        std::vector<std::byte> bytes (snapshot.data->wavBytes.size());
        std::memcpy (bytes.data(), snapshot.data->wavBytes.data(), snapshot.data->wavBytes.size());
        return juce::WebBrowserComponent::Resource { std::move (bytes), "audio/wav" };
    }

    if (path.startsWith ("/sound-lab-reference.wav")
        || path.startsWith ("/sound-lab-candidate.wav"))
    {
        const auto snapshot = soundLabController->snapshot();
        if (! hasExpectedSoundLabGeneration (path, snapshot.generation)
            || snapshot.instrumentGeneration != processor.getParameterSurfaceGeneration()
            || snapshot.match == nullptr)
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

void DandrumAudioProcessorEditor::renderSoundLabFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (! processor.isSoundLabInstrumentCompatible())
    {
        completion (juce::var ("Sound Lab fixture does not match the active instrument"));
        return;
    }
    const auto& fixture = processor.demoConfiguration().soundLabFixturePath;
    if (! soundLabController->startRender (*fixture, processor.getParameterSurfaceGeneration()))
    {
        completion (juce::var ("Sound Lab is already rendering"));
        return;
    }

    completion (acceptedAnalysisJob (soundLabController->snapshot()));
}

void DandrumAudioProcessorEditor::chooseSoundLabReferenceFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (! processor.isSoundLabInstrumentCompatible())
    {
        completion (juce::var ("Sound Lab fixture does not match the active instrument"));
        return;
    }
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
            {
                if (! soundLabController->discardResults())
                {
                    soundLabFileChooser.reset();
                    completion (juce::var ("Sound Lab is busy"));
                    return;
                }
                soundLabReferenceFile = selected;
            }
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
    if (! processor.isSoundLabInstrumentCompatible())
    {
        completion (juce::var ("Sound Lab fixture does not match the active instrument"));
        return;
    }
    const auto& fixture = processor.demoConfiguration().soundLabFixturePath;
    if (! soundLabReferenceFile.existsAsFile())
    {
        completion (juce::var ("Choose a 48 kHz PCM WAV reference first"));
        return;
    }
    if (! soundLabController->startMatch (*fixture,
                                         soundLabReferenceFile.getFullPathName().toStdString(),
                                         processor.getParameterSurfaceGeneration()))
    {
        completion (juce::var ("Sound Lab already has offline work in progress"));
        return;
    }
    completion (acceptedAnalysisJob (soundLabController->snapshot()));
}

void DandrumAudioProcessorEditor::cancelSoundLabFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (! processor.isSoundLabInstrumentCompatible())
    {
        completion (juce::var ("Sound Lab fixture does not match the active instrument"));
        return;
    }
    soundLabController->cancelCurrentWork();
    completion (juce::var());
}

void DandrumAudioProcessorEditor::acceptSoundLabMatchFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (! processor.isSoundLabInstrumentCompatible())
    {
        completion (juce::var ("Sound Lab fixture does not match the active instrument"));
        return;
    }
    const auto snapshot = soundLabController->snapshot();
    if (snapshot.jobId != 0
        && snapshot.instrumentGeneration != processor.getParameterSurfaceGeneration())
    {
        completion (juce::var ("Rejected stale analysis job"));
        return;
    }
    if (snapshot.match == nullptr)
    {
        completion (juce::var (
            "A completed or cancelled Sound Lab match is required before acceptance"));
        return;
    }

    const auto& matchSource = processor.demoConfiguration().matchSourcePath;
    if (! matchSource.has_value())
    {
        completion (juce::var ("Sound Lab match source is not configured for this demo"));
        return;
    }
    if (! processor.reloadInstrumentFromYaml (
            juce::String::fromUTF8 (snapshot.match->patchYaml.data(),
                                    static_cast<int> (snapshot.match->patchYaml.size())),
            juce::File (juce::String (matchSource->string()))))
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
    if (! processor.isSoundLabInstrumentCompatible())
    {
        completion (juce::var ("Sound Lab fixture does not match the active instrument"));
        return;
    }
    const auto analysis = soundLabController->snapshot();
    if (analysis.jobId != 0
        && analysis.instrumentGeneration != processor.getParameterSurfaceGeneration())
    {
        completion (juce::var ("Rejected stale analysis job"));
        return;
    }
    if (! soundLabController->startProposal())
    {
        completion (juce::var (
            "A completed match is required, and Sound Lab must not already be busy"));
        return;
    }
    completion (acceptedAnalysisJob (soundLabController->snapshot()));
}

void DandrumAudioProcessorEditor::getSoundLabAnalysisForWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    if (! processor.isSoundLabInstrumentCompatible())
    {
        completion (juce::var ("Sound Lab fixture does not match the active instrument"));
        return;
    }
    if (! arguments.isEmpty())
    {
        const auto& jobId = arguments[0];
        const auto number = static_cast<double> (jobId);
        if (! (jobId.isInt() || jobId.isInt64() || jobId.isDouble())
            || ! std::isfinite (number) || number < 1.0
            || number > 9007199254740991.0 || std::floor (number) < number
            || static_cast<std::uint64_t> (number) != soundLabController->snapshot().jobId)
        {
            completion (juce::var ("Unknown analysis job ID"));
            return;
        }
    }
    completion (soundLabSnapshotForWeb());
}

juce::var DandrumAudioProcessorEditor::soundLabSnapshotForWeb() const
{
    const auto snapshot = soundLabController->snapshot();
    auto report = std::make_unique<juce::DynamicObject>();
    report->setProperty ("generation", static_cast<juce::int64> (snapshot.generation));
    report->setProperty ("job_id", static_cast<juce::int64> (snapshot.jobId));
    report->setProperty ("instrument_generation",
                         static_cast<juce::int64> (snapshot.instrumentGeneration));
    if (snapshot.jobId != 0 && snapshot.state != SoundLabController::State::idle
        && snapshot.instrumentGeneration != processor.getParameterSurfaceGeneration())
    {
        report->setProperty ("state", "stale");
        return juce::var (report.release());
    }

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
            frame->setProperty ("reference_peak", metric.referencePeak);
            frame->setProperty ("candidate_peak", metric.candidatePeak);
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
    processor.pollInstrumentUiJobs();
    processor.pollMeterDelivery();
    hostBridge.expireNoteSession (juce::Time::getMillisecondCounterHiRes());
    if (hostBridge.publishParameterUpdates (browser))
        return;

    if (soundLabController == nullptr)
        return;

    const auto soundLabGeneration = soundLabController->generation();
    if (soundLabGeneration != lastSeenSoundLabGeneration)
    {
        lastSeenSoundLabGeneration = soundLabGeneration;
        browser.emitEventIfBrowserIsVisible ("soundLabAnalysisChanged", soundLabSnapshotForWeb());
    }
}
