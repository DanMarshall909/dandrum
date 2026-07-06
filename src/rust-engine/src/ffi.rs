use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::path::{Path, PathBuf};
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
    base_dir: PathBuf,
    public_values: BTreeMap<String, ParameterValue>,
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
        .insert(engine_key(engine), FfiLoadedInstrument::from_patch_path(&path, prepared.patch_doc()));

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
    mut_or!(engine, engine, false);

    let Some(parameter_id) = c_string(parameter_id) else {
        return false;
    };

    let mut registry = loaded_instruments()
        .lock()
        .expect("loaded instrument registry should not be poisoned");
    let Some(loaded) = registry.get_mut(&engine_key(engine)) else {
        return false;
    };
    let Some(value) = loaded.public_numeric_value(parameter_id, value) else {
        return false;
    };

    let mut candidate_values = loaded.public_values.clone();
    candidate_values.insert(parameter_id.to_string(), ParameterValue::Number(value));
    let effective_patch = effective_patch_for_values(&loaded.definition, &candidate_values);
    let Ok(prepared) = preparation::prepare_instrument_document(effective_patch, &loaded.base_dir) else {
        return false;
    };

    engine.load_patch_with_sampler_assets(prepared.patch_doc(), prepared.sampler_assets());
    loaded.public_values = candidate_values;

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
    static LOADED_INSTRUMENTS: OnceLock<Mutex<BTreeMap<usize, FfiLoadedInstrument>>> = OnceLock::new();
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

fn effective_patch_for_values(
    definition: &PatchDocument,
    values: &BTreeMap<String, ParameterValue>,
) -> PatchDocument {
    let mut effective_patch = definition.clone();

    for target in &definition.preset_surface.parameters {
        if let Some(value) = values.get(&target.name) {
            apply_public_parameter_value(&mut effective_patch, &target.maps_to, value.clone());
        }
    }

    effective_patch
}

fn apply_public_parameter_value(
    patch: &mut PatchDocument,
    destination: &PortReference,
    value: ParameterValue,
) {
    if let Some(module) = patch
        .modules
        .iter_mut()
        .find(|module| module.id == destination.module_id)
    {
        module
            .parameters
            .insert(destination.port_name.clone(), value);
    }
}

impl FfiLoadedInstrument {
    fn from_patch_path(path: &Path, patch_doc: &PatchDocument) -> Self {
        Self {
            definition: patch_doc.clone(),
            base_dir: path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
            public_values: patch_doc
                .preset_surface
                .parameters
                .iter()
                .map(|target| (target.name.clone(), target.default.clone()))
                .collect(),
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

    macro_rules! null_safety_no_panic_tests {
        ($( $name:ident => $call:expr; )*) => {
            $(
                #[test]
                fn $name() {
                    assert_no_panic(stringify!($name), || unsafe { $call });
                }
            )*
        };
    }

    macro_rules! null_safety_returning_tests {
        ($( $name:ident => $call:expr => $expected:expr; )*) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(unsafe { $call }, $expected);
                }
            )*
        };
    }

    #[test]
    fn c_ffi_create_returns_live_engine_pointer() {
        let engine = dandrum_engine_create();

        assert!(!engine.is_null());

        unsafe { dandrum_engine_destroy(engine) };
    }

    #[test]
    fn c_ffi_renders_public_numeric_parameter_descriptors_from_patch_path() {
        use std::io::Write;

        let mut path = std::env::temp_dir();
        path.push("dandrum_test_public_parameters.yaml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            "metadata:\n  name: Parameter Test\ninstrument:\n  id: dandrum.parameter-test\n  preset_schema_version: 1\npreset_surface:\n  parameters:\n    - name: tone.level\n      type: number\n      default: 0.5\n      min: 0\n      max: 1\n      maps_to: gain.gain\nrender:\n  sample_rate_hz: 48000\n  block_size_frames: 64\n  duration_frames: 128\nmodules:\n  - id: gain\n    type: gain\n  - id: out\n    type: audio_output"
        )
        .unwrap();
        drop(file);

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
        assert_eq!(unsafe { CStr::from_ptr(id.as_ptr()) }.to_str().unwrap(), "tone.level");
        assert_eq!(default_value, 0.5);
        assert_eq!(min_value, 0.0);
        assert_eq!(max_value, 1.0);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn c_ffi_public_numeric_parameter_update_rebuilds_runtime_from_retained_definition() {
        let path = write_parameterised_oscillator_patch("dandrum_test_runtime_parameter_patch.yaml");
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        let parameter_id = std::ffi::CString::new("osc.pitch").unwrap();
        let engine = dandrum_engine_create();

        assert!(unsafe { dandrum_engine_load_patch(engine, c_path.as_ptr()) });
        unsafe { dandrum_engine_prepare_realtime(engine, 48_000.0, 64) };

        let before = render_left_block(engine);
        assert!(unsafe {
            dandrum_engine_set_public_numeric_parameter(engine, parameter_id.as_ptr(), 2.0)
        });
        let after = render_left_block(engine);

        assert_ne!(before, after);

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
            "metadata:\n  name: Runtime Parameter Test\ninstrument:\n  id: dandrum.runtime-parameter-test\n  preset_schema_version: 1\npreset_surface:\n  parameters:\n    - name: osc.pitch\n      type: number\n      default: 1\n      min: 0.25\n      max: 4\n      maps_to: osc.pitch\nrender:\n  sample_rate_hz: 48000\n  block_size_frames: 64\n  duration_frames: 128\nmodules:\n  - id: osc\n    type: oscillator\n    outputs:\n      - name: audio\n        signal_type: audio\n  - id: out\n    type: audio_output\n    inputs:\n      - name: left\n        signal_type: audio\n      - name: right\n        signal_type: audio\nconnections:\n  - from: osc.audio\n    to: out.left\n  - from: osc.audio\n    to: out.right"
        )
        .unwrap();
        drop(file);
        path
    }

    fn render_left_block(engine: *mut crate::synth::DandrumEngine) -> Vec<f32> {
        let mut left = [0.0_f32; 16];
        let mut right = [0.0_f32; 16];
        let rendered = unsafe { dandrum_engine_render(engine, left.as_mut_ptr(), right.as_mut_ptr(), left.len()) };
        assert_eq!(rendered, left.len());
        left.to_vec()
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

    null_safety_no_panic_tests! {
        c_ffi_destroy_null_engine_does_not_crash => {
            dandrum_engine_destroy(std::ptr::null_mut())
        };
        c_ffi_prepare_null_engine_does_not_crash => {
            dandrum_engine_prepare(std::ptr::null_mut(), 48_000.0)
        };
        c_ffi_prepare_realtime_null_engine_does_not_crash => {
            dandrum_engine_prepare_realtime(std::ptr::null_mut(), 48_000.0, 64)
        };
        c_ffi_note_on_null_engine_does_not_crash => {
            dandrum_engine_note_on(std::ptr::null_mut(), 60, 100)
        };
        c_ffi_note_off_null_engine_does_not_crash => {
            dandrum_engine_note_off(std::ptr::null_mut(), 60)
        };
        c_ffi_realtime_event_queue_destroy_null_does_not_crash => {
            dandrum_realtime_event_queue_destroy(std::ptr::null_mut())
        };
    }

    null_safety_returning_tests! {
        c_ffi_load_patch_rejects_null_engine => {
            dandrum_engine_load_patch(std::ptr::null_mut(), std::ptr::null())
        } => false;
        c_ffi_load_patch_rejects_null_path => {
            let engine = dandrum_engine_create();
            let result = dandrum_engine_load_patch(engine, std::ptr::null());
            dandrum_engine_destroy(engine);
            result
        } => false;
        c_ffi_public_numeric_parameter_rejects_null_engine => {
            dandrum_engine_set_public_numeric_parameter(std::ptr::null_mut(), std::ptr::null(), 1.0)
        } => false;
        c_ffi_public_numeric_parameter_rejects_null_id => {
            let engine = dandrum_engine_create();
            let result = dandrum_engine_set_public_numeric_parameter(engine, std::ptr::null(), 1.0);
            dandrum_engine_destroy(engine);
            result
        } => false;
        c_ffi_is_finished_returns_true_for_null_engine => {
            dandrum_engine_is_finished(std::ptr::null())
        } => true;
        c_ffi_realtime_event_queue_note_on_rejects_null_queue => {
            dandrum_realtime_event_queue_note_on(std::ptr::null_mut(), 60, 100)
        } => 1;
        c_ffi_realtime_event_queue_note_off_rejects_null_queue => {
            dandrum_realtime_event_queue_note_off(std::ptr::null_mut(), 60)
        } => 1;
        c_ffi_realtime_event_queue_dropped_count_returns_zero_for_null_queue => {
            dandrum_realtime_event_queue_dropped_count(std::ptr::null())
        } => 0;
    }

    #[test]
    fn c_ffi_engine_lifecycle_create_prepare_note_on_render_is_finished() {
        let engine = dandrum_engine_create();
        assert!(!engine.is_null());

        unsafe { dandrum_engine_prepare(engine, 44_100.0) };
        unsafe { dandrum_engine_note_on(engine, 60, 100) };

        assert!(!unsafe { dandrum_engine_is_finished(engine) });

        let mut left = [0.0_f32; 64];
        let mut right = [0.0_f32; 64];
        let rendered =
            unsafe { dandrum_engine_render(engine, left.as_mut_ptr(), right.as_mut_ptr(), 64) };

        assert_eq!(rendered, 64);
        assert!(left.iter().any(|s| *s != 0.0));
        assert!(right.iter().any(|s| *s != 0.0));

        unsafe { dandrum_engine_destroy(engine) };
    }

    #[test]
    fn c_ffi_engine_starts_finished() {
        let engine = dandrum_engine_create();
        assert!(unsafe { dandrum_engine_is_finished(engine) });
        unsafe { dandrum_engine_destroy(engine) };
    }

    #[test]
    fn c_ffi_load_patch_fails_for_non_existent_path() {
        let engine = dandrum_engine_create();
        let path = std::ffi::CString::new("/nonexistent/patch.yaml").unwrap();

        assert!(!unsafe { dandrum_engine_load_patch(engine, path.as_ptr()) });

        unsafe { dandrum_engine_destroy(engine) };
    }

    #[test]
    fn c_ffi_load_patch_fails_and_preserves_fallback_render_after_attempt() {
        use std::io::Write;

        let engine = dandrum_engine_create();
        unsafe { dandrum_engine_prepare(engine, 44_100.0) };
        unsafe { dandrum_engine_note_on(engine, 60, 100) };

        let mut dir = std::env::temp_dir();
        dir.push("dandrum_test_bad_patch.yaml");
        let mut file = std::fs::File::create(&dir).unwrap();
        writeln!(
            file,
            "metadata:\n  name: Bad\nrender:\n  sample_rate_hz: 48000\n  block_size_frames: 64\n  duration_frames: 128\nmodules: []"
        )
            .unwrap();
        drop(file);

        let bad_path = std::ffi::CString::new(dir.to_str().unwrap().as_bytes()).unwrap();
        assert!(
            !unsafe { dandrum_engine_load_patch(engine, bad_path.as_ptr()) },
            "empty modules should fail graph validation"
        );

        let mut left = [0.0_f32; 64];
        let mut right = [0.0_f32; 64];
        let rendered =
            unsafe { dandrum_engine_render(engine, left.as_mut_ptr(), right.as_mut_ptr(), 64) };

        assert_eq!(rendered, 64);
        assert!(
            left.iter().any(|s| *s != 0.0),
            "fallback synth should still produce audio after failed load"
        );

        std::fs::remove_file(&dir).ok();
        unsafe { dandrum_engine_destroy(engine) };
    }

    #[test]
    fn c_ffi_load_patch_fails_for_empty_patch_and_still_renders_fallback() {
        use std::io::Write;

        let engine = dandrum_engine_create();
        unsafe { dandrum_engine_prepare(engine, 44_100.0) };
        unsafe { dandrum_engine_note_on(engine, 60, 100) };

        let mut dir = std::env::temp_dir();
        dir.push("dandrum_test_empty_patch.yaml");
        let mut file = std::fs::File::create(&dir).unwrap();
        writeln!(file, "").unwrap();
        drop(file);

        let path = std::ffi::CString::new(dir.to_str().unwrap().as_bytes()).unwrap();
        assert!(!unsafe { dandrum_engine_load_patch(engine, path.as_ptr()) });

        let mut left = [0.0_f32; 64];
        let mut right = [0.0_f32; 64];
        let rendered =
            unsafe { dandrum_engine_render(engine, left.as_mut_ptr(), right.as_mut_ptr(), 64) };

        assert_eq!(rendered, 64);
        assert!(
            left.iter().any(|s| *s != 0.0),
            "fallback synth should still produce audio after empty patch load attempt"
        );

        std::fs::remove_file(&dir).ok();
        unsafe { dandrum_engine_destroy(engine) };
    }
}
