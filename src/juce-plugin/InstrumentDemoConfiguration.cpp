#include "InstrumentDemoConfiguration.h"

#if defined(DANDRUM_EMBED_TB303_ASSETS)
#include "Tb303InstrumentBinaryData.h"
#endif

#if defined(DANDRUM_EMBED_SAMPLER_ASSETS)
#include "SamplerBinaryData.h"
#endif

#if defined(DANDRUM_EMBED_TB303_ASSETS) || defined(DANDRUM_EMBED_SAMPLER_ASSETS)
#include <cstring>
#include <juce_core/juce_core.h>
#endif

namespace
{
std::filesystem::path demoAsset (const char* relativePath)
{
    return std::filesystem::path (DANDRUM_SOURCE_ROOT) / relativePath;
}

#if defined(DANDRUM_EMBED_TB303_ASSETS) || defined(DANDRUM_EMBED_SAMPLER_ASSETS)
bool stageBundledResource (const juce::File& file, const char* data, int bytes)
{
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
#if defined(DANDRUM_EMBED_TB303_ASSETS)
    const auto root = juce::File::getSpecialLocation (juce::File::userApplicationDataDirectory)
                          .getChildFile ("Dandrum/TB-303 Example");
    const auto patchFile = root.getChildFile ("patches/tb303-acid.yaml");
    const auto fixtureFile = root.getChildFile ("sound-design/tb303-acid-poc.yaml");
    int patchBytes = 0;
    int fixtureBytes = 0;
    const auto* patchData = Tb303InstrumentBinaryData::getNamedResource ("tb303acid_yaml", patchBytes);
    const auto* fixtureData = Tb303InstrumentBinaryData::getNamedResource ("tb303acidpoc_yaml", fixtureBytes);
    if (patchFile.getParentDirectory().createDirectory()
        && fixtureFile.getParentDirectory().createDirectory())
    {
        stageBundledResource (patchFile, patchData, patchBytes);
        stageBundledResource (fixtureFile, fixtureData, fixtureBytes);
    }
    const std::filesystem::path patch = patchFile.getFullPathName().toStdString();
    const std::filesystem::path fixture = fixtureFile.getFullPathName().toStdString();
#else
    const auto patch = demoAsset ("examples/patches/tb303-acid.yaml");
    const auto fixture = demoAsset ("examples/sound-design/tb303-acid-poc.yaml");
#endif
    return { patch,
             fixture,
             patch,
             "Dandrum TB-303",
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
             "dandrum.synthetic-808-kick" };
}

InstrumentDemoConfiguration InstrumentDemoConfiguration::sampler()
{
#if defined(DANDRUM_EMBED_SAMPLER_ASSETS)
    const auto root = juce::File::getSpecialLocation (juce::File::userApplicationDataDirectory)
                          .getChildFile ("Dandrum/Sampler Example");
    const auto assets = root.getChildFile ("assets");
    const auto patchFile = root.getChildFile ("advanced-drum-kit.yaml");
    int sampleBytes = 0;
    int patchBytes = 0;
    const auto* sampleData = SamplerBinaryData::getNamedResource ("advanceddrums_wav", sampleBytes);
    const auto* patchData = SamplerBinaryData::getNamedResource ("advanceddrumkit_yaml", patchBytes);
    if (assets.createDirectory()
        && stageBundledResource (assets.getChildFile ("advanced-drums.wav"), sampleData, sampleBytes))
        stageBundledResource (patchFile, patchData, patchBytes);
    const std::filesystem::path patch = patchFile.getFullPathName().toStdString();
#else
    const auto patch = demoAsset ("examples/patches/advanced-drum-kit.yaml");
#endif
    return { patch,
             std::nullopt,
             std::nullopt,
             "Dandrum Drum Sampler",
             "dandrum.advanced-drum-kit" };
}
