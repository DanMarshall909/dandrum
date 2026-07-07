#include "PluginProcessor.h"
#include "InstrumentFileWatcher.h"
#include "DefaultPatch.h"

#include <cmath>
#include <iostream>
#include <memory>

namespace
{
constexpr int blockSize = 64;
constexpr double sampleRate = 48000.0;
constexpr int kickNote = 36;

juce::File defaultPatchFile()
{
    return juce::File (juce::String (dandrum::defaultPatchPath().string()));
}

// A copy of the bundled default kick patch with its decay value rewritten, so a
// reload can be proven to have actually taken effect (not just re-read the same
// file). Returns an invalid File if the expected marker is absent, so a future
// drift in the bundled patch fails loudly rather than defanging the test.
juce::File writeKickPatchWithDecay (const juce::String& prefix, const juce::String& decayValue)
{
    auto content = defaultPatchFile().loadFileAsString();
    const juce::String marker = "decay_ms: 650";
    if (! content.contains (marker))
        return {};

    content = content.replace (marker, "decay_ms: " + decayValue);
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile (prefix + juce::String (juce::Random::getSystemRandom().nextInt()) + ".yaml");
    file.replaceWithText (content);
    return file;
}

// Rewrites the decay value of an already-written kick patch file in place,
// simulating an external editor saving an edit to the watched file.
bool editFileDecay (const juce::File& file, const juce::String& from, const juce::String& to)
{
    auto content = file.loadFileAsString();
    const juce::String marker = "decay_ms: " + from;
    if (! content.contains (marker))
        return false;

    content = content.replace (marker, "decay_ms: " + to);
    file.replaceWithText (content);
    return true;
}

bool renderHasSignal (DandrumAudioProcessor& processor)
{
    juce::AudioBuffer<float> buffer (2, blockSize);
    buffer.clear();
    juce::MidiBuffer midi;
    midi.addEvent (juce::MidiMessage::noteOn (1, (juce::uint8) kickNote, (juce::uint8) 100), 0);
    processor.processBlock (buffer, midi);

    for (int channel = 0; channel < buffer.getNumChannels(); ++channel)
    {
        const auto* samples = buffer.getReadPointer (channel);
        for (int i = 0; i < buffer.getNumSamples(); ++i)
        {
            if (! std::isfinite (samples[i]))
                return false;
            if (std::abs (samples[i]) > 0.000001f)
                return true;
        }
    }

    return false;
}

std::unique_ptr<DandrumAudioProcessor> makeProcessorWatching (const juce::File& file)
{
    auto processor = std::make_unique<DandrumAudioProcessor>();
    processor->setPlayConfigDetails (0, 2, sampleRate, blockSize);
    processor->prepareToPlay (sampleRate, blockSize);
    if (! processor->reloadInstrumentFromFile (file))
    {
        std::cerr << "test setup: reloadInstrumentFromFile failed: " << processor->getLastLoadError() << '\n';
        return nullptr;
    }
    return processor;
}
} // namespace

int main()
{
    // 3.1: a detected external change reloads through the standard replacement
    // transaction, and only after the change has been observed as stable.
    {
        const auto file = writeKickPatchWithDecay ("dandrum_watch_reload_", "650");
        if (file == juce::File())
        {
            std::cerr << "bundled default patch no longer contains the expected decay marker\n";
            return 1;
        }

        auto processor = makeProcessorWatching (file);
        if (processor == nullptr)
            return 1;

        if (processor->watchedInstrumentFile() != file)
        {
            std::cerr << "processor did not start watching the reloaded instrument file\n";
            return 1;
        }
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 650"))
        {
            std::cerr << "test setup: watched file did not load the expected baseline decay\n";
            return 1;
        }

        if (! editFileDecay (file, "650", "1900"))
        {
            std::cerr << "could not apply external edit to watched file\n";
            return 1;
        }

        // A single poll detects the change but must not yet reload it.
        processor->pollInstrumentFileForChanges();
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 650"))
        {
            std::cerr << "watched file reloaded before the change was observed as stable\n";
            return 1;
        }

        // Once the change is stable across polls it reloads via the replacement
        // transaction, which returns to the running state afterward.
        processor->pollInstrumentFileForChanges();
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 1900"))
        {
            std::cerr << "detected external change did not reload the instrument\n";
            return 1;
        }
        if (processor->replacementTransactionState() != "running")
        {
            std::cerr << "file-watch reload did not complete through the replacement transaction: "
                      << processor->replacementTransactionState() << '\n';
            return 1;
        }
        if (! renderHasSignal (*processor))
        {
            std::cerr << "processor stopped rendering after a file-watch reload\n";
            return 1;
        }

        processor->releaseResources();
        file.deleteFile();
    }

    // 3.2: a detected change that fails to compile leaves the previous DSP
    // running unchanged and reports the failure.
    {
        const auto file = writeKickPatchWithDecay ("dandrum_watch_reload_fail_", "650");
        if (file == juce::File())
            return 1;

        auto processor = makeProcessorWatching (file);
        if (processor == nullptr)
            return 1;

        if (! renderHasSignal (*processor))
        {
            std::cerr << "test setup: baseline instrument was not rendering before the failed reload\n";
            return 1;
        }

        // An external edit corrupts the file with malformed YAML (an unclosed
        // flow sequence) so it can no longer parse/compile.
        file.replaceWithText ("modules: [unclosed\n");

        processor->pollInstrumentFileForChanges();
        processor->pollInstrumentFileForChanges();

        if (! processor->currentInstrumentYaml().contains ("decay_ms: 650"))
        {
            std::cerr << "a failed file-watch reload replaced the previously loaded instrument\n";
            return 1;
        }
        if (processor->getLastLoadError().isEmpty())
        {
            std::cerr << "a failed file-watch reload did not report an error\n";
            return 1;
        }
        if (processor->replacementTransactionState() != "failed")
        {
            std::cerr << "a failed file-watch reload did not surface the failed transaction state: "
                      << processor->replacementTransactionState() << '\n';
            return 1;
        }
        if (! renderHasSignal (*processor))
        {
            std::cerr << "previous instrument stopped rendering after a failed file-watch reload\n";
            return 1;
        }

        processor->releaseResources();
        file.deleteFile();
    }

    // 3.3: a partially-written (still-changing) file does not trigger a reload
    // until its change signal stabilises.
    {
        const auto file = writeKickPatchWithDecay ("dandrum_watch_reload_partial_", "650");
        if (file == juce::File())
            return 1;

        auto processor = makeProcessorWatching (file);
        if (processor == nullptr)
            return 1;

        // First fragment of an in-progress save: observed once, not yet stable.
        file.replaceWithText ("metadata:\n  name: half-written\n");
        processor->pollInstrumentFileForChanges();
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 650"))
        {
            std::cerr << "a single mid-write observation triggered a premature reload\n";
            return 1;
        }

        // The save then completes with different content, so this poll observes
        // a signal that differs from the earlier fragment and resets the
        // debounce rather than reloading the fragment.
        const auto finalFile = writeKickPatchWithDecay ("dandrum_watch_reload_partial_final_", "1234");
        if (finalFile == juce::File())
            return 1;
        file.replaceWithText (finalFile.loadFileAsString());
        finalFile.deleteFile();

        processor->pollInstrumentFileForChanges();
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 650"))
        {
            std::cerr << "an unstable (still-changing) file triggered a reload before stabilising\n";
            return 1;
        }

        // The file has now stopped changing; the next poll makes it stable and
        // the reload finally happens.
        processor->pollInstrumentFileForChanges();
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 1234"))
        {
            std::cerr << "a stabilised file was not reloaded once it stopped changing\n";
            return 1;
        }

        processor->releaseResources();
        file.deleteFile();
    }

    // 3.4: disabling file watching stops reloads until watching is re-enabled
    // or a manual reload is requested.
    {
        const auto file = writeKickPatchWithDecay ("dandrum_watch_reload_toggle_", "650");
        if (file == juce::File())
            return 1;

        auto processor = makeProcessorWatching (file);
        if (processor == nullptr)
            return 1;

        processor->setFileWatchEnabled (false);
        if (processor->isFileWatchEnabled())
        {
            std::cerr << "setFileWatchEnabled(false) did not disable watching\n";
            return 1;
        }

        if (! editFileDecay (file, "650", "1500"))
            return 1;

        for (int i = 0; i < 3; ++i)
            processor->pollInstrumentFileForChanges();
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 650"))
        {
            std::cerr << "an external edit reloaded while file watching was disabled\n";
            return 1;
        }

        // A manual reload still applies even while watching is disabled.
        if (! processor->reloadInstrumentFromFile (file))
        {
            std::cerr << "manual reload failed while watching was disabled: " << processor->getLastLoadError() << '\n';
            return 1;
        }
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 1500"))
        {
            std::cerr << "manual reload did not apply while watching was disabled\n";
            return 1;
        }

        // Re-enabling watching resumes automatic reloads.
        processor->setFileWatchEnabled (true);
        if (! processor->isFileWatchEnabled())
        {
            std::cerr << "setFileWatchEnabled(true) did not re-enable watching\n";
            return 1;
        }

        if (! editFileDecay (file, "1500", "200"))
            return 1;

        processor->pollInstrumentFileForChanges();
        processor->pollInstrumentFileForChanges();
        if (! processor->currentInstrumentYaml().contains ("decay_ms: 200"))
        {
            std::cerr << "re-enabling file watching did not resume automatic reloads\n";
            return 1;
        }

        processor->releaseResources();
        file.deleteFile();
    }

    return 0;
}
