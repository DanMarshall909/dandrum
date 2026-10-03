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
#include <atomic>
#include <bit>
#include <cmath>
#include <csignal>
#include <cstdlib>
#include <iostream>
#include <memory>
#include <stdexcept>

#if JUCE_LINUX
static std::atomic<bool> rejectSpectralRead { false };
extern "C" bool __real_dandrum_kernel_prepared_source_copy_channel (
    const DandrumKernelWaveformSource*, std::uint16_t, std::uint64_t, float*, std::size_t);
extern "C" bool __wrap_dandrum_kernel_prepared_source_copy_channel (
    const DandrumKernelWaveformSource* source, std::uint16_t channel,
    std::uint64_t start, float* output, std::size_t count)
{
    return ! rejectSpectralRead.load()
        && __real_dandrum_kernel_prepared_source_copy_channel (source, channel, start, output, count);
}
#endif

namespace
{
void require (bool condition, const juce::String& message)
{
    if (! condition)
        throw std::runtime_error (message.toStdString());
}

// Source and host rates differ. Both channels contain distinct bin-centred
// sines, with a nonzero region offset and real fade/loop/slice metadata.
constexpr int startFrame = 768, endFrame = 13056, sourceFrames = 15360;
constexpr int columnCount = 48;

struct Fixture
{
    Fixture()
        : directory (juce::File::getSpecialLocation (juce::File::tempDirectory)
                         .getNonexistentChildFile ("dandrum-spectral-parity", {}, false))
    {
        require (directory.createDirectory().wasOk(), "Cannot create spectral parity fixture");
        juce::AudioBuffer<float> pcm (2, sourceFrames);
        for (int frame = 0; frame < sourceFrames; ++frame)
        {
            pcm.setSample (0, frame, frame < startFrame || frame >= endFrame ? 0.875f
                : 0.5f * static_cast<float> (std::sin (2.0 * juce::MathConstants<double>::pi * 64 * (frame - startFrame) / 1024.0)));
            pcm.setSample (1, frame, 0.25f * static_cast<float> (std::sin (2.0 * juce::MathConstants<double>::pi * 96 * frame / 1024.0)));
        }
        auto stream = directory.getChildFile ("source.wav").createOutputStream();
        require (stream != nullptr, "Cannot open spectral parity WAV");
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
        require (stream->getStatus().wasOk(), "Cannot write spectral parity PCM");
        stream.reset();
        require (patch().replaceWithText (R"YAML(metadata: { name: Spectral Renderer Parity }
instrument: { id: dandrum.spectral-parity, preset_schema_version: 1 }
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
)YAML"), "Cannot write spectral parity patch");
    }
    ~Fixture() { directory.deleteRecursively(); }
    juce::File patch() const { return directory.getChildFile ("instrument.yaml"); }
    void useRetryRegion()
    {
        const auto text = patch().loadFileAsString().replace ("id: body", "id: retry")
            .replace ("signed.body", "signed.retry");
        require (patch().replaceWithText (text), "Cannot prepare uncached retry region");
    }
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
    require (width > 200 && height > 100, "Spectral plot is not visible at its target size");
    const auto colourAt = [&image] (int x, int y) { return image.getPixelAt (x, y).getARGB(); };
    for (const auto& marker : markers)
    {
        const auto x = std::clamp (static_cast<int> (std::lround (marker.fraction * width)), 0, width - 1);
        require (colourAt (x, 1) == marker.colour && colourAt (x, height - 2) == marker.colour,
                 "Prepared marker coordinate differs: " + juce::String (marker.name));
    }
    const auto x = static_cast<int> (std::lround (0.09375 * width)); // Column 4 midpoint.
    const auto row = height / 3; // log(24000/3000) / log(512) = 1/3.
    require (colourAt (x, row) == 0xffeed4b8,
             "Known 0.5 sine band has wrong source-rate row or declared dBFS colour");
    require (colourAt (x, height - 2) == 0xff130f0c,
             "Silent positive-frequency floor did not paint Ink 0");
}

#if ! JUCE_WEB_BROWSER
juce::Component* findComponent (juce::Component& parent, const juce::String& id)
{
    if (parent.getComponentID() == id)
        return &parent;
    for (auto* child : parent.getChildren())
        if (auto* found = findComponent (*child, id))
            return found;
    return nullptr;
}
#endif

class Application final : public juce::JUCEApplication, private juce::Timer
{
public:
    const juce::String getApplicationName() override { return "Dandrum Spectral Parity Test"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise (const juce::String&) override
    {
        try
        {
            std::unique_ptr<juce::AudioProcessor> original (createPluginFilter());
            auto* sampler = dynamic_cast<DandrumAudioProcessor*> (original.get());
            require (sampler != nullptr && sampler->demoConfiguration().instrumentId == "dandrum.advanced-drum-kit",
                     "Spectral parity did not start with the original sampler factory");
            original.release();
            processor.reset (sampler);
            processor->setPlayConfigDetails (0, 2, 44100.0, 64);
            processor->prepareToPlay (44100.0, 64);
            require (processor->reloadInstrumentFromFile (fixture.patch()), "Spectral parity fixture failed to prepare");
            generation = processor->getPreparedUiDocument()->generation;
            const auto request = processor->requestPreparedSpectrum (generation, "signed", "body", 0);
            require (request.has_value(), "Numeric spectral job was rejected");
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
            panel = findComponent (*editor, "prepared-spectrum");
            require (panel != nullptr, "Original sampler did not create its native spectrum");
            auto* toggle = dynamic_cast<juce::TextButton*> (findComponent (*editor, "sample-display-spectral"));
            require (toggle != nullptr, "Native spectral toggle is missing");
            toggle->triggerClick();
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
    enum class Phase { plots, wave, spectrumAgain, hidden, resumed, failed, retried, sampleFree };
    Phase phase = Phase::plots;
    int hiddenPolls = 0;
    Fixture fixture;
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    std::uint32_t generation = 0;
    std::uint64_t numericJob = 0;
    bool compact = false, numericChecked = false, finished = false;
    const double deadline = juce::Time::getMillisecondCounterHiRes() + 25000.0;
   #if JUCE_WEB_BROWSER
    juce::WebBrowserComponent* browser = nullptr;
    unsigned pendingObservations = 0;
    juce::String lastWebObservation;
   #else
    juce::Component* panel = nullptr;
   #endif

    bool verifyNumericResult()
    {
        if (numericChecked)
            return true;
        const auto job = processor->getPreparedSpectrumJobStatus (numericJob);
        require (job.has_value(), "Numeric spectral job disappeared");
        if (job->state == InstrumentUiSpectralService::State::running) return false;
        require (job->state == InstrumentUiSpectralService::State::ready && job->result != nullptr,
                 "Numeric spectrum did not become ready");
        const auto& result = *job->result;
        require (std::abs (processor->getSampleRate() - 44100.0) < 0.5 && result.sampleRateHz == 48000
                     && result.startFrame == startFrame && result.endFrame == endFrame && result.channel == 0
                     && result.columns.size() == columnCount && std::abs (result.frequencyHz[64] - 3000.0) < 0.00001,
                 "Spectrum source frames/rate/channel were replaced by host coordinates");
        for (std::size_t i = 0; i < result.columns.size(); ++i)
        {
            const auto& column = result.columns[i];
            require (column.startFrame == startFrame + i * 256
                         && column.endFrame == std::min<std::uint64_t> (endFrame, startFrame + i * 256 + 1024),
                     "Numeric spectral columns lost prepared bounds");
            if (i < 45)
                require (std::abs (column.magnitudeDbFS[64] + 6.020599913f) < 0.002f
                             && column.magnitudeDbFS[1] < -119.99f,
                         "Known sine/floor numeric magnitude differs");
        }
        numericChecked = true;
        std::cout << "Numeric spectral job ready\n";
        return true;
    }

    void record (const juce::Image& plot)
    {
        verifyPixels (plot);
        require (processor->getPreparedUiDocument()->generation == generation,
                 "Resizing rebuilt the instrument");
        if (const auto* output = std::getenv ("DANDRUM_SPECTRAL_PARITY_SNAPSHOTS"))
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
        std::cout << "SPECTRAL_PARITY " << (compact ? "820x560" : "1200x800")
                  << " plot=" << plot.getWidth() << 'x' << plot.getHeight()
                  << " source=48000 host=44100 frames=768..13056 duration=0.256"
                     " sine_hz=3000 peak_dbfs=-6.0206 prepared_markers=10 PASS\n";
        if (compact)
            phase = Phase::wave;
        else
        {
            compact = true;
            editor->setSize (820, 560);
        }
    }

    void advancePhase()
    {
        switch (phase)
        {
            case Phase::wave: phase = Phase::spectrumAgain; break;
            case Phase::spectrumAgain:
                editor->setVisible (false); phase = Phase::hidden; break;
            case Phase::resumed:
               #if JUCE_LINUX
                rejectSpectralRead.store (true);
                fixture.useRetryRegion();
                require (processor->reloadInstrumentFromFile (fixture.patch()), "Retry fixture reload failed");
                generation = processor->getPreparedUiDocument()->generation;
                phase = Phase::failed;
               #else
                loadSampleFree();
               #endif
                break;
            case Phase::failed:
               #if JUCE_LINUX
                rejectSpectralRead.store (false);
               #endif
                phase = Phase::retried;
                break;
            case Phase::retried: loadSampleFree(); break;
            case Phase::sampleFree:
                require (processor->getPreparedUiDocument()->sources.empty(), "Sample-free reload retained prepared sources");
                std::cout << "SPECTRAL_LIFETIME wave/spectral, hide/show, reload clear PASS\n";
               #if JUCE_LINUX
                std::cout << "SPECTRAL_FAILURE real PCM read failure, retry PASS\n";
               #else
                std::cout << "SKIP: PCM read-failure injection requires the Linux link wrapper\n";
               #endif
                finish (true);
                break;
            case Phase::plots:
            case Phase::hidden: break;
        }
    }

    void loadSampleFree()
    {
        require (processor->reloadInstrumentFromFile (juce::File (
            InstrumentDemoConfiguration::tb303().instrumentPath.string())), "Sample-free instrument failed to reload");
        generation = processor->getPreparedUiDocument()->generation;
        phase = Phase::sampleFree;
    }

    void finish (bool success, const juce::String& error = {})
    {
        if (finished)
            return;
        finished = true;
        stopTimer();
        if (! success)
            std::cerr << "Spectral parity failed: " << error << '\n';
        setApplicationReturnValue (success ? 0 : 1);
        quit();
    }

    void timerCallback() override
    {
        try
        {
            require (juce::Time::getMillisecondCounterHiRes() <= deadline,
                     "Actual spectral renderer timed out"
                    #if JUCE_WEB_BROWSER
                     + juce::String (": ") + lastWebObservation
                     + " (pending observations: " + juce::String (pendingObservations) + ")"
                    #endif
                    );
            if (! verifyNumericResult())
                return;
            if (phase == Phase::hidden)
            {
                if (++hiddenPolls >= 3) { editor->setVisible (true); phase = Phase::resumed; }
                return;
            }
           #if JUCE_WEB_BROWSER
            pollWeb();
           #else
            if (phase != Phase::plots)
            {
                pollNativeLifecycle();
                return;
            }
            if (panel->getName() != "Prepared spectrum: signed.body")
                return;
            const auto snapshot = panel->createComponentSnapshot (panel->getLocalBounds());
            const auto text = [this] (const juce::String& id) {
                auto* label = dynamic_cast<juce::Label*> (findComponent (*panel, id));
                require (label != nullptr && label->isShowing(), "Native spectral axis label missing");
                return label->getText();
            };
            require (text ("spectral-frequency-min") == "47 Hz" && text ("spectral-frequency-max") == "24 kHz"
                         && text ("spectral-time-start") == "0.016 s" && text ("spectral-time-end") == "0.272 s"
                         && text ("spectral-settings") == juce::String::fromUTF8 (
                             "Hann · FFT 1024 · hop 256 · -120..0 dBFS · ch 1 · 48000 Hz · DC omitted"),
                     "Native labels lost source time/frequency/settings");
            record (snapshot.getClippedImage ({64, 52, panel->getWidth() - 80, panel->getHeight() - 110}));
           #endif
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }

   #if ! JUCE_WEB_BROWSER
    void pollNativeLifecycle()
    {
        const auto spectral = phase != Phase::wave;
        auto* button = dynamic_cast<juce::TextButton*> (findComponent (*editor,
            spectral ? "sample-display-spectral" : "sample-display-wave"));
        require (button != nullptr, "Native display switch disappeared");
        if (! button->getToggleState()) { button->triggerClick(); return; }
        if (phase == Phase::sampleFree)
        {
            if (panel->getName() != "NO PREPARED SAMPLE") return;
            for (const auto* id : { "spectral-frequency-min", "spectral-frequency-max", "spectral-time-start", "spectral-time-end", "spectral-settings" })
                require (dynamic_cast<juce::Label*> (findComponent (*panel, id))->getText().isEmpty(),
                         "Native reload retained obsolete spectral labels");
            advancePhase(); return;
        }
        if (phase == Phase::wave)
        {
            auto* wave = findComponent (*editor, "prepared-waveform");
            if (! wave || wave->getName() != "Prepared waveform: signed.body") return;
            require (wave->isShowing() && ! panel->isShowing(), "Native view switch retained both views");
            advancePhase(); return;
        }
        if (phase == Phase::failed)
        {
            if (! panel->getName().contains ("SPECTRUM UNAVAILABLE")) return;
            require (panel->getName().contains ("Prepared spectral analysis failed"), "Native analysis error is unavailable");
            advancePhase(); button->triggerClick(); return;
        }
        if (panel->getName() != (phase == Phase::retried ? "Prepared spectrum: signed.retry" : "Prepared spectrum: signed.body")) return;
        require (panel->isShowing() && ! findComponent (*editor, "prepared-waveform")->isShowing(),
                 "Native spectral view did not resume alone");
        verifyPixels (panel->createComponentSnapshot (panel->getLocalBounds()).getClippedImage (
            {64, 52, panel->getWidth() - 80, panel->getHeight() - 110}));
        advancePhase();
    }
   #endif

   #if JUCE_WEB_BROWSER
    void pollWeb()
    {
        if (pendingObservations >= 2)
            return;
        // Observe the original React mount, native transport and Canvas. No
        // substitute page, injected peaks, helper rendering or mock host.
        const auto script = (juce::String (R"JS((() => {
          try {
          const phase = __PHASE__;
          if (document.querySelector('.generation')?.textContent !== 'GEN __GENERATION__') return '';
          const report = extra => JSON.stringify({phase, viewportWidth:innerWidth,viewportHeight:innerHeight,...extra});
          if (phase === 7) {
            if (document.querySelector('.spectrogram,.waveform')
                || !document.body.textContent.includes('Prepared sample analysis unavailable')) return '';
            return report({lifecycle:true});
          }
          const toggle = document.querySelector(phase === 1 ? '[data-sample-display="wave"]' : '[data-sample-display="spectral"]');
          if (!toggle) return 'WAIT: toggle missing: ' + document.body.textContent;
          if (toggle.getAttribute('aria-pressed') !== 'true') { toggle.click(); return ''; }
          if (phase === 1) {
            if (document.querySelector('.spectrogram')
                || !document.querySelector('.waveform')?.textContent.includes('48000 Hz \u00b7 0.256 s')) return '';
            return report({lifecycle:true});
          }
          const panel = document.querySelector('.spectrogram'), canvas = panel?.querySelector('canvas');
          if (panel?.querySelector('h2')?.textContent !== (phase >= 5 ? 'signed.retry' : 'signed.body')) return '';
          if (phase === 5) {
            if (!panel?.querySelector('.spectral-retry')) return '';
            if (!panel.textContent.includes('Prepared spectral analysis failed')
                || !panel.textContent.includes('SPECTRUM UNAVAILABLE')) throw new Error('Spectral failure is not displayed');
            return report({lifecycle:true});
          }
          if (phase === 6 && panel?.querySelector('.spectral-retry')) {
            panel.querySelector('.spectral-retry').click(); return '';
          }
          if (!canvas || !panel.textContent.includes('48000 Hz \u00b7 0.256 s'))
            return 'WAIT: spectrum: ' + panel?.textContent;
          const label = id => panel.querySelector(`[data-spectral-label="${id}"]`)?.textContent;
          if (label('frequency-min') !== '47 Hz' || label('frequency-max') !== '24 kHz'
              || label('time-start') !== '0.016 s' || label('time-end') !== '0.272 s'
              || !label('settings')?.includes('1024') || !label('settings')?.includes('256')
              || !label('settings')?.includes('DC omitted')) throw new Error('Web spectral labels differ');
          const controls = document.querySelector('.sample-display-controls').getBoundingClientRect();
          for (const node of panel.querySelectorAll('[data-spectral-label]')) {
            const box = node.getBoundingClientRect();
            if (box.width <= 0 || box.height <= 0 || box.left < 0 || box.right > innerWidth
                || box.top < 0 || box.bottom > innerHeight
                || (box.left < controls.right && box.right > controls.left
                  && box.top < controls.bottom && box.bottom > controls.top))
              throw new Error('Spectral label obscured: ' + node.getAttribute('data-spectral-label'));
          }
          const r = canvas.getBoundingClientRect();
          if (innerWidth !== )JS") + juce::String (compact ? 820 : 1200)
            + " || innerHeight !== " + juce::String (compact ? 560 : 800)
            + R"JS( || canvas.width !== Math.round(r.width * devicePixelRatio))
            return 'WAIT: viewport: ' + JSON.stringify({width:innerWidth,height:innerHeight,
              canvas:canvas.width,css:r.width,ratio:devicePixelRatio});
          return report({png:canvas.toDataURL('image/png'),ratio:devicePixelRatio,
            width:r.width,height:r.height,left:r.left,right:r.right,top:r.top,bottom:r.bottom,
            viewportWidth:innerWidth,viewportHeight:innerHeight});
          } catch(error) { return 'ERROR: ' + String(error); }
        })())JS").replace ("__PHASE__", juce::String (static_cast<int> (phase)))
                  .replace ("__GENERATION__", juce::String (generation));
        ++pendingObservations;
        juce::Component::SafePointer<juce::AudioProcessorEditor> alive (editor.get());
        browser->evaluateJavascript (script, [this, alive] (auto evaluation)
        {
            if (alive == nullptr || finished)
                return;
            --pendingObservations;
            try
            {
                require (evaluation.getError() == nullptr, "Original browser observation failed");
                const auto* value = evaluation.getResult();
                require (value != nullptr, "Original browser returned no spectral observation");
                // WebKit may acknowledge an evaluation during initial page
                // navigation without a value. The deadline still requires a
                // real mounted, visible and painted Canvas at both sizes.
                if (value->isUndefined() || value->isVoid())
                    return;
                require (value->isString(), "Original browser returned an invalid spectral observation");
                if (value->toString().isEmpty())
                    return;
                if (value->toString().startsWith ("WAIT: "))
                {
                    lastWebObservation = value->toString();
                    return;
                }
                require (! value->toString().startsWith ("ERROR: "), value->toString());
                const auto result = juce::JSON::parse (value->toString());
                if (result.isObject() && static_cast<int> (result["phase"]) != static_cast<int> (phase)) return;
                if (result.isObject() && (static_cast<int> (result["viewportWidth"]) != editor->getWidth()
                    || static_cast<int> (result["viewportHeight"]) != editor->getHeight()))
                    return;
                require (result.isObject(), "Original browser returned invalid spectral state");
                if (static_cast<bool> (result["lifecycle"])) { advancePhase(); return; }
                require (std::abs (static_cast<double> (result["ratio"]) - 1.0) < 0.00001,
                         "Spectral pixel parity lane requires documented 1x display scaling");
                require (static_cast<double> (result["left"]) >= 0.0
                             && static_cast<double> (result["right"]) <= static_cast<double> (result["viewportWidth"])
                             && static_cast<double> (result["top"]) >= 0.0
                             && static_cast<double> (result["bottom"]) <= static_cast<double> (result["viewportHeight"]),
                         "Actual React spectrum is outside the target viewport");
                juce::MemoryBlock encoded;
                juce::MemoryOutputStream bytes (encoded, false);
                require (juce::Base64::convertFromBase64 (bytes, result["png"].toString().fromFirstOccurrenceOf (",", false, false)),
                         "Original Canvas PNG cannot be decoded");
                const auto image = juce::ImageFileFormat::loadFrom (bytes.getData(), bytes.getDataSize());
                require (image.isValid(), "Original Canvas PNG is invalid");
                if (phase == Phase::plots) record (image);
                else { verifyPixels (image); advancePhase(); }
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
        std::cout << "SKIP: actual spectral renderers require an X display\n";
        return 77;
    }
    std::signal (SIGPIPE, SIG_IGN);
   #endif
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase* { return new Application(); };
    return juce::JUCEApplicationBase::main (argc, const_cast<const char**> (argv));
}
