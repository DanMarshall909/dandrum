use crate::realtime;

pub struct DandrumRealtimeEventQueue {
    queue: realtime::RealtimeEventQueue,
}

#[unsafe(no_mangle)]
pub extern "C" fn dandrum_engine_create() -> *mut crate::synth::DandrumEngine {
    Box::into_raw(Box::new(crate::synth::DandrumEngine::new()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_destroy(engine: *mut crate::synth::DandrumEngine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_load_patch(
    engine: *mut crate::synth::DandrumEngine,
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
pub unsafe extern "C" fn dandrum_engine_prepare(
    engine: *mut crate::synth::DandrumEngine,
    sample_rate: f32,
) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.prepare(sample_rate);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_prepare_realtime(
    engine: *mut crate::synth::DandrumEngine,
    sample_rate: f32,
    max_block_size: usize,
) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.prepare_realtime(sample_rate, max_block_size);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_on(
    engine: *mut crate::synth::DandrumEngine,
    note: u8,
    velocity: u8,
) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.note_on(note, velocity);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_note_off(
    engine: *mut crate::synth::DandrumEngine,
    note: u8,
) {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return;
    };

    engine.note_off(note);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_render(
    engine: *mut crate::synth::DandrumEngine,
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
pub unsafe extern "C" fn dandrum_engine_is_finished(
    engine: *const crate::synth::DandrumEngine,
) -> bool {
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
    fn c_ffi_destroy_null_engine_does_not_crash() {
        unsafe { dandrum_engine_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn c_ffi_load_patch_rejects_null_engine() {
        assert!(!unsafe { dandrum_engine_load_patch(std::ptr::null_mut(), std::ptr::null()) });
    }

    #[test]
    fn c_ffi_prepare_null_engine_does_not_crash() {
        unsafe { dandrum_engine_prepare(std::ptr::null_mut(), 48_000.0) };
    }

    #[test]
    fn c_ffi_prepare_realtime_null_engine_does_not_crash() {
        unsafe { dandrum_engine_prepare_realtime(std::ptr::null_mut(), 48_000.0, 64) };
    }

    #[test]
    fn c_ffi_note_on_null_engine_does_not_crash() {
        unsafe { dandrum_engine_note_on(std::ptr::null_mut(), 60, 100) };
    }

    #[test]
    fn c_ffi_note_off_null_engine_does_not_crash() {
        unsafe { dandrum_engine_note_off(std::ptr::null_mut(), 60) };
    }

    #[test]
    fn c_ffi_is_finished_returns_true_for_null_engine() {
        assert!(unsafe { dandrum_engine_is_finished(std::ptr::null()) });
    }

    #[test]
    fn c_ffi_realtime_event_queue_destroy_null_does_not_crash() {
        unsafe { dandrum_realtime_event_queue_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn c_ffi_realtime_event_queue_note_on_rejects_null_queue() {
        assert_eq!(
            unsafe { dandrum_realtime_event_queue_note_on(std::ptr::null_mut(), 60, 100) },
            1
        );
    }

    #[test]
    fn c_ffi_realtime_event_queue_note_off_rejects_null_queue() {
        assert_eq!(
            unsafe { dandrum_realtime_event_queue_note_off(std::ptr::null_mut(), 60) },
            1
        );
    }

    #[test]
    fn c_ffi_realtime_event_queue_dropped_count_returns_zero_for_null_queue() {
        assert_eq!(
            unsafe { dandrum_realtime_event_queue_dropped_count(std::ptr::null()) },
            0
        );
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
        let rendered = unsafe {
            dandrum_engine_render(engine, left.as_mut_ptr(), right.as_mut_ptr(), 64)
        };

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
        let rendered = unsafe {
            dandrum_engine_render(engine, left.as_mut_ptr(), right.as_mut_ptr(), 64)
        };

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
        let rendered = unsafe {
            dandrum_engine_render(engine, left.as_mut_ptr(), right.as_mut_ptr(), 64)
        };

        assert_eq!(rendered, 64);
        assert!(
            left.iter().any(|s| *s != 0.0),
            "fallback synth should still produce audio after empty patch load attempt"
        );

        std::fs::remove_file(&dir).ok();
        unsafe { dandrum_engine_destroy(engine) };
    }
}
