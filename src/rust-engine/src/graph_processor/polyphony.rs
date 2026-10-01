use crate::builtins::module_kind::ModuleKind;
use crate::compiled_patch::{
    CompiledConstruction, CompiledPatch, CompiledPolyRegion, CompiledPortSpan, CompiledSampleZone,
    SampleChokeMode, SampleSelectionMode,
};
use crate::graph::{PortDirection, SignalType};
use crate::kernel::{
    POLY_DONE_OUTPUT, PolyAllocationPolicy, VOICE_GATE_OUTPUT, VOICE_NOTE_OUTPUT,
    VOICE_VELOCITY_OUTPUT,
};
use crate::sample::PreparedSamplerAssets;
use crate::script::ScriptEvent;

use super::audio_arena::AudioArena;
use super::event_queue::PreparedEventQueues;
use super::outputs::BlockEvent;
use super::render_plan::{AudioBufferPlan, BufferId, EventQueueId, RenderPlan, RenderStep};
#[cfg(test)]
use super::state::PerModuleState;

/// Linear peak amplitude below which a released voice counts as silent.
const RELEASE_SILENCE_THRESHOLD: f32 = 1.0e-4;
/// A released voice without `done` must remain silent for this long to retire.
const RELEASE_SILENCE_SECONDS: f32 = 0.010;
/// A released voice without `done` is retired by this deadline even if audible.
const RELEASE_TIMEOUT_SECONDS: f32 = 5.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PolyVoiceSlot {
    active: bool,
    gate_held: bool,
    note: u8,
    velocity: u8,
    allocation_order: u64,
    quiet_frames_after_release: usize,
    released_frames: usize,
    release_offset_pending: usize,
    last_peak: f32,
    choke_group: Option<(usize, usize)>,
    choke_start_frame: Option<u64>,
    choke_fade_frames: u64,
}

struct SharedSampleMapSelection {
    step_index: usize,
    zones: Box<[CompiledSampleZone]>,
    selection_mode: SampleSelectionMode,
    round_robin_counters: Box<[usize]>,
    initial_seed: u64,
    rng_state: u64,
    choke_mode: SampleChokeMode,
    choke_fade_frames: u64,
}

#[derive(Clone, Copy, Debug)]
struct VoiceIntrinsicBindings {
    note: BufferId,
    velocity: BufferId,
    gate: EventQueueId,
}

#[derive(Clone, Copy, Debug)]
struct PolyOutputBinding {
    voice_span: CompiledPortSpan,
    accumulator_start: usize,
    signal_type: SignalType,
}

#[derive(Clone, Copy, Debug)]
struct ForwardedBufferInput {
    parent_start: usize,
    child_span: CompiledPortSpan,
}

#[derive(Debug)]
struct ForwardedEventInput {
    parent_index: usize,
    child_queues: Box<[EventQueueId]>,
}

#[derive(Clone, Copy, Debug)]
enum DoneBinding {
    Event(EventQueueId),
    Control {
        buffer: BufferId,
        producer_step: usize,
    },
}

pub struct PreparedPolyRuntimeRegion {
    node_id: String,
    #[cfg(test)]
    states: Box<[Box<[PerModuleState]>]>,
    child_module_kinds: Box<[ModuleKind]>,
    auto_retire_map_voice: bool,
    voice_arenas: Box<[AudioArena]>,
    voice_event_queues: Box<[PreparedEventQueues]>,
    nested_regions: Box<[Box<[PreparedPolyRuntimeRegion]>]>,
    output_accumulator: AudioArena,
    audio_buffers_per_voice: usize,
    allocation_policy: PolyAllocationPolicy,
    slots: Box<[PolyVoiceSlot]>,
    next_allocation_order: u64,
    block_start_frame: u64,
    intrinsic_bindings: Option<VoiceIntrinsicBindings>,
    done_binding: Option<DoneBinding>,
    silence_hold_frames: usize,
    release_timeout_frames: usize,
    child_render_plan: RenderPlan,
    prepared_step_executors:
        Box<[Box<[Box<dyn super::realtime_graph_processor::PreparedStepExecutor>]>]>,
    shared_sample_map_selections: Box<[SharedSampleMapSelection]>,
    child_patch: Box<CompiledPatch>,
    forwarded_buffer_inputs: Box<[ForwardedBufferInput]>,
    forwarded_event_inputs: Box<[ForwardedEventInput]>,
    output_bindings: Box<[PolyOutputBinding]>,
}

impl PreparedPolyRuntimeRegion {
    pub(super) fn reset(&mut self) {
        self.retire_all_voices();
        #[cfg(test)]
        for voice in self.states.iter_mut() {
            for state in voice.iter_mut() {
                state.reset_all();
            }
        }
        for voice in self.prepared_step_executors.iter_mut() {
            for executor in voice.iter_mut() {
                executor.reset_all();
            }
        }
        for selection in self.shared_sample_map_selections.iter_mut() {
            selection.round_robin_counters.fill(0);
            selection.rng_state = selection.initial_seed;
        }
        for arena in self.voice_arenas.iter_mut() {
            arena.reset();
        }
        for queues in self.voice_event_queues.iter_mut() {
            queues.clear_all();
        }
        for regions in self.nested_regions.iter_mut() {
            for region in regions.iter_mut() {
                region.reset();
            }
        }
        self.output_accumulator.reset();
        self.next_allocation_order = 1;
        self.block_start_frame = 0;
    }

    pub(super) fn new(
        compiled: &CompiledPolyRegion,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
    ) -> Self {
        #[cfg(test)]
        let states = build_polyphonic_states_from_compiled(
            compiled.child_patch(),
            sample_rate,
            sampler_assets,
            compiled.max_voices(),
        )
        .into_iter()
        .map(Vec::into_boxed_slice)
        .collect::<Vec<_>>()
        .into_boxed_slice();
        let child_module_kinds = compiled
            .child_patch()
            .nodes()
            .iter()
            .map(|node| node.module_kind)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let auto_retire_map_voice = child_module_kinds
            .iter()
            .filter(|kind| **kind != ModuleKind::VoiceIntrinsics)
            .copied()
            .eq([ModuleKind::SampleMapPlayer]);
        let audio_buffers_per_voice = compiled
            .voices()
            .first()
            .map_or(0, |voice| voice.audio_buffer_range().len());
        let event_queues_per_voice = compiled
            .voices()
            .first()
            .map_or(0, |voice| voice.event_queue_range().len());
        let max_block_frames = compiled
            .child_patch()
            .render_settings()
            .block_size_frames
            .max(1) as usize;
        let voice_arenas = (0..compiled.max_voices())
            .map(|_| {
                AudioArena::new(AudioBufferPlan {
                    buffer_count: audio_buffers_per_voice,
                    max_block_frames,
                    max_voices: 1,
                })
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let voice_event_queues = (0..compiled.max_voices())
            .map(|_| {
                PreparedEventQueues::new(event_queues_per_voice, compiled.event_queue_capacity())
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let nested_regions = (0..compiled.max_voices())
            .map(|_| {
                compiled
                    .child_patch()
                    .poly_regions()
                    .iter()
                    .map(|region| Self::new(region, sample_rate, sampler_assets))
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let output_buffer_count = compiled
            .output_accumulators()
            .iter()
            .map(|output| output.span().channel_count)
            .sum();
        let output_accumulator = AudioArena::new(AudioBufferPlan {
            buffer_count: output_buffer_count,
            max_block_frames,
            max_voices: 1,
        });
        let child_render_plan = RenderPlan::from_compiled_patch(
            compiled.child_patch(),
            max_block_frames,
            1,
            compiled.event_queue_capacity(),
        );
        let prepared_step_executors = (0..compiled.max_voices())
            .map(|_| {
                super::realtime_graph_processor::bind_prepared_step_executors(
                    compiled.child_patch(),
                    &child_render_plan,
                    sample_rate,
                    sampler_assets,
                )
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let shared_sample_map_selections = child_render_plan
            .global_steps
            .iter()
            .enumerate()
            .filter_map(|(step_index, step)| {
                let CompiledConstruction::SampleMapPlayer {
                    zones,
                    selection_mode,
                    group_count,
                    selection_seed,
                    ..
                } = &compiled.child_patch().nodes()[step.module_index].construction
                else {
                    return None;
                };
                Some(SharedSampleMapSelection {
                    step_index,
                    zones: zones.clone(),
                    selection_mode: *selection_mode,
                    round_robin_counters: vec![0; *group_count].into_boxed_slice(),
                    initial_seed: *selection_seed,
                    rng_state: *selection_seed,
                    choke_mode: compiled
                        .sample_map_choke()
                        .map_or(SampleChokeMode::Cut, |(mode, _)| mode),
                    choke_fade_frames: compiled.sample_map_choke().map_or(0, |(_, fade_ms)| {
                        ((fade_ms as f32 * sample_rate / 1_000.0).round() as u64).max(1)
                    }),
                })
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let intrinsic_bindings =
            voice_intrinsic_bindings(compiled.child_patch(), &child_render_plan);
        let done_binding = voice_done_binding(compiled, &child_render_plan);
        let mut parent_start = 0;
        let forwarded_buffer_inputs = compiled
            .flattened_voice()
            .root_ports()
            .iter()
            .filter(|port| {
                port.direction() == PortDirection::Input && port.signal_type() != SignalType::Event
            })
            .map(|port| {
                let child_span = compiled
                    .child_patch()
                    .root_bus_plan()
                    .inputs()
                    .iter()
                    .find(|input| input.name() == port.name())
                    .and_then(|input| input.span())
                    .expect("compiled voice input has a reserved span");
                let binding = ForwardedBufferInput {
                    parent_start,
                    child_span,
                };
                parent_start += child_span.channel_count;
                binding
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut parent_event_index = 1; // The poly node's first event input is `notes`.
        let forwarded_event_inputs = compiled
            .flattened_voice()
            .root_ports()
            .iter()
            .filter(|port| {
                port.direction() == PortDirection::Input && port.signal_type() == SignalType::Event
            })
            .map(|port| {
                let child_queues = compiled
                    .flattened_voice()
                    .root_input_destinations()
                    .get(port.name())
                    .into_iter()
                    .flatten()
                    .filter_map(|destination| {
                        let step = child_render_plan.global_steps.iter().find(|step| {
                            compiled.child_patch().nodes()[step.module_index]
                                .id
                                .as_str()
                                == destination.node().as_str()
                        })?;
                        let node = &compiled.child_patch().nodes()[step.module_index];
                        let port_index = node
                            .input_port_names
                            .iter()
                            .position(|name| name == destination.port())?;
                        let ordinal = node.input_port_types[..port_index]
                            .iter()
                            .filter(|kind| **kind == SignalType::Event)
                            .count();
                        step.event_inputs.get(ordinal).copied()
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let binding = ForwardedEventInput {
                    parent_index: parent_event_index,
                    child_queues,
                };
                parent_event_index += 1;
                binding
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut next_accumulator = 0;
        let output_bindings = compiled
            .output_accumulators()
            .iter()
            .filter_map(|output| {
                let voice_span = compiled
                    .child_patch()
                    .root_bus_plan()
                    .outputs()
                    .iter()
                    .find(|port| port.name() == output.name())?
                    .span()?;
                let binding = PolyOutputBinding {
                    voice_span,
                    accumulator_start: next_accumulator,
                    signal_type: output.signal_type(),
                };
                next_accumulator += voice_span.channel_count;
                Some(binding)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            node_id: compiled.node_id().to_string(),
            #[cfg(test)]
            states,
            child_module_kinds,
            auto_retire_map_voice,
            voice_arenas,
            voice_event_queues,
            nested_regions,
            output_accumulator,
            audio_buffers_per_voice,
            allocation_policy: compiled.allocation_policy(),
            slots: vec![PolyVoiceSlot::default(); compiled.max_voices()].into_boxed_slice(),
            next_allocation_order: 1,
            block_start_frame: 0,
            intrinsic_bindings,
            done_binding,
            silence_hold_frames: ((sample_rate * RELEASE_SILENCE_SECONDS).ceil() as usize).max(1),
            release_timeout_frames: ((sample_rate * RELEASE_TIMEOUT_SECONDS).ceil() as usize)
                .max(1),
            child_render_plan,
            prepared_step_executors,
            shared_sample_map_selections,
            child_patch: Box::new(compiled.child_patch().clone()),
            forwarded_buffer_inputs,
            forwarded_event_inputs,
            output_bindings,
        }
    }

    pub(super) fn begin_block(&mut self, frames: usize, block_start_frame: u64) {
        self.block_start_frame = block_start_frame;
        for voice in self.prepared_step_executors.iter_mut() {
            for executor in voice.iter_mut() {
                executor.begin_block();
            }
        }
        for queues in &mut self.voice_event_queues {
            queues.clear_all();
        }
        for regions in &mut self.nested_regions {
            for region in regions.iter_mut() {
                region.begin_block(frames, block_start_frame);
            }
        }
        for voice in 0..self.slots.len() {
            self.write_intrinsic_controls(voice, frames);
        }
    }

    pub(super) fn route_note_events(&mut self, events: &[BlockEvent], frames: usize) {
        for event in events {
            match event.event {
                ScriptEvent::NoteOn { note, velocity } => {
                    self.route_note_on(note, velocity, event.frame_offset, frames);
                }
                ScriptEvent::NoteOff { note } => {
                    self.route_note_off(note, event.frame_offset);
                }
            }
        }
    }

    #[cfg(test)]
    pub(super) fn replace_child_step_kind_for_test(
        &mut self,
        original: ModuleKind,
        replacement: ModuleKind,
    ) {
        let step = self
            .child_render_plan
            .global_steps
            .iter_mut()
            .find(|step| step.module_kind == original)
            .expect("test names a prepared child step");
        step.module_kind = replacement;
    }

    pub(super) fn render_into(
        &mut self,
        parent_arena: &mut AudioArena,
        parent_inputs: &[BufferId],
        parent_event_inputs: &[EventQueueId],
        parent_event_queues: &PreparedEventQueues,
        parent_outputs: &[BufferId],
        frames: usize,
    ) {
        for buffer in 0..self.output_accumulator.buffer_count() {
            self.output_accumulator.clear(BufferId(buffer), frames);
        }
        for &buffer in parent_outputs {
            parent_arena.clear(buffer, frames);
        }

        for voice in 0..self.slots.len() {
            if !self.slots[voice].active {
                continue;
            }
            let arena = &mut self.voice_arenas[voice];
            #[cfg(test)]
            let states = &mut self.states[voice];
            for binding in self.forwarded_buffer_inputs.iter().copied() {
                for channel in 0..binding.child_span.channel_count {
                    let source = parent_inputs[binding.parent_start + channel];
                    let destination = BufferId(binding.child_span.first_buffer + channel);
                    for frame in 0..frames {
                        arena.set_sample(destination, frame, parent_arena.sample(source, frame));
                    }
                }
            }
            for binding in self.forwarded_event_inputs.iter() {
                let Some(events) = parent_event_inputs
                    .get(binding.parent_index)
                    .and_then(|queue| parent_event_queues.queue_ref(queue.0))
                else {
                    continue;
                };
                for destination in binding.child_queues.iter().copied() {
                    let child_queue = self.voice_event_queues[voice]
                        .queue_mut(destination.0)
                        .expect("compiled voice event input has a prepared queue");
                    for event in events.events() {
                        let _ = child_queue.push_at(event.event.clone(), event.frame_offset);
                    }
                }
            }
            let mut audio_audible = false;
            let mut peak = 0.0_f32;
            let mut control_done = false;
            let release_start = self.slots[voice].release_offset_pending.min(frames);
            let mut step_context = super::realtime_graph_processor::PreparedStepContext {
                arena: &mut *arena,
                #[cfg(test)]
                states: &mut *states,
                #[cfg(not(test))]
                states: &mut [],
                poly_regions: &mut self.nested_regions[voice],
                event_queues: &mut self.voice_event_queues[voice],
                compiled: &self.child_patch,
                frames,
                block_start_frame: self.block_start_frame,
            };
            for (step_index, step) in self.child_render_plan.global_steps.iter().enumerate() {
                super::realtime_graph_processor::execute_bound_prepared_step(
                    &mut step_context,
                    step,
                    self.prepared_step_executors[voice][step_index].as_mut(),
                );
                if let Some(DoneBinding::Control {
                    buffer,
                    producer_step,
                }) = self.done_binding
                {
                    if step_index == producer_step {
                        control_done = (0..frames)
                            .any(|frame| step_context.arena.sample(buffer, frame) > 0.0);
                    }
                }
            }

            super::realtime_graph_processor::capture_bound_feedback_delays(
                arena,
                &self.child_render_plan.global_steps,
                &self.child_render_plan.feedback_step_indices,
                &mut self.prepared_step_executors[voice],
                frames,
                &self.child_patch,
            );

            for binding in self.output_bindings.iter().copied() {
                for channel in 0..binding.voice_span.channel_count {
                    let source = BufferId(binding.voice_span.first_buffer + channel);
                    let destination = BufferId(binding.accumulator_start + channel);
                    for frame in 0..frames {
                        let now = self.block_start_frame.saturating_add(frame as u64);
                        let choke_gain = match self.slots[voice].choke_start_frame {
                            None => 1.0,
                            Some(start) if now < start => 1.0,
                            Some(_) if self.slots[voice].choke_fade_frames == 0 => 0.0,
                            Some(start) => {
                                let elapsed = now - start;
                                let fade_frames = self.slots[voice].choke_fade_frames;
                                if elapsed >= fade_frames {
                                    0.0
                                } else {
                                    1.0 - elapsed as f32 / fade_frames as f32
                                }
                            }
                        };
                        let sample = arena.sample(source, frame) * choke_gain;
                        if binding.signal_type == SignalType::Audio {
                            peak = peak.max(sample.abs());
                        }
                        audio_audible |= frame >= release_start
                            && binding.signal_type == SignalType::Audio
                            && sample.abs() > RELEASE_SILENCE_THRESHOLD;
                        let sum = self.output_accumulator.sample(destination, frame) + sample;
                        self.output_accumulator.set_sample(destination, frame, sum);
                    }
                }
            }
            self.slots[voice].last_peak = peak;
            let done = match self.done_binding {
                Some(DoneBinding::Event(queue)) => self.voice_event_queues[voice]
                    .queue_ref(queue.0)
                    .is_some_and(|events| !events.is_empty()),
                Some(DoneBinding::Control { .. }) => control_done,
                None => false,
            };
            let one_shot_finished = self.auto_retire_map_voice
                && self.prepared_step_executors[voice]
                    .iter()
                    .all(|executor| !executor.is_active());
            let choke_finished = self.slots[voice].choke_start_frame.is_some_and(|start| {
                self.block_start_frame.saturating_add(frames as u64)
                    >= start.saturating_add(self.slots[voice].choke_fade_frames)
            });
            if done || one_shot_finished || choke_finished {
                self.slots[voice].active = false;
                for region in self.nested_regions[voice].iter_mut() {
                    region.retire_all_voices();
                }
            } else if self.done_binding.is_none() && !self.slots[voice].gate_held {
                let slot = &mut self.slots[voice];
                let released_this_block = frames - release_start;
                slot.release_offset_pending = slot.release_offset_pending.saturating_sub(frames);
                slot.released_frames = slot.released_frames.saturating_add(released_this_block);
                slot.quiet_frames_after_release = if audio_audible {
                    0
                } else {
                    slot.quiet_frames_after_release
                        .saturating_add(released_this_block)
                };
                if slot.quiet_frames_after_release >= self.silence_hold_frames
                    || slot.released_frames >= self.release_timeout_frames
                {
                    slot.active = false;
                    for region in self.nested_regions[voice].iter_mut() {
                        region.retire_all_voices();
                    }
                }
            }
        }

        for (index, &destination) in parent_outputs.iter().enumerate() {
            let source = BufferId(index);
            for frame in 0..frames {
                parent_arena.set_sample(
                    destination,
                    frame,
                    self.output_accumulator.sample(source, frame),
                );
            }
        }
    }

    fn route_note_on(&mut self, note: u8, velocity: u8, frame_offset: u32, frames: usize) {
        let free = self.slots.iter().position(|slot| !slot.active);
        let selected = free.or_else(|| match self.allocation_policy {
            PolyAllocationPolicy::RejectNew => None,
            PolyAllocationPolicy::OldestSteal => self
                .slots
                .iter()
                .enumerate()
                .filter(|(_, slot)| slot.active)
                .min_by_key(|(_, slot)| slot.allocation_order)
                .map(|(index, _)| index),
            PolyAllocationPolicy::QuietestSteal => self
                .slots
                .iter()
                .enumerate()
                .filter(|(_, slot)| slot.active)
                .min_by(|(_, left), (_, right)| {
                    left.last_peak
                        .total_cmp(&right.last_peak)
                        .then_with(|| left.allocation_order.cmp(&right.allocation_order))
                })
                .map(|(index, _)| index),
        });
        let Some(voice) = selected else { return };

        if self.slots[voice].active {
            let retired_note = self.slots[voice].note;
            for region in self.nested_regions[voice].iter_mut() {
                region.retire_all_voices();
            }
            self.push_gate_event(
                voice,
                ScriptEvent::NoteOff { note: retired_note },
                frame_offset,
            );
        }

        #[cfg(test)]
        for state in self.states[voice].iter_mut() {
            state.reset_voice();
        }
        for executor in self.prepared_step_executors[voice].iter_mut() {
            executor.reset_voice();
        }
        for region in self.nested_regions[voice].iter_mut() {
            region.reset();
        }

        let order = self.next_allocation_order;
        self.next_allocation_order = self.next_allocation_order.wrapping_add(1).max(1);
        self.slots[voice] = PolyVoiceSlot {
            active: true,
            gate_held: true,
            note,
            velocity,
            allocation_order: order,
            quiet_frames_after_release: 0,
            released_frames: 0,
            release_offset_pending: 0,
            last_peak: 0.0,
            choke_group: None,
            choke_start_frame: None,
            choke_fade_frames: 0,
        };
        self.write_intrinsic_controls(voice, frames);
        self.push_gate_event(voice, ScriptEvent::NoteOn { note, velocity }, frame_offset);
        for selection in self.shared_sample_map_selections.iter_mut() {
            let choice = super::arena_processing::select_sample_map_zone(
                &selection.zones,
                selection.selection_mode,
                &mut selection.round_robin_counters,
                &mut selection.rng_state,
                note,
                velocity,
            );
            self.prepared_step_executors[voice][selection.step_index]
                .queue_sample_zone_choice(choice);
            let group = choice.and_then(|index| selection.zones[index].choke_group);
            if let Some(group) = group {
                let key = (selection.step_index, group);
                for other in 0..self.slots.len() {
                    if other == voice
                        || !self.slots[other].active
                        || self.slots[other].choke_group != Some(key)
                    {
                        continue;
                    }
                    if self.slots[other].gate_held {
                        if selection.choke_mode != SampleChokeMode::Release {
                            self.slots[other].choke_start_frame = Some(
                                self.block_start_frame
                                    .saturating_add(u64::from(frame_offset)),
                            );
                            self.slots[other].choke_fade_frames =
                                if selection.choke_mode == SampleChokeMode::Fade {
                                    selection.choke_fade_frames
                                } else {
                                    0
                                };
                        }
                        if let Some(bindings) = self.intrinsic_bindings {
                            let queue = self.voice_event_queues[other]
                                .queue_mut(bindings.gate.0)
                                .expect("prepared choke voice has a gate queue");
                            let _ = queue.push_at(
                                ScriptEvent::NoteOff {
                                    note: self.slots[other].note,
                                },
                                frame_offset,
                            );
                        }
                    }
                    self.slots[other].gate_held = false;
                }
                self.slots[voice].choke_group = Some(key);
            }
        }
    }

    fn route_note_off(&mut self, note: u8, frame_offset: u32) {
        for voice in 0..self.slots.len() {
            if self.slots[voice].active
                && self.slots[voice].gate_held
                && self.slots[voice].note == note
            {
                self.slots[voice].gate_held = false;
                self.slots[voice].release_offset_pending = frame_offset as usize;
                self.push_gate_event(voice, ScriptEvent::NoteOff { note }, frame_offset);
            }
        }
    }

    fn retire_all_voices(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.active = false;
            slot.gate_held = false;
        }
        for queues in self.voice_event_queues.iter_mut() {
            queues.clear_all();
        }
        for regions in self.nested_regions.iter_mut() {
            for region in regions.iter_mut() {
                region.retire_all_voices();
            }
        }
    }

    fn write_intrinsic_controls(&mut self, voice: usize, frames: usize) {
        let Some(bindings) = self.intrinsic_bindings else {
            return;
        };
        let Some(arena) = self.voice_arenas.get_mut(voice) else {
            return;
        };
        let slot = self.slots[voice];
        let note = if slot.active {
            2.0_f32.powf((f32::from(slot.note) - 60.0) / 12.0)
        } else {
            0.0
        };
        let velocity = if slot.active {
            f32::from(slot.velocity) / 127.0
        } else {
            0.0
        };
        arena.fill(bindings.note, frames, note);
        arena.fill(bindings.velocity, frames, velocity);
    }

    fn push_gate_event(&mut self, voice: usize, event: ScriptEvent, frame_offset: u32) {
        let Some(bindings) = self.intrinsic_bindings else {
            return;
        };
        let Some(queue) = self
            .voice_event_queues
            .get_mut(voice)
            .and_then(|queues| queues.queue_mut(bindings.gate.0))
        else {
            return;
        };
        let _ = queue.push_at(event, frame_offset);
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn voice_count(&self) -> usize {
        self.slots.len()
    }

    pub fn states_per_voice(&self) -> usize {
        self.prepared_step_executors
            .first()
            .map_or(0, |executors| executors.len())
    }

    pub fn child_module_kinds(&self) -> &[ModuleKind] {
        &self.child_module_kinds
    }

    pub fn state_instance_address(&self, voice: usize, child_node: usize) -> Option<usize> {
        let step_index = self
            .child_render_plan
            .global_steps
            .iter()
            .position(|step| step.module_index == child_node)?;
        self.prepared_step_executors
            .get(voice)?
            .get(step_index)
            .map(|executor| std::ptr::from_ref(executor.as_ref()).cast::<()>().addr())
    }

    pub fn voice_arena_count(&self) -> usize {
        self.voice_arenas.len()
    }

    pub fn audio_buffers_per_voice(&self) -> usize {
        self.audio_buffers_per_voice
    }

    pub fn voice_event_queue_set_count(&self) -> usize {
        self.voice_event_queues.len()
    }

    pub fn event_queues_per_voice(&self) -> usize {
        self.voice_event_queues
            .first()
            .map_or(0, PreparedEventQueues::queue_count)
    }

    pub fn event_queue_capacity(&self) -> usize {
        self.voice_event_queues
            .first()
            .map_or(0, PreparedEventQueues::capacity_per_queue)
    }

    pub fn output_accumulator_buffer_count(&self) -> usize {
        self.output_accumulator.buffer_count()
    }

    pub fn active_voice_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.active).count()
    }

    #[cfg(test)]
    pub(crate) fn nested_region_for_voice(
        &self,
        voice: usize,
        node_id: &str,
    ) -> Option<&PreparedPolyRuntimeRegion> {
        self.nested_regions
            .get(voice)?
            .iter()
            .find(|region| region.node_id() == node_id)
    }

    #[cfg(test)]
    pub(crate) fn nested_region_for_voice_mut(
        &mut self,
        voice: usize,
        node_id: &str,
    ) -> Option<&mut PreparedPolyRuntimeRegion> {
        self.nested_regions
            .get_mut(voice)?
            .iter_mut()
            .find(|region| region.node_id() == node_id)
    }

    #[cfg(test)]
    pub(crate) fn route_note_event_for_test(&mut self, event: ScriptEvent, frames: usize) {
        self.route_note_events(
            &[BlockEvent {
                frame_offset: 0,
                event,
            }],
            frames,
        );
    }

    pub fn voice_note(&self, voice: usize) -> Option<u8> {
        self.slots
            .get(voice)
            .filter(|slot| slot.active)
            .map(|slot| slot.note)
    }

    pub fn voice_velocity(&self, voice: usize) -> Option<u8> {
        self.slots
            .get(voice)
            .filter(|slot| slot.active)
            .map(|slot| slot.velocity)
    }

    pub fn voice_gate_held(&self, voice: usize) -> Option<bool> {
        self.slots
            .get(voice)
            .filter(|slot| slot.active)
            .map(|slot| slot.gate_held)
    }

    #[cfg(test)]
    pub(crate) fn set_child_control_default_for_test(
        &mut self,
        module_id: &str,
        port_name: &str,
        value: f32,
    ) -> bool {
        self.child_patch
            .set_numeric_parameter_by_target(module_id, port_name, value)
    }

    pub fn voice_note_control(&self, voice: usize) -> Option<f32> {
        let bindings = self.intrinsic_bindings?;
        self.voice_arenas
            .get(voice)
            .map(|arena| arena.sample(bindings.note, 0))
    }

    pub fn voice_velocity_control(&self, voice: usize) -> Option<f32> {
        let bindings = self.intrinsic_bindings?;
        self.voice_arenas
            .get(voice)
            .map(|arena| arena.sample(bindings.velocity, 0))
    }

    #[cfg(test)]
    pub(crate) fn voice_gate_events(&self, voice: usize) -> &[BlockEvent] {
        let Some(bindings) = self.intrinsic_bindings else {
            return &[];
        };
        self.voice_event_queues
            .get(voice)
            .and_then(|queues| queues.queue_ref(bindings.gate.0))
            .map_or(&[], |queue| queue.events())
    }
}

fn is_poly_child_arena_supported(step: &RenderStep) -> bool {
    match step.module_kind {
        ModuleKind::EventFilter => {
            step.input_buffers.is_empty()
                && step.output_buffers.is_empty()
                && step.event_inputs.len() == 1
                && step.event_outputs.len() == 1
        }
        ModuleKind::Adsr => {
            step.input_buffers.len() == 4
                && step.output_buffers.len() == 1
                && step.event_inputs.len() == 1
        }
        ModuleKind::Decay => {
            step.input_buffers.len() == 1
                && step.output_buffers.len() == 1
                && step.event_inputs.len() == 1
        }
        ModuleKind::Impulse => {
            step.input_buffers.is_empty()
                && !step.output_buffers.is_empty()
                && step.event_inputs.len() == 1
        }
        ModuleKind::Sampler => {
            step.input_buffers.len() == 5
                && !step.output_buffers.is_empty()
                && step.event_inputs.len() == 1
        }
        ModuleKind::SamplePlayer => {
            step.input_buffers.len() == 4
                && !step.output_buffers.is_empty()
                && step.event_inputs.len() == 2
        }
        ModuleKind::SampleMapPlayer => {
            step.input_buffers.len() == 5
                && !step.output_buffers.is_empty()
                && step.event_inputs.len() == 1
        }
        ModuleKind::NoteToControl => {
            step.input_buffers.is_empty()
                && step.output_buffers.len() == 4
                && step.event_inputs.len() == 1
                && step.event_outputs.len() == 1
        }
        _ => super::realtime_graph_processor::is_channel_arena_supported(step),
    }
}

pub(crate) fn first_unrenderable_poly_child(
    child: &CompiledPatch,
    max_block_frames: usize,
    event_queue_capacity: usize,
) -> Option<(String, String)> {
    let plan = RenderPlan::from_compiled_patch(child, max_block_frames, 1, event_queue_capacity);
    plan.global_steps
        .iter()
        .find(|step| !is_poly_child_arena_supported(step))
        .map(|step| {
            let node = &child.nodes()[step.module_index];
            (node.id.as_str().to_string(), node.module_type.clone())
        })
}

fn voice_done_binding(compiled: &CompiledPolyRegion, plan: &RenderPlan) -> Option<DoneBinding> {
    let source = compiled
        .flattened_voice()
        .root_output_sources()
        .get(POLY_DONE_OUTPUT)?
        .first()?;
    let (producer_step, step) = plan.global_steps.iter().enumerate().find(|(_, step)| {
        compiled.child_patch().nodes()[step.module_index]
            .id
            .as_str()
            == source.node().as_str()
    })?;
    let node = &compiled.child_patch().nodes()[step.module_index];
    let output_index = node
        .output_port_names
        .iter()
        .position(|name| name == source.port())?;
    let signal_type = node.output_port_types[output_index];
    let is_event = signal_type == crate::graph::SignalType::Event;
    let ordinal = node.output_port_types[..output_index]
        .iter()
        .filter(|kind| (**kind == crate::graph::SignalType::Event) == is_event)
        .count();
    if is_event {
        step.event_outputs
            .get(ordinal)
            .copied()
            .map(DoneBinding::Event)
    } else {
        // Validation permits only event or control for a voice's `done` port.
        step.output_buffers
            .get(ordinal)
            .copied()
            .map(|buffer| DoneBinding::Control {
                buffer,
                producer_step,
            })
    }
}

fn voice_intrinsic_bindings(
    child: &CompiledPatch,
    plan: &RenderPlan,
) -> Option<VoiceIntrinsicBindings> {
    let step = plan
        .global_steps
        .iter()
        .find(|step| step.module_kind == ModuleKind::VoiceIntrinsics)?;
    let node = child.nodes().get(step.module_index)?;
    let non_event_buffer = |port_name: &str| {
        node.output_port_names
            .iter()
            .zip(node.output_port_types.iter())
            .filter(|(_, signal_type)| **signal_type != crate::graph::SignalType::Event)
            .position(|(name, _)| name == port_name)
            .and_then(|index| step.output_buffers.get(index).copied())
    };
    let gate_ordinal = node
        .output_port_names
        .iter()
        .zip(node.output_port_types.iter())
        .filter(|(_, signal_type)| **signal_type == crate::graph::SignalType::Event)
        .position(|(name, _)| name == VOICE_GATE_OUTPUT)?;

    Some(VoiceIntrinsicBindings {
        note: non_event_buffer(VOICE_NOTE_OUTPUT)?,
        velocity: non_event_buffer(VOICE_VELOCITY_OUTPUT)?,
        gate: *step.event_outputs.get(gate_ordinal)?,
    })
}

#[cfg(test)]
pub(super) fn build_polyphonic_states_from_compiled(
    compiled: &CompiledPatch,
    sample_rate: f32,
    sampler_assets: &PreparedSamplerAssets,
    max_voices: usize,
) -> Vec<Vec<PerModuleState>> {
    (0..max_voices)
        .map(|_| {
            compiled
                .nodes()
                .iter()
                .map(|node| PerModuleState::new_compiled(node, sample_rate, sampler_assets))
                .collect::<Vec<_>>()
        })
        .collect()
}
