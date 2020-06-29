//! Spacial and geometrical types and functions.

#[macro_use]
mod approx;
mod bez;
mod dim;
mod flex;
mod range;
mod scale;

pub mod collision;

use std::cmp::Ordering;

pub use approx::ApproxEq;
pub use bez::ParamCurveSolve;
pub use dim::{Dim, VDim};
pub use flex::Flex;
pub use range::{Range, Region};
pub use scale::Scale;

/// Root-finding for polynomials up to degree 3.
pub mod roots {
    pub use kurbo::common::{solve_quadratic, solve_cubic};
}

pub use kurbo::{
    Affine, BezPath, Circle, CubicBez, Ellipse, Insets, Line,
    LineIntersection, Point, QuadBez, Rect, RoundedRect, Size, TranslateScale,
    Vec2, PathEl, PathSeg, SvgParseError, ParamCurve,
    ParamCurveExtrema, Shape,
};

/// A comparison function for partial orderings which panics with
/// `"encountered nan in comparison"` when the comparison fails.
pub fn value_no_nans<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).expect("encountered nan in comparison")
}

/// The maximum of two floats.
pub fn max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

/// The minimum of two floats.
pub fn min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}
