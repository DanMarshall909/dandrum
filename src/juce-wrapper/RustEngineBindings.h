#pragma once

#include <cstddef>
#include <cstdint>

extern "C"
{
struct DandrumEngine;
struct DandrumRealtimeEventQueue;

DandrumEngine* dandrum_engine_create();
void dandrum_engine_destroy (DandrumEngine* engine);
bool dandrum_engine_load_patch (DandrumEngine* engine, const char* path);
std::size_t dandrum_patch_public_numeric_parameter_count (const char* path);
bool dandrum_patch_public_numeric_parameter_descriptor (const char* path,
                                                       std::size_t index,
                                                       char* idBuffer,
                                                       std::size_t idBufferCapacity,
                                                       char* nameBuffer,
                                                       std::size_t nameBufferCapacity,
                                                       double* defaultValue,
                                                       double* minValue,
                                                       double* maxValue);
bool dandrum_engine_set_public_numeric_parameter (DandrumEngine* engine, const char* parameterId, double value);
std::size_t dandrum_engine_public_numeric_parameter_target_count (const DandrumEngine* engine, const char* parameterId);
std::intptr_t dandrum_engine_prepare_public_numeric_parameter_slot_at (DandrumEngine* engine, const char* parameterId, std::size_t targetIndex);
bool dandrum_engine_set_public_numeric_parameter_by_slot (DandrumEngine* engine, std::size_t slotIndex, float value);
void dandrum_engine_prepare (DandrumEngine* engine, float sampleRate);
void dandrum_engine_prepare_realtime (DandrumEngine* engine, float sampleRate, std::size_t maxBlockSize);
void dandrum_engine_note_on (DandrumEngine* engine, unsigned char note, unsigned char velocity);
void dandrum_engine_note_off (DandrumEngine* engine, unsigned char note);
void dandrum_engine_note_on_at (DandrumEngine* engine, unsigned char note, unsigned char velocity, std::size_t frameOffset);
void dandrum_engine_note_off_at (DandrumEngine* engine, unsigned char note, std::size_t frameOffset);
std::size_t dandrum_engine_render (DandrumEngine* engine, float* left, float* right, std::size_t numSamples);
bool dandrum_engine_is_finished (const DandrumEngine* engine);
DandrumRealtimeEventQueue* dandrum_realtime_event_queue_create (std::size_t capacity);
void dandrum_realtime_event_queue_destroy (DandrumRealtimeEventQueue* queue);
unsigned char dandrum_realtime_event_queue_note_on (DandrumRealtimeEventQueue* queue, unsigned char note, unsigned char velocity);
unsigned char dandrum_realtime_event_queue_note_off (DandrumRealtimeEventQueue* queue, unsigned char note);
std::size_t dandrum_realtime_event_queue_dropped_count (const DandrumRealtimeEventQueue* queue);
}
