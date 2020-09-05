use super::*;
use arrayvec::{Array, ArrayVec};
use kurbo::MAX_EXTREMA;
use std::ops::Mul;

/// A wrapper for curves that are monotone in both dimensions.
///
/// This auto-derefs to the wrapped curve, but provides some extra utility and
/// overrides `ParamCurveExtrema` such that bounding-box computation is
/// accelerated.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Monotone<C>(pub C);

impl<C: ParamCurve> Monotone<C> {
    /// The start and end point.
    pub fn points(&self) -> (Point, Point) {
        (self.start(), self.end())
    }

    /// The start/end point which is more to the left.
    pub fn left_point(&self) -> Point {
        let (start, end) = self.points();
        if start.x < end.x { start } else { end }
    }

    /// The start/end point which is more to the right.
    pub fn right_point(&self) -> Point {
        let (start, end) = self.points();
        if start.x > end.x { start } else { end }
    }

    /// The start/end point which is more to the top.
    pub fn top_point(&self) -> Point {
        let (start, end) = self.points();
        if start.y < end.y { start } else { end }
    }

    /// The start/end point which is more to the bottom.
    pub fn bot_point(&self) -> Point {
        let (start, end) = self.points();
        if start.y > end.y { start } else { end }
    }
}

impl<C: ParamCurveSolve> Monotone<C> {
    /// Find the `t` value corresponding to an `x` value, clamped to `0..1`.
    pub fn solve_one_t_for_x(&self, x: f64) -> f64 {
        let (start, end) = self.points();
        let inc = start.x < end.x;
        if (x <= start.x) == inc {
            0.0
        } else if (x >= end.x) == inc {
            1.0
        } else {
            single_root(self.0.solve_t_for_x(x))
        }
    }

    /// Find the `t` value corresponding to a `y` value, clamped to `0..1`.
    pub fn solve_one_t_for_y(&self, y: f64) -> f64 {
        let (start, end) = self.points();
        let inc = start.y < end.y;
        if (y <= start.y) == inc {
            0.0
        } else if (y >= end.y) == inc {
            1.0
        } else {
            single_root(self.0.solve_t_for_y(y))
        }
    }

    /// Find the `y` value corresponding to an `x` value, clamped to the curve's
    /// vertical range.
    pub fn solve_one_y_for_x(&self, x: f64) -> f64 {
        let (left, right) = (self.left_point(), self.right_point());
        if x <= left.x {
            left.y
        } else if x >= right.x {
            right.y
        } else {
            single_root(self.0.solve_y_for_x(x))
        }
    }

    /// Find the `x` value corresponding to a `y` value, clamped to the curve's
    /// horizontal range.
    pub fn solve_one_x_for_y(&self, y: f64) -> f64 {
        let (top, bot) = (self.top_point(), self.bot_point());
        if y <= top.y {
            top.x
        } else if y >= bot.y {
            bot.x
        } else {
            single_root(self.0.solve_x_for_y(y))
        }
    }

    /// Finds the minimal `x` value for this curve in the given vertical range.
    pub fn solve_min_x(&self, vr: Range) -> f64 {
        let (start, end) = self.points();
        self.solve_one_x_for_y(if (start.x < end.x) == (start.y < end.y) {
            vr.start
        } else {
            vr.end
        })
    }

    /// Finds the maximal `x` value for this curve in the given vertical range.
    pub fn solve_max_x(&self, vr: Range) -> f64 {
        self.solve_min_x(vr.end .. vr.start)
    }
}

/// Extract exactly one root or panic.
fn single_root<A: Array<Item = f64>>(vec: ArrayVec<A>) -> f64 {
    match vec.as_slice() {
        [x] => *x,
        [] => panic!("there should be at least one root"),
        _ => panic!("there should be at most one root"),
    }
}

impl Monotone<PathSeg> {
    /// Reverses the path segment.
    pub fn reverse(self) -> Self {
        Monotone(self.0.reverse())
    }

    /// Intersects two monotone path segments, solving analytically if possible
    /// and falling back to bounding box search if not.
    pub fn intersect<A>(&self, other: &Self, accuracy: f64) -> ArrayVec<A>
    where
        A: Array<Item = Point>,
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

impl<C: ApproxEq> ApproxEq for Monotone<C> {
    fn approx_eq(&self, other: &Self, tolerance: f64) -> bool {
        self.0.approx_eq(&other.0, tolerance)
    }
}

impl<C> std::ops::Deref for Monotone<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Monotone<PathSeg> {
        Monotone(PathSeg::Line(Line::new((10.0, 30.0), (50.0, 20.0))))
    }

    #[test]
    fn test_solve_one_t_value_and_coordinate_for_monotone_curve() {
        let line = line();

        assert_approx_eq!(line.solve_one_t_for_x(-10.0), 0.0);
        assert_approx_eq!(line.solve_one_t_for_y(25.0), 0.5);
        assert_approx_eq!(line.solve_one_y_for_x(-10.0), 30.0);
        assert_approx_eq!(line.solve_one_y_for_x(30.0), 25.0);
        assert_approx_eq!(line.solve_one_y_for_x(50.0), 20.0);
        assert_approx_eq!(line.solve_one_x_for_y(30.0), 10.0);
        assert_approx_eq!(line.reverse().solve_one_t_for_x(-10.0), 1.0);
        assert_approx_eq!(line.reverse().solve_one_x_for_y(30.0), 10.0);
    }

    #[test]
    fn test_solve_min_and_max_x_for_monotone_curve() {
        let line = line();

        assert_approx_eq!(line.solve_max_x(25.0 .. 30.0), 30.0);
        assert_approx_eq!(line.solve_min_x(25.0 .. 30.0), 10.0);
        assert_approx_eq!(line.reverse().solve_max_x(25.0 .. 30.0), 30.0);
        assert_approx_eq!(line.reverse().solve_min_x(25.0 .. 30.0), 10.0);
    }
}
