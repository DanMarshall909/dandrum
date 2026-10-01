#pragma once

#include <cstdint>
#include <string>
#include <vector>

// A lightweight timer snapshot of host values. It has no callback, browser, or
// renderer lifetime and can be copied independently of a prepared document.
struct InstrumentUiParameterState
{
    struct Value
    {
        std::string id;
        std::string name;
        float normalisedValue = 0.0f;
    };

    std::uint32_t generation = 0;
    std::uint64_t admittedCommandSequence = 0;
    std::vector<Value> parameters;
};
