//! Linear unit conversion shared by descriptors and per-sample control planes.
//! Callers validate normalized values and finite, ordered physical ranges.
pub(crate) fn physical(min: f64, max: f64, normalized: f64) -> f64 {
    (min + normalized * (max - min)).clamp(min, max)
}
pub(crate) fn normalized(min: f64, max: f64, physical: f64) -> f64 {
    (physical - min) / (max - min)
}
