#pragma once

#include <cmath>
#include <cstdint>
#include <string>

// The same message-thread command service is used by native components and
// browser transport. No browser or JUCE value type enters this contract.
enum class InstrumentUiCommandStatus { accepted, staleGeneration, invalidValue, unknownControl };

struct InstrumentUiSetParameter
{
    std::uint32_t generation;
    std::string id;
    double normalisedValue;
};

struct InstrumentUiCommandReply
{
    InstrumentUiCommandStatus status;
    std::uint32_t generation;
    std::uint64_t sequence;
};

class InstrumentUiCommandHost
{
public:
    virtual ~InstrumentUiCommandHost() = default;
    virtual std::uint32_t uiCommandGeneration() const noexcept = 0;
    virtual InstrumentUiCommandStatus applyUiParameter (
        std::uint32_t generation, const std::string& id, float normalisedValue) = 0;
};

class InstrumentUiCommandService final
{
public:
    explicit InstrumentUiCommandService (InstrumentUiCommandHost& hostToUse) : host (hostToUse) {}

    InstrumentUiCommandReply setParameter (const InstrumentUiSetParameter& request)
    {
        const auto generation = host.uiCommandGeneration();
        if (request.generation != generation)
            return { InstrumentUiCommandStatus::staleGeneration, generation, sequence };
        if (! std::isfinite (request.normalisedValue)
            || request.normalisedValue < 0.0 || request.normalisedValue > 1.0)
            return { InstrumentUiCommandStatus::invalidValue, generation, sequence };

        const auto status = host.applyUiParameter (
            generation, request.id, static_cast<float> (request.normalisedValue));
        if (status == InstrumentUiCommandStatus::accepted)
            ++sequence;
        return { status, host.uiCommandGeneration(), sequence };
    }

private:
    InstrumentUiCommandHost& host;
    std::uint64_t sequence = 0;
};
