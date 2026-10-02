use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::path::PathBuf;
use std::sync::Arc;

use crate::preparation;
use crate::realtime;

use crate::graph::{PortDirection, SignalType};
use crate::graph_processor::RealtimeGraphProcessor;
use crate::kernel::{ChannelCount, PortMetadata};
use crate::patch::RenderSettings;
use crate::sample::{LoadedSample, PreparedSamplerAssets};

macro_rules! mut_or {
    ($ptr:expr, $binding:ident, $ret:expr) => {
        let Some($binding) = (unsafe { $ptr.as_mut() }) else {
            return $ret;
        };
    };
}

macro_rules! ref_or {
    ($ptr:expr, $binding:ident, $ret:expr) => {
        let Some($binding) = (unsafe { $ptr.as_ref() }) else {
            return $ret;
        };
    };
}

pub struct DandrumRealtimeEventQueue {
    queue: realtime::RealtimeEventQueue,
}

/// The caller owns every name pointer for the duration of preparation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DandrumKernelBusDeclaration {
    pub name: *const c_char,
    pub direction: u32,
    pub channel_count: usize,
}

/// Input channel pointers remain caller-owned and are read only during render.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DandrumKernelInputBusView {
    pub name: *const c_char,
    pub channels: *const *const f32,
    pub channel_count: usize,
    pub frame_capacity: usize,
    pub bus_index: usize,
}

/// Output channel pointers remain caller-owned and are written only on success.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DandrumKernelOutputBusView {
    pub name: *const c_char,
    pub channels: *const *mut f32,
    pub channel_count: usize,
    pub frame_capacity: usize,
    pub bus_index: usize,
}

struct PreparedInputBusBinding {
    root_index: Option<usize>,
    channel_count: usize,
}

struct PreparedOutputBusBinding {
    root_index: usize,
    channel_count: usize,
}

pub struct DandrumKernelInstrument {
    prepared: preparation::PreparedKernelInstrument,
    runtime: RealtimeGraphProcessor,
    ports: Vec<PortMetadata>,
    input_bus_bindings: Vec<PreparedInputBusBinding>,
    output_bus_bindings: Vec<PreparedOutputBusBinding>,
    input_scratch: Vec<Vec<Vec<f32>>>,
    output_scratch: Vec<Vec<Vec<f32>>>,
    public_controls: Vec<KernelPublicControl>,
    ui_control_groups: Vec<Option<u8>>,
    max_block_size: usize,
}

/// Owns only copied preparation metadata. It deliberately excludes decoded PCM
/// so readers can outlive an engine replacement without retaining audio assets.
pub struct DandrumKernelUiSnapshot {
    sources: Vec<KernelUiSource>,
    maps: Vec<KernelUiMap>,
    public_control_groups: Vec<Option<u8>>,
}

/// Retains one prepared source independently of an instrument replacement.
/// Create, reduce and destroy only away from the audio callback.
pub struct DandrumKernelWaveformSource {
    source_id: String,
    sample: Arc<LoadedSample>,
}

struct KernelUiSource {
    declaration: crate::kernel::document::SampleSource,
    sample_rate_hz: u32,
    channel_count: u16,
    frame_count: u64,
}

struct KernelUiMap {
    declaration: crate::kernel::document::SampleMap,
    zones: Vec<KernelUiZone>,
}

struct KernelUiZone {
    declaration: crate::kernel::document::SampleZone,
    effective_region: crate::kernel::document::SampleRegion,
    source_index: usize,
    region_index: usize,
}

/// UTF-8 bytes owned by the returned snapshot or waveform source handle.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DandrumKernelStringView {
    pub data: *const c_char,
    pub size: usize,
}

#[repr(C)]
pub struct DandrumKernelUiSource {
    pub id: DandrumKernelStringView,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub frame_count: u64,
}

#[repr(C)]
pub struct DandrumKernelWaveformSourceInfo {
    pub source_id: DandrumKernelStringView,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub frame_count: u64,
    pub content_revision: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DandrumKernelWaveformBucket {
    pub start_frame: u64,
    pub end_frame: u64,
    pub minimum: f32,
    pub maximum: f32,
}

#[repr(C)]
pub struct DandrumKernelUiRegion {
    pub id: DandrumKernelStringView,
    pub start_frame: u64,
    pub end_frame: u64,
    pub root_note: i32,
    pub has_gain_db: bool,
    pub gain_db: f64,
    pub has_pan: bool,
    pub pan: f64,
    pub reverse: bool,
    pub fade_in_ms: f64,
    pub fade_out_ms: f64,
    pub has_loop: bool,
    pub loop_mode: DandrumKernelStringView,
    pub loop_start_frame: u64,
    pub loop_end_frame: u64,
    pub loop_crossfade_ms: f64,
}

#[repr(C)]
pub struct DandrumKernelUiSlice {
    pub id: DandrumKernelStringView,
    pub start_frame: u64,
    pub end_frame: u64,
}

#[repr(C)]
pub struct DandrumKernelUiMap {
    pub id: DandrumKernelStringView,
    pub selection_mode: DandrumKernelStringView,
    pub selection_seed: u64,
}

#[repr(C)]
pub struct DandrumKernelUiZone {
    pub id: DandrumKernelStringView,
    pub source_index: usize,
    pub region_index: usize,
    pub start_frame: u64,
    pub end_frame: u64,
    pub key_low: u8,
    pub key_high: u8,
    pub velocity_low: u8,
    pub velocity_high: u8,
    pub round_robin_group: DandrumKernelStringView,
    pub choke_group: DandrumKernelStringView,
    pub control_group: i32,
    pub weight: u32,
    pub has_gain_db: bool,
    pub gain_db: f64,
    pub has_pan: bool,
    pub pan: f64,
    pub has_pitch_semitones: bool,
    pub pitch_semitones: f64,
}

fn ui_string_view(value: &str) -> DandrumKernelStringView {
    DandrumKernelStringView {
        data: value.as_ptr().cast(),
        size: value.len(),
    }
}

fn sample_control_group(root: &crate::kernel::GraphDefinition, port_name: &str) -> Option<u8> {
    let port = root.ports().iter().find(|port| port.name() == port_name)?;
    let [target] = port.internal_targets() else {
        return None;
    };
    let node = root
        .nodes()
        .iter()
        .find(|node| node.id() == target.node())?;
    if node.definition_ref() != "sample_map_player" {
        return None;
    }
    let (group, _) = target.port().strip_prefix("group_")?.split_once('_')?;
    group.parse::<u8>().ok().filter(|group| *group > 0)
}

struct KernelPublicControl {
    input_index: Option<usize>,
    value: f32,
    min: Option<f64>,
    max: Option<f64>,
}

unsafe fn ffi_name<'a>(pointer: &'a *const c_char) -> Option<&'a str> {
    if pointer.is_null() {
        return None;
    }
    // The returned borrow is used only within the current FFI call. It is
    // never stored; the caller must keep the C string valid for that call.
    unsafe { CStr::from_ptr(*pointer) }.to_str().ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_prepare_file(
    path: *const c_char,
    sample_rate_hz: u32,
    max_block_size: usize,
    declarations: *const DandrumKernelBusDeclaration,
    declaration_count: usize,
) -> *mut DandrumKernelInstrument {
    if sample_rate_hz == 0
        || max_block_size == 0
        || (declaration_count > 0 && declarations.is_null())
    {
        return std::ptr::null_mut();
    }
    let Some(path) = c_path(path) else {
        return std::ptr::null_mut();
    };
    let Ok(patch) = crate::kernel::document::load_kernel_patch_file(&path) else {
        return std::ptr::null_mut();
    };
    let declared = if declaration_count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(declarations, declaration_count) }
    };
    let mut buses = preparation::HostBuses::new();
    let mut declared_inputs = BTreeMap::new();
    let mut declared_outputs = BTreeMap::new();
    let mut declared_input_order = Vec::new();
    let mut declared_output_order = Vec::new();
    for declaration in declared {
        let Some(name) = (unsafe { ffi_name(&declaration.name) }) else {
            return std::ptr::null_mut();
        };
        if name.is_empty() || declaration.channel_count == 0 {
            return std::ptr::null_mut();
        }
        match declaration.direction {
            1 if declared_inputs
                .insert(name.to_string(), declaration.channel_count)
                .is_none() =>
            {
                buses = buses.with_input(name, declaration.channel_count);
                declared_input_order.push((name.to_string(), declaration.channel_count));
            }
            2 if declared_outputs
                .insert(name.to_string(), declaration.channel_count)
                .is_none() =>
            {
                buses = buses.with_output(name, declaration.channel_count);
                declared_output_order.push((name.to_string(), declaration.channel_count));
            }
            _ => return std::ptr::null_mut(),
        }
    }
    for port in patch.root().ports() {
        if port.signal_type() != SignalType::Event {
            continue;
        }
        match port.direction() {
            PortDirection::Input if declared_inputs.contains_key(port.name()) => {
                return std::ptr::null_mut();
            }
            PortDirection::Output if declared_outputs.contains_key(port.name()) => {
                return std::ptr::null_mut();
            }
            PortDirection::Output => {}
            PortDirection::Input => {}
        }
    }
    let settings = RenderSettings {
        sample_rate_hz,
        block_size_frames: max_block_size as u32,
        duration_frames: max_block_size as u64,
    };
    if usize::try_from(settings.block_size_frames).ok() != Some(max_block_size) {
        return std::ptr::null_mut();
    }
    let references = crate::module_package::external_references(patch.root(), patch.registry());
    let mut context = preparation::PreparationContext::new(
        path.parent().unwrap_or_else(|| std::path::Path::new(".")),
        sample_rate_hz,
    );
    if !references.is_empty() {
        let Ok(roots) = crate::module_library::default_host_macro_roots() else {
            return std::ptr::null_mut();
        };
        context = context.with_macro_roots(roots);
    }
    let Ok(prepared) =
        preparation::prepare_kernel_graph_for_planar_ffi(&patch, &settings, &buses, &context)
    else {
        return std::ptr::null_mut();
    };
    let ports = prepared.root_port_metadata();
    let input_names = ports
        .iter()
        .filter(|port| port.direction() == PortDirection::Input)
        .map(|port| port.name().to_string())
        .collect::<Vec<_>>();
    let output_names = ports
        .iter()
        .filter(|port| port.direction() == PortDirection::Output)
        .map(|port| port.name().to_string())
        .collect::<Vec<_>>();
    let planar_output_names = ports
        .iter()
        .filter(|port| {
            port.direction() == PortDirection::Output && port.signal_type() != SignalType::Event
        })
        .map(|port| port.name().to_string())
        .collect::<Vec<_>>();
    let input_bus_bindings = declared_input_order
        .iter()
        .map(|(name, channel_count)| PreparedInputBusBinding {
            root_index: input_names.iter().position(|candidate| candidate == name),
            channel_count: *channel_count,
        })
        .collect::<Vec<_>>();
    let Some(output_bus_bindings) = declared_output_order
        .iter()
        .map(|(name, channel_count)| {
            planar_output_names
                .iter()
                .any(|candidate| candidate == name)
                .then(|| {
                    output_names
                        .iter()
                        .position(|candidate| candidate == name)
                        .map(|root_index| PreparedOutputBusBinding {
                            root_index,
                            channel_count: *channel_count,
                        })
                })
                .flatten()
        })
        .collect::<Option<Vec<_>>>()
    else {
        return std::ptr::null_mut();
    };
    let scratch = |direction| {
        ports
            .iter()
            .filter(|port| port.direction() == direction)
            .map(|port| {
                let ChannelCount::Literal(channels) = port.channels() else {
                    unreachable!("prepared root channels are resolved")
                };
                vec![vec![0.0; max_block_size]; *channels as usize]
            })
            .collect::<Vec<_>>()
    };
    let input_scratch = scratch(PortDirection::Input);
    let output_scratch = scratch(PortDirection::Output);
    let public_controls = patch
        .preset_surface()
        .parameters()
        .iter()
        .map(|alias| KernelPublicControl {
            input_index: input_names
                .iter()
                .position(|name| name == alias.port_name())
                .filter(|_| declared_inputs.contains_key(alias.port_name())),
            value: alias.control_default().default() as f32,
            min: alias.control_default().min(),
            max: alias.control_default().max(),
        })
        .collect();
    let ui_control_groups = patch
        .preset_surface()
        .parameters()
        .iter()
        .map(|alias| sample_control_group(patch.root(), alias.port_name()))
        .collect();
    let runtime = RealtimeGraphProcessor::from_compiled_patch(
        prepared.compiled_patch().clone(),
        sample_rate_hz as f32,
        &PreparedSamplerAssets::empty(),
        max_block_size,
    );
    if !runtime.can_render_root_buses() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(DandrumKernelInstrument {
        prepared,
        runtime,
        ports,
        input_bus_bindings,
        output_bus_bindings,
        input_scratch,
        output_scratch,
        public_controls,
        ui_control_groups,
        max_block_size,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_set_public_numeric_parameter_by_slot(
    engine: *mut DandrumKernelInstrument,
    slot_index: usize,
    value: f64,
) -> bool {
    mut_or!(engine, engine, false);
    let Some(control) = engine.public_controls.get_mut(slot_index) else {
        return false;
    };
    if control.input_index.is_none()
        || !value.is_finite()
        || control.min.is_some_and(|min| value < min)
        || control.max.is_some_and(|max| value > max)
        || !(value as f32).is_finite()
    {
        return false;
    }
    control.value = value as f32;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_destroy(engine: *mut DandrumKernelInstrument) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

/// Call away from the audio callback while the engine is protected against
/// replacement. The returned copy has no pointers into the engine or PCM.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_snapshot_create(
    engine: *const DandrumKernelInstrument,
) -> *mut DandrumKernelUiSnapshot {
    ref_or!(engine, engine, std::ptr::null_mut());
    let assets = engine.prepared.sample_assets();
    let sources = assets
        .sources()
        .iter()
        .map(|source| KernelUiSource {
            declaration: source.declaration().clone(),
            sample_rate_hz: source.sample().sample_rate_hz(),
            channel_count: source.sample().source_channel_count(),
            frame_count: source.sample().frame_count() as u64,
        })
        .collect();
    let maps = assets
        .maps()
        .iter()
        .map(|declaration| {
            let prepared = assets
                .map_by_id(&declaration.id)
                .expect("prepared map is indexed by its declaration ID");
            KernelUiMap {
                declaration: declaration.clone(),
                zones: prepared
                    .zones()
                    .iter()
                    .map(|zone| KernelUiZone {
                        declaration: zone.declaration().clone(),
                        effective_region: zone.effective_region().clone(),
                        source_index: zone.source_index(),
                        region_index: zone.region_index(),
                    })
                    .collect(),
            }
        })
        .collect();
    Box::into_raw(Box::new(DandrumKernelUiSnapshot {
        sources,
        maps,
        public_control_groups: engine.ui_control_groups.clone(),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_snapshot_destroy(
    snapshot: *mut DandrumKernelUiSnapshot,
) {
    if !snapshot.is_null() {
        drop(unsafe { Box::from_raw(snapshot) });
    }
}

/// The caller protects `engine` against replacement during this off-audio call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_waveform_source_create(
    engine: *const DandrumKernelInstrument,
    source_index: usize,
) -> *mut DandrumKernelWaveformSource {
    ref_or!(engine, engine, std::ptr::null_mut());
    let Some(source) = engine.prepared.sample_assets().sources().get(source_index) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(DandrumKernelWaveformSource {
        source_id: source.id().to_owned(),
        sample: source.retained_sample(),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_waveform_source_destroy(
    source: *mut DandrumKernelWaveformSource,
) {
    if !source.is_null() {
        drop(unsafe { Box::from_raw(source) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_waveform_source_info(
    source: *const DandrumKernelWaveformSource,
    output: *mut DandrumKernelWaveformSourceInfo,
) -> bool {
    ref_or!(source, source, false);
    mut_or!(output, output, false);
    *output = DandrumKernelWaveformSourceInfo {
        source_id: ui_string_view(&source.source_id),
        sample_rate_hz: source.sample.sample_rate_hz(),
        channel_count: source.sample.source_channel_count(),
        frame_count: source.sample.frame_count() as u64,
        content_revision: *source.sample.content_revision(),
    };
    true
}

/// `output` must point to `bucket_count` writable elements. Invalid requests
/// leave the caller buffer untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_waveform_reduce(
    source: *const DandrumKernelWaveformSource,
    channel: u16,
    start_frame: u64,
    end_frame: u64,
    output: *mut DandrumKernelWaveformBucket,
    bucket_count: usize,
) -> bool {
    ref_or!(source, source, false);
    if output.is_null() {
        return false;
    }
    let Some(buckets) =
        source
            .sample
            .reduce_waveform(channel, start_frame, end_frame, bucket_count)
    else {
        return false;
    };
    for (destination, bucket) in unsafe { std::slice::from_raw_parts_mut(output, bucket_count) }
        .iter_mut()
        .zip(buckets)
    {
        *destination = DandrumKernelWaveformBucket {
            start_frame: bucket.start_frame,
            end_frame: bucket.end_frame,
            minimum: bucket.minimum,
            maximum: bucket.maximum,
        };
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_source_count(
    snapshot: *const DandrumKernelUiSnapshot,
) -> usize {
    ref_or!(snapshot, snapshot, 0);
    snapshot.sources.len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_source(
    snapshot: *const DandrumKernelUiSnapshot,
    index: usize,
    output: *mut DandrumKernelUiSource,
) -> bool {
    ref_or!(snapshot, snapshot, false);
    mut_or!(output, output, false);
    let Some(source) = snapshot.sources.get(index) else {
        return false;
    };
    *output = DandrumKernelUiSource {
        id: ui_string_view(&source.declaration.id),
        sample_rate_hz: source.sample_rate_hz,
        channel_count: source.channel_count,
        frame_count: source.frame_count,
    };
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_region_count(
    snapshot: *const DandrumKernelUiSnapshot,
    source_index: usize,
) -> usize {
    ref_or!(snapshot, snapshot, 0);
    snapshot
        .sources
        .get(source_index)
        .map_or(0, |source| source.declaration.regions.len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_region(
    snapshot: *const DandrumKernelUiSnapshot,
    source_index: usize,
    region_index: usize,
    output: *mut DandrumKernelUiRegion,
) -> bool {
    ref_or!(snapshot, snapshot, false);
    mut_or!(output, output, false);
    let Some(region) = snapshot
        .sources
        .get(source_index)
        .and_then(|source| source.declaration.regions.get(region_index))
    else {
        return false;
    };
    let loop_settings = region.loop_settings.as_ref();
    *output = DandrumKernelUiRegion {
        id: ui_string_view(&region.id),
        start_frame: region.start_frame,
        end_frame: region.end_frame,
        root_note: region.root_note.map_or(-1, i32::from),
        has_gain_db: region.gain_db.is_some(),
        gain_db: region.gain_db.unwrap_or(0.0),
        has_pan: region.pan.is_some(),
        pan: region.pan.unwrap_or(0.0),
        reverse: region.reverse,
        fade_in_ms: region.fade_in_ms.unwrap_or(0.0),
        fade_out_ms: region.fade_out_ms.unwrap_or(0.0),
        has_loop: loop_settings.is_some(),
        loop_mode: ui_string_view(loop_settings.map_or("", |settings| &settings.mode)),
        loop_start_frame: loop_settings.map_or(0, |settings| settings.start_frame),
        loop_end_frame: loop_settings.map_or(0, |settings| settings.end_frame),
        loop_crossfade_ms: loop_settings
            .and_then(|settings| settings.crossfade_ms)
            .unwrap_or(0.0),
    };
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_slice_count(
    snapshot: *const DandrumKernelUiSnapshot,
    source_index: usize,
) -> usize {
    ref_or!(snapshot, snapshot, 0);
    snapshot
        .sources
        .get(source_index)
        .map_or(0, |source| source.declaration.slices.len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_slice(
    snapshot: *const DandrumKernelUiSnapshot,
    source_index: usize,
    slice_index: usize,
    output: *mut DandrumKernelUiSlice,
) -> bool {
    ref_or!(snapshot, snapshot, false);
    mut_or!(output, output, false);
    let Some(slice) = snapshot
        .sources
        .get(source_index)
        .and_then(|source| source.declaration.slices.get(slice_index))
    else {
        return false;
    };
    *output = DandrumKernelUiSlice {
        id: ui_string_view(&slice.id),
        start_frame: slice.start_frame,
        end_frame: slice.end_frame,
    };
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_map_count(
    snapshot: *const DandrumKernelUiSnapshot,
) -> usize {
    ref_or!(snapshot, snapshot, 0);
    snapshot.maps.len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_map(
    snapshot: *const DandrumKernelUiSnapshot,
    index: usize,
    output: *mut DandrumKernelUiMap,
) -> bool {
    ref_or!(snapshot, snapshot, false);
    mut_or!(output, output, false);
    let Some(map) = snapshot.maps.get(index) else {
        return false;
    };
    *output = DandrumKernelUiMap {
        id: ui_string_view(&map.declaration.id),
        selection_mode: ui_string_view(
            map.declaration
                .selection_mode
                .as_deref()
                .unwrap_or("first_match"),
        ),
        selection_seed: map.declaration.selection_seed,
    };
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_zone_count(
    snapshot: *const DandrumKernelUiSnapshot,
    map_index: usize,
) -> usize {
    ref_or!(snapshot, snapshot, 0);
    snapshot
        .maps
        .get(map_index)
        .map_or(0, |map| map.zones.len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_zone(
    snapshot: *const DandrumKernelUiSnapshot,
    map_index: usize,
    zone_index: usize,
    output: *mut DandrumKernelUiZone,
) -> bool {
    ref_or!(snapshot, snapshot, false);
    mut_or!(output, output, false);
    let Some(zone) = snapshot
        .maps
        .get(map_index)
        .and_then(|map| map.zones.get(zone_index))
    else {
        return false;
    };
    *output = DandrumKernelUiZone {
        id: ui_string_view(zone.declaration.id.as_deref().unwrap_or("")),
        source_index: zone.source_index,
        region_index: zone.region_index,
        start_frame: zone.effective_region.start_frame,
        end_frame: zone.effective_region.end_frame,
        key_low: zone.declaration.key_range[0],
        key_high: zone.declaration.key_range[1],
        velocity_low: zone.declaration.velocity_range[0],
        velocity_high: zone.declaration.velocity_range[1],
        round_robin_group: ui_string_view(
            zone.declaration.round_robin_group.as_deref().unwrap_or(""),
        ),
        choke_group: ui_string_view(zone.declaration.choke_group.as_deref().unwrap_or("")),
        control_group: zone.declaration.control_group.map_or(-1, i32::from),
        weight: zone.declaration.weight.unwrap_or(1),
        has_gain_db: zone.declaration.gain_db.is_some(),
        gain_db: zone.declaration.gain_db.unwrap_or(0.0),
        has_pan: zone.declaration.pan.is_some(),
        pan: zone.declaration.pan.unwrap_or(0.0),
        has_pitch_semitones: zone.declaration.pitch_semitones.is_some(),
        pitch_semitones: zone.declaration.pitch_semitones.unwrap_or(0.0),
    };
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_ui_public_control_group(
    snapshot: *const DandrumKernelUiSnapshot,
    index: usize,
    output: *mut i32,
) -> bool {
    ref_or!(snapshot, snapshot, false);
    mut_or!(output, output, false);
    let Some(group) = snapshot.public_control_groups.get(index) else {
        return false;
    };
    *output = group.map_or(0, i32::from);
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_root_port_count(
    engine: *const DandrumKernelInstrument,
) -> usize {
    ref_or!(engine, engine, 0);
    engine.ports.len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_root_port(
    engine: *const DandrumKernelInstrument,
    index: usize,
    name: *mut c_char,
    name_capacity: usize,
    direction: *mut u32,
    signal_type: *mut u32,
    channels: *mut usize,
) -> bool {
    ref_or!(engine, engine, false);
    let Some(port) = engine.ports.get(index) else {
        return false;
    };
    if direction.is_null()
        || signal_type.is_null()
        || channels.is_null()
        || name.is_null()
        || name_capacity <= port.name().len()
    {
        return false;
    }
    let ChannelCount::Literal(channel_count) = port.channels() else {
        return false;
    };
    let direction_code = match port.direction() {
        PortDirection::Input => 1,
        PortDirection::Output => 2,
    };
    let signal_code = match port.signal_type() {
        SignalType::Audio => 1,
        SignalType::Control => 2,
        SignalType::Event => 3,
    };
    unsafe {
        *direction = direction_code;
        *signal_type = signal_code;
        *channels = *channel_count as usize;
    }
    copy_string_to_c_buffer(port.name(), name, name_capacity)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_total_latency_samples(
    engine: *const DandrumKernelInstrument,
) -> u32 {
    ref_or!(engine, engine, 0);
    engine.prepared.total_latency_samples()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_note_on_at(
    engine: *mut DandrumKernelInstrument,
    note: u8,
    velocity: u8,
    frame_offset: usize,
) -> bool {
    mut_or!(engine, engine, false);
    if note > 127 || velocity > 127 || frame_offset >= engine.max_block_size {
        return false;
    }
    engine
        .runtime
        .try_note_on_at(note, velocity, frame_offset as u32)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_note_off_at(
    engine: *mut DandrumKernelInstrument,
    note: u8,
    frame_offset: usize,
) -> bool {
    mut_or!(engine, engine, false);
    if note > 127 || frame_offset >= engine.max_block_size {
        return false;
    }
    engine.runtime.try_note_off_at(note, frame_offset as u32)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_reset(engine: *mut DandrumKernelInstrument) -> bool {
    mut_or!(engine, engine, false);
    engine.runtime.reset();
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_kernel_render(
    engine: *mut DandrumKernelInstrument,
    inputs: *const DandrumKernelInputBusView,
    input_count: usize,
    outputs: *const DandrumKernelOutputBusView,
    output_count: usize,
    frames: usize,
) -> usize {
    mut_or!(engine, engine, 0);
    if frames == 0
        || frames > engine.max_block_size
        || (input_count > 0 && inputs.is_null())
        || (output_count > 0 && outputs.is_null())
        || engine.output_bus_bindings.is_empty()
        || output_count != engine.output_bus_bindings.len()
    {
        return 0;
    }
    let input_views = if input_count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(inputs, input_count) }
    };
    let output_views = if output_count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(outputs, output_count) }
    };

    // Validate every pointer and shape before reading inputs or writing outputs.
    for (index, view) in input_views.iter().enumerate() {
        let Some(binding) = engine.input_bus_bindings.get(view.bus_index) else {
            return 0;
        };
        if binding.channel_count != view.channel_count
            || view.frame_capacity < frames
            || view.channels.is_null()
            || input_views[..index]
                .iter()
                .any(|prior| prior.bus_index == view.bus_index)
        {
            return 0;
        }
        let channels = unsafe { std::slice::from_raw_parts(view.channels, view.channel_count) };
        if channels.iter().any(|pointer| pointer.is_null()) {
            return 0;
        }
    }
    for (index, view) in output_views.iter().enumerate() {
        let Some(binding) = engine.output_bus_bindings.get(view.bus_index) else {
            return 0;
        };
        if binding.channel_count != view.channel_count
            || view.frame_capacity < frames
            || view.channels.is_null()
            || output_views[..index]
                .iter()
                .any(|prior| prior.bus_index == view.bus_index)
        {
            return 0;
        }
        let channels = unsafe { std::slice::from_raw_parts(view.channels, view.channel_count) };
        if channels.iter().any(|pointer| pointer.is_null()) {
            return 0;
        }
    }
    for bus in engine.input_scratch.iter_mut() {
        for channel in bus.iter_mut() {
            channel.resize(frames, 0.0);
            channel.fill(0.0);
        }
    }
    for control in &engine.public_controls {
        if let Some(index) = control.input_index {
            for channel in &mut engine.input_scratch[index] {
                channel.fill(control.value);
            }
        }
    }
    for bus in engine.output_scratch.iter_mut() {
        for channel in bus.iter_mut() {
            channel.resize(frames, 0.0);
        }
    }
    for view in input_views {
        let Some(index) = engine.input_bus_bindings[view.bus_index].root_index else {
            continue; // A declared host input without a root port is ignored.
        };
        let channels = unsafe { std::slice::from_raw_parts(view.channels, view.channel_count) };
        for (destination, source) in engine.input_scratch[index].iter_mut().zip(channels) {
            destination.copy_from_slice(unsafe { std::slice::from_raw_parts(*source, frames) });
        }
    }
    if engine
        .runtime
        .render_root_buses(&engine.input_scratch, &mut engine.output_scratch)
        != frames
    {
        return 0;
    }
    for view in output_views {
        let index = engine.output_bus_bindings[view.bus_index].root_index;
        let channels = unsafe { std::slice::from_raw_parts(view.channels, view.channel_count) };
        for (source, destination) in engine.output_scratch[index].iter().zip(channels) {
            unsafe { std::ptr::copy_nonoverlapping(source.as_ptr(), *destination, frames) };
        }
    }
    frames
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_patch_public_numeric_parameter_count(
    path: *const c_char,
) -> usize {
    let Some(path) = c_path(path) else {
        return 0;
    };
    let Ok(patch) = crate::kernel::document::load_kernel_patch_file(&path) else {
        return 0;
    };
    patch.preset_surface().parameters().len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_patch_public_numeric_parameter_port_name(
    path: *const c_char,
    index: usize,
    buffer: *mut c_char,
    capacity: usize,
) -> bool {
    let Some(path) = c_path(path) else {
        return false;
    };
    let Ok(patch) = crate::kernel::document::load_kernel_patch_file(&path) else {
        return false;
    };
    let Some(alias) = patch.preset_surface().parameters().get(index) else {
        return false;
    };
    copy_string_to_c_buffer(alias.port_name(), buffer, capacity)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_patch_public_numeric_parameter_descriptor(
    path: *const c_char,
    index: usize,
    id_buffer: *mut c_char,
    id_buffer_capacity: usize,
    name_buffer: *mut c_char,
    name_buffer_capacity: usize,
    default_value: *mut f64,
    min_value: *mut f64,
    max_value: *mut f64,
) -> bool {
    let Some(path) = c_path(path) else {
        return false;
    };
    let Ok(patch) = crate::kernel::document::load_kernel_patch_file(&path) else {
        return false;
    };
    let Some(target) = patch.preset_surface().parameters().get(index) else {
        return false;
    };
    if default_value.is_null() || min_value.is_null() || max_value.is_null() {
        return false;
    }
    let control = target.control_default();
    unsafe {
        *default_value = control.default();
        *min_value = control.min().unwrap_or(0.0);
        *max_value = control.max().unwrap_or(1.0);
    }
    copy_string_to_c_buffer(target.name(), id_buffer, id_buffer_capacity)
        && copy_string_to_c_buffer(target.name(), name_buffer, name_buffer_capacity)
}

#[unsafe(no_mangle)]
pub extern "C" fn dandrum_realtime_event_queue_create(
    capacity: usize,
) -> *mut DandrumRealtimeEventQueue {
    Box::into_raw(Box::new(DandrumRealtimeEventQueue {
        queue: realtime::RealtimeEventQueue::with_capacity(capacity),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_realtime_event_queue_destroy(
    queue: *mut DandrumRealtimeEventQueue,
) {
    if !queue.is_null() {
        drop(unsafe { Box::from_raw(queue) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_realtime_event_queue_note_on(
    queue: *mut DandrumRealtimeEventQueue,
    note: u8,
    velocity: u8,
) -> u8 {
    submit_realtime_queue_event(queue, realtime::RealtimeEvent::NoteOn { note, velocity })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_realtime_event_queue_note_off(
    queue: *mut DandrumRealtimeEventQueue,
    note: u8,
) -> u8 {
    submit_realtime_queue_event(queue, realtime::RealtimeEvent::NoteOff { note })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_realtime_event_queue_dropped_count(
    queue: *const DandrumRealtimeEventQueue,
) -> usize {
    ref_or!(queue, queue, 0);

    queue.queue.dropped_events()
}

fn submit_realtime_queue_event(
    queue: *mut DandrumRealtimeEventQueue,
    event: realtime::RealtimeEvent,
) -> u8 {
    mut_or!(queue, queue, 1);

    match queue.queue.submit(event) {
        realtime::RealtimeEventSubmitStatus::Accepted => 0,
        realtime::RealtimeEventSubmitStatus::Dropped => 1,
    }
}

fn c_path(path: *const c_char) -> Option<PathBuf> {
    c_string(path).map(PathBuf::from)
}

fn c_string<'a>(value: *const c_char) -> Option<&'a str> {
    if value.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(value) }.to_str().ok()
}

fn copy_string_to_c_buffer(value: &str, buffer: *mut c_char, capacity: usize) -> bool {
    if buffer.is_null() || capacity == 0 {
        return false;
    }

    let bytes = value.as_bytes();
    let copied = bytes.len().min(capacity - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.cast::<u8>(), copied);
        *buffer.add(copied) = 0;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel_ffi_patch() -> (tempfile::TempDir, std::ffi::CString) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kernel-ffi.yaml");
        std::fs::write(
            &path,
            "metadata: { name: kernel-ffi }\nports:\n  - { name: input, direction: input, signal: audio, channels: 2, maps_to: amp.audio_in }\n  - { name: master, direction: output, signal: audio, channels: 2, maps_from: amp.audio_out }\nmodules:\n  - { id: amp, type: gain, static: { channels: 2 } }\nconnections: []\n",
        )
        .unwrap();
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        (dir, c_path)
    }

    #[test]
    fn kernel_ffi_enumerates_prepared_ports_and_renders_named_planar_buses() {
        let (_dir, path) = kernel_ffi_patch();
        let input_name = std::ffi::CString::new("input").unwrap();
        let output_name = std::ffi::CString::new("master").unwrap();
        let declarations = [
            DandrumKernelBusDeclaration {
                name: input_name.as_ptr(),
                direction: 1,
                channel_count: 2,
            },
            DandrumKernelBusDeclaration {
                name: output_name.as_ptr(),
                direction: 2,
                channel_count: 2,
            },
        ];
        let engine = unsafe {
            dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, declarations.as_ptr(), 2)
        };
        assert!(!engine.is_null());
        assert_eq!(unsafe { dandrum_kernel_root_port_count(engine) }, 2);
        assert_eq!(unsafe { dandrum_kernel_total_latency_samples(engine) }, 0);
        let mut name = [0_i8; 32];
        let mut direction = 0_u32;
        let mut signal = 0_u32;
        let mut channels = 0_usize;
        assert!(unsafe {
            dandrum_kernel_root_port(
                engine,
                1,
                name.as_mut_ptr(),
                name.len(),
                &mut direction,
                &mut signal,
                &mut channels,
            )
        });
        assert_eq!(
            unsafe { CStr::from_ptr(name.as_ptr()) }.to_str().unwrap(),
            "master"
        );
        assert_eq!((direction, signal, channels), (2, 1, 2));
        assert!(unsafe {
            dandrum_kernel_root_port(
                engine,
                0,
                name.as_mut_ptr(),
                name.len(),
                &mut direction,
                &mut signal,
                &mut channels,
            )
        });
        assert_eq!(
            unsafe { CStr::from_ptr(name.as_ptr()) }.to_str().unwrap(),
            "input"
        );
        assert_eq!((direction, signal, channels), (1, 1, 2));

        let left = [-0.5_f32; 8];
        let right = [0.25_f32; 8];
        let sources = [left.as_ptr(), right.as_ptr()];
        let inputs = [DandrumKernelInputBusView {
            name: input_name.as_ptr(),
            channels: sources.as_ptr(),
            channel_count: 2,
            frame_capacity: 8,
            bus_index: 0,
        }];
        let mut out_left = [0.0_f32; 8];
        let mut out_right = [0.0_f32; 8];
        let destinations = [out_left.as_mut_ptr(), out_right.as_mut_ptr()];
        let outputs = [DandrumKernelOutputBusView {
            name: output_name.as_ptr(),
            channels: destinations.as_ptr(),
            channel_count: 2,
            frame_capacity: 8,
            bus_index: 0,
        }];
        let allocations = crate::test_allocator::count_current_thread_allocations(|| {
            assert_eq!(
                unsafe {
                    dandrum_kernel_render(engine, inputs.as_ptr(), 1, outputs.as_ptr(), 1, 8)
                },
                8
            );
        });
        assert_eq!(allocations, 0);
        assert_eq!(out_left, left);
        assert_eq!(out_right, right);

        // New host buffers on a later call prove the engine kept no old pointers.
        let next_left = [0.75_f32; 8];
        let next_right = [-0.25_f32; 8];
        let next_sources = [next_left.as_ptr(), next_right.as_ptr()];
        let next_inputs = [DandrumKernelInputBusView {
            channels: next_sources.as_ptr(),
            ..inputs[0]
        }];
        assert_eq!(
            unsafe {
                dandrum_kernel_render(engine, next_inputs.as_ptr(), 1, outputs.as_ptr(), 1, 8)
            },
            8
        );
        assert_eq!(out_left, next_left);
        assert_eq!(out_right, next_right);
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_reports_nonzero_resolved_root_latency_to_host() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("latency.yaml");
        std::fs::write(
            &path,
            "metadata: { name: latency }\nports:\n  - { name: input, direction: input, signal: audio, channels: 1, maps_to: wet.audio_in }\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: wet.audio_out }\nmodules:\n  - { id: wet, type: spectral_processor, static: { fft_size: 512, mode: passthrough } }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let input = std::ffi::CString::new("input").unwrap();
        let master = std::ffi::CString::new("master").unwrap();
        let buses = [
            DandrumKernelBusDeclaration {
                name: input.as_ptr(),
                direction: 1,
                channel_count: 1,
            },
            DandrumKernelBusDeclaration {
                name: master.as_ptr(),
                direction: 2,
                channel_count: 1,
            },
        ];

        let engine =
            unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 64, buses.as_ptr(), 2) };
        assert!(!engine.is_null());
        assert_eq!(unsafe { dandrum_kernel_total_latency_samples(engine) }, 511);
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn same_patch_prepares_and_renders_at_two_host_sample_rates() {
        let (_directory, path) = kernel_ffi_patch();
        let input_name = std::ffi::CString::new("input").unwrap();
        let output_name = std::ffi::CString::new("master").unwrap();
        let buses = [
            DandrumKernelBusDeclaration {
                name: input_name.as_ptr(),
                direction: 1,
                channel_count: 2,
            },
            DandrumKernelBusDeclaration {
                name: output_name.as_ptr(),
                direction: 2,
                channel_count: 2,
            },
        ];
        for sample_rate in [44_100, 48_000] {
            let engine = unsafe {
                dandrum_kernel_prepare_file(path.as_ptr(), sample_rate, 8, buses.as_ptr(), 2)
            };
            assert!(!engine.is_null(), "{sample_rate} Hz prepares");
            let left = [-0.5_f32; 8];
            let right = [0.25_f32; 8];
            let input_channels = [left.as_ptr(), right.as_ptr()];
            let input = DandrumKernelInputBusView {
                name: input_name.as_ptr(),
                channels: input_channels.as_ptr(),
                channel_count: 2,
                frame_capacity: 8,
                bus_index: 0,
            };
            let mut output_left = [0.0_f32; 8];
            let mut output_right = [0.0_f32; 8];
            let output_channels = [output_left.as_mut_ptr(), output_right.as_mut_ptr()];
            let output = DandrumKernelOutputBusView {
                name: output_name.as_ptr(),
                channels: output_channels.as_ptr(),
                channel_count: 2,
                frame_capacity: 8,
                bus_index: 0,
            };
            assert_eq!(
                unsafe { dandrum_kernel_render(engine, &input, 1, &output, 1, 8) },
                8,
                "{sample_rate} Hz renders"
            );
            assert_eq!(output_left, left);
            assert_eq!(output_right, right);
            unsafe { dandrum_kernel_destroy(engine) };
        }
    }

    #[test]
    fn kernel_ffi_rejects_invalid_declarations_and_buffer_views_before_writing() {
        let (_dir, path) = kernel_ffi_patch();
        let input_name = std::ffi::CString::new("input").unwrap();
        let output_name = std::ffi::CString::new("master").unwrap();
        let bad_direction = [DandrumKernelBusDeclaration {
            name: output_name.as_ptr(),
            direction: 9,
            channel_count: 2,
        }];
        assert!(
            unsafe {
                dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, bad_direction.as_ptr(), 1)
            }
            .is_null()
        );
        let wrong_width = [DandrumKernelBusDeclaration {
            name: output_name.as_ptr(),
            direction: 2,
            channel_count: 1,
        }];
        assert!(
            unsafe {
                dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, wrong_width.as_ptr(), 1)
            }
            .is_null()
        );
        let declarations = [
            DandrumKernelBusDeclaration {
                name: input_name.as_ptr(),
                direction: 1,
                channel_count: 2,
            },
            DandrumKernelBusDeclaration {
                name: output_name.as_ptr(),
                direction: 2,
                channel_count: 2,
            },
        ];
        let engine = unsafe {
            dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, declarations.as_ptr(), 2)
        };
        assert!(!engine.is_null());
        let mut short_name = [0_i8; 6];
        let mut direction = 0;
        let mut signal = 0;
        let mut channels = 0;
        assert!(!unsafe {
            dandrum_kernel_root_port(
                engine,
                1,
                short_name.as_mut_ptr(),
                short_name.len(),
                &mut direction,
                &mut signal,
                &mut channels,
            )
        });

        let mut left = [9.0_f32; 8];
        let mut right = [9.0_f32; 8];
        let destinations = [left.as_mut_ptr(), right.as_mut_ptr()];
        let output = DandrumKernelOutputBusView {
            name: output_name.as_ptr(),
            channels: destinations.as_ptr(),
            channel_count: 2,
            frame_capacity: 8,
            bus_index: 0,
        };
        let invalid = [
            DandrumKernelOutputBusView {
                channel_count: 1,
                ..output
            },
            DandrumKernelOutputBusView {
                frame_capacity: 7,
                ..output
            },
            DandrumKernelOutputBusView {
                channels: std::ptr::null(),
                ..output
            },
        ];
        for view in invalid {
            assert_eq!(
                unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &view, 1, 8) },
                0
            );
            assert_eq!(left, [9.0; 8]);
            assert_eq!(right, [9.0; 8]);
        }
        let missing_channel = [left.as_mut_ptr(), std::ptr::null_mut()];
        let view = DandrumKernelOutputBusView {
            channels: missing_channel.as_ptr(),
            ..output
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &view, 1, 8) },
            0
        );
        let source = [0.5_f32; 8];
        let missing_source = [source.as_ptr(), std::ptr::null()];
        let input = DandrumKernelInputBusView {
            name: input_name.as_ptr(),
            channels: missing_source.as_ptr(),
            channel_count: 2,
            frame_capacity: 8,
            bus_index: 0,
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, &input, 1, &output, 1, 8) },
            0
        );
        let unknown_input_slot = DandrumKernelInputBusView {
            name: output_name.as_ptr(),
            bus_index: 1,
            ..input
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, &unknown_input_slot, 1, &output, 1, 8) },
            0
        );
        let duplicate_outputs = [output, output];
        assert_eq!(
            unsafe {
                dandrum_kernel_render(
                    engine,
                    std::ptr::null(),
                    0,
                    duplicate_outputs.as_ptr(),
                    2,
                    8,
                )
            },
            0
        );
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 9) },
            0
        );
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, std::ptr::null(), 0, 8) },
            0
        );
        assert_eq!(left, [9.0; 8]);
        assert_eq!(right, [9.0; 8]);
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_prepares_and_renders_a_dynamics_root_bus() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dynamics.yaml");
        std::fs::write(
            &path,
            "metadata: { name: dynamics }\nports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: dynamics.audio_out }\nmodules:\n  - { id: dynamics, type: dynamics-processor }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let name = std::ffi::CString::new("master").unwrap();
        let declaration = DandrumKernelBusDeclaration {
            name: name.as_ptr(),
            direction: 2,
            channel_count: 1,
        };

        let engine =
            unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, &declaration, 1) };
        assert!(!engine.is_null());
        let mut samples = [1.0_f32; 8];
        let channels = [samples.as_mut_ptr()];
        let output = DandrumKernelOutputBusView {
            name: name.as_ptr(),
            channels: channels.as_ptr(),
            channel_count: 1,
            frame_capacity: 8,
            bus_index: 0,
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
            8
        );
        assert_eq!(samples, [0.0; 8]);
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_prepares_and_renders_a_package_relative_sample() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("packaged-sample.yaml");
        std::fs::write(
            &path,
            "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: voice.left }\nmodules:\n  - { id: voice, type: $LIB/1.0.0/sample_voice/sample_voice.yaml }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let master_name = std::ffi::CString::new("master").unwrap();
        let bus = DandrumKernelBusDeclaration {
            name: master_name.as_ptr(),
            direction: 2,
            channel_count: 1,
        };
        let engine = unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, &bus, 1) };
        assert!(
            !engine.is_null(),
            "package-backed patch should prepare through FFI"
        );
        assert!(unsafe { dandrum_kernel_note_on_at(engine, 60, 100, 0) });
        let mut samples = [0.0_f32; 8];
        let channels = [samples.as_mut_ptr()];
        let output = DandrumKernelOutputBusView {
            name: master_name.as_ptr(),
            channels: channels.as_ptr(),
            channel_count: 1,
            frame_capacity: 8,
            bus_index: 0,
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
            8
        );
        assert!((samples[0] - 0.25).abs() < 0.0001, "{samples:?}");
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_enumerates_event_roots_without_treating_them_as_float_buses() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("event-root.yaml");
        std::fs::write(
            &path,
            "metadata: { name: event-root }\nports:\n  - { name: events, direction: output, signal: event, channels: 1 }\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }\nmodules:\n  - { id: amp, type: gain }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let event_name = std::ffi::CString::new("events").unwrap();
        let master_name = std::ffi::CString::new("master").unwrap();
        let master = DandrumKernelBusDeclaration {
            name: master_name.as_ptr(),
            direction: 2,
            channel_count: 1,
        };
        let engine = unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, &master, 1) };
        assert!(
            !engine.is_null(),
            "event output is enumerated without an audio bus"
        );
        assert_eq!(unsafe { dandrum_kernel_root_port_count(engine) }, 2);
        let mut name = [0_i8; 32];
        let mut direction = 0;
        let mut signal = 0;
        let mut channels = 0;
        assert!(unsafe {
            dandrum_kernel_root_port(
                engine,
                0,
                name.as_mut_ptr(),
                name.len(),
                &mut direction,
                &mut signal,
                &mut channels,
            )
        });
        assert_eq!(
            unsafe { CStr::from_ptr(name.as_ptr()) }.to_str().unwrap(),
            "events"
        );
        assert_eq!((direction, signal, channels), (2, 3, 1));
        let mut samples = [7.0_f32; 8];
        let channel = [samples.as_mut_ptr()];
        let output = DandrumKernelOutputBusView {
            name: master_name.as_ptr(),
            channels: channel.as_ptr(),
            channel_count: 1,
            frame_capacity: 8,
            bus_index: 0,
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
            8
        );
        assert_eq!(samples, [0.0; 8]);

        let event_bus = DandrumKernelBusDeclaration {
            name: event_name.as_ptr(),
            direction: 2,
            channel_count: 1,
        };
        let bindings = [master, event_bus];
        assert!(
            unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, bindings.as_ptr(), 2) }
                .is_null()
        );
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_routes_multiple_output_views_by_prepared_index_in_any_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("two-outputs.yaml");
        std::fs::write(
            &path,
            "metadata: { name: two-outputs }\nports:\n  - { name: negative, direction: output, signal: audio, channels: 1, maps_from: low.out }\n  - { name: positive, direction: output, signal: audio, channels: 1, maps_from: high.out }\nmodules:\n  - { id: low, type: control_to_audio, defaults: { in: -0.5 } }\n  - { id: high, type: control_to_audio, defaults: { in: 0.25 } }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let negative_name = std::ffi::CString::new("negative").unwrap();
        let positive_name = std::ffi::CString::new("positive").unwrap();
        let buses = [
            DandrumKernelBusDeclaration {
                name: negative_name.as_ptr(),
                direction: 2,
                channel_count: 1,
            },
            DandrumKernelBusDeclaration {
                name: positive_name.as_ptr(),
                direction: 2,
                channel_count: 1,
            },
        ];
        let engine =
            unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, buses.as_ptr(), 2) };
        assert!(!engine.is_null());
        let mut negative = [0.0_f32; 8];
        let mut positive = [0.0_f32; 8];
        let negative_channel = [negative.as_mut_ptr()];
        let positive_channel = [positive.as_mut_ptr()];
        let views = [
            DandrumKernelOutputBusView {
                name: std::ptr::null(),
                channels: positive_channel.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
                bus_index: 1,
            },
            DandrumKernelOutputBusView {
                name: std::ptr::null(),
                channels: negative_channel.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
                bus_index: 0,
            },
        ];

        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, views.as_ptr(), 2, 8) },
            8
        );
        assert_eq!(negative, [-0.5; 8]);
        assert_eq!(positive, [0.25; 8]);
        negative.fill(9.0);
        positive.fill(9.0);
        let duplicate = [views[0], views[0]];
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, duplicate.as_ptr(), 2, 8) },
            0
        );
        assert_eq!(negative, [9.0; 8]);
        assert_eq!(positive, [9.0; 8]);
        let invalid = DandrumKernelOutputBusView {
            bus_index: 2,
            ..views[0]
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &invalid, 1, 8) },
            0
        );
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_routes_reordered_input_and_output_bus_indices() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("two-inputs.yaml");
        std::fs::write(
            &path,
            "ports:\n  - { name: left_in, direction: input, signal: audio, channels: 1, maps_to: left_gain.audio_in }\n  - { name: right_in, direction: input, signal: audio, channels: 1, maps_to: right_gain.audio_in }\n  - { name: left, direction: output, signal: audio, channels: 1, maps_from: left_gain.audio_out }\n  - { name: right, direction: output, signal: audio, channels: 1, maps_from: right_gain.audio_out }\nmodules:\n  - { id: left_gain, type: gain }\n  - { id: right_gain, type: gain }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let names = ["right_in", "left", "left_in", "right"]
            .map(|name| std::ffi::CString::new(name).unwrap());
        let declarations = [
            DandrumKernelBusDeclaration {
                name: names[0].as_ptr(),
                direction: 1,
                channel_count: 1,
            },
            DandrumKernelBusDeclaration {
                name: names[1].as_ptr(),
                direction: 2,
                channel_count: 1,
            },
            DandrumKernelBusDeclaration {
                name: names[2].as_ptr(),
                direction: 1,
                channel_count: 1,
            },
            DandrumKernelBusDeclaration {
                name: names[3].as_ptr(),
                direction: 2,
                channel_count: 1,
            },
        ];
        let engine = unsafe {
            dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, declarations.as_ptr(), 4)
        };
        assert!(!engine.is_null());
        let left_input = [-0.5_f32; 8];
        let right_input = [0.25_f32; 8];
        let left_source = [left_input.as_ptr()];
        let right_source = [right_input.as_ptr()];
        let inputs = [
            DandrumKernelInputBusView {
                name: std::ptr::null(),
                channels: left_source.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
                bus_index: 1,
            },
            DandrumKernelInputBusView {
                name: std::ptr::null(),
                channels: right_source.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
                bus_index: 0,
            },
        ];
        let mut left_output = [0.0_f32; 8];
        let mut right_output = [0.0_f32; 8];
        let left_destination = [left_output.as_mut_ptr()];
        let right_destination = [right_output.as_mut_ptr()];
        let outputs = [
            DandrumKernelOutputBusView {
                name: std::ptr::null(),
                channels: right_destination.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
                bus_index: 1,
            },
            DandrumKernelOutputBusView {
                name: std::ptr::null(),
                channels: left_destination.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
                bus_index: 0,
            },
        ];
        let allocations = crate::test_allocator::count_current_thread_allocations(|| {
            assert_eq!(
                unsafe {
                    dandrum_kernel_render(engine, inputs.as_ptr(), 2, outputs.as_ptr(), 2, 8)
                },
                8
            );
        });
        assert_eq!(allocations, 0);
        assert_eq!(left_output, left_input);
        assert_eq!(right_output, right_input);
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_accepts_bounded_note_events_for_a_prepared_host() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note-events.yaml");
        std::fs::write(
            &path,
            "metadata: { name: note-events }\nports:\n  - { name: master, direction: output, signal: audio, channels: 2, maps_from: source.out }\nmodules:\n  - { id: source, type: control_to_audio, static: { channels: 2 }, defaults: { in: 0.25 } }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let master_name = std::ffi::CString::new("master").unwrap();
        let bus = DandrumKernelBusDeclaration {
            name: master_name.as_ptr(),
            direction: 2,
            channel_count: 2,
        };
        let engine = unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, &bus, 1) };
        assert!(!engine.is_null());
        assert!(!unsafe { dandrum_kernel_note_on_at(std::ptr::null_mut(), 60, 100, 0) });
        assert!(!unsafe { dandrum_kernel_note_on_at(engine, 60, 100, 8) });
        assert!(!unsafe { dandrum_kernel_note_off_at(engine, 60, 8) });
        assert!(!unsafe { dandrum_kernel_note_on_at(engine, 128, 100, 0) });
        assert!(!unsafe { dandrum_kernel_note_off_at(engine, 128, 0) });
        assert!(unsafe { dandrum_kernel_note_on_at(engine, 60, 100, 3) });
        assert!(unsafe { dandrum_kernel_note_off_at(engine, 60, 6) });
        let mut left = [0.0_f32; 8];
        let mut right = [0.0_f32; 8];
        let channels = [left.as_mut_ptr(), right.as_mut_ptr()];
        let view = DandrumKernelOutputBusView {
            name: master_name.as_ptr(),
            channels: channels.as_ptr(),
            channel_count: 2,
            frame_capacity: 8,
            bus_index: 0,
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &view, 1, 8) },
            8
        );
        assert_eq!(left, [0.25; 8]);
        for _ in 0..crate::graph_processor::prepared_event_capacity(8) {
            assert!(unsafe { dandrum_kernel_note_on_at(engine, 60, 100, 0) });
        }
        assert!(!unsafe { dandrum_kernel_note_on_at(engine, 60, 100, 0) });
        assert!(unsafe { dandrum_kernel_reset(engine) });
        assert!(unsafe { dandrum_kernel_note_on_at(engine, 61, 100, 0) });
        assert!(!unsafe { dandrum_kernel_reset(std::ptr::null_mut()) });
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_poly_voice_starts_steals_and_resets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("poly-timing.yaml");
        std::fs::write(
            &path,
            "ports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: voices.audio }\nmodule_definitions:\n  - type: velocity_voice\n    ports:\n      - { name: audio, direction: output, signal: audio, channels: 1, maps_from: amp.audio_out }\n    modules:\n      - { id: constant, type: control_to_audio, defaults: { in: 1.0 } }\n      - { id: amp, type: gain }\n    connections:\n      - { from: constant.out, to: amp.audio_in }\n      - { from: voice.velocity, to: amp.gain }\nmodules:\n  - { id: voices, type: poly, static: { definition: velocity_voice, max_voices: 1, allocation: oldest-steal } }\nconnections: []\n",
        )
        .unwrap();
        let path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let master_name = std::ffi::CString::new("master").unwrap();
        let bus = DandrumKernelBusDeclaration {
            name: master_name.as_ptr(),
            direction: 2,
            channel_count: 1,
        };
        let engine = unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, &bus, 1) };
        assert!(!engine.is_null());
        let mut samples = [0.0_f32; 8];
        let channels = [samples.as_mut_ptr()];
        let output = DandrumKernelOutputBusView {
            name: master_name.as_ptr(),
            channels: channels.as_ptr(),
            channel_count: 1,
            frame_capacity: 8,
            bus_index: 0,
        };
        assert!(unsafe { dandrum_kernel_note_on_at(engine, 60, 127, 4) });
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
            8
        );
        assert_eq!(samples, [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);

        let allocations = crate::test_allocator::count_current_thread_allocations(|| {
            assert!(unsafe { dandrum_kernel_note_on_at(engine, 61, 64, 4) });
            assert_eq!(
                unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
                8
            );
        });
        assert_eq!(allocations, 0);
        assert_eq!(samples[..4], [1.0; 4]);
        for sample in &samples[4..] {
            assert!((*sample - 64.0 / 127.0).abs() < 0.0001, "{samples:?}");
        }

        assert!(unsafe { dandrum_kernel_reset(engine) });
        samples.fill(f32::NAN);
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
            8
        );
        assert_eq!(samples, [0.0; 8], "reset must retire the active voice");

        assert!(unsafe { dandrum_kernel_note_on_at(engine, 62, 127, 0) });
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
            8
        );
        assert_eq!(samples, [1.0; 8], "the reset engine must accept a new note");
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_prepares_drum_assets_and_renders_at_host_rate() {
        let patch = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/advanced-drum-kit.yaml");
        let path = std::ffi::CString::new(patch.to_str().unwrap()).unwrap();
        let master = std::ffi::CString::new("master").unwrap();
        let bus = DandrumKernelBusDeclaration {
            name: master.as_ptr(),
            direction: 2,
            channel_count: 2,
        };
        for sample_rate in [44_100, 48_000, 96_000] {
            let engine =
                unsafe { dandrum_kernel_prepare_file(path.as_ptr(), sample_rate, 8, &bus, 1) };
            assert!(
                !engine.is_null(),
                "drum kit must prepare at {sample_rate} Hz"
            );
            let mut left = [0.0_f32; 8];
            let mut right = [0.0_f32; 8];
            let channels = [left.as_mut_ptr(), right.as_mut_ptr()];
            let output = DandrumKernelOutputBusView {
                name: master.as_ptr(),
                channels: channels.as_ptr(),
                channel_count: 2,
                frame_capacity: 8,
                bus_index: 0,
            };
            let allocations = crate::test_allocator::count_current_thread_allocations(|| {
                assert!(unsafe { dandrum_kernel_note_on_at(engine, 36, 100, 0) });
                assert_eq!(
                    unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 8) },
                    8
                );
            });
            assert_eq!(allocations, 0);
            assert_eq!(left[0], -0.5);
            assert_eq!(right[0], -0.5);
            unsafe { dandrum_kernel_destroy(engine) };
        }
    }

    fn ui_text(view: DandrumKernelStringView) -> String {
        if view.size == 0 {
            return String::new();
        }
        let bytes = unsafe { std::slice::from_raw_parts(view.data.cast::<u8>(), view.size) };
        std::str::from_utf8(bytes).unwrap().to_owned()
    }

    #[test]
    fn prepared_ui_snapshot_retains_drum_mapping_after_engine_release() {
        let patch = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/advanced-drum-kit.yaml");
        let path = std::ffi::CString::new(patch.to_str().unwrap()).unwrap();
        let master = std::ffi::CString::new("master").unwrap();
        let bus = DandrumKernelBusDeclaration {
            name: master.as_ptr(),
            direction: 2,
            channel_count: 2,
        };
        let engine = unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 96_000, 8, &bus, 1) };
        assert!(!engine.is_null());
        let snapshot = unsafe { dandrum_kernel_ui_snapshot_create(engine) };
        assert!(!snapshot.is_null());
        unsafe { dandrum_kernel_destroy(engine) };

        assert_eq!(unsafe { dandrum_kernel_ui_source_count(snapshot) }, 1);
        let mut source = std::mem::MaybeUninit::<DandrumKernelUiSource>::uninit();
        assert!(unsafe { dandrum_kernel_ui_source(snapshot, 0, source.as_mut_ptr()) });
        let source = unsafe { source.assume_init() };
        assert_eq!(ui_text(source.id), "drums");
        assert_eq!(source.sample_rate_hz, 48_000);
        assert_eq!(source.channel_count, 1);
        assert_eq!(source.frame_count, 51_000);

        assert_eq!(unsafe { dandrum_kernel_ui_region_count(snapshot, 0) }, 6);
        let mut region = std::mem::MaybeUninit::<DandrumKernelUiRegion>::uninit();
        assert!(unsafe { dandrum_kernel_ui_region(snapshot, 0, 2, region.as_mut_ptr()) });
        let region = unsafe { region.assume_init() };
        assert_eq!(ui_text(region.id), "snare_hard_a");
        assert_eq!((region.start_frame, region.end_frame), (20_000, 28_000));

        assert_eq!(unsafe { dandrum_kernel_ui_map_count(snapshot) }, 1);
        let mut map = std::mem::MaybeUninit::<DandrumKernelUiMap>::uninit();
        assert!(unsafe { dandrum_kernel_ui_map(snapshot, 0, map.as_mut_ptr()) });
        let map = unsafe { map.assume_init() };
        assert_eq!(ui_text(map.id), "kit");
        assert_eq!(ui_text(map.selection_mode), "round_robin");
        assert_eq!(unsafe { dandrum_kernel_ui_zone_count(snapshot, 0) }, 7);

        let mut soft = std::mem::MaybeUninit::<DandrumKernelUiZone>::uninit();
        let mut hard_a = std::mem::MaybeUninit::<DandrumKernelUiZone>::uninit();
        let mut hard_b = std::mem::MaybeUninit::<DandrumKernelUiZone>::uninit();
        assert!(unsafe { dandrum_kernel_ui_zone(snapshot, 0, 1, soft.as_mut_ptr()) });
        assert!(unsafe { dandrum_kernel_ui_zone(snapshot, 0, 2, hard_a.as_mut_ptr()) });
        assert!(unsafe { dandrum_kernel_ui_zone(snapshot, 0, 3, hard_b.as_mut_ptr()) });
        let soft = unsafe { soft.assume_init() };
        let hard_a = unsafe { hard_a.assume_init() };
        let hard_b = unsafe { hard_b.assume_init() };
        assert_eq!(
            (
                soft.key_low,
                soft.key_high,
                soft.velocity_low,
                soft.velocity_high
            ),
            (38, 38, 1, 63)
        );
        assert_eq!(
            (
                hard_a.key_low,
                hard_a.key_high,
                hard_a.velocity_low,
                hard_a.velocity_high
            ),
            (38, 38, 64, 127)
        );
        assert_eq!(ui_text(hard_a.round_robin_group), "hard_snare");
        assert_eq!(ui_text(hard_b.round_robin_group), "hard_snare");
        assert_eq!(
            (
                soft.control_group,
                hard_a.control_group,
                hard_b.control_group
            ),
            (2, 2, 2)
        );
        assert_eq!((hard_a.source_index, hard_a.region_index), (0, 2));
        assert_eq!((hard_a.start_frame, hard_a.end_frame), (20_000, 28_000));

        let mut group = -1;
        assert!(unsafe { dandrum_kernel_ui_public_control_group(snapshot, 0, &mut group) });
        assert_eq!(group, 0, "shared pitch control must be instrument-scoped");
        assert!(unsafe { dandrum_kernel_ui_public_control_group(snapshot, 5, &mut group) });
        assert_eq!(group, 1, "kick pitch control must address group 1");
        assert!(unsafe { dandrum_kernel_ui_public_control_group(snapshot, 9, &mut group) });
        assert_eq!(group, 2, "snare pitch control must address group 2");
        assert!(!unsafe { dandrum_kernel_ui_public_control_group(snapshot, 23, &mut group) });

        let mut absent = std::mem::MaybeUninit::<DandrumKernelUiSource>::uninit();
        assert!(!unsafe { dandrum_kernel_ui_source(snapshot, 1, absent.as_mut_ptr()) });
        let mut absent_region = std::mem::MaybeUninit::<DandrumKernelUiRegion>::uninit();
        let mut absent_slice = std::mem::MaybeUninit::<DandrumKernelUiSlice>::uninit();
        let mut absent_map = std::mem::MaybeUninit::<DandrumKernelUiMap>::uninit();
        let mut absent_zone = std::mem::MaybeUninit::<DandrumKernelUiZone>::uninit();
        assert!(!unsafe { dandrum_kernel_ui_region(snapshot, 0, 6, absent_region.as_mut_ptr()) });
        assert!(!unsafe { dandrum_kernel_ui_slice(snapshot, 0, 0, absent_slice.as_mut_ptr()) });
        assert!(!unsafe { dandrum_kernel_ui_map(snapshot, 1, absent_map.as_mut_ptr()) });
        assert!(!unsafe { dandrum_kernel_ui_zone(snapshot, 0, 7, absent_zone.as_mut_ptr()) });
        assert_eq!(
            unsafe { dandrum_kernel_ui_source_count(std::ptr::null()) },
            0
        );
        assert_eq!(unsafe { dandrum_kernel_ui_map_count(std::ptr::null()) }, 0);
        assert!(!unsafe {
            dandrum_kernel_ui_public_control_group(snapshot, 0, std::ptr::null_mut())
        });
        assert!(unsafe { dandrum_kernel_ui_snapshot_create(std::ptr::null()) }.is_null());
        unsafe { dandrum_kernel_ui_snapshot_destroy(snapshot) };
        unsafe { dandrum_kernel_ui_snapshot_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn prepared_ui_snapshot_preserves_loops_slices_and_effective_zone_bounds() {
        let directory = tempfile::tempdir().unwrap();
        let bundled = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/assets/advanced-drums.wav");
        std::fs::copy(bundled, directory.path().join("drums.wav")).unwrap();
        let patch = directory.path().join("metadata.yaml");
        std::fs::write(
            &patch,
            r#"metadata: { name: metadata }
assets:
  sample_sources:
    - id: source
      path: drums.wav
      regions:
        - id: body
          start_frame: 0
          end_frame: 2000
          root_note: 60
          gain_db: -3
          pan: 0.25
          reverse: true
          fade_in_ms: 1
          fade_out_ms: 2
          loop: { mode: forward, start_frame: 200, end_frame: 1800, crossfade_ms: 1 }
      slices:
        - { id: transient, start_frame: 100, end_frame: 300 }
  sample_maps:
    - id: kit
      zones:
        - id: hit
          region: source.body
          region_override: { start_frame: 100, end_frame: 1900 }
          key_range: [60, 60]
          velocity_range: [1, 127]
          choke_group: hats
          control_group: 2
          weight: 3
          gain_db: -6
          pan: 0.5
          pitch_semitones: 2
ports:
  - { name: master, direction: output, signal: audio, channels: 1, maps_from: osc.audio }
modules:
  - { id: osc, type: oscillator }
"#,
        )
        .unwrap();
        let path = std::ffi::CString::new(patch.to_str().unwrap()).unwrap();
        let master = std::ffi::CString::new("master").unwrap();
        let bus = DandrumKernelBusDeclaration {
            name: master.as_ptr(),
            direction: 2,
            channel_count: 1,
        };
        let engine = unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, &bus, 1) };
        assert!(!engine.is_null());
        let snapshot = unsafe { dandrum_kernel_ui_snapshot_create(engine) };
        assert!(!snapshot.is_null());
        unsafe { dandrum_kernel_destroy(engine) };

        std::fs::remove_file(&patch).unwrap();
        std::fs::remove_file(directory.path().join("drums.wav")).unwrap();

        let mut region = std::mem::MaybeUninit::<DandrumKernelUiRegion>::uninit();
        assert!(unsafe { dandrum_kernel_ui_region(snapshot, 0, 0, region.as_mut_ptr()) });
        let region = unsafe { region.assume_init() };
        assert_eq!(ui_text(region.id), "body");
        assert_eq!((region.root_note, region.reverse), (60, true));
        assert!(region.has_gain_db && region.has_pan && region.has_loop);
        assert_eq!((region.gain_db, region.pan), (-3.0, 0.25));
        assert_eq!((region.fade_in_ms, region.fade_out_ms), (1.0, 2.0));
        assert_eq!(ui_text(region.loop_mode), "forward");
        assert_eq!(
            (region.loop_start_frame, region.loop_end_frame),
            (200, 1800)
        );
        assert_eq!(region.loop_crossfade_ms, 1.0);

        assert_eq!(unsafe { dandrum_kernel_ui_slice_count(snapshot, 0) }, 1);
        let mut slice = std::mem::MaybeUninit::<DandrumKernelUiSlice>::uninit();
        assert!(unsafe { dandrum_kernel_ui_slice(snapshot, 0, 0, slice.as_mut_ptr()) });
        let slice = unsafe { slice.assume_init() };
        assert_eq!(ui_text(slice.id), "transient");
        assert_eq!((slice.start_frame, slice.end_frame), (100, 300));

        let mut map = std::mem::MaybeUninit::<DandrumKernelUiMap>::uninit();
        assert!(unsafe { dandrum_kernel_ui_map(snapshot, 0, map.as_mut_ptr()) });
        let map = unsafe { map.assume_init() };
        assert_eq!(ui_text(map.selection_mode), "first_match");
        let mut zone = std::mem::MaybeUninit::<DandrumKernelUiZone>::uninit();
        assert!(unsafe { dandrum_kernel_ui_zone(snapshot, 0, 0, zone.as_mut_ptr()) });
        let zone = unsafe { zone.assume_init() };
        assert_eq!(ui_text(zone.id), "hit");
        assert_eq!(ui_text(zone.choke_group), "hats");
        assert_eq!((zone.start_frame, zone.end_frame), (100, 1900));
        assert_eq!((zone.control_group, zone.weight), (2, 3));
        assert!(zone.has_gain_db && zone.has_pan && zone.has_pitch_semitones);
        assert_eq!(
            (zone.gain_db, zone.pan, zone.pitch_semitones),
            (-6.0, 0.5, 2.0)
        );

        assert_eq!(unsafe { dandrum_kernel_ui_region_count(snapshot, 1) }, 0);
        assert_eq!(unsafe { dandrum_kernel_ui_slice_count(snapshot, 1) }, 0);
        assert_eq!(unsafe { dandrum_kernel_ui_zone_count(snapshot, 1) }, 0);
        assert!(!unsafe { dandrum_kernel_ui_region(snapshot, 0, 1, std::ptr::null_mut()) });
        assert!(!unsafe { dandrum_kernel_ui_slice(snapshot, 0, 1, std::ptr::null_mut()) });
        assert!(!unsafe { dandrum_kernel_ui_zone(snapshot, 0, 1, std::ptr::null_mut()) });
        unsafe { dandrum_kernel_ui_snapshot_destroy(snapshot) };
    }

    #[test]
    fn retained_waveform_source_reduces_signed_channels_after_engine_release() {
        let directory = tempfile::tempdir().unwrap();
        let wav = directory.path().join("source.wav");
        crate::wav::write_wav_stereo_i16(
            std::fs::File::create(&wav).unwrap(),
            48_000,
            &[0.0, -0.75, 0.5, 0.0],
            &[0.0, 0.25, -0.25, 0.0],
        )
        .unwrap();
        let patch = directory.path().join("waveform.yaml");
        std::fs::write(&patch, "metadata: { name: waveform }\nassets:\n  sample_sources:\n    - id: source\n      path: source.wav\n      regions:\n        - { id: full, start_frame: 0, end_frame: 4 }\nports:\n  - { name: master, direction: output, signal: audio, channels: 1, maps_from: osc.audio }\nmodules:\n  - { id: osc, type: oscillator }\n").unwrap();
        let path = std::ffi::CString::new(patch.to_str().unwrap()).unwrap();
        let master = std::ffi::CString::new("master").unwrap();
        let bus = DandrumKernelBusDeclaration {
            name: master.as_ptr(),
            direction: 2,
            channel_count: 1,
        };
        let engine = unsafe { dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 8, &bus, 1) };
        assert!(!engine.is_null());
        assert!(unsafe { dandrum_kernel_waveform_source_create(engine, 1) }.is_null());
        let source = unsafe { dandrum_kernel_waveform_source_create(engine, 0) };
        assert!(!source.is_null());
        unsafe { dandrum_kernel_destroy(engine) };
        std::fs::remove_file(wav).unwrap();

        let mut info = std::mem::MaybeUninit::<DandrumKernelWaveformSourceInfo>::uninit();
        assert!(unsafe { dandrum_kernel_waveform_source_info(source, info.as_mut_ptr()) });
        let info = unsafe { info.assume_init() };
        assert_eq!(ui_text(info.source_id), "source");
        assert_eq!(
            (info.sample_rate_hz, info.channel_count, info.frame_count),
            (48_000, 2, 4)
        );
        assert_ne!(info.content_revision, [0; 32]);

        let mut bucket = [DandrumKernelWaveformBucket::default(); 1];
        assert!(unsafe { dandrum_kernel_waveform_reduce(source, 0, 0, 4, bucket.as_mut_ptr(), 1) });
        assert_eq!((bucket[0].start_frame, bucket[0].end_frame), (0, 4));
        assert!((bucket[0].minimum + 0.75).abs() < 0.0001);
        assert!((bucket[0].maximum - 0.5).abs() < 0.0001);
        assert!(unsafe { dandrum_kernel_waveform_reduce(source, 1, 0, 4, bucket.as_mut_ptr(), 1) });
        assert!((bucket[0].minimum + 0.25).abs() < 0.0001);
        assert!((bucket[0].maximum - 0.25).abs() < 0.0001);
        let mut split = [DandrumKernelWaveformBucket::default(); 2];
        assert!(unsafe { dandrum_kernel_waveform_reduce(source, 0, 0, 4, split.as_mut_ptr(), 2) });
        assert_eq!((split[0].start_frame, split[0].end_frame), (0, 2));
        assert_eq!((split[1].start_frame, split[1].end_frame), (2, 4));
        assert!((split[0].minimum + 0.75).abs() < 0.0001);
        assert_eq!(split[0].maximum, 0.0);
        assert_eq!(split[1].minimum, 0.0);
        assert!((split[1].maximum - 0.5).abs() < 0.0001);
        let prior = (bucket[0].minimum, bucket[0].maximum);
        assert!(!unsafe {
            dandrum_kernel_waveform_reduce(source, 2, 0, 4, bucket.as_mut_ptr(), 1)
        });
        assert!(!unsafe {
            dandrum_kernel_waveform_reduce(source, 0, 0, 5, bucket.as_mut_ptr(), 1)
        });
        assert!(!unsafe {
            dandrum_kernel_waveform_reduce(source, 0, 0, 4, std::ptr::null_mut(), 1)
        });
        assert_eq!((bucket[0].minimum, bucket[0].maximum), prior);
        unsafe { dandrum_kernel_waveform_source_destroy(source) };
        unsafe { dandrum_kernel_waveform_source_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn kernel_ffi_reset_clears_effect_tail_and_allows_reuse() {
        let patch = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/patches/reverb-demo.yaml");
        let path = std::ffi::CString::new(patch.to_str().unwrap()).unwrap();
        let input_name = std::ffi::CString::new("in").unwrap();
        let output_name = std::ffi::CString::new("master").unwrap();
        let declarations = [
            DandrumKernelBusDeclaration {
                name: input_name.as_ptr(),
                direction: 1,
                channel_count: 2,
            },
            DandrumKernelBusDeclaration {
                name: output_name.as_ptr(),
                direction: 2,
                channel_count: 2,
            },
        ];
        let engine = unsafe {
            dandrum_kernel_prepare_file(path.as_ptr(), 48_000, 128, declarations.as_ptr(), 2)
        };
        assert!(!engine.is_null());

        let mut source = [0.0_f32; 128];
        source[0] = 1.0;
        let mut left = [0.0_f32; 128];
        let mut right = [0.0_f32; 128];
        let destinations = [left.as_mut_ptr(), right.as_mut_ptr()];
        let output = DandrumKernelOutputBusView {
            name: output_name.as_ptr(),
            channels: destinations.as_ptr(),
            channel_count: 2,
            frame_capacity: 128,
            bus_index: 0,
        };
        let render = |source: &[f32; 128]| {
            let sources = [source.as_ptr(), source.as_ptr()];
            let input = DandrumKernelInputBusView {
                name: input_name.as_ptr(),
                channels: sources.as_ptr(),
                channel_count: 2,
                frame_capacity: 128,
                bus_index: 0,
            };
            unsafe { dandrum_kernel_render(engine, &input, 1, &output, 1, 128) }
        };

        assert_eq!(render(&source), 128);
        let first_block = left;
        assert!(first_block.iter().any(|sample| sample.abs() > 0.0));
        source.fill(0.0);
        let mut heard_tail = false;
        for _ in 0..64 {
            assert_eq!(render(&source), 128);
            heard_tail |= left.iter().any(|sample| sample.abs() > 0.0);
        }
        assert!(heard_tail, "the reset check needs an excited effect tail");

        assert!(unsafe { dandrum_kernel_reset(engine) });
        for _ in 0..64 {
            left.fill(f32::NAN);
            right.fill(f32::NAN);
            assert_eq!(render(&source), 128);
            assert_eq!(left, [0.0; 128], "reset must clear the left reverb tail");
            assert_eq!(right, [0.0; 128], "reset must clear the right reverb tail");
        }
        source[0] = 1.0;
        assert_eq!(render(&source), 128);
        assert_eq!(left, first_block, "the reset engine must process new input");
        unsafe { dandrum_kernel_destroy(engine) };
    }

    fn assert_no_panic(name: &str, f: impl FnOnce()) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        assert!(result.is_ok(), "{name}");
    }

    #[test]
    fn c_ffi_public_parameter_metadata_rejects_legacy_patch_paths() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy-patch.yaml");
        std::fs::write(
            &path,
            "metadata: { name: Legacy }\nrender: { sample_rate_hz: 48000, block_size_frames: 64, duration_frames: 64 }\nmodules:\n  - { id: out, type: audio_output }\n",
        )
        .unwrap();
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { dandrum_patch_public_numeric_parameter_count(c_path.as_ptr()) },
            0
        );

        let mut id = [0_i8; 64];
        let mut name = [0_i8; 64];
        let mut default_value = 0.0;
        let mut min_value = 0.0;
        let mut max_value = 0.0;
        let result = unsafe {
            dandrum_patch_public_numeric_parameter_descriptor(
                c_path.as_ptr(),
                0,
                id.as_mut_ptr(),
                id.len(),
                name.as_mut_ptr(),
                name.len(),
                &mut default_value,
                &mut min_value,
                &mut max_value,
            )
        };

        assert!(!result);
    }

    #[test]
    fn c_ffi_discovers_kernel_patch_public_control_aliases() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/patches/synthetic-808-kick.yaml");
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { dandrum_patch_public_numeric_parameter_count(c_path.as_ptr()) },
            6
        );
        let mut id = [0_i8; 64];
        let mut name = [0_i8; 64];
        let mut default_value = 0.0;
        let mut min_value = 0.0;
        let mut max_value = 0.0;
        assert!(unsafe {
            dandrum_patch_public_numeric_parameter_descriptor(
                c_path.as_ptr(),
                1,
                id.as_mut_ptr(),
                id.len(),
                name.as_mut_ptr(),
                name.len(),
                &mut default_value,
                &mut min_value,
                &mut max_value,
            )
        });
        assert_eq!(
            unsafe { CStr::from_ptr(id.as_ptr()) }.to_str().unwrap(),
            "kick.decay_ms"
        );
        assert_eq!(default_value, 650.0);
        assert_eq!(min_value, 50.0);
        assert_eq!(max_value, 2_000.0);
    }

    #[test]
    fn c_ffi_kernel_public_control_slot_drives_a_bound_root_input() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/patches/synthetic-808-kick.yaml");
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let mut port_name = [0_i8; 64];
        assert!(unsafe {
            dandrum_patch_public_numeric_parameter_port_name(
                c_path.as_ptr(),
                1,
                port_name.as_mut_ptr(),
                port_name.len(),
            )
        });
        assert_eq!(
            unsafe { CStr::from_ptr(port_name.as_ptr()) }
                .to_str()
                .unwrap(),
            "decay_ms"
        );
        let master = std::ffi::CString::new("master").unwrap();
        let decay = std::ffi::CString::new("decay_ms").unwrap();
        let buses = [
            DandrumKernelBusDeclaration {
                name: master.as_ptr(),
                direction: 2,
                channel_count: 2,
            },
            DandrumKernelBusDeclaration {
                name: decay.as_ptr(),
                direction: 1,
                channel_count: 1,
            },
        ];
        let render = |value| {
            let engine = unsafe {
                dandrum_kernel_prepare_file(
                    c_path.as_ptr(),
                    48_000,
                    128,
                    buses.as_ptr(),
                    buses.len(),
                )
            };
            assert!(!engine.is_null());
            assert!(!unsafe {
                dandrum_kernel_set_public_numeric_parameter_by_slot(engine, 1, f64::NAN)
            });
            assert!(!unsafe {
                dandrum_kernel_set_public_numeric_parameter_by_slot(engine, 1, 2_001.0)
            });
            assert!(!unsafe {
                dandrum_kernel_set_public_numeric_parameter_by_slot(engine, 6, value)
            });
            let mut left = [0.0_f32; 128];
            let mut right = [0.0_f32; 128];
            let channels = [left.as_mut_ptr(), right.as_mut_ptr()];
            let output = DandrumKernelOutputBusView {
                name: master.as_ptr(),
                channels: channels.as_ptr(),
                channel_count: 2,
                frame_capacity: 128,
                bus_index: 0,
            };
            let allocations = crate::test_allocator::count_current_thread_allocations(|| {
                assert!(unsafe {
                    dandrum_kernel_set_public_numeric_parameter_by_slot(engine, 1, value)
                });
                assert!(unsafe { dandrum_kernel_note_on_at(engine, 36, 110, 0) });
                assert_eq!(
                    unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &output, 1, 128) },
                    128
                );
            });
            assert_eq!(
                allocations, 0,
                "parameter update and render remain allocation-free"
            );
            unsafe { dandrum_kernel_destroy(engine) };
            left
        };
        assert_ne!(render(250.0), render(1_400.0));
        let master_only = [buses[0]];
        let unbound = unsafe {
            dandrum_kernel_prepare_file(c_path.as_ptr(), 48_000, 128, master_only.as_ptr(), 1)
        };
        assert!(!unbound.is_null());
        assert!(!unsafe { dandrum_kernel_set_public_numeric_parameter_by_slot(unbound, 1, 250.0) });
        unsafe { dandrum_kernel_destroy(unbound) };
    }

    #[test]
    fn c_ffi_realtime_event_queue_reports_submission_status() {
        let queue = dandrum_realtime_event_queue_create(1);

        assert!(!queue.is_null());
        assert_eq!(
            unsafe { dandrum_realtime_event_queue_note_on(queue, 60, 100) },
            0
        );
        assert_eq!(
            unsafe { dandrum_realtime_event_queue_note_off(queue, 60) },
            1
        );
        assert_eq!(
            unsafe { dandrum_realtime_event_queue_dropped_count(queue) },
            1
        );

        unsafe { dandrum_realtime_event_queue_destroy(queue) };
    }

    #[test]
    fn c_ffi_null_calls_are_safe() {
        assert_no_panic("destroy null", || unsafe {
            dandrum_kernel_destroy(std::ptr::null_mut())
        });
        assert!(
            unsafe {
                dandrum_kernel_prepare_file(std::ptr::null(), 48_000, 64, std::ptr::null(), 0)
            }
            .is_null()
        );
        assert!(!unsafe {
            dandrum_kernel_set_public_numeric_parameter_by_slot(std::ptr::null_mut(), 0, 1.0)
        });
        assert!(!unsafe { dandrum_kernel_reset(std::ptr::null_mut()) });
        assert_no_panic("destroy queue null", || unsafe {
            dandrum_realtime_event_queue_destroy(std::ptr::null_mut())
        });
    }
}
