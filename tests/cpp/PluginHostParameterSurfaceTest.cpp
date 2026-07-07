#include "PluginProcessor.h"
#include "DefaultPatch.h"

#include <algorithm>
#include <iostream>
#include <memory>

namespace
{
juce::File writePatchWithNoPublicParameters()
{
    auto file = juce::File::getSpecialLocation (juce::File::tempDirectory)
                    .getChildFile ("dandrum_host_surface_no_public_parameters_"
                                   + juce::String (juce::Random::getSystemRandom().nextInt())
                                   + ".yaml");
    file.replaceWithText (
        "metadata:\n"
        "  name: Host Surface No Public Parameters\n"
        "instrument:\n"
        "  id: dandrum.host-surface-no-public-parameters\n"
        "  preset_schema_version: 1\n"
        "render:\n"
        "  sample_rate_hz: 48000\n"
        "  block_size_frames: 64\n"
        "  duration_frames: 128\n"
        "modules:\n"
        "  - id: osc\n"
        "    type: oscillator\n"
        "  - id: mixer\n"
        "    type: audio_mixer\n"
        "  - id: out\n"
        "    type: audio_output\n"
        "    inputs:\n"
        "      - name: left\n"
        "        signal_type: audio\n"
        "      - name: right\n"
        "        signal_type: audio\n"
        "connections:\n"
        "  - from: osc.audio\n"
        "    to: mixer.inputs\n"
        "  - from: mixer.mix\n"
        "    to: out.left\n"
        "  - from: mixer.mix\n"
        "    to: out.right\n");
    return file;
}

bool hostParameterListContains (const juce::Array<juce::AudioProcessorParameter*>& parameters,
                                juce::RangedAudioParameter* parameter)
{
    return std::find (parameters.begin(), parameters.end(), parameter) != parameters.end();
}
} // namespace

int main()
{
    constexpr int expectedFixedHostParameterSlots = 64;
    constexpr int blockSize = 64;

    auto processor = std::make_unique<DandrumAudioProcessor>();
    processor->setPlayConfigDetails (0, 2, 48000.0, blockSize);
    processor->prepareToPlay (48000.0, blockSize);

    if (! processor->isInstrumentLoaded())
    {
        std::cerr << "default instrument failed to load: " << processor->getLastLoadError() << '\n';
        return 1;
    }

    const auto initialHostParameters = processor->getParameters();
    if (initialHostParameters.size() != expectedFixedHostParameterSlots)
    {
        std::cerr << "plugin exposed " << initialHostParameters.size()
                  << " host parameters; expected fixed slot count "
                  << expectedFixedHostParameterSlots << '\n';
        return 1;
    }

    auto* tuneParameter = processor->getParameterForPublicId ("kick.tune_hz");
    if (tuneParameter == nullptr)
    {
        std::cerr << "default instrument did not expose kick.tune_hz through the public-id surface\n";
        return 1;
    }
    if (! hostParameterListContains (initialHostParameters, tuneParameter))
    {
        std::cerr << "public parameter kick.tune_hz was not backed by a JUCE host parameter slot\n";
        return 1;
    }

    const auto initialGeneration = processor->getParameterSurfaceGeneration();

    auto editor = std::unique_ptr<juce::AudioProcessorEditor> (processor->createEditor());
    if (editor == nullptr)
    {
        std::cerr << "createEditor returned null even though the plugin reports hasEditor()\n";
        return 1;
    }
    if (editor->getAudioProcessor() != processor.get())
    {
        std::cerr << "created editor is not attached to the processor under test\n";
        return 1;
    }

    const auto replacement = writePatchWithNoPublicParameters();
    if (! processor->reloadInstrumentFromFile (replacement))
    {
        std::cerr << "reloadInstrumentFromFile failed for no-public-parameter test patch: "
                  << processor->getLastLoadError() << '\n';
        return 1;
    }

    const auto reloadedHostParameters = processor->getParameters();
    if (reloadedHostParameters.size() != expectedFixedHostParameterSlots)
    {
        std::cerr << "host parameter count changed after instrument reload: "
                  << reloadedHostParameters.size() << '\n';
        return 1;
    }
    if (processor->getParameterForPublicId ("kick.tune_hz") != nullptr)
    {
        std::cerr << "removed public parameter kick.tune_hz was still visible after reload\n";
        return 1;
    }
    if (! processor->getActivePublicParameterIds().isEmpty())
    {
        std::cerr << "no-public-parameter instrument still reported active public parameter IDs\n";
        return 1;
    }
    if (processor->getParameterSurfaceGeneration() == initialGeneration)
    {
        std::cerr << "parameter surface generation did not change after instrument reload\n";
        return 1;
    }
    if (processor->getLastReloadWarning().isEmpty())
    {
        std::cerr << "instrument reload removed public parameters without surfacing a warning\n";
        return 1;
    }

    editor.reset();
    processor->releaseResources();
    replacement.deleteFile();
    return 0;
}
