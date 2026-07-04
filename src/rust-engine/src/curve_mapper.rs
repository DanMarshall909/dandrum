#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveKind {
    Linear,
    Exponential,
    Logarithmic,
    SCurve,
    SoftClip,
    HardClip,
    Step,
}

impl CurveKind {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "linear" => Some(Self::Linear),
            "exponential" => Some(Self::Exponential),
            "logarithmic" => Some(Self::Logarithmic),
            "s_curve" => Some(Self::SCurve),
            "soft_clip" => Some(Self::SoftClip),
            "hard_clip" => Some(Self::HardClip),
            "step" => Some(Self::Step),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CurveMapper {
    curve: CurveKind,
    steps: u32,
}

impl CurveMapper {
    pub const MIN_STEPS: u32 = 2;
    pub const DEFAULT_STEPS: u32 = 4;
    const OUTPUT_LIMIT: f32 = 1_000_000.0;

    pub fn new(curve: CurveKind, steps: u32) -> Self {
        Self {
            curve,
            steps: steps.max(Self::MIN_STEPS),
        }
    }

    pub fn process(&self, input: f32, amount: f32, bias: f32, scale: f32, offset: f32) -> f32 {
        let dry = finite_or_zero(input);
        let biased = finite_or_zero(input + bias).clamp(0.0, 1.0);
        let wet = self.apply_curve(biased);
        let blend = finite_or_zero(amount).clamp(0.0, 1.0);
        let mapped = dry + (wet - dry) * blend;
        finite_or_zero(mapped * finite_or_zero(scale) + finite_or_zero(offset))
            .clamp(-Self::OUTPUT_LIMIT, Self::OUTPUT_LIMIT)
    }

    fn apply_curve(&self, input: f32) -> f32 {
        match self.curve {
            CurveKind::Linear => input,
            CurveKind::Exponential => input * input,
            CurveKind::Logarithmic => input.sqrt(),
            CurveKind::SCurve => input * input * (3.0 - 2.0 * input),
            CurveKind::SoftClip => {
                let centered = input * 2.0 - 1.0;
                ((centered * 2.0).tanh() / 2.0_f32.tanh()) * 0.5 + 0.5
            }
            CurveKind::HardClip => input.clamp(0.0, 1.0),
            CurveKind::Step => {
                let last_step = (self.steps.max(Self::MIN_STEPS) - 1) as f32;
                (input * last_step).round() / last_step
            }
        }
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.0001;

    fn map(curve: CurveKind, input: f32) -> f32 {
        CurveMapper::new(curve, CurveMapper::DEFAULT_STEPS).process(input, 1.0, 0.0, 1.0, 0.0)
    }

    #[test]
    fn linear_curve_keeps_control_value() {
        assert!((map(CurveKind::Linear, 0.5) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn exponential_curve_emphasizes_higher_values() {
        assert!((map(CurveKind::Exponential, 0.5) - 0.25).abs() < EPSILON);
    }

    #[test]
    fn logarithmic_curve_emphasizes_lower_values() {
        assert!((map(CurveKind::Logarithmic, 0.25) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn s_curve_eases_midrange() {
        let mapper = CurveMapper::new(CurveKind::SCurve, CurveMapper::DEFAULT_STEPS);

        assert!(mapper.process(0.25, 1.0, 0.0, 1.0, 0.0) < 0.25);
        assert!(mapper.process(0.75, 1.0, 0.0, 1.0, 0.0) > 0.75);
    }

    #[test]
    fn soft_clip_saturates_without_hard_edges() {
        assert!(map(CurveKind::SoftClip, 0.25) < 0.25);
        assert!(map(CurveKind::SoftClip, 0.75) > 0.75);
    }

    #[test]
    fn hard_clip_bounds_control_value() {
        let mapper = CurveMapper::new(CurveKind::HardClip, CurveMapper::DEFAULT_STEPS);

        assert!((mapper.process(2.0, 1.0, 0.0, 1.0, 0.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn step_curve_quantizes_control_value() {
        let mapper = CurveMapper::new(CurveKind::Step, 4);

        assert!((mapper.process(0.60, 1.0, 0.0, 1.0, 0.0) - (2.0 / 3.0)).abs() < EPSILON);
    }

    #[test]
    fn amount_blends_between_input_and_curve() {
        let mapper = CurveMapper::new(CurveKind::Exponential, CurveMapper::DEFAULT_STEPS);

        assert!((mapper.process(0.5, 0.0, 0.0, 1.0, 0.0) - 0.5).abs() < EPSILON);
        assert!((mapper.process(0.5, 0.5, 0.0, 1.0, 0.0) - 0.375).abs() < EPSILON);
    }

    #[test]
    fn scale_and_offset_adjust_mapped_output() {
        let mapper = CurveMapper::new(CurveKind::Linear, CurveMapper::DEFAULT_STEPS);

        assert!((mapper.process(0.25, 1.0, 0.0, 2.0, -0.25) - 0.25).abs() < EPSILON);
    }

    #[test]
    fn invalid_inputs_produce_finite_bounded_output() {
        let mapper = CurveMapper::new(CurveKind::Exponential, CurveMapper::DEFAULT_STEPS);

        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
            let output =
                mapper.process(value, f32::INFINITY, f32::NAN, f32::MAX, f32::NEG_INFINITY);

            assert!(output.is_finite());
            assert!(output.abs() <= CurveMapper::OUTPUT_LIMIT);
        }
    }
}
