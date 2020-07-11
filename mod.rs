//! Spacial and geometrical types and functions.

#[macro_use]
mod approx;
mod dim;
mod flex;
mod intersect;
mod monotone;
mod scale;
mod shape_group;
mod solve;

use std::cmp::Ordering;

pub use approx::{ApproxEq, value_approx};
pub use dim::{Dim, VDim};
pub use flex::Flex;
pub use intersect::find_intersections_bbox;
pub use monotone::Monotone;
pub use scale::Scale;
pub use shape_group::ShapeGroup;
pub use solve::{ParamCurveSolve, MAX_SOLVE};

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
    Size, SvgParseError, TranslateScale, Vec2, MAX_EXTREMA,
};

/// A float range.
pub type Range = std::ops::Range<f64>;

/// A comparison function for partial orderings which panics with
/// `"encountered nan in comparison"` when the comparison fails.
pub fn value_no_nans<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).expect("encountered nan in comparison")
}

/// An comparison function which returns equal when a value falls into a range
/// and less or greater when it is before or after the range.
pub fn position(range: Range, v: f64) -> Ordering {
    if range.start > v {
        Ordering::Greater
    } else if range.end <= v {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

/// Additional methods for rectangles.
pub trait RectExt {
    /// Whether this rectangle overlaps with the other one.
    fn overlaps(&self, other: &Self) -> bool;
}

impl RectExt for Rect {
    fn overlaps(&self, other: &Self) -> bool {
        self.x1 > other.x0 && other.x1 > self.x0
        && self.y1 > other.y0 && other.y1 > self.y0
    }
}

/// Additional methods for path segments.
pub trait PathSegExt {
    /// Apply an affine transformation.
    fn apply_affine(self, affine: Affine) -> Self;

    /// Apply a translate-scale transformation.
    fn apply_translate_scale(self, ts: TranslateScale) -> Self;
}

impl PathSegExt for PathSeg {
    fn apply_affine(self, affine: Affine) -> PathSeg {
        match self {
            PathSeg::Line(line) => PathSeg::Line(affine * line),
            PathSeg::Quad(quad) => PathSeg::Quad(affine * quad),
            PathSeg::Cubic(cubic) => PathSeg::Cubic(affine * cubic),
        }
    }

    fn apply_translate_scale(self, ts: TranslateScale) -> PathSeg {
        match self {
            PathSeg::Line(line) => PathSeg::Line(ts * line),
            PathSeg::Quad(quad) => PathSeg::Quad(ts * quad),
            PathSeg::Cubic(cubic) => PathSeg::Cubic(ts * cubic),
        }
    }
}
