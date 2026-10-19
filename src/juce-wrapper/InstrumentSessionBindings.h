#pragma once

#include "RustEngineBindings.h"

#include <cstddef>
#include <cstdint>

struct DandrumInstrumentSession;
struct DandrumInstrumentSessionJob;

struct DandrumInstrumentSessionSnapshot
{
    std::uint32_t contractVersion;
    std::uint64_t instanceId;
    std::uint64_t attachmentId;
    std::uint64_t revision;
    std::uint64_t nextRequestId;
    std::size_t undoCount;
    std::size_t redoCount;
    std::size_t attachmentLimit;
    std::size_t historyLimit;
    std::size_t outcomeLimit;
    std::size_t activeRequestLimit;
    std::size_t yamlSize;
};

struct DandrumInstrumentSessionHostCallbacks
{
    void* (*activate) (void* context, DandrumKernelInstrument* candidate,
                      const std::uint8_t* yaml, std::size_t yamlSize,
                      std::uint64_t revision, std::size_t undoCount,
                      std::size_t redoCount);
    void (*publish) (void* context, void* lease, std::uint64_t revision,
                     std::size_t undoCount, std::size_t redoCount);
    void (*abort) (void* context, void* lease);
};

extern "C"
{
DandrumInstrumentSession* dandrum_instrument_session_create (
    const std::uint8_t* yaml, std::size_t yamlSize, const char* documentRoot,
    std::uint32_t sampleRateHz, std::size_t maxBlockSize,
    const DandrumKernelBusDeclaration* buses, std::size_t busCount);
void dandrum_instrument_session_destroy (DandrumInstrumentSession* session);
bool dandrum_instrument_session_snapshot (
    const DandrumInstrumentSession* session, DandrumInstrumentSessionSnapshot* output);
bool dandrum_instrument_session_connect (
    DandrumInstrumentSession* session, DandrumInstrumentSessionSnapshot* output);
bool dandrum_instrument_session_disconnect (
    DandrumInstrumentSession* session, std::uint64_t instanceId,
    std::uint64_t attachmentId);
bool dandrum_instrument_session_connection_snapshot (
    const DandrumInstrumentSession* session, std::uint64_t instanceId,
    std::uint64_t attachmentId, DandrumInstrumentSessionSnapshot* output);
bool dandrum_instrument_session_copy_yaml (
    const DandrumInstrumentSession* session, std::uint8_t* output, std::size_t capacity);
DandrumInstrumentSessionJob* dandrum_instrument_session_admit_mapping (
    DandrumInstrumentSession* session, std::uint64_t requestId,
    std::uint64_t expectedRevision, const std::uint8_t* mapId,
    std::size_t mapIdSize, const std::uint8_t* zoneId, std::size_t zoneIdSize,
    std::uint8_t keyLow, std::uint8_t keyHigh, char* error,
    std::size_t errorCapacity);
DandrumInstrumentSessionJob* dandrum_instrument_session_admit_mapping_for_connection (
    DandrumInstrumentSession* session, std::uint64_t instanceId,
    std::uint64_t attachmentId, std::uint64_t requestId,
    std::uint64_t expectedRevision, const std::uint8_t* mapId,
    std::size_t mapIdSize, const std::uint8_t* zoneId, std::size_t zoneIdSize,
    std::uint8_t keyLow, std::uint8_t keyHigh, char* error,
    std::size_t errorCapacity);
DandrumInstrumentSessionJob* dandrum_instrument_session_admit_undo_for_connection (
    DandrumInstrumentSession* session, std::uint64_t instanceId,
    std::uint64_t attachmentId, std::uint64_t requestId,
    std::uint64_t expectedRevision, char* error, std::size_t errorCapacity);
bool dandrum_instrument_session_prepare_job (DandrumInstrumentSessionJob* job);
bool dandrum_instrument_session_fail_job (
    DandrumInstrumentSession* session, DandrumInstrumentSessionJob* job,
    const std::uint8_t* message, std::size_t messageSize);
bool dandrum_instrument_session_complete_job (
    DandrumInstrumentSession* session, DandrumInstrumentSessionJob* job,
    const DandrumInstrumentSessionHostCallbacks* callbacks, void* context,
    std::uint64_t* revision, std::size_t* undoCount, std::size_t* redoCount,
    char* error, std::size_t errorCapacity);
bool dandrum_instrument_session_reconfigure (
    DandrumInstrumentSession* session, std::uint32_t sampleRateHz,
    std::size_t maxBlockSize,
    const DandrumInstrumentSessionHostCallbacks* callbacks, void* context,
    char* error, std::size_t errorCapacity);
void dandrum_instrument_session_job_destroy (DandrumInstrumentSessionJob* job);
}
