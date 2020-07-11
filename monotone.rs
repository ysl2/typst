use std::ops::Mul;
use arrayvec::{Array, ArrayVec};
use super::*;

/// A wrapper for curves that are monotone in both dimensions.
///
/// This auto-derefs to the wrapped curve, but provides some extra utility and
/// overrides `ParamCurveExtrema` such that bounding-box computation is
/// accelerated.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Monotone<C>(pub C);

impl Monotone<PathSeg> {
    /// Reverses the path segment.
    pub fn reverse(self) -> Self {
        Monotone(self.0.reverse())
    }

    /// Intersects two monotone path segments, solving analytically if possible
    /// and falling back to bounding box search if not.
    pub fn intersect<A>(&self, other: &Self, accuracy: f64) -> ArrayVec<A>
    where
        A: Array<Item=Point>
    {
        match (self.0, other.0) {
            (seg, PathSeg::Line(line)) | (PathSeg::Line(line), seg) => {
                if !self.bounding_box().overlaps(&other.bounding_box()) {
                    return ArrayVec::new();
                }

                seg.intersect_line(line)
                    .into_iter()
                    .map(|sect| line.eval(sect.line_t))
                    .collect()
            }

            _ => find_intersections_bbox(self, other, accuracy),
        }
    }
}

impl<C: ParamCurve> ParamCurve for Monotone<C> {
    fn eval(&self, t: f64) -> Point {
        self.0.eval(t)
    }

    fn start(&self) -> Point {
        self.0.start()
    }

    fn end(&self) -> Point {
        self.0.end()
    }

    fn subsegment(&self, range: Range) -> Self {
        Monotone(self.0.subsegment(range))
    }

    fn subdivide(&self) -> (Self, Self) {
        let (a, b) = self.0.subdivide();
        (Monotone(a), Monotone(b))
    }
}

impl<C: ParamCurve> ParamCurveExtrema for Monotone<C> {
    fn extrema(&self) -> ArrayVec<[f64; MAX_EXTREMA]> {
        ArrayVec::new()
    }

    fn extrema_ranges(&self) -> ArrayVec<[Range; 5]> {
        let mut result = ArrayVec::new();
        result.push(0.0 .. 1.0);
        result
    }

    fn bounding_box(&self) -> Rect {
        Rect::from_points(self.start(), self.end())
    }
}

impl Mul<Monotone<PathSeg>> for TranslateScale {
    type Output = Monotone<PathSeg>;

    fn mul(self, other: Monotone<PathSeg>) -> Monotone<PathSeg> {
        Monotone(other.0.apply_translate_scale(self))
    }
}

impl<C> std::ops::Deref for Monotone<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
