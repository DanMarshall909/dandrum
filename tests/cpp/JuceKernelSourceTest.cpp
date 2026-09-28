#include "../../src/juce-wrapper/RustEngineSource.h"

#include <cmath>
#include <filesystem>
#include <fstream>

int main()
{
    const auto path = std::filesystem::temp_directory_path() / "dandrum-juce-kernel-source-test.yaml";
    {
        std::ofstream file (path);
        file << "metadata: { name: juce-kernel-source }\n"
                "ports:\n"
                "  - { name: master, direction: output, signal: audio, channels: 2, maps_from: source.out }\n"
                "modules:\n"
                "  - { id: source, type: control_to_audio, static: { channels: 2 }, defaults: { in: 0.25 } }\n"
                "connections: []\n";
    }

    RustEngineSource source;
    source.prepareToPlay (8, 48000.0);
    const auto loaded = source.loadPatch (juce::String (path.string()));
    if (! loaded)
        return 1;

    juce::AudioBuffer<float> buffer (2, 8);
    buffer.clear();
    source.getNextAudioBlock (juce::AudioSourceChannelInfo (&buffer, 0, 8));
    for (int frame = 0; frame < 8; ++frame)
    {
        if (std::abs (buffer.getSample (0, frame) - 0.25f) > 0.00001f)
            return 2;
        if (std::abs (buffer.getSample (1, frame)) > 0.00001f)
            return 3;
    }
    source.prepareToPlay (8, 44100.0);
    juce::AudioBuffer<float> laterBuffer (2, 12);
    laterBuffer.clear();
    source.getNextAudioBlock (juce::AudioSourceChannelInfo (&laterBuffer, 0, 12));
    for (int frame = 0; frame < 12; ++frame)
    {
        if (std::abs (laterBuffer.getSample (0, frame) - 0.25f) > 0.00001f)
            return 5;
    }
    std::filesystem::remove (path);
    const auto legacyPath = std::filesystem::path (DANDRUM_SOURCE_ROOT)
                          / "src/rust-engine/tests/fixtures/unify-graph-kernel/legacy/polyphonic-pad.yaml";
    if (source.loadPatch (juce::String (legacyPath.string())))
        return 4;
    buffer.clear();
    source.getNextAudioBlock (juce::AudioSourceChannelInfo (&buffer, 0, 8));
    if (std::abs (buffer.getSample (0, 0) - 0.25f) > 0.00001f)
        return 6;
    return 0;
}
