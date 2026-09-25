use std::ffi::{CStr, c_char};

use crate::sound_analysis::AnalysisFrame;

/// One immutable offline fixture render shared across the C++ editor bridge.
///
/// The handle deliberately owns both the analysis frames and the encoded WAV
/// so callers cannot accidentally display metrics from a different render.
pub struct DandrumSoundFixtureRender {
    result: Result<SoundFixtureArtifact, String>,
}

struct SoundFixtureArtifact {
    sample_rate_hz: u32,
    duration_frames: u64,
    metrics: Vec<AnalysisFrame>,
    wav_bytes: Vec<u8>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_create(
    fixture_path: *const c_char,
) -> *mut DandrumSoundFixtureRender {
    if fixture_path.is_null() {
        return std::ptr::null_mut();
    }

    let result = unsafe { CStr::from_ptr(fixture_path) }
        .to_str()
        .map_err(|_| "sound fixture path was not valid UTF-8".to_string())
        .and_then(render_fixture_artifact);
    Box::into_raw(Box::new(DandrumSoundFixtureRender { result }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_destroy(
    render: *mut DandrumSoundFixtureRender,
) {
    if !render.is_null() {
        drop(unsafe { Box::from_raw(render) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_is_ok(
    render: *const DandrumSoundFixtureRender,
) -> bool {
    unsafe { render.as_ref() }.is_some_and(|render| render.result.is_ok())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_error_message(
    render: *const DandrumSoundFixtureRender,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let Some(render) = (unsafe { render.as_ref() }) else {
        return false;
    };
    let message = render.result.as_ref().err().map_or("", String::as_str);
    copy_string_to_c_buffer(message, buffer, buffer_capacity)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_sample_rate_hz(
    render: *const DandrumSoundFixtureRender,
) -> u32 {
    unsafe { artifact(render) }.map_or(0, |artifact| artifact.sample_rate_hz)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_duration_frames(
    render: *const DandrumSoundFixtureRender,
) -> u64 {
    unsafe { artifact(render) }.map_or(0, |artifact| artifact.duration_frames)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_metric_count(
    render: *const DandrumSoundFixtureRender,
) -> usize {
    unsafe { artifact(render) }.map_or(0, |artifact| artifact.metrics.len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_metric(
    render: *const DandrumSoundFixtureRender,
    index: usize,
    time_seconds: *mut f64,
    rms: *mut f64,
    peak: *mut f64,
    spectral_centroid_hz: *mut f64,
    has_spectral_centroid: *mut bool,
) -> bool {
    if time_seconds.is_null()
        || rms.is_null()
        || peak.is_null()
        || spectral_centroid_hz.is_null()
        || has_spectral_centroid.is_null()
    {
        return false;
    }
    let Some(frame) =
        (unsafe { artifact(render) }).and_then(|artifact| artifact.metrics.get(index))
    else {
        return false;
    };

    unsafe {
        *time_seconds = frame.time_seconds;
        *rms = frame.rms;
        *peak = frame.peak;
        *has_spectral_centroid = frame.spectral_centroid_hz.is_some();
        *spectral_centroid_hz = frame.spectral_centroid_hz.unwrap_or(0.0);
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_wav_size(
    render: *const DandrumSoundFixtureRender,
) -> usize {
    unsafe { artifact(render) }.map_or(0, |artifact| artifact.wav_bytes.len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_fixture_render_copy_wav(
    render: *const DandrumSoundFixtureRender,
    buffer: *mut u8,
    buffer_capacity: usize,
) -> bool {
    let Some(artifact) = (unsafe { artifact(render) }) else {
        return false;
    };
    if buffer.is_null() || buffer_capacity < artifact.wav_bytes.len() {
        return false;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            artifact.wav_bytes.as_ptr(),
            buffer,
            artifact.wav_bytes.len(),
        );
    }
    true
}

fn render_fixture_artifact(path: &str) -> Result<SoundFixtureArtifact, String> {
    let fixture = crate::sound_workbench::load_sound_fixture_file(path)?;
    let render = crate::sound_workbench::render_sound_fixture(&fixture)?;
    let mut wav_bytes = Vec::new();
    crate::wav::write_wav_stereo_i16(
        &mut wav_bytes,
        render.sample_rate_hz,
        &render.left,
        &render.right,
    )
    .expect("writing a WAV to an in-memory byte vector cannot fail");

    Ok(SoundFixtureArtifact {
        sample_rate_hz: render.sample_rate_hz,
        duration_frames: render.left.len() as u64,
        metrics: render.metrics,
        wav_bytes,
    })
}

unsafe fn artifact<'a>(
    render: *const DandrumSoundFixtureRender,
) -> Option<&'a SoundFixtureArtifact> {
    unsafe { render.as_ref() }?.result.as_ref().ok()
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
    use std::ffi::{CStr, CString};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn acid_fixture_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/sound-design/tb303-acid-poc.yaml")
    }

    #[test]
    fn ffi_render_handle_owns_one_coherent_acid_fixture_artifact() {
        let path = CString::new(acid_fixture_path().to_string_lossy().as_bytes()).unwrap();
        let render = unsafe { dandrum_sound_fixture_render_create(path.as_ptr()) };

        assert!(!render.is_null());
        assert!(unsafe { dandrum_sound_fixture_render_is_ok(render) });
        assert_eq!(
            unsafe { dandrum_sound_fixture_render_sample_rate_hz(render) },
            48_000
        );
        assert_eq!(
            unsafe { dandrum_sound_fixture_render_duration_frames(render) },
            528_000
        );

        let metric_count = unsafe { dandrum_sound_fixture_render_metric_count(render) };
        assert!(metric_count > 4_000);
        let mut time_seconds = 0.0;
        let mut rms = 0.0;
        let mut peak = 0.0;
        let mut centroid_hz = 0.0;
        let mut has_centroid = false;
        assert!(unsafe {
            dandrum_sound_fixture_render_metric(
                render,
                metric_count / 2,
                &mut time_seconds,
                &mut rms,
                &mut peak,
                &mut centroid_hz,
                &mut has_centroid,
            )
        });
        assert!(time_seconds.is_finite() && time_seconds > 0.0);
        assert!(rms.is_finite() && rms >= 0.0);
        assert!(peak.is_finite() && peak >= rms);
        assert!(!has_centroid || centroid_hz.is_finite());

        let wav_size = unsafe { dandrum_sound_fixture_render_wav_size(render) };
        assert!(wav_size > 44);
        let mut wav = vec![0_u8; wav_size];
        assert!(unsafe {
            dandrum_sound_fixture_render_copy_wav(render, wav.as_mut_ptr(), wav.len())
        });
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert!(!unsafe {
            dandrum_sound_fixture_render_copy_wav(render, wav.as_mut_ptr(), wav.len() - 1)
        });
        assert!(!unsafe {
            dandrum_sound_fixture_render_metric(
                render,
                metric_count,
                &mut time_seconds,
                &mut rms,
                &mut peak,
                &mut centroid_hz,
                &mut has_centroid,
            )
        });

        unsafe { dandrum_sound_fixture_render_destroy(render) };
    }

    #[test]
    fn ffi_render_handle_preserves_errors_and_rejects_invalid_access() {
        let missing = CString::new("/definitely/missing/dandrum-sound-fixture.yaml").unwrap();
        let render = unsafe { dandrum_sound_fixture_render_create(missing.as_ptr()) };

        assert!(!render.is_null());
        assert!(!unsafe { dandrum_sound_fixture_render_is_ok(render) });
        let mut error_buffer = [0_i8; 512];
        assert!(unsafe {
            dandrum_sound_fixture_render_error_message(
                render,
                error_buffer.as_mut_ptr(),
                error_buffer.len(),
            )
        });
        let error = unsafe { CStr::from_ptr(error_buffer.as_ptr()) }
            .to_str()
            .expect("render error should be UTF-8");
        assert!(error.contains("failed to read sound fixture"));
        let mut truncated_error = [b'X' as c_char; 8];
        assert!(unsafe {
            dandrum_sound_fixture_render_error_message(render, truncated_error.as_mut_ptr(), 5)
        });
        assert_eq!(
            truncated_error.map(|byte| byte as u8),
            [b'f', b'a', b'i', b'l', 0, b'X', b'X', b'X']
        );
        assert!(!unsafe {
            dandrum_sound_fixture_render_error_message(render, error_buffer.as_mut_ptr(), 0)
        });
        assert!(!unsafe {
            dandrum_sound_fixture_render_error_message(
                std::ptr::null(),
                error_buffer.as_mut_ptr(),
                error_buffer.len(),
            )
        });
        assert_eq!(
            unsafe { dandrum_sound_fixture_render_metric_count(render) },
            0
        );
        assert!(!unsafe {
            dandrum_sound_fixture_render_metric(
                render,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        });
        let mut wav_buffer = [0_u8; 44];
        assert!(!unsafe {
            dandrum_sound_fixture_render_copy_wav(render, wav_buffer.as_mut_ptr(), wav_buffer.len())
        });
        unsafe { dandrum_sound_fixture_render_destroy(render) };

        let invalid_utf8 = [0xff_u8, 0];
        let invalid_path =
            unsafe { dandrum_sound_fixture_render_create(invalid_utf8.as_ptr().cast::<c_char>()) };
        assert!(!invalid_path.is_null());
        assert!(!unsafe { dandrum_sound_fixture_render_is_ok(invalid_path) });
        unsafe { dandrum_sound_fixture_render_destroy(invalid_path) };

        let temp_dir = std::env::temp_dir().join(format!(
            "dandrum-sound-workbench-ffi-analysis-error-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).expect("temporary fixture directory should be created");
        let source = fs::read_to_string(acid_fixture_path()).expect("source fixture should read");
        let patch_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/patches/tb303-acid.yaml")
            .canonicalize()
            .expect("acid patch should resolve");
        let invalid_analysis_fixture = temp_dir.join("invalid-analysis.yaml");
        fs::write(
            &invalid_analysis_fixture,
            source
                .replacen(
                    "patch: ../patches/tb303-acid.yaml",
                    &format!("patch: {}", patch_path.display()),
                    1,
                )
                .replacen("max_frequency_hz: 20000", "max_frequency_hz: 30000", 1),
        )
        .expect("invalid analysis fixture should write");
        let invalid_analysis_path =
            CString::new(invalid_analysis_fixture.to_string_lossy().as_bytes()).unwrap();
        let invalid_analysis =
            unsafe { dandrum_sound_fixture_render_create(invalid_analysis_path.as_ptr()) };
        assert!(!unsafe { dandrum_sound_fixture_render_is_ok(invalid_analysis) });
        unsafe { dandrum_sound_fixture_render_destroy(invalid_analysis) };
        fs::remove_dir_all(temp_dir).expect("temporary fixture directory should be removed");

        assert!(unsafe { dandrum_sound_fixture_render_create(std::ptr::null()) }.is_null());
        assert!(!unsafe { dandrum_sound_fixture_render_is_ok(std::ptr::null()) });
        assert_eq!(
            unsafe { dandrum_sound_fixture_render_wav_size(std::ptr::null()) },
            0
        );
        unsafe { dandrum_sound_fixture_render_destroy(std::ptr::null_mut()) };
    }
}
