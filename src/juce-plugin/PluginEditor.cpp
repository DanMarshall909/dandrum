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
    setResizeLimits (760, 430, 1500, 900);
    setSize (1180, 650);
    browser.goToURL (juce::WebBrowserComponent::getResourceProviderRoot());
    lastSeenParameterSurfaceGeneration = processor.getParameterSurfaceGeneration();
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

    completion (juce::var (result));
}

void DandrumAudioProcessorEditor::timerCallback()
{
    const auto generation = processor.getParameterSurfaceGeneration();
    if (generation == lastSeenParameterSurfaceGeneration)
        return;

    lastSeenParameterSurfaceGeneration = generation;
    browser.refresh();
}
