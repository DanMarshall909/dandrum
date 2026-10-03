#pragma once

namespace liveRendererCheck
{
juce::Component* find (juce::Component& parent, const juce::String& id)
{
    if (parent.getComponentID() == id) return &parent;
    for (auto* child : parent.getChildren())
        if (auto* result = find (*child, id)) return result;
    return nullptr;
}

class Application final : public juce::JUCEApplication, private juce::Timer
{
public:
    const juce::String getApplicationName() override { return "Dandrum Live Renderer Check"; }
    const juce::String getApplicationVersion() override { return "1"; }
    void initialise (const juce::String&) override
    {
        try
        {
            directory = juce::File::getSpecialLocation (juce::File::tempDirectory)
                .getNonexistentChildFile ("dandrum-live-renderer", {}, false);
            require (directory.createDirectory().wasOk(), "Cannot create live renderer fixture");
            require (patch().replaceWithText (R"YAML(metadata: { name: Live Renderer Fixture }
instrument: { id: dandrum.live-renderer-fixture, preset_schema_version: 1 }
ports:
  - { name: level, direction: input, signal: control, channels: 1, default: 0, min: -1, max: 1, maps_to: amp.gain }
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: amp.audio_out }
preset_surface:
  parameters:
    - { name: fixture.level, maps_to: level }
modules:
  - { id: source, type: control_to_audio, static: { channels: 2 }, defaults: { in: 1 } }
  - { id: amp, type: gain, static: { channels: 2 } }
connections:
  - { from: source.out, to: amp.audio_in }
)YAML"), "Cannot write live renderer fixture");
            std::unique_ptr<juce::AudioProcessor> original (createPluginFilter());
            auto* sampler = dynamic_cast<DandrumAudioProcessor*> (original.get());
            require (sampler && sampler->demoConfiguration().instrumentId == "dandrum.advanced-drum-kit",
                     "Live renderer did not start with original sampler factory");
            original.release(); processor.reset (sampler);
            processor->setPlayConfigDetails (0, 2, 96000, 64); processor->prepareToPlay (96000, 64);
            require (processor->reloadInstrumentFromFile (patch()), "Live renderer fixture failed to load");
            processor->setFileWatchEnabled (false);
            generation = processor->getParameterSurfaceGeneration();
            setLevel (0.75f);
            for (auto& session : occupiedSessions)
            {
                session = processor->uiCommands().createSession();
                require (processor->subscribeLiveAnalysis (session, generation, 3), "Cannot occupy real live-analysis slots");
            }
            editor.reset (processor->createEditor());
            require (editor != nullptr, "Live renderer factory produced no editor");
            editor->setSize (1200, 800); editor->addToDesktop (juce::ComponentPeer::windowHasTitleBar);
            editor->setVisible (true);
           #if JUCE_WEB_BROWSER
            auto* web = dynamic_cast<DandrumAudioProcessorEditor*> (editor.get());
            require (web, "Live renderer factory produced no Web editor");
            browser = &PluginEditorBridgeTestProbe::runtimeBrowser (*web);
           #else
            require (find (*editor, "sample-display-scope"), "Original native editor has no live Scope control");
           #endif
            startTimer (25);
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }
    void shutdown() override
    {
        stopTimer(); editor.reset(); processor.reset(); directory.deleteRecursively();
    }
private:
    juce::File directory;
    juce::File patch() const { return directory.getChildFile ("instrument.yaml"); }
    std::unique_ptr<DandrumAudioProcessor> processor;
    std::unique_ptr<juce::AudioProcessorEditor> editor;
    std::uint32_t generation = 0;
    int stage = -1, selectedStage = -2;
    std::array<std::uint64_t, 4> occupiedSessions {};
    std::int64_t processedFrames = 0, minimumStartFrame = 0;
    bool finished = false;
    double deadline = juce::Time::getMillisecondCounterHiRes() + 25000;
   #if JUCE_WEB_BROWSER
    juce::WebBrowserComponent* browser = nullptr;
    bool evaluating = false, observedRejectedReply = false;
   #endif
    void setLevel (float normalised)
    {
        auto* parameter = processor->getParameterForPublicId ("fixture.level");
        require (parameter, "Live renderer host parameter is absent");
        parameter->setValueNotifyingHost (normalised);
    }
    bool compact() const { return stage == 3 || stage == 5; }
    bool spectral() const { return stage == 4 || stage == 5; }
    int channel() const { return stage == 1 ? 1 : 0; }
    float amplitude() const { return stage == 1 ? 0.0f : stage < 2 || stage >= 6 ? 0.5f : -0.5f; }
    void releaseSlots()
    {
        for (auto& session : occupiedSessions)
        {
            require (processor->unsubscribeLiveAnalysis (session), "Cannot release occupied live-analysis slot");
            processor->uiCommands().closeSession (session); session = 0;
        }
        std::cout << "LIVE_RENDERER admission_rejection=PASS\n";
    }
    void verifyPixels (const juce::Image& plot)
    {
        const auto w = plot.getWidth(), h = plot.getHeight();
        require (w > 200 && h >= 100, "Live plot is not visible at target size");
        if (! spectral())
        {
            const auto y = static_cast<int> (std::lround ((1 - amplitude()) * h / 2));
            for (const int bucket : { 9, 29, 67, 111 })
            {
                const auto x = static_cast<int> (std::lround ((bucket + 0.5) / 128 * w));
                require (plot.getPixelAt (x, y).getARGB() == 0xfff6aa69
                             && plot.getPixelAt (x, std::max (0, y - 2)).getARGB() == 0xff161616,
                         "Actual live scope lost the selected signed amplitude");
            }
        }
        else
        {
            // A constant signed half-scale PCM window has -6.0206 dBFS at
            // Hann bin 1 (93.75 Hz); higher bins are at the -120 dBFS floor.
            const auto y = static_cast<int> (std::lround (6.020599913 / 120 * h));
            bool observed = false;
            for (int px = 0; px < 3; ++px)
                for (int py = std::max (0, y - 2); py <= y + 2; ++py)
                {
                    const auto colour = plot.getPixelAt (px, py);
                    observed |= colour.getRed() > 180 && colour.getGreen() > 100 && colour.getBlue() < 140;
                }
            require (observed, "Actual live spectrum lost the known Hann bin-1 amplitude");
            require (plot.getPixelAt (w / 2, h / 4).getARGB() == 0xff161616,
                     "Actual live spectrum invented energy above the floor");
        }
    }
    void advance (const juce::Image& image)
    {
        verifyPixels (image);
        std::cout << "LIVE_RENDERER stage=" << stage << " mode=" << (spectral() ? "spectrum" : "scope")
                  << " size=" << (compact() ? "820x560" : "1200x800") << " signed=" << amplitude() << " PASS\n";
        if (const auto* output = std::getenv ("DANDRUM_LIVE_RENDERER_SNAPSHOTS"))
        {
            const auto file = juce::File (output).getChildFile (
               #if JUCE_WEB_BROWSER
                "web-"
               #else
                "native-"
               #endif
                + juce::String (stage) + ".png");
            file.getParentDirectory().createDirectory(); juce::MemoryOutputStream stream;
            juce::PNGImageFormat png;
            require (png.writeImageToStream (image, stream)
                && file.replaceWithData (stream.getData(), stream.getDataSize()), "Cannot retain actual live plot");
        }
        if (++stage > 8) { finish (true); return; }
        if (stage == 7) minimumStartFrame = processedFrames;
        editor->setSize (compact() ? 820 : 1200, compact() ? 560 : 800);
        if (stage == 2) { setLevel (0.25f); minimumStartFrame = processedFrames; }
        if (stage == 6)
        {
            require (processor->reloadInstrumentFromFile (patch()), "Live renderer reload failed");
            generation = processor->getParameterSurfaceGeneration(); setLevel (0.75f);
            processedFrames = minimumStartFrame = 0;
        }
    }
    void finish (bool passed, const juce::String& error = {})
    {
        if (finished) return;
        finished = true; stopTimer();
        if (! passed) std::cerr << "Live renderer failed: " << error << '\n';
        setApplicationReturnValue (passed ? 0 : 1); quit();
    }
    void timerCallback() override
    {
        try
        {
           #if JUCE_WEB_BROWSER
            require (juce::Time::getMillisecondCounterHiRes() < deadline,
                stage == -1 && observedRejectedReply
                    ? "React live admission rejection was left as waiting after its real reply"
                    : "Actual live renderer timed out");
           #else
            require (juce::Time::getMillisecondCounterHiRes() < deadline, "Actual live renderer timed out");
           #endif
            juce::AudioBuffer<float> audio (2, 64); juce::MidiBuffer midi;
            for (int i = 0; i < 16; ++i) { audio.clear(); processor->processBlock (audio, midi); }
            processedFrames += 1024;
            require (std::bit_cast<std::uint32_t> (audio.getSample (0, 63))
                    == std::bit_cast<std::uint32_t> (stage < 2 || stage >= 6 ? 0.5f : -0.5f)
                && std::abs (audio.getSample (1, 63)) < 0.000001f, "Real engine did not render literal signed fixture output");
           #if JUCE_WEB_BROWSER
            pollWeb();
           #else
            if (stage == -1)
            {
                if (selectedStage != stage)
                {
                    auto* scope = dynamic_cast<juce::Button*> (find (*editor, "sample-display-scope"));
                    require (scope, "No native Scope control for admission check"); scope->triggerClick(); selectedStage = stage;
                }
                else
                {
                    auto* frames = dynamic_cast<juce::Label*> (find (*editor, "live-analysis-frames"));
                    require (frames && frames->getText().containsIgnoreCase ("unavailable"),
                        "Native live admission rejection was left as waiting for audio");
                    releaseSlots();
                    dynamic_cast<juce::Button*> (find (*editor, "sample-display-wave"))->triggerClick();
                    stage = 0; minimumStartFrame = processedFrames;
                }
                return;
            }
            if (stage == 7)
            {
                if (selectedStage != stage)
                {
                    auto* wave = dynamic_cast<juce::Button*> (find (*editor, "sample-display-wave"));
                    require (wave, "Native hidden-view check has no Wave control"); wave->triggerClick(); selectedStage = stage;
                }
                else if (processedFrames >= minimumStartFrame + 4096)
                {
                    require (! find (*editor, "live-analysis")->isVisible(), "Native live view remained visible after Wave selection");
                    std::cout << "LIVE_RENDERER hidden_view=PASS\n";
                    stage = 8; minimumStartFrame = processedFrames;
                }
                return;
            }
            if (selectedStage != stage)
            {
                auto* mode = dynamic_cast<juce::Button*> (find (*editor,
                    spectral() ? "sample-display-live-spectrum" : "sample-display-scope"));
                auto* selected = dynamic_cast<juce::Button*> (find (*editor,
                    channel() == 0 ? "live-channel-l" : "live-channel-r"));
                require (mode && selected, "Original native editor is missing live controls");
                mode->triggerClick(); selected->triggerClick(); selectedStage = stage;
                return;
            }
            auto* panel = find (*editor, "live-analysis");
            if (! panel || ! panel->getName().startsWith ("Live " + juce::String (spectral() ? "spectrum" : "scope")
                + " GEN " + juce::String (generation) + " ch " + juce::String (channel() + 1))) return;
            const auto frameText = panel->getName().fromFirstOccurrenceOf (" frames ", false, false);
            if (frameText.upToFirstOccurrenceOf ("..", false, false).getLargeIntValue() < minimumStartFrame) return;
            require (frameText.fromFirstOccurrenceOf ("..", false, false).getLargeIntValue()
                - frameText.upToFirstOccurrenceOf ("..", false, false).getLargeIntValue() == 1024,
                "Actual native live display has incoherent frame bounds");
            const auto* frames = dynamic_cast<juce::Label*> (find (*editor, "live-analysis-frames"));
            const auto* settings = dynamic_cast<juce::Label*> (find (*editor, "live-analysis-settings"));
            require (frames && frames->getText().contains ("96000 Hz") && frames->getText().contains ("10.667 ms")
                && settings && settings->getText().contains ("Hann") && settings->getText().contains ("FFT 1024")
                && settings->getText().contains ("hop 256") && settings->getText().contains ("-120..0 dBFS"),
                "Actual native live measurement settings are missing or use the wrong rate");
            if (spectral()) require (settings->getText().contains ("93.75..48000 Hz")
                && settings->getText().contains ("DC omitted"), "Native log-frequency labels use wrong host rate");
            const auto snapshot = panel->createComponentSnapshot (panel->getLocalBounds());
            advance (snapshot.getClippedImage ({16, 52, panel->getWidth() - 32, panel->getHeight() - 116}));
           #endif
        }
        catch (const std::exception& error) { finish (false, error.what()); }
    }
   #if JUCE_WEB_BROWSER
    void pollWeb()
    {
        if (evaluating) return;
        const auto script = juce::String (R"JS((() => {
          if (!document.querySelector('.sampler .lower')) return '';
          if (!window.__liveReplyObserver) {
            const backend = window.__JUCE__?.backend;
            if (!backend) return '';
            const get = backend.getNativeFunction.bind(backend);
            // Record the actual native Promise reply without changing its value,
            // request, rejection or ordering. No substitute host or view data.
            backend.getNativeFunction = name => {
              const original = get(name);
              return name === 'subscribeLiveAnalysis' ? (...args) => original(...args).then(reply => {
                window.__liveSubscribeReply = {generation:args[0], reply}; return reply;
              }) : original;
            };
            window.__liveReplyObserver = true;
          }
          const mode = document.querySelector('[data-sample-display=")JS")
            + (stage == 7 ? "wave" : spectral() ? "live-spectrum" : "scope") + R"JS("]');
          if (!mode) return 'ERROR: Original React editor has no live display control';
          if (mode.getAttribute('aria-pressed') !== 'true') { mode.click(); return ''; }
          if ()JS" + juce::String (stage) + R"JS( === -1) {
            const panel = document.querySelector('.live-analysis');
            if (!panel) return '';
            const actual = window.__liveSubscribeReply;
            return JSON.stringify({replySeen:!!actual, reply:actual?.reply,
              admissionRejected:panel.textContent.toLowerCase().includes('unavailable')});
          }
          if ()JS" + juce::String (stage) + R"JS( === 7) {
            if (document.querySelector('.live-analysis')) return 'ERROR: Live view remained mounted after Wave selection';
            return JSON.stringify({hidden:true});
          }
          const channel = document.querySelector('[data-live-channel=")JS" + juce::String (channel()) + R"JS("]');
          if (!channel) return 'ERROR: Original React editor has no live channel control';
          if (channel.getAttribute('aria-pressed') !== 'true') { channel.click(); return ''; }
          const panel = document.querySelector('.live-analysis'), canvas = panel?.querySelector('canvas');
          if (!canvas || panel.dataset.generation !== ')JS" + juce::String (generation) + R"JS('
            || panel.dataset.channel !== ')JS" + juce::String (channel()) + R"JS('
            || panel.dataset.mode !== ')JS" + (spectral() ? "spectrum" : "scope") + R"JS(') return '';
          if (!panel.textContent.includes('96000 Hz') || !panel.textContent.includes('1024')
            || !panel.textContent.includes('Hann') || !panel.textContent.includes('-120'))
            return 'ERROR: Actual live measurement settings are missing';
          const r = canvas.getBoundingClientRect();
          if (innerWidth !== )JS" + juce::String (compact() ? 820 : 1200)
            + " || innerHeight !== " + juce::String (compact() ? 560 : 800) + R"JS() return '';
          return JSON.stringify({png:canvas.toDataURL('image/png'), ratio:devicePixelRatio,
            left:r.left,top:r.top,right:r.right,bottom:r.bottom,width:innerWidth,height:innerHeight,
            start:panel.dataset.startFrame,end:panel.dataset.endFrame});
        })())JS";
        evaluating = true;
        juce::Component::SafePointer<juce::AudioProcessorEditor> alive (editor.get());
        browser->evaluateJavascript (script, [this, alive] (auto evaluation)
        {
            if (! alive || finished) return;
            evaluating = false;
            try
            {
                require (! evaluation.getError(), "Original live browser observation failed");
                const auto* value = evaluation.getResult();
                if (! value || value->isVoid() || value->isUndefined() || value->toString().isEmpty()) return;
                require (! value->toString().startsWith ("ERROR: "), value->toString());
                const auto result = juce::JSON::parse (value->toString());
                if (stage == -1)
                {
                    if (! static_cast<bool> (result["replySeen"])) return;
                    require (result["reply"].isBool() && ! static_cast<bool> (result["reply"]),
                        "Original host admitted a fifth live-analysis subscription");
                    observedRejectedReply = true;
                    if (! static_cast<bool> (result["admissionRejected"])) return;
                    releaseSlots(); stage = 0; minimumStartFrame = processedFrames;
                    browser->evaluateJavascript ("document.querySelector('[data-sample-display=wave]').click()", {});
                    return;
                }
                if (stage == 7 && static_cast<bool> (result["hidden"]))
                {
                    if (processedFrames < minimumStartFrame + 4096) return;
                    std::cout << "LIVE_RENDERER hidden_view=PASS\n";
                    stage = 8; minimumStartFrame = processedFrames; return;
                }
                if (result["start"].toString().getLargeIntValue() < minimumStartFrame) return;
                require (result.isObject() && std::abs (static_cast<double> (result["ratio"]) - 1) < 0.000001,
                    "Live pixel lane requires documented 1x display scaling");
                require (static_cast<double> (result["left"]) >= 0 && static_cast<double> (result["top"]) >= 0
                    && static_cast<double> (result["right"]) <= static_cast<double> (result["width"])
                    && static_cast<double> (result["bottom"]) <= static_cast<double> (result["height"]),
                    "Actual live Canvas is outside target viewport");
                require (result["end"].toString().getLargeIntValue() - result["start"].toString().getLargeIntValue() == 1024,
                    "Actual live display has incoherent output-stream frame bounds");
                juce::MemoryOutputStream bytes;
                require (juce::Base64::convertFromBase64 (bytes, result["png"].toString().fromFirstOccurrenceOf (",", false, false)),
                    "Cannot decode original live Canvas");
                const auto image = juce::ImageFileFormat::loadFrom (bytes.getData(), bytes.getDataSize());
                require (image.isValid(), "Original live Canvas is invalid"); advance (image);
            }
            catch (const std::exception& error) { finish (false, error.what()); }
        });
    }
   #endif
};
}
