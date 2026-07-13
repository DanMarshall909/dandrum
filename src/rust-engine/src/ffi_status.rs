use std::ffi::{CStr, c_char};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

fn last_error() -> &'static Mutex<String> {
    static LAST_ERROR: OnceLock<Mutex<String>> = OnceLock::new();
    LAST_ERROR.get_or_init(|| Mutex::new(String::new()))
}

fn set_last_error(message: impl Into<String>) {
    *last_error()
        .lock()
        .expect("last FFI error should not be poisoned") = message.into();
}

fn clear_last_error() {
    set_last_error(String::new());
}

fn c_path(path: *const c_char) -> Option<PathBuf> {
    if path.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(path) }
        .to_str()
        .ok()
        .map(PathBuf::from)
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

/// Plugin-facing load entrypoint that preserves the last structured Rust
/// preparation error for non-audio-thread status queries.
///
/// The existing `dandrum_engine_load_patch` function is still used for the
/// actual registry-populating load once validation succeeds, so all public
/// parameter slot APIs continue to see the same loaded-instrument registry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_load_patch_with_error(
    engine: *mut crate::synth::DandrumEngine,
    path: *const c_char,
) -> bool {
    if engine.is_null() {
        set_last_error("engine pointer was null");
        return false;
    }

    let Some(path_buf) = c_path(path) else {
        set_last_error("patch path was null or not valid UTF-8");
        return false;
    };

    match crate::preparation::prepare_instrument_file(&path_buf) {
        Ok(_) => {
            let loaded = unsafe { crate::ffi::dandrum_engine_load_patch(engine, path) };
            if loaded {
                clear_last_error();
            } else {
                set_last_error(
                    "prepared instrument could not be published through the legacy FFI load path",
                );
            }
            loaded
        }
        Err(error) => {
            set_last_error(error.to_diagnostics().to_string());
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_engine_last_error_message(
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let message = last_error()
        .lock()
        .expect("last FFI error should not be poisoned")
        .clone();

    copy_string_to_c_buffer(&message, buffer, buffer_capacity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn load_patch_with_error_preserves_structured_error_for_status_queries() {
        let engine = crate::ffi::dandrum_engine_create();
        let missing = CString::new("/definitely/missing/dandrum.yaml").unwrap();

        assert!(!unsafe { dandrum_engine_load_patch_with_error(engine, missing.as_ptr()) });

        let mut buffer = [0_i8; 512];
        assert!(unsafe { dandrum_engine_last_error_message(buffer.as_mut_ptr(), buffer.len()) });
        let message = unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_str()
            .expect("error message should be UTF-8");

        assert!(
            message.contains("LOADING")
                || message.contains("failed")
                || message.contains("No such file")
        );

        unsafe { crate::ffi::dandrum_engine_destroy(engine) };
    }
}
