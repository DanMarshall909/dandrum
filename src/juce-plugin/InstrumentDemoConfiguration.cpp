#include "InstrumentDemoConfiguration.h"

#include "KickWebUi.h"
#include "SamplerWebUi.h"
#include "Tb303WebUi.h"

#if defined(DANDRUM_EMBED_SAMPLER_ASSETS)
#include <cstring>
#include <juce_core/juce_core.h>
#include "SamplerBinaryData.h"
#endif

namespace
{
std::filesystem::path demoAsset (const char* relativePath)
{
    return std::filesystem::path (DANDRUM_SOURCE_ROOT) / relativePath;
}

#if defined(DANDRUM_EMBED_SAMPLER_ASSETS)
bool stageSamplerResource (const juce::File& file, const char* resourceName)
{
    int bytes = 0;
    const auto* data = SamplerBinaryData::getNamedResource (resourceName, bytes);
    if (data == nullptr || bytes <= 0)
        return false;

    juce::MemoryBlock current;
    if (file.loadFileAsData (current)
        && current.getSize() == static_cast<std::size_t> (bytes)
        && std::memcmp (current.getData(), data, current.getSize()) == 0)
        return true;

    return file.replaceWithData (data, static_cast<std::size_t> (bytes));
}
#endif
}

InstrumentDemoConfiguration InstrumentDemoConfiguration::tb303()
{
    const auto patch = demoAsset ("examples/patches/tb303-acid.yaml");
    const auto fixture = demoAsset ("examples/sound-design/tb303-acid-poc.yaml");
    return { patch,
             fixture,
             patch,
             "Dandrum TB-303",
             Tb303WebUi::indexHtml,
             "dandrum.tb303-acid" };
}

InstrumentDemoConfiguration InstrumentDemoConfiguration::kick()
{
    const auto patch = demoAsset ("examples/patches/synthetic-808-kick.yaml");
    const auto fixture = demoAsset ("examples/sound-design/synthetic-808-kick-poc.yaml");
    return { patch,
             fixture,
             patch,
             "Dandrum 808 Kick",
             KickWebUi::indexHtml,
             "dandrum.synthetic-808-kick" };
}

InstrumentDemoConfiguration InstrumentDemoConfiguration::sampler()
{
#if defined(DANDRUM_EMBED_SAMPLER_ASSETS)
    const auto root = juce::File::getSpecialLocation (juce::File::userApplicationDataDirectory)
                          .getChildFile ("Dandrum/Sampler Example");
    const auto assets = root.getChildFile ("assets");
    const auto patchFile = root.getChildFile ("advanced-drum-kit.yaml");
    if (assets.createDirectory()
        && stageSamplerResource (assets.getChildFile ("advanced-drums.wav"), "advanceddrums_wav"))
        stageSamplerResource (patchFile, "advanceddrumkit_yaml");
    const std::filesystem::path patch = patchFile.getFullPathName().toStdString();
#else
    const auto patch = demoAsset ("examples/patches/advanced-drum-kit.yaml");
#endif
    return { patch,
             std::nullopt,
             std::nullopt,
             "Dandrum Drum Sampler",
             SamplerWebUi::indexHtml,
             "dandrum.advanced-drum-kit" };
}
