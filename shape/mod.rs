//! Shapes and curves.

use super::approx::ApproxEq;
use super::primitive::*;

mod intersect;
mod monotone;
mod shape_group;
mod solve;

pub use kurbo::{
    BezPath, Circle, CubicBez, Ellipse, Line, QuadBez, Rect, RoundedRect,
    PathEl, PathSeg, SvgParseError, ParamCurve, ParamCurveExtrema, Shape,
};

pub use intersect::find_intersections_bbox;
pub use monotone::Monotone;
pub use shape_group::ShapeGroup;
pub use solve::ParamCurveSolve;

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

impl_approx_eq!(Line [p0, p1]);
impl_approx_eq!(QuadBez [p0, p1, p2]);
impl_approx_eq!(CubicBez [p0, p1, p2, p3]);
impl_approx_eq!(Rect [x0, y0, x1, y1]);

impl ApproxEq for PathSeg {
    /// Compares the control points directly if both curves are of the same
    /// kind or converts to cubic and compares control points then if not.
    ///
    /// Please note that this can still return `false` when the two segments
    /// coincide, because two different sets of control points can induce the
    /// same curve.
    fn approx_eq(&self, other: &Self, tolerance: f64) -> bool {
        use PathSeg::*;
        match (self, other) {
            (Line(a), Line(b)) => a.approx_eq(&b, tolerance),
            (Quad(a), Quad(b)) => a.approx_eq(&b, tolerance),
            (Cubic(a), Cubic(b)) => a.approx_eq(&b, tolerance),
            (a, b) => a.to_cubic().approx_eq(&b.to_cubic(), tolerance),
        }
    }
}
