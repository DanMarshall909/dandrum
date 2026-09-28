#include "InstrumentDemoConfiguration.h"

#include "DefaultPatch.h"
#include "KickWebUi.h"
#include "Tb303WebUi.h"

InstrumentDemoConfiguration InstrumentDemoConfiguration::tb303()
{
    const auto patch = dandrum::findRepositoryExample ("examples/patches/tb303-acid.yaml");
    return { patch,
             dandrum::findRepositoryExample ("examples/sound-design/tb303-acid-poc.yaml"),
             patch,
             "Dandrum TB-303",
             Tb303WebUi::indexHtml };
}

InstrumentDemoConfiguration InstrumentDemoConfiguration::kick()
{
    const auto patch = dandrum::findRepositoryExample ("examples/patches/synthetic-808-kick.yaml");
    return { patch,
             dandrum::findRepositoryExample ("examples/sound-design/synthetic-808-kick-poc.yaml"),
             patch,
             "Dandrum 808 Kick",
             KickWebUi::indexHtml };
}
