#pragma once

#include <cstddef>
#include <cstdint>

extern "C"
{
struct DandrumKernelInstrument;
struct DandrumRealtimeEventQueue;
struct DandrumSoundFixtureRender;
struct DandrumSoundMatch;
struct DandrumGraphProposal;

using DandrumSoundMatchProgressCallback = bool (*) (void* context,
                                                    std::size_t completedEvaluations,
                                                    std::size_t maxEvaluations,
                                                    double bestTotal);
using DandrumCancellationCallback = bool (*) (void* context);

// Direction: 1=input, 2=output. Signal type: 1=audio, 2=control, 3=event.
// Bus declarations and planar views are for audio/control ports. Event root
// ports are enumerable but have no float bus binding in this ABI.
// Names, pointer arrays, and channel buffers must stay valid through each call.
// Calls that render or destroy the same instrument must not overlap.
struct DandrumKernelBusDeclaration
{
    const char* name;
    std::uint32_t direction;
    std::size_t channelCount;
};
struct DandrumKernelInputBusView
{
    const char* name;
    const float* const* channels;
    std::size_t channelCount;
    std::size_t frameCapacity;
};
struct DandrumKernelOutputBusView
{
    const char* name;
    float* const* channels;
    std::size_t channelCount;
    std::size_t frameCapacity;
};

DandrumKernelInstrument* dandrum_kernel_prepare_file (const char* path,
                                                       std::uint32_t sampleRateHz,
                                                       std::size_t maxBlockSize,
                                                       const DandrumKernelBusDeclaration* buses,
                                                       std::size_t busCount);
void dandrum_kernel_destroy (DandrumKernelInstrument* instrument);
std::size_t dandrum_kernel_root_port_count (const DandrumKernelInstrument* instrument);
bool dandrum_kernel_root_port (const DandrumKernelInstrument* instrument,
                               std::size_t index,
                               char* name,
                               std::size_t nameCapacity,
                               std::uint32_t* direction,
                               std::uint32_t* signalType,
                               std::size_t* channels);
std::uint32_t dandrum_kernel_total_latency_samples (const DandrumKernelInstrument* instrument);
bool dandrum_kernel_note_on_at (DandrumKernelInstrument* instrument,
                                unsigned char note,
                                unsigned char velocity,
                                std::size_t frameOffset);
bool dandrum_kernel_note_off_at (DandrumKernelInstrument* instrument,
                                 unsigned char note,
                                 std::size_t frameOffset);
bool dandrum_kernel_reset (DandrumKernelInstrument* instrument);
bool dandrum_kernel_set_public_numeric_parameter_by_slot (DandrumKernelInstrument* instrument,
                                                           std::size_t slotIndex,
                                                           double value);
std::size_t dandrum_kernel_render (DandrumKernelInstrument* instrument,
                                   const DandrumKernelInputBusView* inputs,
                                   std::size_t inputCount,
                                   const DandrumKernelOutputBusView* outputs,
                                   std::size_t outputCount,
                                   std::size_t frames);

std::size_t dandrum_patch_public_numeric_parameter_count (const char* path);
bool dandrum_patch_public_numeric_parameter_port_name (const char* path,
                                                        std::size_t index,
                                                        char* buffer,
                                                        std::size_t capacity);
bool dandrum_patch_public_numeric_parameter_descriptor (const char* path,
                                                       std::size_t index,
                                                       char* idBuffer,
                                                       std::size_t idBufferCapacity,
                                                       char* nameBuffer,
                                                       std::size_t nameBufferCapacity,
                                                       double* defaultValue,
                                                       double* minValue,
                                                       double* maxValue);
DandrumRealtimeEventQueue* dandrum_realtime_event_queue_create (std::size_t capacity);
void dandrum_realtime_event_queue_destroy (DandrumRealtimeEventQueue* queue);
unsigned char dandrum_realtime_event_queue_note_on (DandrumRealtimeEventQueue* queue, unsigned char note, unsigned char velocity);
unsigned char dandrum_realtime_event_queue_note_off (DandrumRealtimeEventQueue* queue, unsigned char note);
std::size_t dandrum_realtime_event_queue_dropped_count (const DandrumRealtimeEventQueue* queue);
DandrumSoundFixtureRender* dandrum_sound_fixture_render_create (const char* fixturePath);
void dandrum_sound_fixture_render_destroy (DandrumSoundFixtureRender* render);
bool dandrum_sound_fixture_render_is_ok (const DandrumSoundFixtureRender* render);
bool dandrum_sound_fixture_render_error_message (const DandrumSoundFixtureRender* render,
                                                 char* buffer,
                                                 std::size_t bufferCapacity);
std::uint32_t dandrum_sound_fixture_render_sample_rate_hz (const DandrumSoundFixtureRender* render);
std::uint64_t dandrum_sound_fixture_render_duration_frames (const DandrumSoundFixtureRender* render);
std::size_t dandrum_sound_fixture_render_metric_count (const DandrumSoundFixtureRender* render);
bool dandrum_sound_fixture_render_metric (const DandrumSoundFixtureRender* render,
                                          std::size_t index,
                                          double* timeSeconds,
                                          double* rms,
                                          double* peak,
                                          double* spectralCentroidHz,
                                          bool* hasSpectralCentroid);
std::size_t dandrum_sound_fixture_render_wav_size (const DandrumSoundFixtureRender* render);
bool dandrum_sound_fixture_render_copy_wav (const DandrumSoundFixtureRender* render,
                                            std::uint8_t* buffer,
                                            std::size_t bufferCapacity);
DandrumSoundMatch* dandrum_sound_match_create (const char* fixturePath,
                                               const char* referencePath,
                                               DandrumSoundMatchProgressCallback progressCallback,
                                               void* context);
void dandrum_sound_match_destroy (DandrumSoundMatch* matched);
bool dandrum_sound_match_is_ok (const DandrumSoundMatch* matched);
bool dandrum_sound_match_error_message (const DandrumSoundMatch* matched,
                                        char* buffer,
                                        std::size_t bufferCapacity);
bool dandrum_sound_match_was_cancelled (const DandrumSoundMatch* matched);
std::uint32_t dandrum_sound_match_sample_rate_hz (const DandrumSoundMatch* matched);
std::uint64_t dandrum_sound_match_duration_frames (const DandrumSoundMatch* matched);
std::size_t dandrum_sound_match_manifest_json_size (const DandrumSoundMatch* matched);
bool dandrum_sound_match_copy_manifest_json (const DandrumSoundMatch* matched,
                                             char* buffer,
                                             std::size_t bufferCapacity);
std::size_t dandrum_sound_match_patch_yaml_size (const DandrumSoundMatch* matched);
bool dandrum_sound_match_copy_patch_yaml (const DandrumSoundMatch* matched,
                                          char* buffer,
                                          std::size_t bufferCapacity);
std::size_t dandrum_sound_match_parameter_count (const DandrumSoundMatch* matched);
bool dandrum_sound_match_parameter (const DandrumSoundMatch* matched,
                                    std::size_t index,
                                    char* idBuffer,
                                    std::size_t idBufferCapacity,
                                    double* min,
                                    double* max,
                                    double* initial,
                                    double* best,
                                    double* normalized);
std::size_t dandrum_sound_match_metric_count (const DandrumSoundMatch* matched);
bool dandrum_sound_match_metric (const DandrumSoundMatch* matched,
                                 std::size_t index,
                                 bool reference,
                                 double* timeSeconds,
                                 double* rms,
                                 double* peak,
                                 double* spectralCentroidHz,
                                 bool* hasSpectralCentroid);
std::size_t dandrum_sound_match_wav_size (const DandrumSoundMatch* matched, bool reference);
bool dandrum_sound_match_copy_wav (const DandrumSoundMatch* matched,
                                   bool reference,
                                   std::uint8_t* buffer,
                                   std::size_t bufferCapacity);
DandrumGraphProposal* dandrum_graph_proposal_create (
    const DandrumSoundMatch* matched,
    DandrumCancellationCallback cancellationCallback,
    void* context);
void dandrum_graph_proposal_destroy (DandrumGraphProposal* proposal);
bool dandrum_graph_proposal_is_ok (const DandrumGraphProposal* proposal);
bool dandrum_graph_proposal_error_message (const DandrumGraphProposal* proposal,
                                           char* buffer,
                                           std::size_t bufferCapacity);
bool dandrum_graph_proposal_patch_name (const DandrumGraphProposal* proposal,
                                        char* buffer,
                                        std::size_t bufferCapacity);
bool dandrum_graph_proposal_provider_id (const DandrumGraphProposal* proposal,
                                         char* buffer,
                                         std::size_t bufferCapacity);
std::size_t dandrum_graph_proposal_explanation_size (const DandrumGraphProposal* proposal);
bool dandrum_graph_proposal_copy_explanation (const DandrumGraphProposal* proposal,
                                              char* buffer,
                                              std::size_t bufferCapacity);
std::size_t dandrum_graph_proposal_patch_yaml_size (const DandrumGraphProposal* proposal);
bool dandrum_graph_proposal_copy_patch_yaml (const DandrumGraphProposal* proposal,
                                             char* buffer,
                                             std::size_t bufferCapacity);
std::size_t dandrum_graph_proposal_parameter_count (const DandrumGraphProposal* proposal);
bool dandrum_graph_proposal_parameter (const DandrumGraphProposal* proposal,
                                       std::size_t index,
                                       char* buffer,
                                       std::size_t bufferCapacity);
}
