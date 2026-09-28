#include "InstrumentHostWebBridge.h"

#include "SharedInstrumentUi.h"

#include <cstring>
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
}

InstrumentHostWebBridge::InstrumentHostWebBridge (DandrumAudioProcessor& processorToUse)
    : processor (processorToUse),
      lastSeenParameterSurfaceGeneration (processorToUse.getParameterSurfaceGeneration())
{
}

const char* InstrumentHostWebBridge::bootstrapScript() noexcept
{
    return nativeFunctionBootstrap;
}

std::array<InstrumentHostWebBridge::NativeFunctionEntry, 4>
InstrumentHostWebBridge::nativeFunctions()
{
    return {{
        { "setParameter",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              setParameterFromWeb (arguments, std::move (completion));
          } },
        { "getParameters",
          [this] (const juce::Array<juce::var>& arguments,
                  juce::WebBrowserComponent::NativeFunctionCompletion completion)
          {
              getParametersForWeb (arguments, std::move (completion));
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
        lastSeenParameterSurfaceGeneration = generation;
        browser.refresh();
        return true;
    }

    browser.emitEventIfBrowserIsVisible ("parameterValuesChanged", parameterSnapshotForWeb());
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

void InstrumentHostWebBridge::getParametersForWeb (
    const juce::Array<juce::var>&,
    juce::WebBrowserComponent::NativeFunctionCompletion completion) const
{
    completion (parameterSnapshotForWeb());
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

    if (! processor.enqueueEditorNoteOn (static_cast<int> (arguments[0]),
                                         static_cast<float> (arguments[1])))
    {
        completion (juce::var ("Editor MIDI queue is full; note-on was dropped"));
        return;
    }

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

    if (! processor.enqueueEditorNoteOff (static_cast<int> (arguments[0])))
    {
        completion (juce::var ("Editor MIDI queue is full; note-off was dropped"));
        return;
    }

    completion (juce::var());
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

std::uint32_t InstrumentHostWebBridge::lastSeenSurfaceGeneration() const noexcept
{
    return lastSeenParameterSurfaceGeneration;
}
