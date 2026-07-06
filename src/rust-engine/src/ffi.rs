use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::patch::{ParameterValue, PatchDocument, PortReference, PresetTargetType};
use crate::preparation;
use crate::realtime;

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

struct FfiLoadedInstrument {
    definition: PatchDocument,
    public_values: BTreeMap<String, ParameterValue>,
    public_bindings: BTreeMap<String, Vec<PublicParameterBinding>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PublicParameterBinding {
    module_id: String,
    parameter_name: String,
}

#[unsafe(no_mangle)]
pub extern "C" fn dandrum_engine_create() -> *mut crate::synth::DandrumEngine {
    Box::into_raw(Box::new(crate::synth::DandrumEngine::new()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_destroy(engine: *mut crate::synth::DandrumEngine) {
    if !engine.is_null() {
        loaded_instruments()
            .lock()
            .expect("loaded instrument registry should not be poisoned")
            .remove(&engine_key(engine));
        drop(unsafe { Box::from_raw(engine) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_load_patch(
    engine: *mut crate::synth::DandrumEngine,
    path: *const c_char,
) -> bool {
    let engine_key = engine_key(engine);
    mut_or!(engine, engine, false);

    let Some(path) = c_path(path) else {
        return false;
    };

    let Ok(prepared) = preparation::prepare_instrument_file(&path) else {
        return false;
    };

    engine.load_patch_with_sampler_assets(prepared.patch_doc(), prepared.sampler_assets());
    loaded_instruments()
        .lock()
        .expect("loaded instrument registry should not be poisoned")
        .insert(engine_key, FfiLoadedInstrument::from_patch(prepared.patch_doc()));

    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_patch_public_numeric_parameter_count(path: *const c_char) -> usize {
    let Some(path) = c_path(path) else {
        return 0;
    };
    let Ok(patch) = crate::patch::load_patch_file(&path) else {
        return 0;
    };

    patch
        .preset_surface
        .parameters
        .iter()
        .filter(|target| is_numeric_target(target.value_type, &target.default))
        .count()
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
    let Ok(patch) = crate::patch::load_patch_file(&path) else {
        return false;
    };
    let Some(target) = patch
        .preset_surface
        .parameters
        .iter()
        .filter(|target| is_numeric_target(target.value_type, &target.default))
        .nth(index)
    else {
        return false;
    };
    let Some(default) = number_value(&target.default) else {
        return false;
    };

    if default_value.is_null() || min_value.is_null() || max_value.is_null() {
        return false;
    }

    unsafe {
        *default_value = default;
        *min_value = target.min.unwrap_or(0.0);
        *max_value = target.max.unwrap_or(1.0);
    }

    copy_string_to_c_buffer(&target.name, id_buffer, id_buffer_capacity)
        && copy_string_to_c_buffer(&target.name, name_buffer, name_buffer_capacity)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_set_public_numeric_parameter(
    engine: *mut crate::synth::DandrumEngine,
    parameter_id: *const c_char,
    value: f64,
) -> bool {
    let engine_key = engine_key(engine);
    mut_or!(engine, engine, false);

    let Some(parameter_id) = c_string(parameter_id) else {
        return false;
    };

    let mut registry = loaded_instruments()
        .lock()
        .expect("loaded instrument registry should not be poisoned");
    let Some(loaded) = registry.get_mut(&engine_key) else {
        return false;
    };
    let Some(value) = loaded.public_numeric_value(parameter_id, value) else {
        return false;
    };
    let Some(bindings) = loaded.public_bindings.get(parameter_id).cloned() else {
        return false;
    };

    let mut applied = false;
    for binding in bindings {
        applied |= engine.set_numeric_parameter_by_target(
            &binding.module_id,
            &binding.parameter_name,
            value as f32,
        );
    }

    if !applied {
        return false;
    }

    loaded
        .public_values
        .insert(parameter_id.to_string(), ParameterValue::Number(value));

    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_prepare(
    engine: *mut crate::synth::DandrumEngine,
    sample_rate: f32,
) {
    mut_or!(engine, engine, ());

    engine.prepare(sample_rate);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_prepare_realtime(
    engine: *mut crate::synth::DandrumEngine,
    sample_rate: f32,
    max_block_size: usize,
) {
    mut_or!(engine, engine, ());

    engine.prepare_realtime(sample_rate, max_block_size);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_on(
    engine: *mut crate::synth::DandrumEngine,
    note: u8,
    velocity: u8,
) {
    mut_or!(engine, engine, ());

    engine.note_on(note, velocity);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_on_at(
    engine: *mut crate::synth::DandrumEngine,
    note: u8,
    velocity: u8,
    frame_offset: usize,
) {
    mut_or!(engine, engine, ());

    engine.note_on_at(note, velocity, frame_offset as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_off_at(
    engine: *mut crate::synth::DandrumEngine,
    note: u8,
    frame_offset: usize,
) {
    mut_or!(engine, engine, ());

    engine.note_off_at(note, frame_offset as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_off(
    engine: *mut crate::synth::DandrumEngine,
    note: u8,
) {
    mut_or!(engine, engine, ());

    engine.note_off(note);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_render(
    engine: *mut crate::synth::DandrumEngine,
    left: *mut f32,
    right: *mut f32,
    num_samples: usize,
) -> usize {
    mut_or!(engine, engine, 0);

    if left.is_null() || right.is_null() {
        return 0;
    }

    let left = unsafe { std::slice::from_raw_parts_mut(left, num_samples) };
    let right = unsafe { std::slice::from_raw_parts_mut(right, num_samples) };

    engine.render(left, right)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_is_finished(
    engine: *const crate::synth::DandrumEngine,
) -> bool {
    ref_or!(engine, engine, true);

    engine.is_finished()
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

fn loaded_instruments() -> &'static Mutex<BTreeMap<usize, FfiLoadedInstrument>> {
    static LOADED_INSTRUMENTS: OnceLock<Mutex<BTreeMap<usize, FfiLoadedInstrument>>> =
        OnceLock::new();
    LOADED_INSTRUMENTS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn engine_key(engine: *const crate::synth::DandrumEngine) -> usize {
    engine as usize
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

fn is_numeric_target(value_type: PresetTargetType, default: &ParameterValue) -> bool {
    matches!(value_type, PresetTargetType::Number | PresetTargetType::Integer)
        && number_value(default).is_some()
}

fn number_value(value: &ParameterValue) -> Option<f64> {
    match value {
        ParameterValue::Number(value) => Some(*value),
        _ => None,
    }
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

impl FfiLoadedInstrument {
    fn from_patch(patch_doc: &PatchDocument) -> Self {
        Self {
            definition: patch_doc.clone(),
            public_values: patch_doc
                .preset_surface
                .parameters
                .iter()
                .map(|target| (target.name.clone(), target.default.clone()))
                .collect(),
            public_bindings: public_parameter_bindings_by_id(patch_doc),
        }
    }

    fn public_numeric_value(&self, parameter_id: &str, value: f64) -> Option<f64> {
        let target = self
            .definition
            .preset_surface
            .parameters
            .iter()
            .find(|target| target.name == parameter_id)?;

        if !is_numeric_target(target.value_type, &target.default) {
            return None;
        }

        Some(clamp_public_value(value, target.min, target.max))
    }
}

fn public_parameter_bindings_by_id(
    patch: &PatchDocument,
) -> BTreeMap<String, Vec<PublicParameterBinding>> {
    patch
        .preset_surface
        .parameters
        .iter()
        .map(|target| {
            (
                target.name.clone(),
                expand_public_parameter_target(patch, &target.maps_to),
            )
        })
        .collect()
}

fn expand_public_parameter_target(
    patch: &PatchDocument,
    target: &PortReference,
) -> Vec<PublicParameterBinding> {
    let Some(module) = patch
        .modules
        .iter()
        .find(|module| module.id == target.module_id)
    else {
        return vec![binding(&target.module_id, &target.port_name)];
    };

    let Some(definition) = patch
        .module_definitions
        .iter()
        .find(|definition| definition.module_type == module.module_type)
    else {
        return vec![binding(&target.module_id, &target.port_name)];
    };

    let Some(parameter) = definition
        .parameters
        .iter()
        .find(|parameter| parameter.name == target.port_name)
    else {
        return vec![binding(&target.module_id, &target.port_name)];
    };

    parameter
        .maps_to
        .iter()
        .map(|mapped| {
            binding(
                &format!("{}::{}", target.module_id, mapped.module_id),
                &mapped.port_name,
            )
        })
        .collect()
}

fn binding(module_id: &str, parameter_name: &str) -> PublicParameterBinding {
    PublicParameterBinding {
        module_id: module_id.to_string(),
        parameter_name: parameter_name.to_string(),
    }
}

fn clamp_public_value(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    match (min, max) {
        (Some(min), Some(max)) if min <= max => value.clamp(min, max),
        (Some(min), _) => value.max(min),
        (_, Some(max)) => value.min(max),
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_no_panic(name: &str, f: impl FnOnce()) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        assert!(result.is_ok(), "{name}");
    }

    #[test]
    fn c_ffi_create_returns_live_engine_pointer() {
        let engine = dandrum_engine_create();

        assert!(!engine.is_null());

        unsafe { dandrum_engine_destroy(engine) };
    }

    #[test]
    fn c_ffi_renders_public_numeric_parameter_descriptors_from_patch_path() {
        let path = write_parameterised_oscillator_patch("dandrum_test_public_parameters.yaml");
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { dandrum_patch_public_numeric_parameter_count(c_path.as_ptr()) }, 1);

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

        assert!(result);
        assert_eq!(unsafe { CStr::from_ptr(id.as_ptr()) }.to_str().unwrap(), "osc.pitch");
        assert_eq!(default_value, 1.0);
        assert_eq!(min_value, 0.25);
        assert_eq!(max_value, 4.0);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn c_ffi_public_numeric_parameter_update_writes_runtime_slot() {
        let path = write_parameterised_oscillator_patch("dandrum_test_runtime_parameter_patch.yaml");
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let parameter_id = std::ffi::CString::new("osc.pitch").unwrap();
        let engine = dandrum_engine_create();

        assert!(unsafe { dandrum_engine_load_patch(engine, c_path.as_ptr()) });
        assert_eq!(unsafe { (*engine).numeric_parameter_value("osc", "pitch") }, Some(1.0));
        assert!(unsafe {
            dandrum_engine_set_public_numeric_parameter(engine, parameter_id.as_ptr(), 2.0)
        });
        assert_eq!(unsafe { (*engine).numeric_parameter_value("osc", "pitch") }, Some(2.0));

        unsafe { dandrum_engine_destroy(engine) };
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn c_ffi_public_numeric_parameter_update_rejects_unknown_parameter() {
        let path = write_parameterised_oscillator_patch("dandrum_test_unknown_parameter_patch.yaml");
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let parameter_id = std::ffi::CString::new("missing.parameter").unwrap();
        let engine = dandrum_engine_create();

        assert!(unsafe { dandrum_engine_load_patch(engine, c_path.as_ptr()) });
        assert!(!unsafe {
            dandrum_engine_set_public_numeric_parameter(engine, parameter_id.as_ptr(), 2.0)
        });

        unsafe { dandrum_engine_destroy(engine) };
        std::fs::remove_file(path).ok();
    }

    fn write_parameterised_oscillator_patch(file_name: &str) -> PathBuf {
        use std::io::Write;

        let mut path = std::env::temp_dir();
        path.push(file_name);
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            "metadata:\n  name: Runtime Parameter Test\ninstrument:\n  id: dandrum.runtime-parameter-test\n  preset_schema_version: 1\npreset_surface:\n  parameters:\n    - name: osc.pitch\n      type: number\n      default: 1\n      min: 0.25\n      max: 4\n      maps_to: osc.pitch\nrender:\n  sample_rate_hz: 48000\n  block_size_frames: 64\n  duration_frames: 128\nmodules:\n  - id: osc\n    type: oscillator\n    parameters:\n      pitch: 1\n    outputs:\n      - name: audio\n        signal_type: audio\n  - id: out\n    type: audio_output\n    inputs:\n      - name: left\n        signal_type: audio\n      - name: right\n        signal_type: audio\nconnections:\n  - from: osc.audio\n    to: out.left\n  - from: osc.audio\n    to: out.right"
        )
        .unwrap();
        drop(file);
        path
    }

    #[test]
    fn c_ffi_render_rejects_null_engine_and_buffers() {
        let mut left = [0.0_f32; 8];
        let mut right = [0.0_f32; 8];

        assert_eq!(
            unsafe {
                dandrum_engine_render(
                    std::ptr::null_mut(),
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    8,
                )
            },
            0
        );

        let engine = dandrum_engine_create();
        assert_eq!(
            unsafe { dandrum_engine_render(engine, std::ptr::null_mut(), right.as_mut_ptr(), 8) },
            0
        );
        assert_eq!(
            unsafe { dandrum_engine_render(engine, left.as_mut_ptr(), std::ptr::null_mut(), 8) },
            0
        );

        unsafe { dandrum_engine_destroy(engine) };
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
            dandrum_engine_destroy(std::ptr::null_mut())
        });
        assert_no_panic("prepare null", || unsafe {
            dandrum_engine_prepare(std::ptr::null_mut(), 48_000.0)
        });
        assert!(!unsafe { dandrum_engine_load_patch(std::ptr::null_mut(), std::ptr::null()) });
        assert!(!unsafe {
            dandrum_engine_set_public_numeric_parameter(std::ptr::null_mut(), std::ptr::null(), 1.0)
        });
        assert!(unsafe { dandrum_engine_is_finished(std::ptr::null()) });
    }
}
