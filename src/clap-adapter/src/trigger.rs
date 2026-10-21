//! Concrete headless CLAP projection of the persistent Trigger owner.
use clack_extensions::{
    audio_ports::*,
    log::{HostLog, LogSeverity},
    note_ports::*,
    params::*,
    state::*,
};
use clack_plugin::events::{
    Event,
    event_types::{MidiEvent, ParamValueEvent},
};
use clack_plugin::stream::{InputStream, OutputStream};
use clack_plugin::{host::HostInfo, plugin::PluginInstance, prelude::*};
use dandrum_trigger_host::{
    TriggerHostConfiguration,
    parameters::{ParameterController, PublicParameter},
    runtime::{ControlEvent, RuntimeAudio, TimedEvent},
};
use std::io::{Read, Write as IoWrite};
use std::{
    cell::{Cell, RefCell},
    ffi::CStr,
    fmt::Write,
    num::NonZeroU64,
    path::PathBuf,
    sync::atomic::{AtomicU8, Ordering},
};

const EVENT_LIMIT: usize = 64;
const EMPTY_EVENT: TimedEvent = TimedEvent {
    frame_offset: 0,
    event: ControlEvent::NoteOff { note: 0 },
};
struct Shared {
    diagnostic: AtomicU8,
}
impl PluginShared<'_> for Shared {}
impl Shared {
    fn reject(&self, host: &HostSharedHandle<'_>, reason: u8) {
        self.diagnostic.fetch_or(reason, Ordering::Relaxed);
        host.request_callback();
    }
}
struct Main<'a> {
    controller: RefCell<ParameterController>,
    parked: Cell<Option<RuntimeAudio>>,
    parameters: Vec<PublicParameter>,
    host: HostMainThreadHandle<'a>,
    shared: &'a Shared,
}
impl<'a> PluginMainThread<'a, Shared> for Main<'a> {
    fn on_main_thread(&self) {
        let reasons = self.shared.diagnostic.swap(0, Ordering::Relaxed);
        if let Some(log) = self.host.get_extension::<HostLog>() {
            for (bit, message) in [
                (1, c"DANDRUM_TRIGGER_INVALID_PROCESS"),
                (2, c"DANDRUM_TRIGGER_INVALID_FLUSH"),
            ] {
                if reasons & bit != 0 {
                    log.log(&self.host.shared(), LogSeverity::Warning, message);
                }
            }
        }
    }
}
struct TriggerAdapter;
impl Plugin for TriggerAdapter {
    type Shared<'a> = Shared;
    type MainThread<'a> = Main<'a>;
    type AudioProcessor<'a> = Processor<'a>;
    fn declare_extensions(builder: &mut PluginExtensions<Self>, _: Option<&Shared>) {
        builder
            .register::<PluginParams>()
            .register::<PluginAudioPorts>()
            .register::<PluginNotePorts>()
            .register::<PluginState>();
    }
}
pub(super) fn instance<'a>(
    host: HostInfo<'a>,
    descriptor: &'a PluginDescriptor,
    path: Option<PathBuf>,
) -> PluginInstance<'a> {
    PluginInstance::new::<TriggerAdapter>(
        host,
        descriptor,
        |_| {
            Ok(Shared {
                diagnostic: AtomicU8::new(0),
            })
        },
        move |host, shared| {
            let path =
                path.filter(|p| p.is_absolute() && p.is_file())
                    .ok_or(PluginError::Message(
                        "Trigger requires an absolute plugin file path",
                    ))?;
            let root = path
                .parent()
                .ok_or(PluginError::Message("Trigger plugin has no parent"))?
                .join("DandrumTrigger.resources");
            let (controller, audio) = ParameterController::open(
                root,
                TriggerHostConfiguration {
                    sample_rate_hz: 48000,
                    max_block_frames: 16,
                },
                NonZeroU64::new(1).unwrap(),
                64,
                256,
            )
            .map_err(|_| PluginError::Message("Trigger packaged session failed"))?;
            let parameters = controller.parameters().to_vec();
            Ok(Main {
                controller: RefCell::new(controller),
                parked: Cell::new(Some(audio)),
                host,
                shared,
                parameters,
            })
        },
    )
}
impl Main<'_> {
    fn parameter(&self, id: ClapId) -> Option<&PublicParameter> {
        self.parameters.get(id.get().checked_sub(1)? as usize)
    }
}
impl PluginMainThreadParams for Main<'_> {
    fn count(&self) -> u32 {
        self.parameters.len() as u32
    }
    fn get_info(&self, index: u32, writer: &mut ParamInfoWriter) {
        if let Some(p) = self.parameters.get(index as usize) {
            writer.set(&ParamInfo {
                id: ClapId::new(index + 1),
                flags: ParamInfoFlags::IS_AUTOMATABLE,
                cookie: Default::default(),
                name: p.id.as_bytes(),
                module: b"",
                min_value: p.physical_min,
                max_value: p.physical_max,
                default_value: p.physical_default,
            });
        }
    }
    fn get_value(&self, id: ClapId) -> Option<f64> {
        self.controller
            .try_borrow()
            .ok()?
            .physical_value(id.get().checked_sub(1)? as usize)
    }
    fn value_to_text(
        &self,
        id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        self.parameter(id).ok_or(std::fmt::Error)?;
        write!(writer, "{value:.4}")
    }
    fn text_to_value(&self, id: ClapId, text: &CStr) -> Option<f64> {
        let p = self.parameter(id)?;
        let value: f64 = text.to_str().ok()?.parse().ok()?;
        (value.is_finite() && (p.physical_min..=p.physical_max).contains(&value)).then_some(value)
    }
    fn flush(&self, input: &InputEvents, _: &mut OutputEvents) {
        let accepted = if let Some(mut audio) = self.parked.take() {
            let accepted = flush_parameters(&mut audio, &self.parameters, input);
            self.parked.set(Some(audio));
            accepted
        } else {
            false
        };
        if !accepted {
            self.shared.reject(&self.host.shared(), 2);
        }
    }
}

impl PluginAudioPortsImpl for Main<'_> {
    fn count(&self, input: bool) -> u32 {
        u32::from(!input)
    }
    fn get(&self, index: u32, input: bool, writer: &mut AudioPortInfoWriter) {
        if index == 0 && !input {
            writer.set(&AudioPortInfo {
                id: ClapId::new(0),
                name: b"main",
                channel_count: 2,
                flags: AudioPortFlags::IS_MAIN,
                port_type: Some(AudioPortType::STEREO),
                in_place_pair: None,
            });
        }
    }
}
impl PluginNotePortsImpl for Main<'_> {
    fn count(&self, input: bool) -> u32 {
        u32::from(input)
    }
    fn get(&self, index: u32, input: bool, writer: &mut NotePortInfoWriter) {
        if index == 0 && input {
            writer.set(&NotePortInfo {
                id: ClapId::new(0),
                name: b"MIDI 1 notes",
                supported_dialects: NoteDialects::MIDI,
                preferred_dialect: Some(NoteDialect::Midi),
            });
        }
    }
}
/// Check the entire foreign list before changing the runtime. Typed casts check
/// payload size; Clack's event-space cast alone does not perform that check.
fn translate(
    input: &InputEvents,
    parameters: &[PublicParameter],
    frames: Option<usize>,
    scratch: &mut [TimedEvent; EVENT_LIMIT],
) -> Result<usize, ()> {
    if input.len() as usize > EVENT_LIMIT {
        return Err(());
    }
    let mut count = 0;
    let mut previous = 0;
    for index in 0..input.len() {
        let event = input.get(index).ok_or(())?;
        let header = event.header();
        if header.size() < std::mem::size_of::<clack_plugin::events::EventHeader>() as u32 {
            return Err(());
        }
        let offset = header.time();
        if let Some(frames) = frames {
            if offset < previous || offset as usize >= frames {
                return Err(());
            }
            previous = offset;
        }
        if header.as_raw().space_id != 0 {
            continue;
        }
        let value = if header.type_id() == ParamValueEvent::TYPE_ID {
            let value = event.as_event::<ParamValueEvent>().ok_or(())?;
            let p = value
                .param_id()
                .and_then(|id| id.get().checked_sub(1))
                .and_then(|index| parameters.get(index as usize))
                .ok_or(())?;
            if !value.pckn().matches_all()
                || !value.value().is_finite()
                || !(value.value() as f32).is_finite()
                || !(p.physical_min..=p.physical_max).contains(&value.value())
            {
                return Err(());
            }
            Some(ControlEvent::Parameter {
                index: p.root_index,
                value: value.value(),
            })
        } else if frames.is_some() && header.type_id() == MidiEvent::TYPE_ID {
            let midi = event.as_event::<MidiEvent>().ok_or(())?;
            let [status, note, velocity] = midi.data();
            if matches!(status & 0xf0, 0x80 | 0x90) {
                if midi.port_index() != 0 || note >= 128 || velocity >= 128 {
                    return Err(());
                }
                Some(if status & 0xf0 == 0x90 && velocity > 0 {
                    ControlEvent::NoteOn { note, velocity }
                } else {
                    ControlEvent::NoteOff { note }
                })
            } else {
                None
            }
        } else {
            None
        };
        if let Some(event) = value {
            scratch[count] = TimedEvent {
                frame_offset: if frames.is_some() { offset } else { 0 },
                event,
            };
            count += 1;
        }
    }
    Ok(count)
}
fn flush_parameters(
    audio: &mut RuntimeAudio,
    parameters: &[PublicParameter],
    input: &InputEvents,
) -> bool {
    let mut scratch = [EMPTY_EVENT; EVENT_LIMIT];
    let Ok(count) = translate(input, parameters, None, &mut scratch) else {
        return false;
    };
    let mut values = [ControlEvent::NoteOff { note: 0 }; EVENT_LIMIT];
    for (target, source) in values.iter_mut().zip(&scratch[..count]) {
        *target = source.event;
    }
    audio.flush_parameters(&values[..count]).is_ok()
}
fn silence(audio: &mut Audio) {
    for mut port in audio.output_ports() {
        match port.channels() {
            Ok(SampleType::F32(mut channels)) => {
                for channel in channels.iter_mut() {
                    channel.fill(0.0);
                }
            }
            Ok(SampleType::F64(mut channels)) => {
                for channel in channels.iter_mut() {
                    channel.fill(0.0);
                }
            }
            Ok(SampleType::Both(mut f32s, mut f64s)) => {
                for c in f32s.iter_mut() {
                    c.fill(0.0);
                }
                for c in f64s.iter_mut() {
                    c.fill(0.0);
                }
            }
            Err(_) => {}
        }
    }
}
struct Processor<'a> {
    audio: RuntimeAudio,
    parameters: Vec<PublicParameter>,
    maximum: usize,
    host: HostAudioProcessorHandle<'a>,
    shared: &'a Shared,
    scratch: [TimedEvent; EVENT_LIMIT],
}
impl PluginAudioProcessorParams for Processor<'_> {
    fn flush(&mut self, input: &InputEvents, _: &mut OutputEvents) {
        if !flush_parameters(&mut self.audio, &self.parameters, input) {
            self.shared.reject(&self.host.shared(), 2);
        }
    }
}
impl<'a> PluginAudioProcessor<'a, Shared, Main<'a>> for Processor<'a> {
    fn activate(
        host: HostAudioProcessorHandle<'a>,
        main: &Main<'a>,
        shared: &'a Shared,
        config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        if !config.sample_rate.is_finite()
            || config.sample_rate.fract() != 0.0
            || !(1.0..=f64::from(u32::MAX)).contains(&config.sample_rate)
            || config.min_frames_count == 0
            || config.max_frames_count < config.min_frames_count
            || config.max_frames_count > 16384
        {
            return Err(PluginError::Message(
                "Unsupported Trigger audio configuration",
            ));
        }
        let mut audio = main
            .parked
            .take()
            .ok_or(PluginError::Message("Trigger endpoint already active"))?;
        let result = (|| {
            let mut controller = main
                .controller
                .try_borrow_mut()
                .map_err(|_| PluginError::Message("Trigger main owner busy"))?;
            controller
                .service_quiescent(&mut audio)
                .map_err(|_| PluginError::Message("Trigger endpoint service failed"))?;
            controller
                .reconfigure_quiescent(
                    &mut audio,
                    TriggerHostConfiguration {
                        sample_rate_hz: config.sample_rate as u32,
                        max_block_frames: config.max_frames_count,
                    },
                )
                .map_err(|_| PluginError::Message("Trigger render preparation failed"))?;
            Ok(())
        })();
        if let Err(error) = result {
            main.parked.set(Some(audio));
            return Err(error);
        }
        Ok(Self {
            audio,
            parameters: main.parameters.clone(),
            maximum: config.max_frames_count as usize,
            host,
            shared,
            scratch: [EMPTY_EVENT; EVENT_LIMIT],
        })
    }
    fn deactivate(self, main: &Main<'a>) {
        main.parked.set(Some(self.audio));
    }
    fn reset(&mut self) {
        self.audio.reset();
    }
    fn process(
        &mut self,
        _: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        let frames = audio.frames_count() as usize;
        let count = if frames > 0
            && frames <= self.maximum
            && audio.input_port_count() == 0
            && audio.output_port_count() == 1
        {
            translate(
                events.input,
                &self.parameters,
                Some(frames),
                &mut self.scratch,
            )
        } else {
            Err(())
        };
        let accepted = if let Ok(count) = count {
            if let Some(mut port) = audio.output_port(0) {
                if let Ok(SampleType::F32(mut channels)) = port.channels() {
                    if channels.channel_count() == 2 {
                        let mut channels = channels.iter_mut();
                        if let (Some(left), Some(right)) = (channels.next(), channels.next()) {
                            self.audio
                                .process_events(left, right, &self.scratch[..count])
                                .is_ok()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        if !accepted {
            silence(&mut audio);
            self.shared.reject(&self.host.shared(), 1);
        }
        Ok(ProcessStatus::Continue)
    }
}

impl PluginStateImpl for Main<'_> {
    fn save(&self, output: &mut OutputStream) -> Result<(), PluginError> {
        let mut bytes = Vec::new();
        {
            let controller = self
                .controller
                .try_borrow()
                .map_err(|_| PluginError::Message("Trigger main owner busy"))?;
            controller
                .save_state(&mut bytes)
                .map_err(|_| PluginError::Message("Trigger state save failed"))?;
        }
        // The host stream may reenter the plugin. No controller borrow crosses it.
        output
            .write_all(&bytes)
            .map_err(|_| PluginError::Message("Trigger state stream write failed"))
    }
    fn load(&self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut bytes = Vec::new();
        input
            .take(dandrum_trigger_host::state::STATE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| PluginError::Message("Trigger state stream read failed"))?;
        if bytes.len() as u64 > dandrum_trigger_host::state::STATE_LIMIT {
            return Err(PluginError::Message("Trigger state too large"));
        }
        {
            let mut controller = self
                .controller
                .try_borrow_mut()
                .map_err(|_| PluginError::Message("Trigger main owner busy"))?;
            controller
                .load_state(&mut bytes.as_slice())
                .map_err(|_| PluginError::Message("Invalid Trigger state"))?;
            if let Some(mut audio) = self.parked.take() {
                let result = controller.service_quiescent(&mut audio);
                self.parked.set(Some(audio));
                result.map_err(|_| {
                    PluginError::Message("Trigger restored endpoint service failed")
                })?;
            }
        }
        if let Some(params) = self.host.get_extension::<HostParams>() {
            params.rescan(&self.host, ParamRescanFlags::VALUES);
        }
        Ok(())
    }
}
