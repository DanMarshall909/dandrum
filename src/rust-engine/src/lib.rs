pub mod core;

pub mod compiled_patch;

pub mod graph_processor;

pub mod builtins;

pub mod graph;

pub mod cli;

pub mod patch;

pub mod script;

pub mod sample;

pub mod synth;

pub mod wav;

pub mod voice_allocator;

pub mod fft;

pub mod filter;
pub mod delay_line;
pub mod echo;

pub mod realtime;

pub mod crossover;

pub mod spectral;

pub mod envelope_detector;

pub mod audio_loading;

pub mod dynamics_processor;
pub mod saturator;
pub mod convolution;

pub use synth::DandrumEngine;

pub struct DandrumRealtimeEventQueue {
    queue: realtime::RealtimeEventQueue,
}

#[unsafe(no_mangle)]
pub extern "C" fn dandrum_engine_create() -> *mut DandrumEngine {
    Box::into_raw(Box::new(DandrumEngine::new()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_destroy(engine: *mut DandrumEngine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_load_patch(
    engine: *mut DandrumEngine,
    path: *const std::ffi::c_char,
) -> bool {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return false;
    };

    let c_str = match unsafe { std::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let patch_path = std::path::Path::new(c_str);
    let patch_doc = match crate::patch::load_patch_file(patch_path) {
        Ok(doc) => match crate::patch::validate_patch_schema(&doc) {
            Ok(_) => doc,
            Err(_) => return false,
        },
        Err(_) => return false,
    };

    let graph = crate::graph::Graph::from_patch_declarations(&patch_doc);
    if graph.validate().is_err() {
        return false;
    }

    let base_dir = patch_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let sampler_assets = match crate::sample::prepare_sampler_assets(&patch_doc, base_dir) {
        Ok(assets) => assets,
        Err(_) => return false,
    };

    engine.load_patch_with_sampler_assets(&patch_doc, &sampler_assets);
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_prepare(engine: *mut DandrumEngine, sample_rate: f32) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.prepare(sample_rate);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_on(
    engine: *mut DandrumEngine,
    note: u8,
    velocity: u8,
) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.note_on(note, velocity);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_off(engine: *mut DandrumEngine, note: u8) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.note_off(note);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_render(
    engine: *mut DandrumEngine,
    left: *mut f32,
    right: *mut f32,
    num_samples: usize,
) -> usize {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return 0;
    };

    if left.is_null() || right.is_null() {
        return 0;
    }

    let left = unsafe { std::slice::from_raw_parts_mut(left, num_samples) };
    let right = unsafe { std::slice::from_raw_parts_mut(right, num_samples) };

    engine.render(left, right)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_is_finished(engine: *const DandrumEngine) -> bool {
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return true;
    };

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
    let Some(queue) = (unsafe { queue.as_ref() }) else {
        return 0;
    };

    queue.queue.dropped_events()
}

fn submit_realtime_queue_event(
    queue: *mut DandrumRealtimeEventQueue,
    event: realtime::RealtimeEvent,
) -> u8 {
    let Some(queue) = (unsafe { queue.as_mut() }) else {
        return 1;
    };

    match queue.queue.submit(event) {
        realtime::RealtimeEventSubmitStatus::Accepted => 0,
        realtime::RealtimeEventSubmitStatus::Dropped => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_ffi_create_returns_live_engine_pointer() {
        let engine = dandrum_engine_create();

        assert!(!engine.is_null());

        unsafe { dandrum_engine_destroy(engine) };
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
        assert_eq!(unsafe { dandrum_realtime_event_queue_note_on(queue, 60, 100) }, 0);
        assert_eq!(unsafe { dandrum_realtime_event_queue_note_off(queue, 60) }, 1);
        assert_eq!(unsafe { dandrum_realtime_event_queue_dropped_count(queue) }, 1);

        unsafe { dandrum_realtime_event_queue_destroy(queue) };
    }
}
