pub mod core;

pub mod graph;

pub mod cli;

pub mod patch;

pub mod synth;

pub use synth::DandrumEngine;

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
