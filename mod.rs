//! Spacial and geometrical types and functions.

#[macro_use]
mod approx;
mod bez;
mod flo;
mod length;
mod point;
mod range;
mod rect;
mod scale;
mod size;
mod vec;

pub mod collision;
pub mod roots;

use std::cmp::Ordering;

pub use approx::ApproxEq;
pub use bez::{Bez, BezShape, ParseSvgError};
pub use length::{pt, min, max, Length, ParseLengthError, FlexLength};
pub use point::Point;
pub use range::{Range, Region};
pub use rect::Rect;
pub use scale::Scale;
pub use size::{Dim, Margins, Size, VDim};
pub use vec::Vec2;

/// A comparison function for partial orderings which panics with
/// `"encountered nan in comparison"` when the comparison fails.
pub fn value_no_nans<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).expect("encountered nan in comparison")
}
