use crate::graph::builtin_ports;
use crate::saturator::{HardClipCurve, Saturator, SinFoldCurve, SoftClipCurve, TanhCurve};

use super::outputs::ModuleOutputs;

pub(super) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub(super) fn log_lerp(a: f32, b: f32, t: f32) -> f32 {
    let clamped = t.clamp(0.0, 1.0);
    (a.ln() + (b.ln() - a.ln()) * clamped).exp()
}

pub(super) fn has_signal(buf: &[f32]) -> bool {
    buf.iter().any(|&v| v != 0.0)
}

pub(super) fn normalized_position(value: f32, sample_len: usize) -> f32 {
    (value.clamp(0.0, 1.0) * sample_len as f32).min(sample_len.saturating_sub(1) as f32)
}

pub(super) fn normalized_end_position(value: f32, sample_len: usize) -> f32 {
    (value.clamp(0.0, 1.0) * sample_len as f32).clamp(0.0, sample_len as f32)
}

pub(super) fn audio_output(port_name: &str, audio: Vec<f32>) -> ModuleOutputs {
    let mut outputs = ModuleOutputs::empty();
    outputs.audio.insert(port_name.to_string(), audio);
    outputs
}

pub(super) fn stereo_audio_output(left: Vec<f32>, right: Vec<f32>) -> ModuleOutputs {
    let mut outputs = ModuleOutputs::empty();
    outputs
        .audio
        .insert(builtin_ports::AUDIO_OUT_L.to_string(), left);
    outputs
        .audio
        .insert(builtin_ports::AUDIO_OUT_R.to_string(), right);
    outputs
}

pub(super) fn set_curve_by_index(processor: &mut Saturator, idx: usize) {
    match idx {
        0 => processor.set_curve(Box::new(TanhCurve)),
        1 => processor.set_curve(Box::new(HardClipCurve)),
        2 => processor.set_curve(Box::new(SoftClipCurve)),
        3 => processor.set_curve(Box::new(SinFoldCurve)),
        _ => processor.set_curve(Box::new(TanhCurve)),
    }
}
