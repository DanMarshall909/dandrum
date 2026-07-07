//! Shared test assertions.
//!
//! These helpers are only meaningful in tests, but the macro is exported at the
//! crate root (not `#[cfg(test)]`) so integration tests under `tests/` — which
//! link the library compiled without `cfg(test)` — can use it too.

/// Assert that two floating-point numbers are equal within `tolerance`.
///
/// Works for any float type whose difference supports `.abs()` (e.g. `f32`,
/// `f64`). The comparison is inclusive: it passes when
/// `(actual - expected).abs() <= tolerance`.
///
/// An optional trailing `because` reason (with `format!`-style arguments)
/// explains why the values are expected to match; it is shown on failure.
///
/// ```
/// use dandrum_engine::assert_approx_eq;
///
/// assert_approx_eq!(0.1_f32 + 0.2, 0.3, 1e-6);
/// assert_approx_eq!(0.1_f64 + 0.2, 0.3, 1e-9, "floating point addition rounds");
/// let gain = 1.0_f32;
/// assert_approx_eq!(gain, 1.0, 1e-6, "unity gain leaves the signal unchanged");
/// ```
#[macro_export]
macro_rules! assert_approx_eq {
    ($actual:expr, $expected:expr, $tolerance:expr $(,)?) => {
        $crate::assert_approx_eq!(
            $actual,
            $expected,
            $tolerance,
            "the values should be approximately equal"
        )
    };
    ($actual:expr, $expected:expr, $tolerance:expr, $($because:tt)+) => {{
        let actual = $actual;
        let expected = $expected;
        let tolerance = $tolerance;
        let difference = (actual - expected).abs();
        assert!(
            difference <= tolerance,
            "expected `{actual}` to be within `{tolerance}` of `{expected}`, \
             but the difference was `{difference}` — because {}",
            format_args!($($because)+),
        );
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn passes_when_values_are_within_tolerance() {
        assert_approx_eq!(0.1_f32 + 0.2, 0.3, 1e-6);
        assert_approx_eq!(0.1_f64 + 0.2, 0.3, 1e-9);
    }

    #[test]
    fn passes_at_exactly_the_tolerance_boundary() {
        assert_approx_eq!(1.0_f64, 1.5, 0.5, "the difference equals the tolerance");
    }

    #[test]
    #[should_panic(expected = "because unity gain leaves the signal unchanged")]
    fn fails_when_outside_tolerance_and_reports_the_because_reason() {
        assert_approx_eq!(2.0_f32, 1.0, 1e-6, "unity gain leaves the signal unchanged");
    }

    #[test]
    #[should_panic(expected = "the difference was")]
    fn failure_message_includes_the_actual_difference() {
        assert_approx_eq!(1.0_f64, 0.0, 0.25);
    }
}
