use crate::builtins::module_kind::ModuleKind;
use crate::compiled_patch::{CompiledPatch, CompiledPolyRegion, CompiledPortSpan};
use crate::graph::SignalType;
use crate::kernel::{
    POLY_DONE_OUTPUT, PolyAllocationPolicy, VOICE_GATE_OUTPUT, VOICE_NOTE_OUTPUT,
    VOICE_VELOCITY_OUTPUT,
};
use crate::sample::PreparedSamplerAssets;
use crate::script::ScriptEvent;

use super::audio_arena::AudioArena;
use super::event_queue::PreparedEventQueues;
use super::outputs::BlockEvent;
use super::process_context::ProcessContext;
use super::render_plan::{
    AudioBufferPlan, BufferId, CompiledEventEdge, EventQueueId, RenderPlan, RenderStep,
};
use super::state::PerModuleState;

/// Linear peak amplitude below which a released voice counts as silent.
const RELEASE_SILENCE_THRESHOLD: f32 = 1.0e-4;
/// A released voice without `done` must remain silent for this long to retire.
const RELEASE_SILENCE_SECONDS: f32 = 0.010;
/// A released voice without `done` is retired by this deadline even if audible.
const RELEASE_TIMEOUT_SECONDS: f32 = 5.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PolyVoiceSlot {
    active: bool,
    gate_held: bool,
    note: u8,
    velocity: u8,
    allocation_order: u64,
    quiet_frames_after_release: usize,
    released_frames: usize,
    release_offset_pending: usize,
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
enum DoneBinding {
    Event(EventQueueId),
    Control(BufferId),
}

pub struct PreparedPolyRuntimeRegion {
    node_id: String,
    states: Box<[Box<[PerModuleState]>]>,
    child_module_kinds: Box<[ModuleKind]>,
    voice_arenas: Box<[AudioArena]>,
    voice_event_queues: Box<[PreparedEventQueues]>,
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
    child_patch: Box<CompiledPatch>,
    output_bindings: Box<[PolyOutputBinding]>,
}

impl PreparedPolyRuntimeRegion {
    pub(super) fn new(
        compiled: &CompiledPolyRegion,
        sample_rate: f32,
        sampler_assets: &PreparedSamplerAssets,
    ) -> Self {
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
        let intrinsic_bindings =
            voice_intrinsic_bindings(compiled.child_patch(), &child_render_plan);
        let done_binding = voice_done_binding(compiled, &child_render_plan);
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
            states,
            child_module_kinds,
            voice_arenas,
            voice_event_queues,
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
            child_patch: Box::new(compiled.child_patch().clone()),
            output_bindings,
        }
    }

    pub(super) fn begin_block(&mut self, frames: usize, block_start_frame: u64) {
        self.block_start_frame = block_start_frame;
        for queues in &mut self.voice_event_queues {
            queues.clear_all();
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

    pub(super) fn render_into(
        &mut self,
        parent_arena: &mut AudioArena,
        parent_outputs: &[BufferId],
        frames: usize,
    ) {
        for buffer in 0..self.output_accumulator.buffer_count() {
            self.output_accumulator.clear(BufferId(buffer), frames);
        }
        for &buffer in parent_outputs {
            parent_arena.clear(buffer, frames);
        }

        debug_assert!(
            self.child_render_plan
                .global_steps
                .iter()
                .all(is_poly_child_arena_supported)
        );

        for voice in 0..self.slots.len() {
            if !self.slots[voice].active {
                continue;
            }
            let arena = &mut self.voice_arenas[voice];
            let states = &mut self.states[voice];
            let mut audio_audible = false;
            let release_start = self.slots[voice].release_offset_pending.min(frames);
            for step in self.child_render_plan.global_steps.iter() {
                for edge in step.incoming_event_edges.iter().copied() {
                    let _ = self.voice_event_queues[voice].route_event_edge(edge);
                }
                super::realtime_graph_processor::clear_and_route_arena_inputs(
                    arena,
                    step,
                    frames,
                    // Control defaults are owned by the child compiled patch.
                    // The render plan's slot ids index this exact patch.
                    //
                    // Keeping this as a shared immutable borrow is the Rust
                    // equivalent of sharing readonly construction metadata
                    // across voice instances while their DSP state stays
                    // disjoint.
                    &self.child_patch,
                );
                match step.module_kind {
                    ModuleKind::EventFilter => {
                        let PerModuleState::EventFilter { note } = &states[step.module_index]
                        else {
                            unreachable!()
                        };
                        let edge = CompiledEventEdge {
                            source: step.event_inputs[0],
                            destination: step.event_outputs[0],
                        };
                        let _ =
                            self.voice_event_queues[voice].route_filtered_event_edge(edge, *note);
                    }
                    ModuleKind::Adsr => {
                        let events = self.voice_event_queues[voice]
                            .queue_ref(step.event_inputs[0].0)
                            .map_or(&[][..], |queue| queue.events());
                        let mut context = ProcessContext::new(
                            arena,
                            &step.input_buffers,
                            &step.output_buffers,
                            frames,
                        );
                        super::arena_processing::process_adsr(
                            &mut states[step.module_index],
                            &mut context,
                            events,
                            self.block_start_frame,
                        );
                    }
                    _ => super::realtime_graph_processor::process_channel_arena_step(
                        arena, states, step, frames,
                    ),
                }
            }

            for binding in self.output_bindings.iter().copied() {
                for channel in 0..binding.voice_span.channel_count {
                    let source = BufferId(binding.voice_span.first_buffer + channel);
                    let destination = BufferId(binding.accumulator_start + channel);
                    for frame in 0..frames {
                        let sample = arena.sample(source, frame);
                        audio_audible |= frame >= release_start
                            && binding.signal_type == SignalType::Audio
                            && sample.abs() > RELEASE_SILENCE_THRESHOLD;
                        let sum = self.output_accumulator.sample(destination, frame) + sample;
                        self.output_accumulator.set_sample(destination, frame, sum);
                    }
                }
            }
            let done = match self.done_binding {
                Some(DoneBinding::Event(queue)) => self.voice_event_queues[voice]
                    .queue_ref(queue.0)
                    .is_some_and(|events| !events.is_empty()),
                Some(DoneBinding::Control(buffer)) => arena.sample(buffer, 0) > 0.0,
                None => false,
            };
            if done {
                self.slots[voice].active = false;
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
        });
        let Some(voice) = selected else { return };

        if self.slots[voice].active {
            let retired_note = self.slots[voice].note;
            self.push_gate_event(
                voice,
                ScriptEvent::NoteOff { note: retired_note },
                frame_offset,
            );
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
        };
        self.write_intrinsic_controls(voice, frames);
        self.push_gate_event(voice, ScriptEvent::NoteOn { note, velocity }, frame_offset);
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
        self.states.len()
    }

    pub fn states_per_voice(&self) -> usize {
        self.states.first().map_or(0, |states| states.len())
    }

    pub fn child_module_kinds(&self) -> &[ModuleKind] {
        &self.child_module_kinds
    }

    pub fn state_instance_address(&self, voice: usize, child_node: usize) -> Option<usize> {
        self.states
            .get(voice)?
            .get(child_node)
            .map(|state| std::ptr::from_ref(state).addr())
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
    let step = plan.global_steps.iter().find(|step| {
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
            .map(DoneBinding::Control)
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
