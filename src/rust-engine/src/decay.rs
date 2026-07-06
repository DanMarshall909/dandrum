#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecayCurve {
    Linear,
    Exponential,
}

impl DecayCurve {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "linear" => Some(Self::Linear),
            "exponential" => Some(Self::Exponential),
            _ => None,
        }
    }
}
