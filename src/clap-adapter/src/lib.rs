//! Experimental headless CLAP rendering proof. State and editor support are separate gates.
use clack_extensions::{audio_ports::*, note_ports::*, params::*};
use clack_plugin::entry::{Entry, EntryFactories, EntryLoadError};
use clack_plugin::events::spaces::CoreEventSpace;
use clack_plugin::factory::plugin::{PluginFactoryImpl, PluginFactoryWrapper};
use clack_plugin::host::HostInfo;
use clack_plugin::plugin::PluginInstance;
use clack_plugin::prelude::*;
use dandrum_engine::graph::PortDirection;
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::{ChannelCount, document::load_kernel_patch_str};
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{HostBuses, prepare_kernel_graph_with_buses};
use dandrum_engine::PreparedSamplerAssets;
use std::ffi::CStr;
use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};

const TB303: &str = include_str!("../../../examples/patches/tb303-acid.yaml");
const GAIN: &str = include_str!("../fixtures/signed-gain.yaml");
const EVENT_LIMIT: u32 = 1024;
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Tb303,
    Gain,
}

impl Kind {
    fn document(self) -> &'static str {
        match self {
            Self::Tb303 => TB303,
            Self::Gain => GAIN,
        }
    }
}

struct Parameter {
    name: String,
    root: String,
    min: f64,
    max: f64,
    default: f64,
    value: AtomicU64,
}

struct Shared {
    kind: Kind,
    parameters: Vec<Parameter>,
    rejected_blocks: AtomicU64,
}
impl PluginShared<'_> for Shared {}
impl Shared {
    fn new(kind: Kind) -> Result<Self, PluginError> {
        let patch=load_kernel_patch_str(kind.document()).map_err(|_| PluginError::Message("Invalid built-in patch"))?;
        let parameters=patch.preset_surface().parameters().iter().map(|alias| {
            let default=alias.control_default();
            Parameter { name: alias.name().into(), root: alias.port_name().into(),
                min: default.min().unwrap_or(0.0), max: default.max().unwrap_or(1.0),
                default: default.default(), value: AtomicU64::new(default.default().to_bits()) }
        }).collect();
        Ok(Self { kind, parameters, rejected_blocks: AtomicU64::new(0) })
    }
    fn parameter(&self, id: ClapId) -> Option<&Parameter> {
        self.parameters.get(id.get().checked_sub(1)? as usize)
    }
    fn valid_value(&self,event: &clack_plugin::events::event_types::ParamValueEvent) -> bool {
        event.param_id().and_then(|id| self.parameter(id)).is_some_and(|p| event.pckn().matches_all() &&
            event.value().is_finite() && (p.min..=p.max).contains(&event.value()))
    }
    fn flush(&self,input: &InputEvents) {
        // Bounded, global parameter-only synchronization; persistence is not implemented here.
        if input.len()>EVENT_LIMIT || input.iter().any(|e| matches!(e.as_core_event(),
            Some(CoreEventSpace::ParamValue(v)) if !self.valid_value(v))) { return; }
        for event in input {
            if let Some(CoreEventSpace::ParamValue(v))=event.as_core_event() {
                if let Some(p)=v.param_id().and_then(|id| self.parameter(id)) { p.value.store(v.value().to_bits(),Ordering::Relaxed); }
            }
        }
    }
}
struct Main<'a> {
    shared: &'a Shared,
}
impl<'a> PluginMainThread<'a,Shared> for Main<'a> {}
struct Adapter;
impl Plugin for Adapter {
    type Shared<'a> = Shared;
    type MainThread<'a> = Main<'a>;
    type AudioProcessor<'a> = Processor<'a>;
    fn declare_extensions(builder: &mut PluginExtensions<Self>,_: Option<&Shared>) {
        builder.register::<PluginAudioPorts>().register::<PluginNotePorts>().register::<PluginParams>();
    }
}
struct Factory {
    descriptors: [PluginDescriptor; 2],
}
impl PluginFactoryImpl for Factory {
    fn plugin_count(&self) -> u32 {
        2
    }

    fn plugin_descriptor(&self, index: u32) -> Option<&PluginDescriptor> {
        self.descriptors.get(index as usize)
    }
    fn create_plugin<'a>(&'a self,host:HostInfo<'a>,id:&CStr)->Option<PluginInstance<'a>> {
        let index=self.descriptors.iter().position(|d|d.id()==Some(id))?;
        let kind=if index==0 { Kind::Tb303 } else { Kind::Gain };
        Some(PluginInstance::new::<Adapter>(host,&self.descriptors[index],
            move |_|Shared::new(kind), |_,shared|Ok(Main { shared })))
    }
}
struct ProofEntry {
    factory: PluginFactoryWrapper<Factory>,
}
impl Entry for ProofEntry {
    fn new(_:Option<&CStr>)->Result<Self,EntryLoadError> {
        use clack_plugin::plugin::features::*;
        Ok(Self { factory: PluginFactoryWrapper::new(Factory { descriptors: [
            PluginDescriptor::new("org.dandrum.experimental.tb303","Dandrum Experimental TB303")
                .with_vendor("Dandrum").with_version("0.1.0").with_features([INSTRUMENT,SYNTHESIZER,STEREO]),
            PluginDescriptor::new("org.dandrum.experimental.signed-gain","Dandrum Experimental Signed Gain")
                .with_vendor("Dandrum").with_version("0.1.0").with_features([AUDIO_EFFECT,STEREO])
        ]})})
    }
    fn declare_factories<'a>(&'a self, builder: &mut EntryFactories<'a>) {
        builder.register_factory(&self.factory);
    }
}
clack_plugin::clack_export_entry!(ProofEntry);

impl PluginAudioPortsImpl for Main<'_> {
    fn count(&self,input:bool)->u32 { u32::from(!input || self.shared.kind==Kind::Gain) }
    fn get(&self,index:u32,input:bool,writer:&mut AudioPortInfoWriter) {
        if index==0 && PluginAudioPortsImpl::count(self,input)>0 {
            writer.set(&AudioPortInfo { id:ClapId::new(0),name:b"main",channel_count:2,
                flags:AudioPortFlags::IS_MAIN,port_type:Some(AudioPortType::STEREO),in_place_pair:None });
        }
    }
}
impl PluginNotePortsImpl for Main<'_> {
    fn count(&self,input:bool)->u32 { u32::from(input && self.shared.kind==Kind::Tb303) }
    fn get(&self,index:u32,input:bool,writer:&mut NotePortInfoWriter) {
        if index==0 && PluginNotePortsImpl::count(self,input)>0 {
            writer.set(&NotePortInfo { id:ClapId::new(0),name:b"MIDI 1 notes",supported_dialects:NoteDialects::MIDI,
                preferred_dialect:Some(NoteDialect::Midi) });
        }
    }
}
impl PluginMainThreadParams for Main<'_> {
    fn count(&self)->u32 { self.shared.parameters.len() as u32 }
    fn get_info(&self,index:u32,writer:&mut ParamInfoWriter) {
        if let Some(p)=self.shared.parameters.get(index as usize) { writer.set(&ParamInfo {
            id:ClapId::new(index+1), flags:ParamInfoFlags::IS_AUTOMATABLE, cookie:Default::default(),
            name:p.name.as_bytes(),module:b"",min_value:p.min,max_value:p.max,default_value:p.default }); }
    }
    fn get_value(&self,id:ClapId)->Option<f64> { Some(f64::from_bits(self.shared.parameter(id)?.value.load(Ordering::Relaxed))) }
    fn value_to_text(&self,id:ClapId,value:f64,writer:&mut ParamDisplayWriter)->std::fmt::Result {
        if self.shared.parameter(id).is_none() { return Err(std::fmt::Error); }
        write!(writer,"{value:.4}")
    }
    fn text_to_value(&self,id:ClapId,text:&CStr)->Option<f64> {
        let p=self.shared.parameter(id)?; let value:f64=text.to_str().ok()?.parse().ok()?;
        (value.is_finite() && (p.min..=p.max).contains(&value)).then_some(value)
    }
    fn flush(&self,input:&InputEvents,_:&mut OutputEvents) { self.shared.flush(input); }
}
struct Processor<'a> {
    shared: &'a Shared,
    runtime: RealtimeGraphProcessor,
    maximum: usize,
    inputs: Vec<Vec<Vec<f32>>>,
    outputs: Vec<Vec<Vec<f32>>>,
    parameter_inputs: Vec<usize>,
    host_input: [Vec<f32>; 2],
    audio_input: Option<usize>,
}
impl PluginAudioProcessorParams for Processor<'_> {
    fn flush(&mut self,input:&InputEvents,_:&mut OutputEvents) { self.shared.flush(input); }
}
impl<'a> PluginAudioProcessor<'a,Shared,Main<'a>> for Processor<'a> {
    fn activate(_:HostAudioProcessorHandle<'a>,_:&Main<'a>,shared:&'a Shared,
        config:PluginAudioConfiguration)->Result<Self,PluginError> {
        if !config.sample_rate.is_finite() || !(1.0..=f64::from(u32::MAX)).contains(&config.sample_rate) ||
            config.min_frames_count==0 ||
            config.max_frames_count<config.min_frames_count || config.max_frames_count>65536 {
            return Err(PluginError::Message("Unsupported experimental audio configuration"));
        }
        let maximum=config.max_frames_count as usize;
        let patch=load_kernel_patch_str(shared.kind.document()).map_err(|_|PluginError::Message("Invalid built-in patch"))?;
        // These resource-free proof graphs use integer preparation metadata;
        // the runtime below receives the actual host rate in the engine's f32 format.
        let settings=RenderSettings { sample_rate_hz:config.sample_rate.round() as u32,block_size_frames:config.max_frames_count,duration_frames:maximum as u64 };
        let mut buses=HostBuses::new();
        for p in patch.root().ports() {
            let ChannelCount::Literal(n)=p.channels() else { return Err(PluginError::Message("Nonliteral root channels")); };
            buses=match p.direction() { PortDirection::Input=>buses.with_input(p.name(),*n as usize),
                PortDirection::Output=>buses.with_output(p.name(),2) };
        }
        let prepared=prepare_kernel_graph_with_buses(patch.root(),patch.registry(),&settings,&buses)
            .map_err(|_|PluginError::Message("Built-in preparation failed"))?;
        let metadata=prepared.root_port_metadata();
        let names:Vec<_>=metadata.iter().filter(|p|p.direction()==PortDirection::Input).map(|p|p.name()).collect();
        let parameter_inputs=shared.parameters.iter().map(|p|names.iter().position(|n|*n==p.root))
            .collect::<Option<Vec<_>>>().ok_or(PluginError::Message("Missing public root control"))?;
        let inputs=metadata.iter().filter(|p|p.direction()==PortDirection::Input).map(|p| {
            let ChannelCount::Literal(n)=p.channels() else { unreachable!("validated authored roots") };
            vec![vec![0.0;maximum];*n as usize]
        }).collect();
        let audio_input=if shared.kind==Kind::Gain { names.iter().position(|n|*n=="input") } else { None };
        let runtime=RealtimeGraphProcessor::from_compiled_patch(prepared.compiled_patch().clone(),
            config.sample_rate as f32,&PreparedSamplerAssets::empty(),maximum);
        Ok(Self { shared,runtime,maximum,inputs,outputs:vec![vec![vec![0.0;maximum];2]],
            parameter_inputs,host_input:[vec![0.0;maximum],vec![0.0;maximum]],audio_input })
    }
    fn reset(&mut self) {
        self.runtime.reset();
    }
    fn process(&mut self,_:Process,mut audio:Audio,events:Events)->Result<ProcessStatus,PluginError> {
        let frames=audio.frames_count() as usize;
        let event_count=events.input.len();
        let mut valid=frames>0 && frames<=self.maximum && audio.output_port_count()==1 &&
            audio.input_port_count()==usize::from(self.shared.kind==Kind::Gain) && event_count<=EVENT_LIMIT;
        let mut previous=0;
        let mut note_batch_events=0_usize;
        let count_to_validate=if event_count<=EVENT_LIMIT { event_count } else { 0 };
        for index in 0..count_to_validate {
            let Some(e) = events.input.get(index) else {
                valid = false;
                continue;
            };
            let time=e.header().time();
            valid &= time>=previous && (time as usize)<frames;
            if time != previous {
                note_batch_events = 0;
            }
            previous=time;
            match e.as_core_event() {
                Some(CoreEventSpace::ParamValue(v))=>valid &= self.shared.valid_value(v),
                Some(CoreEventSpace::Midi(m))=>{
                    valid &= self.shared.kind==Kind::Tb303 && m.port_index()==0 &&
                        m.data()[0]>=0x80 && m.data()[1]<128 && m.data()[2]<128;
                    if matches!(m.data()[0] & 0xf0, 0x80 | 0x90) {
                        note_batch_events += 1;
                        valid &= note_batch_events <= self.runtime.pending_event_capacity();
                    }
                },
                _=>{}
            }
        }
        if self.shared.kind==Kind::Gain {
            if let Some(port)=audio.input_port(0) {
                if let Ok(SampleType::F32(channels) | SampleType::Both(channels, _))=port.channels() {
                    valid &= channels.channel_count()==2;
                    for (target,source) in self.host_input.iter_mut().zip(channels.iter()) {
                        if frames<=target.len() && source.len()>=frames {
                            valid &= source[..frames].iter().all(|v|v.is_finite());
                            target[..frames].copy_from_slice(&source[..frames]);
                        } else { valid=false; }
                    }
                } else { valid=false; }
            } else { valid=false; }
        }
        if let Some(mut port)=audio.output_port(0) {
            match port.channels() {
                Ok(SampleType::F32(mut channels) | SampleType::Both(mut channels, _))=>{
                    valid &= channels.channel_count()==2;
                    if valid {
                        let mut buffers=[None,None];
                        for (destination,source) in buffers.iter_mut().zip(channels.iter_mut()) { *destination=Some(source); }
                        for batch in events.input.batch() {
                            let start=batch.first_sample(); let end=batch.next_batch_first_sample().unwrap_or(frames);
                            for e in batch.events() {
                                match e.as_core_event() {
                                    Some(CoreEventSpace::ParamValue(v))=>{
                                        if let Some(p)=v.param_id().and_then(|id| self.shared.parameter(id)) { p.value.store(v.value().to_bits(),Ordering::Relaxed); }
                                    }
                                    Some(CoreEventSpace::Midi(m))=>{
                                        let [status,note,velocity]=m.data();
                                        let accepted=match status & 0xf0 {
                                            0x90 if velocity>0=>self.runtime.try_note_on_at(note,velocity,0),
                                            0x80|0x90=>self.runtime.try_note_off_at(note,0), _=>true };
                                        if !accepted { valid=false; }
                                    }
                                    _=>{}
                                }
                            }
                            let length=end-start;
                            for bus in &mut self.inputs { for plane in bus { plane.resize(length,0.0); } }
                            for (p,&index) in self.shared.parameters.iter().zip(&self.parameter_inputs) {
                                self.inputs[index][0].fill(f64::from_bits(p.value.load(Ordering::Relaxed)) as f32);
                            }
                            if let Some(index)=self.audio_input { for (plane,source) in self.inputs[index].iter_mut().zip(&self.host_input) {
                                plane.copy_from_slice(&source[start..end]);
                            }}
                            for plane in &mut self.outputs[0] { plane.resize(length,0.0); }
                            if self.runtime.render_root_buses(&self.inputs, &mut self.outputs) != length {
                                valid = false;
                            }
                            for (destination,source) in buffers.iter_mut().zip(&self.outputs[0]) {
                                if let Some(destination)=destination { destination[start..end].copy_from_slice(source); }
                            }
                        }
                        if !valid {
                            for buffer in buffers.iter_mut().flatten() {
                                buffer.fill(0.0);
                            }
                        }
                    } else { for buffer in channels.iter_mut() { buffer.fill(0.0); } }
                }
                Ok(SampleType::F64(mut channels))=>{ valid=false; for buffer in channels.iter_mut() { buffer.fill(0.0); } }
                Err(_)=>{ valid=false; }
            }
        } else { valid=false; }
        if !valid {
            // Never log, allocate a diagnostic string, or return an SDK-logged PluginError here.
            self.shared.rejected_blocks.fetch_add(1,Ordering::Relaxed);
            return Ok(ProcessStatus::Continue);
        }
        Ok(ProcessStatus::Continue)
    }
}
