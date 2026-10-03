#include "PluginProcessor.h"

#if JUCE_WEB_BROWSER
 #include "PluginEditor.h"

struct PluginEditorBridgeTestProbe
{
    static juce::WebBrowserComponent& runtimeBrowser (DandrumAudioProcessorEditor& editor)
    {
        return editor.browser;
    }
};
#endif

#include <array>
#include <bit>
#include <cmath>
#include <csignal>
#include <cstdlib>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace
{
void require (bool condition, const juce::String& message)
{
    if (! condition)
        throw std::runtime_error (message.toStdString());
}

// A nonzero source offset and four distinguishable signed envelopes. Each of
// the 512 requested buckets contains 24 frames; channel 1 has unrelated data.
constexpr std::array minimum { -0.75f, -0.25f, -0.5f, -0.125f };
constexpr std::array maximum { 0.5f, 0.75f, 0.25f, 0.125f };
constexpr std::array topFraction { 0.25, 0.125, 0.375, 0.4375 };
constexpr std::array bottomFraction { 0.875, 0.625, 0.75, 0.5625 };
constexpr int startFrame = 768, endFrame = 13056, sourceFrames = 15360;
constexpr int bucketCount = 512, framesPerBucket = 24;

struct Fixture
{
    Fixture()
        : directory (juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getNonexistentChildFile ("dandrum-waveform-parity", {}, false))
    {
        require (directory.createDirectory().wasOk(), "Cannot create waveform parity fixture");
        juce::AudioBuffer<float> pcm (2, sourceFrames);
        for (int frame = 0; frame < sourceFrames; ++frame)
        {
            const auto group = std::clamp ((frame - startFrame) / 3072, 0, 3);
            pcm.setSample (0, frame, frame < startFrame || frame >= endFrame
                ? 0.875f : frame % 2 == 0 ? minimum[static_cast<std::size_t> (group)]
                                        : maximum[static_cast<std::size_t> (group)]);
            pcm.setSample (1, frame, 0.0625f);
        }
        auto stream = directory.getChildFile ("source.wav").createOutputStream();
        require (stream != nullptr, "Cannot open waveform parity WAV");
        // Write the literal PCM16 values. A float-to-WAV encoder may dither or
        // quantize them, which would invalidate the independent exact oracle.
        stream->write ("RIFF", 4); stream->writeInt (36 + sourceFrames * 4);
        stream->write ("WAVEfmt ", 8); stream->writeInt (16);
        stream->writeShort (1); stream->writeShort (2); stream->writeInt (48000);
        stream->writeInt (192000); stream->writeShort (4); stream->writeShort (16);
        stream->write ("data", 4); stream->writeInt (sourceFrames * 4);
        for (int frame = 0; frame < sourceFrames; ++frame)
            for (int channel = 0; channel < 2; ++channel)
                stream->writeShort (static_cast<short> (pcm.getSample (channel, frame) * 32768.0f));
        stream->flush();
        require (stream->getStatus().wasOk(), "Cannot write waveform parity PCM");
        stream.reset();
        require (patch().replaceWithText (R"YAML(metadata: { name: Waveform Renderer Parity }
instrument: { id: dandrum.waveform-parity, preset_schema_version: 1 }
assets:
  sample_sources:
    - id: signed
      path: source.wav
      slices:
        - { id: early, start_frame: 3840, end_frame: 4992 }
        - { id: late, start_frame: 8448, end_frame: 9984 }
      regions:
        - id: body
          start_frame: 768
          end_frame: 13056
          fade_in_ms: 2.5
          fade_out_ms: 7.5
          loop: { mode: forward, start_frame: 2304, end_frame: 11520 }
  sample_maps:
    - id: pads
      selection_mode: first_match
      zones:
        - { id: signed, region: signed.body, key_range: [60, 60], velocity_range: [1, 127] }
ports:
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: player.audio }
modules:
  - { id: midi, type: midi_input }
  - { id: player, type: sample_map_player, static: { sample_map: pads, channels: 2, max_voices: 2 } }
connections:
  - { from: midi.events, to: player.note }
)YAML"), "Cannot write waveform parity patch");
    }
    ~Fixture() { directory.deleteRecursively(); }
    juce::File patch() const { return directory.getChildFile ("instrument.yaml"); }
    juce::File directory;
};

struct Marker { const char* name; double fraction; juce::uint32 colour; };
constexpr std::array markers {
    Marker { "region start", 0.0, 0xff8da79a },
    Marker { "region end", 1.0, 0xff8da79a },
    Marker { "source-rate fade in", 5.0 / 512.0, 0xff8da79a },
    Marker { "source-rate fade out", 497.0 / 512.0, 0xff8da79a },
    Marker { "loop start", 0.125, 0xffe2bf72 },
    Marker { "loop end", 0.875, 0xffe2bf72 },
    Marker { "early slice start", 0.25, 0xffab9ee9 },
    Marker { "early slice end", 0.34375, 0xffab9ee9 },
    Marker { "late slice start", 0.625, 0xffab9ee9 },
    Marker { "late slice end", 0.75, 0xffab9ee9 }
};

// This oracle derives coordinates from literal fixture facts, never from a
// production geometry/view model or from the other renderer's output.
void verifyPixels (const juce::Image& image)
{
    const auto width = image.getWidth(), height = image.getHeight();
    require (width > 200 && height > 100, "Waveform plot is not visible at its target size");
    const auto colourAt = [&image] (int x, int y) { return image.getPixelAt (x, y).getARGB(); };
    for (const auto& marker : markers)
    {
        const auto x = std::clamp (static_cast<int> (std::lround (marker.fraction * width)), 0, width - 1);
        require (colourAt (x, 1) == marker.colour && colourAt (x, height - 2) == marker.colour,
                 "Prepared marker coordinate differs: " + juce::String (marker.name));
    }
    for (std::size_t group = 0; group < 4; ++group)
    {
        const auto bucket = static_cast<int> (group) * 128 + 32;
        const auto x = static_cast<int> (std::lround ((bucket + 0.5) / bucketCount * width));
        const auto top = static_cast<int> (std::lround (topFraction[group] * height));
        const auto bottom = static_cast<int> (std::lround (bottomFraction[group] * height));
        require (colourAt (x, top) == 0xff7ce0aa && colourAt (x, bottom) == 0xff7ce0aa
                     && colourAt (x, top - 1) == 0xff111916 && colourAt (x, bottom + 1) == 0xff111916,
                 "Signed envelope coordinate differs in source quarter " + juce::String (static_cast<int> (group))
                     + " plot=" + juce::String (width) + "x" + juce::String (height)
                     + " x=" + juce::String (x) + " top=" + juce::String (top) + " bottom=" + juce::String (bottom)
                     + " pixels=" + juce::String::toHexString (colourAt (x, top - 1))
                     + "," + juce::String::toHexString (colourAt (x, top))
                     + "," + juce::String::toHexString (colourAt (x, bottom))
                     + "," + juce::String::toHexString (colourAt (x, bottom + 1)));
    }
}

#if ! JUCE_WEB_BROWSER
juce::Component* findPanel (juce::Component& parent)
{
    if (parent.getComponentID() == "prepared-waveform")
        return &parent;
    for (auto* child : parent.getChildren())
        if (auto* found = findPanel (*child))
            return found;
    return nullptr;
}
#endif

class Application final : public juce::JUCEApplication, private juce::Timer
{
public:
    const juce::String getApplicationName() override { return "Dandrum Waveform Parity Test"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise (const juce::String&) override
    {
        try
        {
            std::unique_ptr<juce::AudioProcessor> original (createPluginFilter());
            auto* sampler = dynamic_cast<DandrumAudioProcessor*> (original.get());
            require (sampler != nullptr && sampler->demoConfiguration().instrumentId == "dandrum.advanced-drum-kit",
                     "Waveform parity did not start with the original sampler factory");
            original.release();
            processor.reset (sampler);
            processor->setPlayConfigDetails (0, 2, 44100.0, 64);
            processor->prepareToPlay (44100.0, 64);
            require (processor->reloadInstrumentFromFile (fixture.patch()), "Waveform parity fixture failed to prepare");
            generation = processor->getPreparedUiDocument()->generation;
            const auto request = processor->requestPreparedWaveform (generation, "signed", "body", 0, bucketCount);
            require (request.has_value(), "Numeric waveform job was rejected");
            numericJob = *request;
            editor.reset (processor->createEditor());
            require (editor != nullptr, "Original sampler did not create an editor");
            editor->setSize (1200, 800);
            editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
            editor->setTopLeftPosition (30, 30);
            editor->setVisible (true);
           #if JUCE_WEB_BROWSER
            auto* web = dynamic_cast<DandrumAudioProcessorEditor*> (editor.get());
            require (web != nullptr, "Original sampler did not create its Web editor");
            browser = &PluginEditorBridgeTestProbe::runtimeBrowser (*web);
           #else
            panel = findPanel (*editor);
            require (panel != nullptr, "Original sampler did not create its native waveform");
           #endif
            startTimer (25);
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }
    void shutdown() override
    {
        stopTimer();
        editor.reset();
        processor.reset();
    }

private:
    Fixture fixture;
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    std::uint32_t generation = 0;
    std::uint64_t numericJob = 0;
    bool compact = false, numericChecked = false, finished = false;
    const double deadline = juce::Time::getMillisecondCounterHiRes() + 25000.0;
   #if JUCE_WEB_BROWSER
    juce::WebBrowserComponent* browser = nullptr;
    bool evaluating = false;
   #else
    juce::Component* panel = nullptr;
   #endif

    bool verifyNumericResult()
    {
        if (numericChecked)
            return true;
        const auto job = processor->getPreparedWaveformJobStatus (numericJob);
        require (job.has_value(), "Numeric waveform job disappeared");
        if (job->state == InstrumentUiWaveformService::State::running)
            return false;
        require (job->state == InstrumentUiWaveformService::State::ready && job->result != nullptr,
                 "Numeric waveform did not become ready");
        const auto& result = *job->result;
        require (std::abs (processor->getSampleRate() - 44100.0) < 0.5 && result.sampleRateHz == 48000
                     && result.startFrame == startFrame && result.endFrame == endFrame
                     && result.channel == 0 && result.buckets.size() == bucketCount,
                 "Source frames/rate/channel were replaced by host coordinates");
        for (std::size_t i = 0; i < result.buckets.size(); ++i)
        {
            const auto& bucket = result.buckets[i];
            require (bucket.startFrame == startFrame + i * framesPerBucket
                         && bucket.endFrame == startFrame + (i + 1) * framesPerBucket
                         && std::bit_cast<std::uint32_t> (bucket.minimum) == std::bit_cast<std::uint32_t> (minimum[i / 128])
                         && std::bit_cast<std::uint32_t> (bucket.maximum) == std::bit_cast<std::uint32_t> (maximum[i / 128]),
                     "Numeric job lost signed extrema or source offsets at bucket " + juce::String (static_cast<int> (i)));
        }
        numericChecked = true;
        return true;
    }

    void record (const juce::Image& plot)
    {
        verifyPixels (plot);
        require (processor->getPreparedUiDocument()->generation == generation,
                 "Resizing rebuilt the instrument");
        if (const auto* output = std::getenv ("DANDRUM_WAVEFORM_PARITY_SNAPSHOTS"))
        {
            auto file = juce::File (juce::String (output)).getChildFile (
               #if JUCE_WEB_BROWSER
                "web-"
               #else
                "native-"
               #endif
                + juce::String (compact ? "820x560.png" : "1200x800.png"));
            file.getParentDirectory().createDirectory();
            juce::MemoryOutputStream stream;
            juce::PNGImageFormat png;
            require (png.writeImageToStream (plot, stream)
                         && file.replaceWithData (stream.getData(), stream.getDataSize()), "Cannot save actual renderer plot");
        }
        std::cout << "WAVEFORM_PARITY " << (compact ? "820x560" : "1200x800")
                  << " plot=" << plot.getWidth() << 'x' << plot.getHeight()
                  << " source=48000 host=44100 frames=768..13056 duration=0.256"
                     " signed_quarters=4 prepared_markers=10 PASS\n";
        if (compact)
            finish (true);
        else
        {
            compact = true;
            editor->setSize (820, 560);
        }
    }

    void finish (bool success, const juce::String& error = {})
    {
        if (finished)
            return;
        finished = true;
        stopTimer();
        if (! success)
            std::cerr << "Waveform parity failed: " << error << '\n';
        setApplicationReturnValue (success ? 0 : 1);
        quit();
    }

    void timerCallback() override
    {
        try
        {
            require (juce::Time::getMillisecondCounterHiRes() <= deadline, "Actual waveform renderer timed out");
            if (! verifyNumericResult())
                return;
           #if JUCE_WEB_BROWSER
            pollWeb();
           #else
            if (panel->getName() != "Prepared waveform: signed.body")
                return;
            const auto snapshot = panel->createComponentSnapshot (panel->getLocalBounds());
            record (snapshot.getClippedImage ({16, 52, panel->getWidth() - 32, panel->getHeight() - 76}));
           #endif
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }

   #if JUCE_WEB_BROWSER
    void pollWeb()
    {
        if (evaluating)
            return;
        // Observe the original React mount, native transport and Canvas. No
        // substitute page, injected peaks, helper rendering or mock host.
        const auto script = juce::String (R"JS((() => {
          if (!window.__waveformPaintObservation) {
            window.__waveformPaintObservation = new MutationObserver(() => {
              const panel = document.querySelector('.waveform'), canvas = panel?.querySelector('canvas');
              if (!canvas || !panel.textContent.includes('48000 Hz \u00b7 0.256 s')
                  || canvas.width < 200 || canvas.width !== Math.round(canvas.clientWidth * devicePixelRatio)) return;
              window.__waveformPaintChecks = (window.__waveformPaintChecks || 0) + 1;
              const pixel = canvas.getContext('2d').getImageData(0, 1, 1, 1).data;
              if (pixel[0] !== 141 || pixel[1] !== 167 || pixel[2] !== 154 || pixel[3] !== 255)
                window.__waveformPaintFailure = 'Web waveform labels committed before prepared Canvas paint';
            });
            window.__waveformPaintObservation.observe(document.body, {subtree:true,childList:true,characterData:true});
          }
          if (window.__waveformPaintFailure) return 'ERROR: ' + window.__waveformPaintFailure;
          const panel = document.querySelector('.waveform'), canvas = panel?.querySelector('canvas');
          if (!canvas || !panel.textContent.includes('48000 Hz \u00b7 0.256 s')) return '';
          const r = canvas.getBoundingClientRect();
          if (innerWidth !== )JS") + juce::String (compact ? 820 : 1200)
            + " || innerHeight !== " + juce::String (compact ? 560 : 800)
            + R"JS( || canvas.width !== Math.round(r.width * devicePixelRatio)) return '';
          return JSON.stringify({png:canvas.toDataURL('image/png'),ratio:devicePixelRatio,
            paintChecks:window.__waveformPaintChecks || 0,
            width:r.width,height:r.height,left:r.left,right:r.right,top:r.top,bottom:r.bottom,
            viewportWidth:innerWidth,viewportHeight:innerHeight});
        })())JS";
        evaluating = true;
        juce::Component::SafePointer<juce::AudioProcessorEditor> alive (editor.get());
        browser->evaluateJavascript (script, [this, alive] (auto evaluation)
        {
            if (alive == nullptr || finished)
                return;
            evaluating = false;
            try
            {
                require (evaluation.getError() == nullptr, "Original browser observation failed");
                const auto* value = evaluation.getResult();
                require (value != nullptr, "Original browser returned no waveform observation");
                // WebKit may acknowledge an evaluation during initial page
                // navigation without a value. The deadline still requires a
                // real mounted, visible and painted Canvas at both sizes.
                if (value->isUndefined() || value->isVoid())
                    return;
                require (value->isString(), "Original browser returned an invalid waveform observation");
                if (value->toString().isEmpty())
                    return;
                require (! value->toString().startsWith ("ERROR: "), value->toString());
                const auto result = juce::JSON::parse (value->toString());
                require (result.isObject() && std::abs (static_cast<double> (result["ratio"]) - 1.0) < 0.00001,
                         "Waveform pixel parity lane requires documented 1x display scaling");
                require (static_cast<int> (result["paintChecks"]) > 0,
                         "Waveform paint observer never checked actual ready labels");
                require (static_cast<double> (result["left"]) >= 0.0
                             && static_cast<double> (result["right"]) <= static_cast<double> (result["viewportWidth"])
                             && static_cast<double> (result["top"]) >= 0.0
                             && static_cast<double> (result["bottom"]) <= static_cast<double> (result["viewportHeight"]),
                         "Actual React waveform is outside the target viewport");
                juce::MemoryBlock encoded;
                juce::MemoryOutputStream bytes (encoded, false);
                require (juce::Base64::convertFromBase64 (bytes, result["png"].toString().fromFirstOccurrenceOf (",", false, false)),
                         "Original Canvas PNG cannot be decoded");
                const auto image = juce::ImageFileFormat::loadFrom (bytes.getData(), bytes.getDataSize());
                require (image.isValid(), "Original Canvas PNG is invalid");
                record (image);
            }
            catch (const std::exception& error) { finish (false, error.what()); }
        });
    }
   #endif
};
}

int main (int argc, char** argv)
{
   #if JUCE_LINUX
    if (std::getenv ("DISPLAY") == nullptr)
    {
        std::cout << "SKIP: actual waveform renderers require an X display\n";
        return 77;
    }
    std::signal (SIGPIPE, SIG_IGN);
   #endif
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase* { return new Application(); };
    return juce::JUCEApplicationBase::main (argc, const_cast<const char**> (argv));
}
