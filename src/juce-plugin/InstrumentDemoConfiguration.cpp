#include "InstrumentDemoConfiguration.h"

#include "KickWebUi.h"
#include "Tb303WebUi.h"

namespace
{
std::filesystem::path demoAsset (const char* relativePath)
{
    return std::filesystem::path (DANDRUM_SOURCE_ROOT) / relativePath;
}
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
