//! Spacial and geometrical types and functions.

#[macro_use]
mod approx;
mod bez;
mod collision;
mod dim;
mod flex;
pub mod range;
mod scale;

use std::cmp::Ordering;

pub use approx::{ApproxEq, value_approx};
pub use bez::{intersect_curves, Monotone, ParamCurveSolve, MAX_SOLVE};
pub use collision::PlacementGroup;
pub use dim::{Dim, VDim};
pub use flex::Flex;
pub use scale::Scale;

/// Root-finding for polynomials up to degree 3.
pub mod roots {
    use arrayvec::ArrayVec;
    pub use kurbo::common::{solve_quadratic, solve_cubic};

    /// Find roots of linear equation.
    pub fn solve_linear(c0: f64, c1: f64) -> ArrayVec<[f64; 1]> {
        let mut result = ArrayVec::new();
        let root = -c0 / c1;
        if root.is_finite() {
            result.push(root);
        } else if c0 == 0.0 && c1 == 0.0 {
            result.push(0.0);
        }
        return result;
    }
}

pub use kurbo::{
    Affine, BezPath, Circle, CubicBez, Ellipse, Insets, Line, PathEl, PathSeg,
    ParamCurve, ParamCurveExtrema, Point, QuadBez, Rect, RoundedRect, Shape,
    Size, SvgParseError, TranslateScale, Vec2,
    MAX_EXTREMA,
};

/// A comparison function for partial orderings which panics with
/// `"encountered nan in comparison"` when the comparison fails.
pub fn value_no_nans<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).expect("encountered nan in comparison")
}
