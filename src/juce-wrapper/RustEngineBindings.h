#pragma once

#include <cstddef>

extern "C"
{
struct DandrumEngine;

DandrumEngine* dandrum_engine_create();
void dandrum_engine_destroy (DandrumEngine* engine);
void dandrum_engine_prepare (DandrumEngine* engine, float sampleRate);
void dandrum_engine_note_on (DandrumEngine* engine, unsigned char note, unsigned char velocity);
void dandrum_engine_note_off (DandrumEngine* engine, unsigned char note);
std::size_t dandrum_engine_render (DandrumEngine* engine, float* left, float* right, std::size_t numSamples);
bool dandrum_engine_is_finished (const DandrumEngine* engine);
}
