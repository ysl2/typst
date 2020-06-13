//! Spacial and geometrical types and functions.

#[macro_use]
mod approx;
mod length;
mod point;
mod range;
mod scale;
mod shape;
mod size;
mod vec;

use std::cmp::Ordering;

pub use approx::ApproxEq;
pub use length::{pt, Length, ParseLengthError, FlexLength};
pub use point::Point;
pub use range::{Range, RangeKey, Region};
pub use scale::Scale;
pub use shape::{Rect, Shape};
pub use size::{Dim, Margins, Size, VDim};
pub use vec::Vec2;

/// A comparison function for partial orderings which panics with
/// `"encountered nan in comparison"` when the comparison fails.
pub fn value_no_nans<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).expect("encountered nan in comparison")
}
