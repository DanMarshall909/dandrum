use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::path::PathBuf;

use crate::preparation;
use crate::realtime;

use crate::graph::{PortDirection, SignalType};
use crate::graph_processor::RealtimeGraphProcessor;
use crate::kernel::{ChannelCount, PortMetadata};
use crate::patch::RenderSettings;
use crate::sample::PreparedSamplerAssets;

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
}

/// Output channel pointers remain caller-owned and are written only on success.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DandrumKernelOutputBusView {
    pub name: *const c_char,
    pub channels: *const *mut f32,
    pub channel_count: usize,
    pub frame_capacity: usize,
}

pub struct DandrumKernelInstrument {
    prepared: preparation::PreparedKernelInstrument,
    runtime: RealtimeGraphProcessor,
    ports: Vec<PortMetadata>,
    declared_inputs: BTreeMap<String, usize>,
    declared_outputs: BTreeMap<String, usize>,
    input_names: Vec<String>,
    output_names: Vec<String>,
    planar_output_names: Vec<String>,
    input_scratch: Vec<Vec<Vec<f32>>>,
    output_scratch: Vec<Vec<Vec<f32>>>,
    public_controls: Vec<KernelPublicControl>,
    max_block_size: usize,
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
            }
            2 if declared_outputs
                .insert(name.to_string(), declaration.channel_count)
                .is_none() =>
            {
                buses = buses.with_output(name, declaration.channel_count);
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
    let Ok(prepared) = preparation::prepare_kernel_graph_for_planar_ffi(
        patch.root(),
        patch.registry(),
        &settings,
        &buses,
        &context,
    ) else {
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
        declared_inputs,
        declared_outputs,
        input_names,
        output_names,
        planar_output_names,
        input_scratch,
        output_scratch,
        public_controls,
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
        || engine.planar_output_names.is_empty()
        || output_count != engine.planar_output_names.len()
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
        let Some(name) = (unsafe { ffi_name(&view.name) }) else {
            return 0;
        };
        if engine.declared_inputs.get(name) != Some(&view.channel_count)
            || view.frame_capacity < frames
            || view.channels.is_null()
            || input_views[..index]
                .iter()
                .any(|prior| unsafe { ffi_name(&prior.name) } == Some(name))
        {
            return 0;
        }
        let channels = unsafe { std::slice::from_raw_parts(view.channels, view.channel_count) };
        if channels.iter().any(|pointer| pointer.is_null()) {
            return 0;
        }
    }
    for (index, view) in output_views.iter().enumerate() {
        let Some(name) = (unsafe { ffi_name(&view.name) }) else {
            return 0;
        };
        if engine.declared_outputs.get(name) != Some(&view.channel_count)
            || !engine
                .planar_output_names
                .iter()
                .any(|expected| expected == name)
            || view.frame_capacity < frames
            || view.channels.is_null()
            || output_views[..index]
                .iter()
                .any(|prior| unsafe { ffi_name(&prior.name) } == Some(name))
        {
            return 0;
        }
        let channels = unsafe { std::slice::from_raw_parts(view.channels, view.channel_count) };
        if channels.iter().any(|pointer| pointer.is_null()) {
            return 0;
        }
    }
    if engine.planar_output_names.iter().any(|expected| {
        !output_views
            .iter()
            .any(|view| unsafe { ffi_name(&view.name) } == Some(expected.as_str()))
    }) {
        return 0;
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
        let Some(name) = (unsafe { ffi_name(&view.name) }) else {
            unreachable!("validated input name")
        };
        let Some(index) = engine
            .input_names
            .iter()
            .position(|candidate| candidate == name)
        else {
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
        let Some(name) = (unsafe { ffi_name(&view.name) }) else {
            unreachable!("validated output name")
        };
        let index = engine
            .output_names
            .iter()
            .position(|candidate| candidate == name)
            .expect("validated output name");
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
        }];
        let mut out_left = [0.0_f32; 8];
        let mut out_right = [0.0_f32; 8];
        let destinations = [out_left.as_mut_ptr(), out_right.as_mut_ptr()];
        let outputs = [DandrumKernelOutputBusView {
            name: output_name.as_ptr(),
            channels: destinations.as_ptr(),
            channel_count: 2,
            frame_capacity: 8,
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
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, &input, 1, &output, 1, 8) },
            0
        );
        let wrong_direction_input = DandrumKernelInputBusView {
            name: output_name.as_ptr(),
            ..input
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, &wrong_direction_input, 1, &output, 1, 8) },
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
    fn kernel_ffi_routes_multiple_output_views_by_name_in_any_order() {
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
                name: positive_name.as_ptr(),
                channels: positive_channel.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
            },
            DandrumKernelOutputBusView {
                name: negative_name.as_ptr(),
                channels: negative_channel.as_ptr(),
                channel_count: 1,
                frame_capacity: 8,
            },
        ];

        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, views.as_ptr(), 2, 8) },
            8
        );
        assert_eq!(negative, [-0.5; 8]);
        assert_eq!(positive, [0.25; 8]);
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
        };
        assert_eq!(
            unsafe { dandrum_kernel_render(engine, std::ptr::null(), 0, &view, 1, 8) },
            8
        );
        assert_eq!(left, [0.25; 8]);
        for _ in 0..8 {
            assert!(unsafe { dandrum_kernel_note_on_at(engine, 60, 100, 0) });
        }
        assert!(!unsafe { dandrum_kernel_note_on_at(engine, 60, 100, 0) });
        assert!(unsafe { dandrum_kernel_reset(engine) });
        assert!(unsafe { dandrum_kernel_note_on_at(engine, 61, 100, 0) });
        assert!(!unsafe { dandrum_kernel_reset(std::ptr::null_mut()) });
        unsafe { dandrum_kernel_destroy(engine) };
    }

    #[test]
    fn kernel_ffi_poly_voice_starts_and_steals_at_the_event_sample() {
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
