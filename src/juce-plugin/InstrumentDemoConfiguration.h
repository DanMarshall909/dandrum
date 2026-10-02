#pragma once

#include <filesystem>
#include <optional>
#include <string>

// Values are copied into each processor and then held immutable for its lifetime.
// A saved plugin state may still replace the running instrument independently.
struct InstrumentDemoConfiguration
{
    std::filesystem::path instrumentPath;
    std::optional<std::filesystem::path> soundLabFixturePath;
    std::optional<std::filesystem::path> matchSourcePath;
    std::string title;
    std::string instrumentId;

    static InstrumentDemoConfiguration tb303();
    static InstrumentDemoConfiguration kick();
    static InstrumentDemoConfiguration sampler();
};
