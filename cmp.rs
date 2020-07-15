//! Comparison functions.

use std::cmp::Ordering;
use super::approx::ApproxEq;
use super::primitive::Range;

/// A comparison function for partial orderings.
///
/// Panics with `"encountered nan in comparison"` when the comparison returns
/// `None`.
pub fn value_no_nans<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).expect("encountered nan in comparison")
}

/// An approximate comparison function for floats.
///
/// Returns equal when the the values are approximately equal and falls back to
/// `value_no_nans` otherwise.
pub fn value_approx(a: &f64, b: &f64, tolerance: f64) -> Ordering {
    if a.approx_eq(b, tolerance) {
        Ordering::Equal
    } else {
        value_no_nans(a, b)
    }
}

/// A comparison function for ranges and values.
///
/// Returns equal when the value falls into the range and less or greater when
/// it is before or after the range.
pub fn position(range: Range, v: f64) -> Ordering {
    if range.start > v {
        Ordering::Greater
    } else if range.end <= v {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}
