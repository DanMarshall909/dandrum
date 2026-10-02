#pragma once

#include <cstddef>
#include <cstdint>

extern "C"
{
struct DandrumKernelInstrument;
struct DandrumKernelUiSnapshot;
struct DandrumKernelWaveformSource;
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
// Declaration names are resolved during preparation. Planar views use the
// zero-based index among declarations of the same direction; view names are
// informational and are not read during rendering. Pointer arrays and channel
// buffers must stay valid through each render call.
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
    std::size_t busIndex { 0 };
};
struct DandrumKernelOutputBusView
{
    const char* name;
    float* const* channels;
    std::size_t channelCount;
    std::size_t frameCapacity;
    std::size_t busIndex { 0 };
};

// Views are UTF-8 and point into an independently owned UI snapshot or
// waveform source handle. They remain valid until that owner is destroyed.
struct DandrumKernelStringView
{
    const char* data;
    std::size_t size;
};
struct DandrumKernelUiSource
{
    DandrumKernelStringView id;
    std::uint32_t sampleRateHz;
    std::uint16_t channelCount;
    std::uint64_t frameCount;
};
struct DandrumKernelWaveformSourceInfo
{
    DandrumKernelStringView sourceId;
    std::uint32_t sampleRateHz;
    std::uint16_t channelCount;
    std::uint64_t frameCount;
    std::uint8_t contentRevision[32];
};
struct DandrumKernelWaveformBucket
{
    std::uint64_t startFrame;
    std::uint64_t endFrame;
    float minimum;
    float maximum;
};
struct DandrumKernelUiRegion
{
    DandrumKernelStringView id;
    std::uint64_t startFrame;
    std::uint64_t endFrame;
    std::int32_t rootNote;
    bool hasGainDb;
    double gainDb;
    bool hasPan;
    double pan;
    bool reverse;
    double fadeInMs;
    double fadeOutMs;
    bool hasLoop;
    DandrumKernelStringView loopMode;
    std::uint64_t loopStartFrame;
    std::uint64_t loopEndFrame;
    double loopCrossfadeMs;
};
struct DandrumKernelUiSlice
{
    DandrumKernelStringView id;
    std::uint64_t startFrame;
    std::uint64_t endFrame;
};
struct DandrumKernelUiMap
{
    DandrumKernelStringView id;
    DandrumKernelStringView selectionMode;
    std::uint64_t selectionSeed;
};
struct DandrumKernelUiZone
{
    DandrumKernelStringView id;
    std::size_t sourceIndex;
    std::size_t regionIndex;
    std::uint64_t startFrame;
    std::uint64_t endFrame;
    std::uint8_t keyLow;
    std::uint8_t keyHigh;
    std::uint8_t velocityLow;
    std::uint8_t velocityHigh;
    DandrumKernelStringView roundRobinGroup;
    DandrumKernelStringView chokeGroup;
    std::int32_t controlGroup;
    std::uint32_t weight;
    bool hasGainDb;
    double gainDb;
    bool hasPan;
    double pan;
    bool hasPitchSemitones;
    double pitchSemitones;
};

DandrumKernelInstrument* dandrum_kernel_prepare_file (const char* path,
                                                       std::uint32_t sampleRateHz,
                                                       std::size_t maxBlockSize,
                                                       const DandrumKernelBusDeclaration* buses,
                                                       std::size_t busCount);
void dandrum_kernel_destroy (DandrumKernelInstrument* instrument);
// Create only while the engine pointer is protected against replacement. The
// snapshot copies metadata, never decoded audio, and is used off the callback.
DandrumKernelUiSnapshot* dandrum_kernel_ui_snapshot_create (const DandrumKernelInstrument* instrument);
void dandrum_kernel_ui_snapshot_destroy (DandrumKernelUiSnapshot* snapshot);
// Create while the engine is protected against replacement. The retained
// source is independent of the engine and must be reduced/destroyed off audio.
DandrumKernelWaveformSource* dandrum_kernel_waveform_source_create (
    const DandrumKernelInstrument* instrument, std::size_t sourceIndex);
void dandrum_kernel_waveform_source_destroy (DandrumKernelWaveformSource* source);
bool dandrum_kernel_waveform_source_info (const DandrumKernelWaveformSource* source,
                                         DandrumKernelWaveformSourceInfo* output);
// Off-audio contiguous source-channel copy for analysis. output holds frameCount
// floats; invalid requests leave it untouched. Ownership stays with the retained
// source (including when the instrument or its original file no longer exists).
bool dandrum_kernel_prepared_source_copy_channel (const DandrumKernelWaveformSource* source,
                                                 std::uint16_t channel, std::uint64_t startFrame,
                                                 float* output, std::size_t frameCount);
// output must hold bucketCount elements; invalid requests leave it untouched.
bool dandrum_kernel_waveform_reduce (const DandrumKernelWaveformSource* source,
                                    std::uint16_t channel, std::uint64_t startFrame,
                                    std::uint64_t endFrame, DandrumKernelWaveformBucket* output,
                                    std::size_t bucketCount);
std::size_t dandrum_kernel_ui_source_count (const DandrumKernelUiSnapshot* snapshot);
bool dandrum_kernel_ui_source (const DandrumKernelUiSnapshot* snapshot, std::size_t index,
                               DandrumKernelUiSource* output);
std::size_t dandrum_kernel_ui_region_count (const DandrumKernelUiSnapshot* snapshot, std::size_t sourceIndex);
bool dandrum_kernel_ui_region (const DandrumKernelUiSnapshot* snapshot, std::size_t sourceIndex,
                               std::size_t regionIndex, DandrumKernelUiRegion* output);
std::size_t dandrum_kernel_ui_slice_count (const DandrumKernelUiSnapshot* snapshot, std::size_t sourceIndex);
bool dandrum_kernel_ui_slice (const DandrumKernelUiSnapshot* snapshot, std::size_t sourceIndex,
                              std::size_t sliceIndex, DandrumKernelUiSlice* output);
std::size_t dandrum_kernel_ui_map_count (const DandrumKernelUiSnapshot* snapshot);
bool dandrum_kernel_ui_map (const DandrumKernelUiSnapshot* snapshot, std::size_t mapIndex,
                            DandrumKernelUiMap* output);
std::size_t dandrum_kernel_ui_zone_count (const DandrumKernelUiSnapshot* snapshot, std::size_t mapIndex);
bool dandrum_kernel_ui_zone (const DandrumKernelUiSnapshot* snapshot, std::size_t mapIndex,
                             std::size_t zoneIndex, DandrumKernelUiZone* output);
// Slot order matches prepared public controls; 0 is instrument scope, positive
// values identify sample-map control groups. Returns false for an invalid slot.
bool dandrum_kernel_ui_public_control_group (const DandrumKernelUiSnapshot* snapshot,
                                              std::size_t index, std::int32_t* output);
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
