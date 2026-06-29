#pragma once

#include <cstddef>

extern "C"
{
struct DandrumEngine;
struct DandrumRealtimeEventQueue;

DandrumEngine* dandrum_engine_create();
void dandrum_engine_destroy (DandrumEngine* engine);
bool dandrum_engine_load_patch (DandrumEngine* engine, const char* path);
void dandrum_engine_prepare (DandrumEngine* engine, float sampleRate);
void dandrum_engine_prepare_realtime (DandrumEngine* engine, float sampleRate, std::size_t maxBlockSize);
void dandrum_engine_note_on (DandrumEngine* engine, unsigned char note, unsigned char velocity);
void dandrum_engine_note_off (DandrumEngine* engine, unsigned char note);
std::size_t dandrum_engine_render (DandrumEngine* engine, float* left, float* right, std::size_t numSamples);
bool dandrum_engine_is_finished (const DandrumEngine* engine);
DandrumRealtimeEventQueue* dandrum_realtime_event_queue_create (std::size_t capacity);
void dandrum_realtime_event_queue_destroy (DandrumRealtimeEventQueue* queue);
unsigned char dandrum_realtime_event_queue_note_on (DandrumRealtimeEventQueue* queue, unsigned char note, unsigned char velocity);
unsigned char dandrum_realtime_event_queue_note_off (DandrumRealtimeEventQueue* queue, unsigned char note);
std::size_t dandrum_realtime_event_queue_dropped_count (const DandrumRealtimeEventQueue* queue);
}
