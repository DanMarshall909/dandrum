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
    rc::Rc,
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
#[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
mod automation;
#[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
mod gui;
#[cfg(all(
    feature = "trigger-editor",
    target_os = "linux",
    not(target_env = "gnu")
))]
compile_error!("trigger-editor requires the GNU Linux loader ABI (glibc)");

struct Core {
    controller: RefCell<ParameterController>,
    parked: Cell<Option<RuntimeAudio>>,
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    automation: RefCell<automation::Main>,
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    parked_output: Cell<Option<automation::Output>>,
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    resource_root: PathBuf,
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    plugin_path: PathBuf,
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    gui: RefCell<gui::EditorState>,
}
struct Main<'a> {
    core: Rc<Core>,
    parameters: Vec<PublicParameter>,
    host: HostMainThreadHandle<'a>,
    shared: &'a Shared,
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    timer: gui::TimerRegistration,
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    creating: Cell<bool>,
}
impl<'a> PluginMainThread<'a, Shared> for Main<'a> {
    fn on_main_thread(&self) {
        #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
        self.service_parameter_delivery();
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
        #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
        builder
            .register::<clack_extensions::gui::PluginGui>()
            .register::<clack_extensions::timer::PluginTimer>();
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
                &root,
                TriggerHostConfiguration {
                    sample_rate_hz: 48000,
                    max_block_frames: 16,
                },
                wire_instance_id()?,
                64,
                256,
            )
            .map_err(|_| PluginError::Message("Trigger packaged session failed"))?;
            #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
            let gui = gui::EditorState::default();
            #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
            let controller = {
                let mut controller = controller;
                gui.start_worker(&mut controller)?;
                controller
            };
            let parameters = controller.parameters().to_vec();
            #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
            let (automation, output) = automation::pair();
            Ok(Main {
                core: Rc::new(Core {
                    controller: RefCell::new(controller),
                    parked: Cell::new(Some(audio)),
                    #[cfg(all(
                        feature = "trigger-editor",
                        target_os = "linux",
                        target_env = "gnu"
                    ))]
                    automation: RefCell::new(automation),
                    #[cfg(all(
                        feature = "trigger-editor",
                        target_os = "linux",
                        target_env = "gnu"
                    ))]
                    parked_output: Cell::new(Some(output)),
                    #[cfg(all(
                        feature = "trigger-editor",
                        target_os = "linux",
                        target_env = "gnu"
                    ))]
                    resource_root: root,
                    #[cfg(all(
                        feature = "trigger-editor",
                        target_os = "linux",
                        target_env = "gnu"
                    ))]
                    plugin_path: path,
                    #[cfg(all(
                        feature = "trigger-editor",
                        target_os = "linux",
                        target_env = "gnu"
                    ))]
                    gui: RefCell::new(gui),
                }),
                host,
                shared,
                parameters,
                #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
                timer: gui::TimerRegistration::default(),
                #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
                creating: Cell::new(false),
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
        self.core
            .controller
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
    fn flush(&self, input: &InputEvents, _output: &mut OutputEvents) {
        let accepted = if let Some(mut audio) = self.core.parked.take() {
            let accepted = flush_parameters(&mut audio, &self.parameters, input);
            #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
            if accepted {
                self.core.service_automation();
                if let Some(mut delivery) = self.core.parked_output.take() {
                    deliver_automation(&mut delivery, _output);
                    self.core.parked_output.set(Some(delivery));
                }
            }
            self.core.parked.set(Some(audio));
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
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    automation: automation::Output,
}
impl PluginAudioProcessorParams for Processor<'_> {
    fn flush(&mut self, input: &InputEvents, _output: &mut OutputEvents) {
        if !flush_parameters(&mut self.audio, &self.parameters, input) {
            self.shared.reject(&self.host.shared(), 2);
        } else {
            #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
            if deliver_automation(&mut self.automation, _output) != 0 {
                self.host.shared().request_callback();
            }
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
            .core
            .parked
            .take()
            .ok_or(PluginError::Message("Trigger endpoint already active"))?;
        let result = (|| {
            let mut controller = main
                .core
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
            main.core.parked.set(Some(audio));
            return Err(error);
        }
        #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
        let automation = match main.core.parked_output.take() {
            Some(output) => output,
            None => {
                main.core.parked.set(Some(audio));
                return Err(PluginError::Message(
                    "Trigger delivery endpoint already active",
                ));
            }
        };
        Ok(Self {
            #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
            automation,
            audio,
            parameters: main.parameters.clone(),
            maximum: config.max_frames_count as usize,
            host,
            shared,
            scratch: [EMPTY_EVENT; EVENT_LIMIT],
        })
    }
    fn deactivate(self, main: &Main<'a>) {
        main.core.parked.set(Some(self.audio));
        #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
        main.core.parked_output.set(Some(self.automation));
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
        #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
        if accepted && deliver_automation(&mut self.automation, events.output) != 0 {
            self.host.shared().request_callback();
        }
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
                .core
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
                .core
                .controller
                .try_borrow_mut()
                .map_err(|_| PluginError::Message("Trigger main owner busy"))?;
            controller
                .load_state(&mut bytes.as_slice())
                .map_err(|_| PluginError::Message("Invalid Trigger state"))?;
            if let Some(mut audio) = self.core.parked.take() {
                let result = controller.service_quiescent(&mut audio);
                self.core.parked.set(Some(audio));
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

// Wire identities are scoped to this loaded factory, not other copied DSOs.
fn wire_instance_id() -> Result<NonZeroU64, PluginError> {
    #[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
    {
        use std::sync::atomic::AtomicU64;
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let id = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| PluginError::Message("Trigger instance identity exhausted"))?;
        Ok(NonZeroU64::new(id).expect("factory counter starts nonzero and never wraps"))
    }
    #[cfg(not(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu")))]
    Ok(NonZeroU64::new(1).unwrap())
}

#[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
impl Core {
    fn service_automation(&self) {
        let Ok(mut controller) = self.controller.try_borrow_mut() else {
            return;
        };
        let Ok(mut automation) = self.automation.try_borrow_mut() else {
            return;
        };
        automation.service(&mut controller);
    }
}
#[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
fn deliver_automation(delivery: &mut automation::Output, output: &mut OutputEvents) -> usize {
    use clack_plugin::events::{
        Pckn,
        event_types::{ParamGestureBeginEvent, ParamGestureEndEvent},
    };
    use dandrum_trigger_host::parameters::AutomationEvent;
    delivery.deliver(|event| match event {
        AutomationEvent::Begin { parameter_index } => output
            .try_push(ParamGestureBeginEvent::new(
                0,
                ClapId::new(parameter_index as u32 + 1),
            ))
            .is_ok(),
        AutomationEvent::Value {
            parameter_index,
            value,
        } => output
            .try_push(ParamValueEvent::new(
                0,
                ClapId::new(parameter_index as u32 + 1),
                Pckn::match_all(),
                value,
            ))
            .is_ok(),
        AutomationEvent::End { parameter_index } => output
            .try_push(ParamGestureEndEvent::new(
                0,
                ClapId::new(parameter_index as u32 + 1),
            ))
            .is_ok(),
    })
}

#[cfg(all(feature = "trigger-editor", target_os = "linux", target_env = "gnu"))]
impl Main<'_> {
    fn service_parameter_delivery(&self) {
        self.core.service_editor();
        let pending = self
            .core
            .automation
            .try_borrow()
            .is_ok_and(|mirror| mirror.pending());
        if pending && let Some(params) = self.host.get_extension::<HostParams>() {
            params.request_flush(&self.host.shared());
        }
    }
}
