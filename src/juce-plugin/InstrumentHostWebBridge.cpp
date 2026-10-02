#include "InstrumentHostWebBridge.h"

#include "SharedInstrumentUi.h"

#include <charconv>
#include <cmath>
#include <cstring>
#include <limits>
#include <memory>
#include <string>
#include <utility>
#include <vector>

namespace
{
std::vector<std::byte> toBytes (const char* value)
{
    const auto length = std::char_traits<char>::length (value);
    std::vector<std::byte> bytes (length);
    std::memcpy (bytes.data(), value, length);
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

bool isNumeric (const juce::var& value)
{
    return value.isInt() || value.isInt64() || value.isDouble();
}

std::optional<std::uint32_t> parseGeneration (const juce::var& value)
{
    if (! isNumeric (value))
        return std::nullopt;
    const auto requested = static_cast<double> (value);
    if (! std::isfinite (requested) || requested < 0.0
        || requested > std::numeric_limits<std::uint32_t>::max()
        || std::floor (requested) < requested)
        return std::nullopt;
    return static_cast<std::uint32_t> (requested);
}

std::optional<int> parseMidiNote (const juce::var& value)
{
    if (! isNumeric (value))
        return std::nullopt;
    const auto note = static_cast<double> (value);
    if (! std::isfinite (note) || note < 0.0 || note > 127.0 || std::floor (note) < note)
        return std::nullopt;
    return static_cast<int> (note);
}

std::optional<float> parseMidiVelocity (const juce::var& value)
{
    if (! isNumeric (value))
        return std::nullopt;
    const auto velocity = static_cast<double> (value);
    if (! std::isfinite (velocity) || velocity <= 0.0 || velocity > 1.0)
        return std::nullopt;
    return static_cast<float> (velocity);
}

std::optional<std::uint64_t> parseJobId (const juce::var& value)
{
    if (! isNumeric (value))
        return std::nullopt;
    const auto number = static_cast<double> (value);
    if (! std::isfinite (number) || number < 1.0 || number > 9007199254740991.0
        || std::floor (number) < number)
        return std::nullopt;
    return static_cast<std::uint64_t> (number);
}

std::optional<std::uint64_t> parsePositiveDecimalId (const juce::var& value)
{
    if (! value.isString())
        return std::nullopt;
    const auto digits = value.toString().toStdString();
    std::uint64_t sequence = 0;
    const auto [end, error] = std::from_chars (digits.data(), digits.data() + digits.size(),
                                               sequence);
    if (error != std::errc {} || end != digits.data() + digits.size() || sequence == 0)
        return std::nullopt;
    return sequence;
}

juce::var meterPacketForWeb (const InstrumentUiMeterDelivery::Packet& packet)
{
    auto result = std::make_unique<juce::DynamicObject>();
    result->setProperty ("sequence", juce::String (std::to_string (packet.sequence)));
    result->setProperty ("generation", static_cast<juce::int64> (packet.meter.generation));
    result->setProperty ("stream_id", juce::String (std::to_string (packet.meter.streamId)));
    result->setProperty ("first_sample", juce::String (std::to_string (packet.meter.firstSample)));
    result->setProperty ("end_sample", juce::String (std::to_string (packet.meter.endSample)));
    result->setProperty ("observed_samples", static_cast<juce::int64> (packet.meter.observedSamples));
    result->setProperty ("complete", packet.meter.complete);
    result->setProperty ("display_complete", packet.display.complete);
    result->setProperty ("display_valid", packet.display.valid);
    juce::Array<juce::var> peak, rms, clipped, clipTicket;
    juce::Array<juce::var> displayPeak, displayRms, displayClipped;
    for (std::size_t channel = 0; channel < InstrumentUiMeterCapture::channelCount; ++channel)
    {
        peak.add (packet.meter.peak[channel]);
        rms.add (packet.meter.rms[channel]);
        clipped.add (packet.clip.latched[channel]);
        clipTicket.add (juce::String (std::to_string (packet.clip.ticket[channel])));
        displayPeak.add (packet.display.peak[channel]);
        displayRms.add (packet.display.rms[channel]);
        displayClipped.add (packet.display.clipped[channel]);
    }
    result->setProperty ("peak", juce::var (peak));
    result->setProperty ("rms", juce::var (rms));
    result->setProperty ("clipped", juce::var (clipped));
    result->setProperty ("clip_ticket", juce::var (clipTicket));
    result->setProperty ("display_peak", juce::var (displayPeak));
    result->setProperty ("display_rms", juce::var (displayRms));
    result->setProperty ("display_clipped", juce::var (displayClipped));
    return juce::var (result.release());
}

juce::var preparedDocumentForWeb (const InstrumentUiDocument& document)
{
    juce::var result (new juce::DynamicObject());
    auto* root = result.getDynamicObject();
    root->setProperty ("generation", static_cast<juce::int64> (document.generation));
    root->setProperty ("instrumentId", juce::String (document.instrumentId));

    juce::Array<juce::var> parameters;
    for (const auto& parameter : document.parameters)
    {
        juce::var item (new juce::DynamicObject());
        auto* value = item.getDynamicObject();
        value->setProperty ("id", juce::String (parameter.id));
        value->setProperty ("name", juce::String (parameter.name));
        value->setProperty ("normalisedValue", parameter.normalisedValue);
        value->setProperty ("minValue", parameter.minValue);
        value->setProperty ("maxValue", parameter.maxValue);
        value->setProperty ("scope", parameter.scope == InstrumentUiDocument::ControlScope::instrument
                                      ? "instrument" : "sampleGroup");
        if (parameter.controlGroup)
            value->setProperty ("controlGroup", *parameter.controlGroup);
        parameters.add (item);
    }
    root->setProperty ("parameters", juce::var (parameters));

    juce::Array<juce::var> sources;
    for (const auto& source : document.sources)
    {
        juce::var item (new juce::DynamicObject());
        auto* value = item.getDynamicObject();
        value->setProperty ("id", juce::String (source.id));
        value->setProperty ("sampleRateHz", static_cast<juce::int64> (source.sampleRateHz));
        value->setProperty ("channelCount", static_cast<int> (source.channelCount));
        value->setProperty ("frameCount", juce::String (std::to_string (source.frameCount)));
        juce::Array<juce::var> regions;
        for (const auto& region : source.regions)
        {
            juce::var regionItem (new juce::DynamicObject());
            auto* regionValue = regionItem.getDynamicObject();
            regionValue->setProperty ("id", juce::String (region.id));
            regionValue->setProperty ("startFrame", juce::String (std::to_string (region.startFrame)));
            regionValue->setProperty ("endFrame", juce::String (std::to_string (region.endFrame)));
            regionValue->setProperty ("reverse", region.reverse);
            regionValue->setProperty ("fadeInMs", region.fadeInMs);
            regionValue->setProperty ("fadeOutMs", region.fadeOutMs);
            if (region.rootNote)
                regionValue->setProperty ("rootNote", *region.rootNote);
            if (region.gainDb)
                regionValue->setProperty ("gainDb", *region.gainDb);
            if (region.pan)
                regionValue->setProperty ("pan", *region.pan);
            if (region.loop)
            {
                juce::var loop (new juce::DynamicObject());
                auto* loopValue = loop.getDynamicObject();
                loopValue->setProperty ("mode", juce::String (region.loop->mode));
                loopValue->setProperty ("startFrame", juce::String (
                    std::to_string (region.loop->startFrame)));
                loopValue->setProperty ("endFrame", juce::String (
                    std::to_string (region.loop->endFrame)));
                loopValue->setProperty ("crossfadeMs", region.loop->crossfadeMs);
                regionValue->setProperty ("loop", loop);
            }
            regions.add (regionItem);
        }
        value->setProperty ("regions", juce::var (regions));
        juce::Array<juce::var> slices;
        for (const auto& slice : source.slices)
        {
            juce::var sliceItem (new juce::DynamicObject());
            auto* sliceValue = sliceItem.getDynamicObject();
            sliceValue->setProperty ("id", juce::String (slice.id));
            sliceValue->setProperty ("startFrame", juce::String (std::to_string (slice.startFrame)));
            sliceValue->setProperty ("endFrame", juce::String (std::to_string (slice.endFrame)));
            slices.add (sliceItem);
        }
        value->setProperty ("slices", juce::var (slices));
        sources.add (item);
    }
    root->setProperty ("sources", juce::var (sources));

    juce::Array<juce::var> maps;
    for (const auto& map : document.maps)
    {
        juce::var item (new juce::DynamicObject());
        auto* value = item.getDynamicObject();
        value->setProperty ("id", juce::String (map.id));
        value->setProperty ("selectionMode", juce::String (map.selectionMode));
        value->setProperty ("selectionSeed", juce::String (std::to_string (map.selectionSeed)));
        juce::Array<juce::var> zones;
        for (const auto& zone : map.zones)
        {
            juce::var zoneItem (new juce::DynamicObject());
            auto* zoneValue = zoneItem.getDynamicObject();
            zoneValue->setProperty ("id", juce::String (zone.id));
            zoneValue->setProperty ("sourceIndex", static_cast<juce::int64> (zone.sourceIndex));
            zoneValue->setProperty ("regionIndex", static_cast<juce::int64> (zone.regionIndex));
            zoneValue->setProperty ("startFrame", juce::String (std::to_string (zone.startFrame)));
            zoneValue->setProperty ("endFrame", juce::String (std::to_string (zone.endFrame)));
            zoneValue->setProperty ("keyLow", static_cast<int> (zone.keyLow));
            zoneValue->setProperty ("keyHigh", static_cast<int> (zone.keyHigh));
            zoneValue->setProperty ("velocityLow", static_cast<int> (zone.velocityLow));
            zoneValue->setProperty ("velocityHigh", static_cast<int> (zone.velocityHigh));
            zoneValue->setProperty ("roundRobinGroup", juce::String (zone.roundRobinGroup));
            zoneValue->setProperty ("chokeGroup", juce::String (zone.chokeGroup));
            zoneValue->setProperty ("weight", static_cast<juce::int64> (zone.weight));
            if (zone.controlGroup)
                zoneValue->setProperty ("controlGroup", *zone.controlGroup);
            if (zone.gainDb)
                zoneValue->setProperty ("gainDb", *zone.gainDb);
            if (zone.pan)
                zoneValue->setProperty ("pan", *zone.pan);
            if (zone.pitchSemitones)
                zoneValue->setProperty ("pitchSemitones", *zone.pitchSemitones);
            zones.add (zoneItem);
        }
        value->setProperty ("zones", juce::var (zones));
        maps.add (item);
    }
    root->setProperty ("maps", juce::var (maps));

    juce::var capabilities (new juce::DynamicObject());
    auto* supported = capabilities.getDynamicObject();
    supported->setProperty ("sampleKeyMap", document.capabilities.sampleKeyMap);
    supported->setProperty ("preparedWaveform", document.capabilities.preparedWaveform);
    supported->setProperty ("synthLayer", document.capabilities.synthLayer);
    supported->setProperty ("nestedPatchLayer", document.capabilities.nestedPatchLayer);
    supported->setProperty ("moduleChain", document.capabilities.moduleChain);
    supported->setProperty ("patternSequencer", document.capabilities.patternSequencer);
    supported->setProperty ("hostTransport", document.capabilities.hostTransport);
    root->setProperty ("capabilities", capabilities);
    return result;
}

juce::var waveformStatusForWeb (const InstrumentUiWaveformService::Snapshot& snapshot)
{
    juce::var reply (new juce::DynamicObject());
    auto* value = reply.getDynamicObject();
    value->setProperty ("job_id", juce::String (std::to_string (snapshot.jobId)));
    value->setProperty ("generation", static_cast<juce::int64> (snapshot.generation));
    switch (snapshot.state)
    {
        case InstrumentUiWaveformService::State::running: value->setProperty ("state", "running"); break;
        case InstrumentUiWaveformService::State::ready: value->setProperty ("state", "ready"); break;
        case InstrumentUiWaveformService::State::failed: value->setProperty ("state", "failed"); break;
        case InstrumentUiWaveformService::State::cancelled: value->setProperty ("state", "cancelled"); break;
        case InstrumentUiWaveformService::State::stale: value->setProperty ("state", "stale"); break;
    }
    value->setProperty ("error", juce::String (snapshot.error));
    if (snapshot.result)
    {
        const auto& waveform = *snapshot.result;
        juce::var data (new juce::DynamicObject());
        auto* result = data.getDynamicObject();
        result->setProperty ("sourceId", juce::String (waveform.sourceId));
        result->setProperty ("regionId", juce::String (waveform.regionId));
        result->setProperty ("sampleRateHz", static_cast<juce::int64> (waveform.sampleRateHz));
        result->setProperty ("channel", static_cast<int> (waveform.channel));
        result->setProperty ("startFrame", juce::String (std::to_string (waveform.startFrame)));
        result->setProperty ("endFrame", juce::String (std::to_string (waveform.endFrame)));
        constexpr char hexDigits[] = "0123456789abcdef";
        std::string revision;
        revision.reserve (waveform.contentRevision.size() * 2);
        for (const auto byte : waveform.contentRevision)
        {
            revision.push_back (hexDigits[byte >> 4]);
            revision.push_back (hexDigits[byte & 0x0f]);
        }
        result->setProperty ("contentRevision", juce::String (revision));
        juce::Array<juce::var> buckets;
        for (const auto& bucket : waveform.buckets)
        {
            juce::var item (new juce::DynamicObject());
            auto* extrema = item.getDynamicObject();
            extrema->setProperty ("startFrame", juce::String (std::to_string (bucket.startFrame)));
            extrema->setProperty ("endFrame", juce::String (std::to_string (bucket.endFrame)));
            extrema->setProperty ("minimum", bucket.minimum);
            extrema->setProperty ("maximum", bucket.maximum);
            buckets.add (item);
        }
        result->setProperty ("buckets", juce::var (buckets));
        value->setProperty ("result", data);
    }
    return reply;
}

juce::var commandReplyForWeb (InstrumentUiCommandReply reply, const juce::String& publicId)
{
    switch (reply.status)
    {
        case InstrumentUiCommandStatus::accepted:
        {
            auto result = std::make_unique<juce::DynamicObject>();
            result->setProperty ("status", "accepted");
            result->setProperty ("generation", static_cast<juce::int64> (reply.generation));
            result->setProperty ("sequence", static_cast<juce::int64> (reply.sequence));
            return juce::var (result.release());
        }
        case InstrumentUiCommandStatus::staleGeneration:
            return juce::var ("Rejected stale instrument generation");
        case InstrumentUiCommandStatus::invalidValue:
            return juce::var ("Parameter value must be finite and in range 0..1");
        case InstrumentUiCommandStatus::unknownControl:
            return juce::var ("Unknown public parameter: " + publicId);
        case InstrumentUiCommandStatus::gestureActive:
            return juce::var ("Another gesture is active for this editor session");
        case InstrumentUiCommandStatus::noGesture:
            return juce::var ("No active gesture for this editor session");
    }
    return juce::var ("Unknown UI command result");
}
}

InstrumentHostWebBridge::InstrumentHostWebBridge (DandrumAudioProcessor& processorToUse)
    : processor (processorToUse),
      lastSeenParameterSurfaceGeneration (processorToUse.getParameterSurfaceGeneration()),
      sessionId (processorToUse.uiCommands().createSession())
{
}

InstrumentHostWebBridge::~InstrumentHostWebBridge()
{
    processor.cancelPreparedWaveformSession (sessionId);
    processor.unsubscribeMeter (sessionId);
    processor.closeEditorNoteSession (sessionId);
    processor.uiCommands().closeSession (sessionId);
}

const char* InstrumentHostWebBridge::bootstrapScript() noexcept
{
    return nativeFunctionBootstrap;
}

std::array<InstrumentHostWebBridge::NativeFunctionEntry, 19>
InstrumentHostWebBridge::nativeFunctions()
{
    return {{
        { "setParameter",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              setParameterFromWeb (arguments, std::move (completion));
          } },
        { "beginGesture",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              beginGestureFromWeb (arguments, std::move (completion));
          } },
        { "endGesture",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              endGestureFromWeb (arguments, std::move (completion));
          } },
        { "getParameters",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              getParametersForWeb (arguments, std::move (completion));
          } },
        { "getParameterState",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              getParameterStateForWeb (arguments, std::move (completion));
          } },
        { "getPreparedDocument",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              getPreparedDocumentForWeb (arguments, std::move (completion));
          } },
        { "requestWaveform",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              requestWaveformFromWeb (arguments, std::move (completion));
          } },
        { "getWaveformJobStatus",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              getWaveformJobStatusFromWeb (arguments, std::move (completion));
          } },
        { "cancelWaveform",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              cancelWaveformFromWeb (arguments, std::move (completion));
          } },
        { "noteOn",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              noteOnFromWeb (arguments, std::move (completion));
          } },
        { "noteOff",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              noteOffFromWeb (arguments, std::move (completion));
          } },
        { "noteHeartbeat",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              noteHeartbeatFromWeb (arguments, std::move (completion));
          } },
        { "reloadInstrument",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              reloadInstrumentFromWeb (arguments, std::move (completion));
          } },
        { "getUiJobStatus",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              getUiJobStatusFromWeb (arguments, std::move (completion));
          } },
        { "subscribeMeter",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              subscribeMeterFromWeb (arguments, std::move (completion));
          } },
        { "setMeterVisible",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              setMeterVisibleFromWeb (arguments, std::move (completion));
          } },
        { "getMeterPacket",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              getMeterPacketForWeb (arguments, std::move (completion));
          } },
        { "ackMeterPacket",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              ackMeterPacketFromWeb (arguments, std::move (completion));
          } },
        { "ackMeterClip",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              ackMeterClipFromWeb (arguments, std::move (completion));
          } }
    }};
}

juce::WebBrowserComponent::Options InstrumentHostWebBridge::addNativeFunctions (
    juce::WebBrowserComponent::Options options)
{
    for (auto& [name, callback] : nativeFunctions())
        options = options.withNativeFunction (name, std::move (callback));
    return options;
}

std::optional<juce::WebBrowserComponent::Resource>
InstrumentHostWebBridge::provideResource (const juce::String& path,
                                          const std::string& pageHtml) const
{
    if (path == "/" || path == "/index.html")
        return juce::WebBrowserComponent::Resource {
            toBytes (pageHtml.c_str()), "text/html" };

    if (path == "/shared-instrument-ui.js")
        return juce::WebBrowserComponent::Resource {
            toBytes (SharedInstrumentUi::script), "text/javascript" };

    return std::nullopt;
}

bool InstrumentHostWebBridge::publishParameterUpdates (juce::WebBrowserComponent& browser)
{
    const auto generation = processor.getParameterSurfaceGeneration();
    if (generation != lastSeenParameterSurfaceGeneration)
    {
        processor.cancelPreparedWaveformSession (sessionId);
        processor.unsubscribeMeter (sessionId);
        processor.uiCommands().closeSession (sessionId);
        lastSeenParameterSurfaceGeneration = generation;
        browser.refresh();
        return true;
    }

    auto state = parameterStateForWeb();
    browser.emitEventIfBrowserIsVisible ("parameterStateChanged", state);
    browser.emitEventIfBrowserIsVisible ("parameterValuesChanged",
                                        state.getProperty ("parameters", {}));
    return false;
}

void InstrumentHostWebBridge::setParameterFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.size() < 2)
    {
        completion (juce::var ("setParameter expects parameter id and normalised value"));
        return;
    }

    if (! isNumeric (arguments[1]))
    {
        completion (juce::var ("Parameter value must be a finite number in range 0..1"));
        return;
    }
    const auto publicId = arguments[0].toString();
    const auto hasGeneration = arguments.size() >= 3;
    auto generation = processor.getParameterSurfaceGeneration();
    if (hasGeneration)
    {
        const auto requested = parseGeneration (arguments[2]);
        if (! requested)
        {
            completion (juce::var ("Invalid instrument generation"));
            return;
        }
        generation = *requested;
    }

    const auto reply = processor.uiCommands().setParameter (
        { generation, publicId.toStdString(), static_cast<double> (arguments[1]), sessionId });
    completion (! hasGeneration && reply.status == InstrumentUiCommandStatus::accepted
        ? juce::var() : commandReplyForWeb (reply, publicId));
}

void InstrumentHostWebBridge::beginGestureFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.isEmpty())
    {
        completion (juce::var ("beginGesture expects a parameter id"));
        return;
    }
    const auto publicId = arguments[0].toString();
    auto generation = processor.getParameterSurfaceGeneration();
    if (arguments.size() >= 2)
    {
        const auto requested = parseGeneration (arguments[1]);
        if (! requested)
        {
            completion (juce::var ("Invalid instrument generation"));
            return;
        }
        generation = *requested;
    }
    completion (commandReplyForWeb (processor.uiCommands().beginGesture (
        { generation, publicId.toStdString(), sessionId }), publicId));
}

void InstrumentHostWebBridge::endGestureFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.isEmpty())
    {
        completion (juce::var ("endGesture expects a parameter id"));
        return;
    }
    const auto publicId = arguments[0].toString();
    auto generation = processor.getParameterSurfaceGeneration();
    if (arguments.size() >= 2)
    {
        const auto requested = parseGeneration (arguments[1]);
        if (! requested)
        {
            completion (juce::var ("Invalid instrument generation"));
            return;
        }
        generation = *requested;
    }
    completion (commandReplyForWeb (processor.uiCommands().endGesture (
        { generation, publicId.toStdString(), sessionId }), publicId));
}

void InstrumentHostWebBridge::getParametersForWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    completion (parameterSnapshotForWeb());
}

void InstrumentHostWebBridge::getParameterStateForWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    completion (parameterStateForWeb());
}

void InstrumentHostWebBridge::getPreparedDocumentForWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    const auto document = processor.getPreparedUiDocument();
    completion (document ? preparedDocumentForWeb (*document) : juce::var {});
}

void InstrumentHostWebBridge::requestWaveformFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.size() != 5 || ! arguments[0].isString() || ! arguments[1].isString()
        || arguments[0].toString().isEmpty() || arguments[1].toString().isEmpty())
    {
        completion (juce::var ("requestWaveform expects source ID, region ID, channel, bucket count and generation"));
        return;
    }
    const auto channel = parseGeneration (arguments[2]);
    const auto bucketCount = parseGeneration (arguments[3]);
    const auto generation = parseGeneration (arguments[4]);
    if (! channel || *channel > std::numeric_limits<std::uint16_t>::max()
        || ! bucketCount || *bucketCount == 0
        || *bucketCount > InstrumentUiWaveformService::maxBuckets)
    {
        completion (juce::var ("requestWaveform requires a valid channel and bucket count"));
        return;
    }
    if (! generation)
    {
        completion (juce::var ("requestWaveform requires a valid instrument generation"));
        return;
    }
    if (*generation != processor.getParameterSurfaceGeneration())
    {
        completion (juce::var ("Rejected stale instrument generation"));
        return;
    }
    const auto jobId = processor.requestPreparedWaveform (
        *generation, arguments[0].toString().toStdString(),
        arguments[1].toString().toStdString(), static_cast<std::uint16_t> (*channel),
        *bucketCount, sessionId);
    if (! jobId)
    {
        completion (juce::var ("Waveform request unavailable for this prepared region"));
        return;
    }
    juce::var reply (new juce::DynamicObject());
    reply.getDynamicObject()->setProperty ("status", "accepted");
    reply.getDynamicObject()->setProperty ("job_id", juce::String (std::to_string (*jobId)));
    reply.getDynamicObject()->setProperty ("generation", static_cast<juce::int64> (*generation));
    completion (reply);
}

void InstrumentHostWebBridge::getWaveformJobStatusFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    if (arguments.size() != 1)
    {
        completion (juce::var ("getWaveformJobStatus requires a job ID"));
        return;
    }
    const auto jobId = parsePositiveDecimalId (arguments[0]);
    if (! jobId)
    {
        completion (juce::var ("getWaveformJobStatus requires a valid decimal job ID"));
        return;
    }
    const auto status = processor.getPreparedWaveformJobStatus (*jobId);
    completion (status ? waveformStatusForWeb (*status)
                       : juce::var ("Unknown waveform job ID"));
}

void InstrumentHostWebBridge::cancelWaveformFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    const auto jobId = arguments.size() == 1
        ? parsePositiveDecimalId (arguments[0]) : std::nullopt;
    const auto status = jobId ? processor.getPreparedWaveformJobStatus (*jobId)
                              : std::nullopt;
    completion (status && status->sessionId == sessionId
                && processor.cancelPreparedWaveformJob (*jobId));
}

void InstrumentHostWebBridge::noteOnFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.size() < 2)
    {
        completion (juce::var ("noteOn expects MIDI note and normalised velocity"));
        return;
    }

    const auto note = parseMidiNote (arguments[0]);
    const auto velocity = parseMidiVelocity (arguments[1]);
    if (! note || ! velocity)
    {
        completion (juce::var ("noteOn requires a valid MIDI note and velocity"));
        return;
    }
    if (arguments.size() >= 3)
    {
        const auto generation = parseGeneration (arguments[2]);
        if (! generation)
        {
            completion (juce::var ("noteOn requires a valid instrument generation"));
            return;
        }
        if (*generation != processor.getParameterSurfaceGeneration())
        {
            completion (juce::var ("Rejected stale instrument generation"));
            return;
        }
    }
    if (! processor.enqueueEditorNoteOn (*note, *velocity, sessionId))
    {
        completion (juce::var ("This note is held by another editor session"));
        return;
    }

    noteSessionActive = true;
    lastNoteHeartbeatMilliseconds = juce::Time::getMillisecondCounterHiRes();
    completion (juce::var());
}

void InstrumentHostWebBridge::noteOffFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.isEmpty())
    {
        completion (juce::var ("noteOff expects a MIDI note"));
        return;
    }

    const auto note = parseMidiNote (arguments[0]);
    if (! note)
    {
        completion (juce::var ("noteOff requires a valid MIDI note"));
        return;
    }
    if (arguments.size() >= 2)
    {
        const auto generation = parseGeneration (arguments[1]);
        if (! generation)
        {
            completion (juce::var ("noteOff requires a valid instrument generation"));
            return;
        }
        if (*generation != processor.getParameterSurfaceGeneration())
        {
            completion (juce::var ("Rejected stale instrument generation"));
            return;
        }
    }
    if (! processor.enqueueEditorNoteOff (*note, sessionId))
    {
        completion (juce::var ("This note is held by another editor session"));
        return;
    }

    lastNoteHeartbeatMilliseconds = juce::Time::getMillisecondCounterHiRes();
    completion (juce::var());
}

void InstrumentHostWebBridge::noteHeartbeatFromWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (noteSessionActive)
        lastNoteHeartbeatMilliseconds = juce::Time::getMillisecondCounterHiRes();
    completion (juce::var());
}

void InstrumentHostWebBridge::reloadInstrumentFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.size() != 2 || ! arguments[0].isString()
        || arguments[0].toString().isEmpty()
        || ! juce::File::isAbsolutePath (arguments[0].toString()))
    {
        completion (juce::var ("reloadInstrument requires an absolute instrument path and generation"));
        return;
    }
    const auto generation = parseGeneration (arguments[1]);
    if (! generation)
    {
        completion (juce::var ("reloadInstrument requires a valid instrument generation"));
        return;
    }
    if (*generation != processor.getParameterSurfaceGeneration())
    {
        completion (juce::var ("Rejected stale instrument generation"));
        return;
    }

    const auto jobId = processor.requestInstrumentReloadJob (
        juce::File (arguments[0].toString()), *generation);
    if (! jobId)
    {
        completion (juce::var ("Instrument reload is unavailable or already running"));
        return;
    }
    auto reply = std::make_unique<juce::DynamicObject>();
    reply->setProperty ("status", "accepted");
    reply->setProperty ("job_id", static_cast<juce::int64> (*jobId));
    reply->setProperty ("generation", static_cast<juce::int64> (*generation));
    completion (juce::var (reply.release()));
}

void InstrumentHostWebBridge::getUiJobStatusFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (arguments.size() != 1)
    {
        completion (juce::var ("getUiJobStatus requires a job ID"));
        return;
    }
    const auto jobId = parseJobId (arguments[0]);
    if (! jobId)
    {
        completion (juce::var ("getUiJobStatus requires a valid job ID"));
        return;
    }
    const auto status = processor.getInstrumentUiJobStatus (*jobId);
    if (! status)
    {
        completion (juce::var ("Unknown UI job ID"));
        return;
    }
    auto reply = std::make_unique<juce::DynamicObject>();
    reply->setProperty ("job_id", static_cast<juce::int64> (status->id));
    reply->setProperty ("generation", static_cast<juce::int64> (status->generation));
    switch (status->state)
    {
        case DandrumAudioProcessor::UiJobState::running: reply->setProperty ("state", "running"); break;
        case DandrumAudioProcessor::UiJobState::completed: reply->setProperty ("state", "completed"); break;
        case DandrumAudioProcessor::UiJobState::failed: reply->setProperty ("state", "failed"); break;
        case DandrumAudioProcessor::UiJobState::stale: reply->setProperty ("state", "stale"); break;
    }
    reply->setProperty ("error", status->error);
    completion (juce::var (reply.release()));
}

void InstrumentHostWebBridge::subscribeMeterFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    const auto generation = arguments.isEmpty() ? std::nullopt : parseGeneration (arguments[0]);
    completion (juce::var (generation.has_value()
                           && processor.subscribeMeter (sessionId, *generation)));
}

void InstrumentHostWebBridge::setMeterVisibleFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    const auto generation = arguments.size() < 2 ? std::nullopt : parseGeneration (arguments[1]);
    completion (juce::var (generation.has_value()
                           && *generation == processor.getParameterSurfaceGeneration()
                           && arguments[0].isBool()
                           && processor.setMeterSessionVisible (sessionId,
                                                                static_cast<bool> (arguments[0]))));
}

void InstrumentHostWebBridge::getMeterPacketForWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    if (const auto packet = processor.takeMeterPacket (sessionId))
        completion (meterPacketForWeb (*packet));
    else
        completion (juce::var());
}

void InstrumentHostWebBridge::ackMeterPacketFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    const auto sequence = arguments.isEmpty() ? std::nullopt : parsePositiveDecimalId (arguments[0]);
    const auto generation = arguments.size() < 2 ? std::nullopt : parseGeneration (arguments[1]);
    completion (juce::var (sequence.has_value() && generation.has_value()
                           && processor.acknowledgeMeterPacket (
                               sessionId, *generation, *sequence)));
}

void InstrumentHostWebBridge::ackMeterClipFromWeb (
    const juce::Array<juce::var>& arguments,
    juce::WebBrowserComponent::NativeFunctionCompletion completion)
{
    const auto channel = arguments.isEmpty() ? std::nullopt : parseGeneration (arguments[0]);
    const auto generation = arguments.size() < 2 ? std::nullopt : parseGeneration (arguments[1]);
    const auto ticket = arguments.size() < 3 ? std::nullopt : parsePositiveDecimalId (arguments[2]);
    completion (juce::var (channel.has_value() && *channel < InstrumentUiMeterCapture::channelCount
                           && generation.has_value() && ticket.has_value()
                           && processor.acknowledgeMeterClip (*channel, *generation, *ticket)));
}

bool InstrumentHostWebBridge::expireNoteSession (double nowMilliseconds) noexcept
{
    constexpr double timeoutMilliseconds = 2000.0;
    if (! noteSessionActive
        || nowMilliseconds - lastNoteHeartbeatMilliseconds < timeoutMilliseconds)
        return false;
    processor.closeEditorNoteSession (sessionId);
    noteSessionActive = false;
    return true;
}

juce::var InstrumentHostWebBridge::parameterSnapshotForWeb() const
{
    juce::Array<juce::var> result;
    for (const auto& parameter : processor.getPublicParameterSnapshot())
    {
        auto object = std::make_unique<juce::DynamicObject>();
        object->setProperty ("id", parameter.id);
        object->setProperty ("name", parameter.displayName);
        object->setProperty ("value", parameter.normalisedValue);
        result.add (juce::var (object.release()));
    }
    return juce::var (result);
}

juce::var InstrumentHostWebBridge::parameterStateForWeb() const
{
    const auto state = processor.getUiParameterState();
    auto result = std::make_unique<juce::DynamicObject>();
    result->setProperty ("generation", static_cast<juce::int64> (state.generation));
    result->setProperty ("sequence", static_cast<juce::int64> (state.admittedCommandSequence));
    juce::Array<juce::var> values;
    for (const auto& parameter : state.parameters)
    {
        auto item = std::make_unique<juce::DynamicObject>();
        item->setProperty ("id", juce::String (parameter.id));
        item->setProperty ("name", juce::String (parameter.name));
        item->setProperty ("value", parameter.normalisedValue);
        values.add (juce::var (item.release()));
    }
    result->setProperty ("parameters", juce::var (values));
    return juce::var (result.release());
}

std::uint32_t InstrumentHostWebBridge::lastSeenSurfaceGeneration() const noexcept
{
    return lastSeenParameterSurfaceGeneration;
}
