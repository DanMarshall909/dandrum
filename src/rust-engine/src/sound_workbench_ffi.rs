use std::ffi::{CStr, c_char, c_void};

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

pub type DandrumSoundMatchProgressCallback = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        completed_evaluations: usize,
        max_evaluations: usize,
        best_total: f64,
    ) -> bool,
>;

pub type DandrumCancellationCallback = Option<unsafe extern "C" fn(context: *mut c_void) -> bool>;

pub struct DandrumSoundMatch {
    result: Result<SoundMatchFfiArtifact, String>,
}

struct SoundMatchFfiArtifact {
    fixture: crate::sound_workbench::SoundFixture,
    matched: crate::sound_matching::SoundMatchArtifact,
    manifest_json: String,
}

pub struct DandrumGraphProposal {
    result: Result<crate::graph_proposal::ValidatedGraphProposal, String>,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_create(
    fixture_path: *const c_char,
    reference_path: *const c_char,
    progress_callback: DandrumSoundMatchProgressCallback,
    context: *mut c_void,
) -> *mut DandrumSoundMatch {
    if fixture_path.is_null() || reference_path.is_null() {
        return std::ptr::null_mut();
    }

    let result =
        unsafe { read_c_string(fixture_path, "sound fixture path") }.and_then(|fixture_path| {
            unsafe { read_c_string(reference_path, "reference WAV path") }.and_then(
                |reference_path| {
                    match_fixture_artifact(fixture_path, reference_path, progress_callback, context)
                },
            )
        });
    Box::into_raw(Box::new(DandrumSoundMatch { result }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_destroy(matched: *mut DandrumSoundMatch) {
    if !matched.is_null() {
        drop(unsafe { Box::from_raw(matched) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_is_ok(matched: *const DandrumSoundMatch) -> bool {
    unsafe { matched.as_ref() }.is_some_and(|matched| matched.result.is_ok())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_error_message(
    matched: *const DandrumSoundMatch,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let Some(matched) = (unsafe { matched.as_ref() }) else {
        return false;
    };
    copy_string_to_c_buffer(
        matched.result.as_ref().err().map_or("", String::as_str),
        buffer,
        buffer_capacity,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_was_cancelled(
    matched: *const DandrumSoundMatch,
) -> bool {
    unsafe { match_artifact(matched) }.is_some_and(|artifact| {
        artifact.matched.manifest.status == crate::sound_matching::SoundMatchStatus::Cancelled
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_sample_rate_hz(
    matched: *const DandrumSoundMatch,
) -> u32 {
    unsafe { match_artifact(matched) }.map_or(0, |artifact| artifact.matched.sample_rate_hz)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_duration_frames(
    matched: *const DandrumSoundMatch,
) -> u64 {
    unsafe { match_artifact(matched) }.map_or(0, |artifact| artifact.matched.duration_frames)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_manifest_json_size(
    matched: *const DandrumSoundMatch,
) -> usize {
    unsafe { match_artifact(matched) }.map_or(0, |artifact| artifact.manifest_json.len() + 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_copy_manifest_json(
    matched: *const DandrumSoundMatch,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let Some(artifact) = (unsafe { match_artifact(matched) }) else {
        return false;
    };
    copy_complete_string_to_c_buffer(&artifact.manifest_json, buffer, buffer_capacity)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_parameter_count(
    matched: *const DandrumSoundMatch,
) -> usize {
    unsafe { match_artifact(matched) }.map_or(0, |artifact| {
        artifact.matched.manifest.best_parameters.len()
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_parameter(
    matched: *const DandrumSoundMatch,
    index: usize,
    id_buffer: *mut c_char,
    id_buffer_capacity: usize,
    min: *mut f64,
    max: *mut f64,
    initial: *mut f64,
    best: *mut f64,
    normalized: *mut f64,
) -> bool {
    if min.is_null() || max.is_null() || initial.is_null() || best.is_null() || normalized.is_null()
    {
        return false;
    }
    let Some(parameter) = (unsafe { match_artifact(matched) })
        .and_then(|artifact| artifact.matched.manifest.best_parameters.get(index))
    else {
        return false;
    };
    if !copy_string_to_c_buffer(&parameter.id, id_buffer, id_buffer_capacity) {
        return false;
    }
    unsafe {
        *min = parameter.min;
        *max = parameter.max;
        *initial = parameter.initial;
        *best = parameter.best;
        *normalized = parameter.normalized;
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_metric_count(
    matched: *const DandrumSoundMatch,
) -> usize {
    unsafe { match_artifact(matched) }.map_or(0, |artifact| {
        artifact
            .matched
            .candidate_metrics
            .len()
            .min(artifact.matched.reference_metrics.len())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_metric(
    matched: *const DandrumSoundMatch,
    index: usize,
    reference: bool,
    time_seconds: *mut f64,
    rms: *mut f64,
    spectral_centroid_hz: *mut f64,
    has_spectral_centroid: *mut bool,
) -> bool {
    if time_seconds.is_null()
        || rms.is_null()
        || spectral_centroid_hz.is_null()
        || has_spectral_centroid.is_null()
    {
        return false;
    }
    let Some(artifact) = (unsafe { match_artifact(matched) }) else {
        return false;
    };
    let metrics = if reference {
        &artifact.matched.reference_metrics
    } else {
        &artifact.matched.candidate_metrics
    };
    let Some(metric) = metrics.get(index) else {
        return false;
    };
    unsafe {
        *time_seconds = metric.time_seconds;
        *rms = metric.rms;
        *has_spectral_centroid = metric.spectral_centroid_hz.is_some();
        *spectral_centroid_hz = metric.spectral_centroid_hz.unwrap_or(0.0);
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_wav_size(
    matched: *const DandrumSoundMatch,
    reference: bool,
) -> usize {
    unsafe { match_artifact(matched) }
        .map_or(0, |artifact| match_wav_bytes(artifact, reference).len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_sound_match_copy_wav(
    matched: *const DandrumSoundMatch,
    reference: bool,
    buffer: *mut u8,
    buffer_capacity: usize,
) -> bool {
    let Some(artifact) = (unsafe { match_artifact(matched) }) else {
        return false;
    };
    let wav = match_wav_bytes(artifact, reference);
    if buffer.is_null() || buffer_capacity < wav.len() {
        return false;
    }
    unsafe { std::ptr::copy_nonoverlapping(wav.as_ptr(), buffer, wav.len()) };
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_create(
    matched: *const DandrumSoundMatch,
    cancellation_callback: DandrumCancellationCallback,
    context: *mut c_void,
) -> *mut DandrumGraphProposal {
    let provider = crate::codex_cli_provider::CodexCliGraphProposalProvider::new();
    unsafe {
        create_graph_proposal_with_provider(matched, &provider, cancellation_callback, context)
    }
}

unsafe fn create_graph_proposal_with_provider(
    matched: *const DandrumSoundMatch,
    provider: &dyn crate::graph_proposal::GraphProposalProvider,
    cancellation_callback: DandrumCancellationCallback,
    context: *mut c_void,
) -> *mut DandrumGraphProposal {
    let result = (unsafe { match_artifact(matched) })
        .ok_or_else(|| "a completed sound match is required for a graph proposal".to_string())
        .and_then(|artifact| {
            let request = crate::graph_proposal::build_graph_proposal_request(
                &artifact.fixture,
                &artifact.matched.manifest,
            )?;
            let is_cancelled =
                || cancellation_callback.is_some_and(|callback| !unsafe { callback(context) });
            let patch_root = artifact
                .fixture
                .patch
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            crate::graph_proposal::request_validated_graph_proposal(
                provider,
                &request,
                patch_root,
                &is_cancelled,
            )
        });
    Box::into_raw(Box::new(DandrumGraphProposal { result }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_destroy(proposal: *mut DandrumGraphProposal) {
    if !proposal.is_null() {
        drop(unsafe { Box::from_raw(proposal) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_is_ok(
    proposal: *const DandrumGraphProposal,
) -> bool {
    unsafe { proposal.as_ref() }.is_some_and(|proposal| proposal.result.is_ok())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_error_message(
    proposal: *const DandrumGraphProposal,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let Some(proposal) = (unsafe { proposal.as_ref() }) else {
        return false;
    };
    copy_string_to_c_buffer(
        proposal.result.as_ref().err().map_or("", String::as_str),
        buffer,
        buffer_capacity,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_patch_name(
    proposal: *const DandrumGraphProposal,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    copy_proposal_string(proposal, buffer, buffer_capacity, |proposal| {
        &proposal.patch_name
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_provider_id(
    proposal: *const DandrumGraphProposal,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    copy_proposal_string(proposal, buffer, buffer_capacity, |proposal| {
        &proposal.provider_id
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_explanation_size(
    proposal: *const DandrumGraphProposal,
) -> usize {
    unsafe { proposal_artifact(proposal) }.map_or(0, |proposal| proposal.explanation.len() + 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_copy_explanation(
    proposal: *const DandrumGraphProposal,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let Some(proposal) = (unsafe { proposal_artifact(proposal) }) else {
        return false;
    };
    copy_complete_string_to_c_buffer(&proposal.explanation, buffer, buffer_capacity)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_patch_yaml_size(
    proposal: *const DandrumGraphProposal,
) -> usize {
    unsafe { proposal_artifact(proposal) }.map_or(0, |proposal| proposal.patch_yaml.len() + 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_copy_patch_yaml(
    proposal: *const DandrumGraphProposal,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let Some(proposal) = (unsafe { proposal_artifact(proposal) }) else {
        return false;
    };
    copy_complete_string_to_c_buffer(&proposal.patch_yaml, buffer, buffer_capacity)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_parameter_count(
    proposal: *const DandrumGraphProposal,
) -> usize {
    unsafe { proposal_artifact(proposal) }
        .map_or(0, |proposal| proposal.suggested_search_parameters.len())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dandrum_graph_proposal_parameter(
    proposal: *const DandrumGraphProposal,
    index: usize,
    buffer: *mut c_char,
    buffer_capacity: usize,
) -> bool {
    let Some(parameter) = (unsafe { proposal_artifact(proposal) })
        .and_then(|proposal| proposal.suggested_search_parameters.get(index))
    else {
        return false;
    };
    copy_string_to_c_buffer(parameter, buffer, buffer_capacity)
}

fn match_fixture_artifact(
    fixture_path: &str,
    reference_path: &str,
    progress_callback: DandrumSoundMatchProgressCallback,
    context: *mut c_void,
) -> Result<SoundMatchFfiArtifact, String> {
    let fixture = crate::sound_workbench::load_sound_fixture_file(fixture_path)?;
    let matched =
        crate::sound_matching::match_sound_fixture(&fixture, reference_path, |progress| {
            progress_callback.is_none_or(|callback| unsafe {
                callback(
                    context,
                    progress.completed_evaluations,
                    progress.max_evaluations,
                    progress.best_total,
                )
            })
        })?;
    let manifest_json = serde_json::to_string(&matched.manifest)
        .map_err(|error| format!("failed to serialize sound match manifest: {error}"))?;
    Ok(SoundMatchFfiArtifact {
        fixture,
        matched,
        manifest_json,
    })
}

unsafe fn match_artifact<'a>(
    matched: *const DandrumSoundMatch,
) -> Option<&'a SoundMatchFfiArtifact> {
    unsafe { matched.as_ref() }?.result.as_ref().ok()
}

fn match_wav_bytes(artifact: &SoundMatchFfiArtifact, reference: bool) -> &[u8] {
    if reference {
        &artifact.matched.reference_wav_bytes
    } else {
        &artifact.matched.candidate_wav_bytes
    }
}

unsafe fn proposal_artifact<'a>(
    proposal: *const DandrumGraphProposal,
) -> Option<&'a crate::graph_proposal::ValidatedGraphProposal> {
    unsafe { proposal.as_ref() }?.result.as_ref().ok()
}

fn copy_proposal_string(
    proposal: *const DandrumGraphProposal,
    buffer: *mut c_char,
    buffer_capacity: usize,
    select: impl FnOnce(&crate::graph_proposal::ValidatedGraphProposal) -> &str,
) -> bool {
    let Some(proposal) = (unsafe { proposal_artifact(proposal) }) else {
        return false;
    };
    copy_string_to_c_buffer(select(proposal), buffer, buffer_capacity)
}

unsafe fn read_c_string<'a>(value: *const c_char, name: &str) -> Result<&'a str, String> {
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| format!("{name} was not valid UTF-8"))
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

fn copy_complete_string_to_c_buffer(value: &str, buffer: *mut c_char, capacity: usize) -> bool {
    if capacity < value.len() + 1 {
        return false;
    }
    copy_string_to_c_buffer(value, buffer, capacity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

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

    struct ProgressCapture {
        calls: usize,
        completed: usize,
        maximum: usize,
        best: f64,
        continue_work: bool,
    }

    unsafe extern "C" fn capture_match_progress(
        context: *mut std::ffi::c_void,
        completed: usize,
        maximum: usize,
        best: f64,
    ) -> bool {
        let capture = unsafe { &mut *context.cast::<ProgressCapture>() };
        capture.calls += 1;
        capture.completed = completed;
        capture.maximum = maximum;
        capture.best = best;
        capture.continue_work
    }

    struct TestProposalProvider {
        response: Result<crate::graph_proposal::GraphProposalResponse, String>,
        requests: Mutex<Vec<crate::graph_proposal::GraphProposalRequest>>,
    }

    impl crate::graph_proposal::GraphProposalProvider for TestProposalProvider {
        fn provider_id(&self) -> &str {
            "ffi-test-provider"
        }

        fn capabilities(&self) -> crate::graph_proposal::GraphProposalCapabilities {
            crate::graph_proposal::GraphProposalCapabilities {
                structured_output: true,
                cancellation: true,
            }
        }

        fn propose(
            &self,
            request: &crate::graph_proposal::GraphProposalRequest,
            _is_cancelled: &dyn Fn() -> bool,
        ) -> Result<crate::graph_proposal::GraphProposalResponse, String> {
            self.requests.lock().unwrap().push(request.clone());
            self.response.clone()
        }
    }

    fn short_match_files() -> (tempfile::TempDir, CString, CString) {
        let directory = tempfile::tempdir().unwrap();
        let patch_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples/patches/tb303-acid.yaml")
            .canonicalize()
            .unwrap();
        let source = fs::read_to_string(acid_fixture_path()).unwrap();
        let fixture_path = directory.path().join("short-acid.yaml");
        fs::write(
            &fixture_path,
            source
                .replacen(
                    "patch: ../patches/tb303-acid.yaml",
                    &format!("patch: {}", patch_path.display()),
                    1,
                )
                .replace("duration_frames: 528000", "duration_frames: 96000")
                .replace("max_evaluations: 16", "max_evaluations: 2")
                .replace("start_frame: 48000", "start_frame: 0")
                .replacen("length_frames: 96000", "length_frames: 12000", 1)
                .replace("repetitions: 4", "repetitions: 1"),
        )
        .unwrap();
        let fixture = crate::sound_workbench::load_sound_fixture_file(&fixture_path).unwrap();
        let render = crate::sound_workbench::render_sound_fixture(&fixture).unwrap();
        let reference_path = directory.path().join("reference.wav");
        let mut reference = fs::File::create(&reference_path).unwrap();
        crate::wav::write_wav_stereo_i16(
            &mut reference,
            render.sample_rate_hz,
            &render.left,
            &render.right,
        )
        .unwrap();
        (
            directory,
            CString::new(fixture_path.to_string_lossy().as_bytes()).unwrap(),
            CString::new(reference_path.to_string_lossy().as_bytes()).unwrap(),
        )
    }

    #[test]
    fn ffi_match_handle_owns_progress_parameters_manifest_metrics_and_both_wavs() {
        let (_directory, fixture_path, reference_path) = short_match_files();
        let mut progress = ProgressCapture {
            calls: 0,
            completed: 0,
            maximum: 0,
            best: f64::INFINITY,
            continue_work: true,
        };

        let matched = unsafe {
            dandrum_sound_match_create(
                fixture_path.as_ptr(),
                reference_path.as_ptr(),
                Some(capture_match_progress),
                (&mut progress as *mut ProgressCapture).cast(),
            )
        };

        assert!(!matched.is_null());
        assert!(unsafe { dandrum_sound_match_is_ok(matched) });
        assert!(!unsafe { dandrum_sound_match_was_cancelled(matched) });
        assert_eq!(progress.calls, 2);
        assert_eq!(progress.completed, 2);
        assert_eq!(progress.maximum, 2);
        assert!(progress.best.is_finite());
        assert_eq!(
            unsafe { dandrum_sound_match_sample_rate_hz(matched) },
            48_000
        );
        assert_eq!(
            unsafe { dandrum_sound_match_duration_frames(matched) },
            96_000
        );

        let manifest_size = unsafe { dandrum_sound_match_manifest_json_size(matched) };
        let mut manifest = vec![0_i8; manifest_size];
        assert!(unsafe {
            dandrum_sound_match_copy_manifest_json(matched, manifest.as_mut_ptr(), manifest.len())
        });
        let manifest = unsafe { CStr::from_ptr(manifest.as_ptr()) }
            .to_str()
            .unwrap();
        assert!(manifest.contains("\"completed_evaluations\":2"));
        assert!(manifest.contains("\"reference_sha256\""));

        assert_eq!(unsafe { dandrum_sound_match_parameter_count(matched) }, 4);
        let mut id = [0_i8; 128];
        let mut min = 0.0;
        let mut max = 0.0;
        let mut initial = 0.0;
        let mut best = 0.0;
        let mut normalized = 0.0;
        assert!(unsafe {
            dandrum_sound_match_parameter(
                matched,
                0,
                id.as_mut_ptr(),
                id.len(),
                &mut min,
                &mut max,
                &mut initial,
                &mut best,
                &mut normalized,
            )
        });
        assert!(!unsafe { CStr::from_ptr(id.as_ptr()) }.to_bytes().is_empty());
        assert!(min <= initial && initial <= max);
        assert!(min <= best && best <= max);
        assert!((0.0..=1.0).contains(&normalized));

        let metric_count = unsafe { dandrum_sound_match_metric_count(matched) };
        assert!(metric_count > 1);
        let mut time = 0.0;
        let mut candidate_rms = 0.0;
        let mut candidate_centroid = 0.0;
        let mut candidate_has_centroid = false;
        assert!(unsafe {
            dandrum_sound_match_metric(
                matched,
                0,
                false,
                &mut time,
                &mut candidate_rms,
                &mut candidate_centroid,
                &mut candidate_has_centroid,
            )
        });
        assert!(time.is_finite() && candidate_rms.is_finite());
        assert!(!candidate_has_centroid || candidate_centroid.is_finite());

        for reference in [false, true] {
            let size = unsafe { dandrum_sound_match_wav_size(matched, reference) };
            let mut wav = vec![0_u8; size];
            assert!(size > 44);
            assert!(unsafe {
                dandrum_sound_match_copy_wav(matched, reference, wav.as_mut_ptr(), wav.len())
            });
            assert_eq!(&wav[0..4], b"RIFF");
        }

        unsafe { dandrum_sound_match_destroy(matched) };
    }

    #[test]
    fn ffi_match_progress_can_cancel_and_preserve_the_best_completed_candidate() {
        let (_directory, fixture_path, reference_path) = short_match_files();
        let mut progress = ProgressCapture {
            calls: 0,
            completed: 0,
            maximum: 0,
            best: f64::INFINITY,
            continue_work: false,
        };

        let matched = unsafe {
            dandrum_sound_match_create(
                fixture_path.as_ptr(),
                reference_path.as_ptr(),
                Some(capture_match_progress),
                (&mut progress as *mut ProgressCapture).cast(),
            )
        };

        assert!(unsafe { dandrum_sound_match_is_ok(matched) });
        assert!(unsafe { dandrum_sound_match_was_cancelled(matched) });
        assert_eq!(progress.calls, 1);
        assert_eq!(progress.completed, 1);
        assert!(unsafe { dandrum_sound_match_wav_size(matched, false) } > 44);
        unsafe { dandrum_sound_match_destroy(matched) };
    }

    #[test]
    fn ffi_proposal_handle_exposes_only_locally_validated_provider_results() {
        const VALID_PROPOSAL: &str = r#"
metadata:
  name: FFI proposal
instrument:
  id: dandrum.ffi-proposal
  preset_schema_version: 1
preset_surface:
  parameters:
    - name: oscillator.pitch
      type: number
      default: 1
      min: 0.25
      max: 4
      maps_to: osc.pitch
render:
  sample_rate_hz: 48000
  block_size_frames: 64
modules:
  - { id: osc, type: oscillator }
  - { id: mixer, type: audio_mixer }
  - id: out
    type: audio_output
    inputs:
      - { name: left, signal_type: audio }
      - { name: right, signal_type: audio }
connections:
  - { from: osc.audio, to: mixer.inputs }
  - { from: mixer.mix, to: out.left }
  - { from: mixer.mix, to: out.right }
"#;
        let (_directory, fixture_path, reference_path) = short_match_files();
        let matched = unsafe {
            dandrum_sound_match_create(
                fixture_path.as_ptr(),
                reference_path.as_ptr(),
                None,
                std::ptr::null_mut(),
            )
        };
        let provider = TestProposalProvider {
            response: Ok(crate::graph_proposal::GraphProposalResponse {
                patch_yaml: VALID_PROPOSAL.to_string(),
                explanation: "Expose pitch for the next deterministic search.".to_string(),
                suggested_search_parameters: vec!["oscillator.pitch".to_string()],
            }),
            requests: Mutex::new(Vec::new()),
        };

        let proposal = unsafe {
            create_graph_proposal_with_provider(matched, &provider, None, std::ptr::null_mut())
        };

        assert!(unsafe { dandrum_graph_proposal_is_ok(proposal) });
        assert_eq!(provider.requests.lock().unwrap().len(), 1);
        let mut name = [0_i8; 128];
        assert!(unsafe {
            dandrum_graph_proposal_patch_name(proposal, name.as_mut_ptr(), name.len())
        });
        assert_eq!(
            unsafe { CStr::from_ptr(name.as_ptr()) }.to_str().unwrap(),
            "FFI proposal"
        );
        assert_eq!(
            unsafe { dandrum_graph_proposal_parameter_count(proposal) },
            1
        );
        let mut parameter = [0_i8; 128];
        assert!(unsafe {
            dandrum_graph_proposal_parameter(proposal, 0, parameter.as_mut_ptr(), parameter.len())
        });
        assert_eq!(
            unsafe { CStr::from_ptr(parameter.as_ptr()) }
                .to_str()
                .unwrap(),
            "oscillator.pitch"
        );
        assert!(unsafe { dandrum_graph_proposal_patch_yaml_size(proposal) } > 100);
        assert!(unsafe { dandrum_graph_proposal_explanation_size(proposal) } > 10);
        unsafe { dandrum_graph_proposal_destroy(proposal) };
        unsafe { dandrum_sound_match_destroy(matched) };
    }

    #[test]
    fn ffi_match_and_proposal_handles_reject_null_and_preserve_retryable_errors() {
        let missing = CString::new("/definitely/missing/reference.wav").unwrap();
        let fixture = CString::new(acid_fixture_path().to_string_lossy().as_bytes()).unwrap();
        let matched = unsafe {
            dandrum_sound_match_create(
                fixture.as_ptr(),
                missing.as_ptr(),
                None,
                std::ptr::null_mut(),
            )
        };
        assert!(!unsafe { dandrum_sound_match_is_ok(matched) });
        let mut error = [0_i8; 256];
        assert!(unsafe {
            dandrum_sound_match_error_message(matched, error.as_mut_ptr(), error.len())
        });
        assert!(
            unsafe { CStr::from_ptr(error.as_ptr()) }
                .to_str()
                .unwrap()
                .contains("reference WAV")
        );
        assert_eq!(unsafe { dandrum_sound_match_parameter_count(matched) }, 0);
        assert_eq!(unsafe { dandrum_sound_match_wav_size(matched, false) }, 0);
        unsafe { dandrum_sound_match_destroy(matched) };

        assert!(
            unsafe {
                dandrum_sound_match_create(
                    std::ptr::null(),
                    std::ptr::null(),
                    None,
                    std::ptr::null_mut(),
                )
            }
            .is_null()
        );
        assert!(!unsafe { dandrum_sound_match_is_ok(std::ptr::null()) });
        assert!(!unsafe { dandrum_graph_proposal_is_ok(std::ptr::null()) });
        unsafe { dandrum_sound_match_destroy(std::ptr::null_mut()) };
        unsafe { dandrum_graph_proposal_destroy(std::ptr::null_mut()) };
    }
}
