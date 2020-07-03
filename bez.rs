use std::ops::Range;
use arrayvec::{Array, ArrayVec};
use super::{
    roots, ApproxEq, CubicBez, Line, ParamCurve, ParamCurveExtrema, PathSeg,
    Point, QuadBez, Rect, MAX_EXTREMA,
};

/// Find all the intersections of two curves.
///
/// The points are in no particular order.
///
/// The size of the array-vec can be defined by the caller to give a boost in
/// performance in situations were there is a known bound on the number of
/// intersections. This is because this function is recursive and quite a few of
/// those vecs will be allocated on the stack depending on the `accuracy`. To be
/// safe in a cubic bezier situation, use `9`. For monotone curves, use `3`.
///
/// This function computes many bounding boxes of curves. Since this operation
/// is very fast for monotone curves, consider using the `Monotone` wrapper if
/// your curves are monotone.
///
/// # Panics
/// This will panic if the capacity of the array-vec is exceeded.
pub fn intersect<C, A>(a: &C, b: &C, accuracy: f64) -> ArrayVec<A>
where
    C: ParamCurveExtrema,
    A: Array<Item=Point>,
{
    let mut result = ArrayVec::new();

    let ba = a.bounding_box();
    let bb = b.bounding_box();

    // When the bounding boxes don't overlap we have no intersection.
    if ba.x1 < bb.x0 || bb.x1 < ba.x0 || ba.y1 < bb.y0 || bb.y1 < ba.y0 {
        return result;
    }

    // When the bounding boxes do overlap, but one of the curves is smaller than
    // the accuracy, any point inside that curve is fine as our intersection, so
    // we just pick the center of its bounding box.
    if ba.width() < accuracy && ba.height() < accuracy {
        result.push(ba.center());
        return result;
    } else if bb.width() < accuracy && bb.height() < accuracy {
        result.push(bb.center());
        return result;
    }

    // When we are not at the accuracy level, we continue by subdividing both
    // curves and intersecting each pair.
    let (a1, a2) = a.subdivide();
    let (b1, b2) = b.subdivide();

    let mut extend = |values: ArrayVec<A>| {
        for point in values {
            // We don't want to count intersections twice.
            if !result.iter().any(|p| p.approx_eq(&point, 10.0 * accuracy)) {
                result.push(point);
            }
        }
    };

    extend(intersect(&a1, &b1, accuracy));
    extend(intersect(&a1, &b2, accuracy));
    extend(intersect(&a2, &b1, accuracy));
    extend(intersect(&a2, &b2, accuracy));

    result
}

/// A wrapper for curves that are monotone in both dimensions.
///
/// This auto-derefs to the wrapped curve, but overrides `ParamCurveExtrema`
/// such that bounding-box computation is accelerated.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Monotone<C>(pub C);

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

    fn subsegment(&self, range: Range<f64>) -> Self {
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

    fn extrema_ranges(&self) -> ArrayVec<[Range<f64>; 5]> {
        let mut result = ArrayVec::new();
        result.push(0.0 .. 1.0);
        result
    }

    fn bounding_box(&self) -> Rect {
        Rect::from_points(self.start(), self.end())
    }
}

impl<C> std::ops::Deref for Monotone<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A parameterized curve that can solve its `t` values for a coordinate value.
pub trait ParamCurveSolve: ParamCurve {
    /// Find the `t` values corresponding to an `x` value.
    fn solve_t_for_x(&self, x: f64) -> ArrayVec<[f64; MAX_SOLVE]>;

    /// Find the `t` values corresponding to an `y` value.
    fn solve_t_for_y(&self, y: f64) -> ArrayVec<[f64; MAX_SOLVE]>;

    /// Find the `y` values corresponding to an `x` value.
    fn solve_y_for_x(&self, x: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        self.solve_t_for_x(x)
            .into_iter()
            .map(|t| self.eval(t).y)
            .collect()
    }

    /// Find the `x` values corresponding to an `y` value.
    fn solve_x_for_y(&self, y: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        self.solve_t_for_y(y)
            .into_iter()
            .map(|t| self.eval(t).x)
            .collect()
    }
}

/// The maximum number of solved `t` values for a coordinate value that can be
/// reported in the `ParamCurveSolve` trait.
pub const MAX_SOLVE: usize = 3;

impl ParamCurveSolve for PathSeg {
    fn solve_t_for_x(&self, x: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        match self {
            PathSeg::Line(line) => line.solve_t_for_x(x),
            PathSeg::Quad(quad) => quad.solve_t_for_x(x),
            PathSeg::Cubic(cubic) => cubic.solve_t_for_x(x),
        }
    }

    fn solve_t_for_y(&self, y: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        match self {
            PathSeg::Line(line) => line.solve_t_for_y(y),
            PathSeg::Quad(quad) => quad.solve_t_for_y(y),
            PathSeg::Cubic(cubic) => cubic.solve_t_for_y(y),
        }
    }
}

impl ParamCurveSolve for CubicBez {
    fn solve_t_for_x(&self, x: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_cubic_t_for_v(self.p0.x, self.p1.x, self.p2.x, self.p3.x, x)
    }

    fn solve_t_for_y(&self, y: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_cubic_t_for_v(self.p0.y, self.p1.y, self.p2.y, self.p3.y, y)
    }
}

impl ParamCurveSolve for QuadBez {
    fn solve_t_for_x(&self, x: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_quad_t_for_v(self.p0.x, self.p1.x, self.p2.x, x)
    }

    fn solve_t_for_y(&self, y: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_quad_t_for_v(self.p0.y, self.p1.y, self.p2.y, y)
    }
}

impl ParamCurveSolve for Line {
    fn solve_t_for_x(&self, x: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_line_t_for_v(self.p0.x, self.p1.x, x)
    }

    fn solve_t_for_y(&self, y: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_line_t_for_v(self.p0.y, self.p1.y, y)
    }
}

/// Find all `t` values where the cubic curve has the given `v` value in the
/// dimension for which the control values are given.
fn solve_cubic_t_for_v(
    p0: f64,
    p1: f64,
    p2: f64,
    p3: f64,
    v: f64,
) -> ArrayVec<[f64; MAX_SOLVE]> {
    let c3 = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let c2 = 3.0 * p0 - 6.0 * p1 + 3.0 * p2;
    let c1 = -3.0 * p0 + 3.0 * p1;
    let c0 = p0 - v;

    filter_t(roots::solve_cubic(c0, c1, c2, c3))
}

/// Find all `t` values matching `v` for a quadratic curve.
fn solve_quad_t_for_v(
    p0: f64,
    p1: f64,
    p2: f64,
    v: f64,
) -> ArrayVec<[f64; MAX_SOLVE]> {
    let c2 = p0 - 2.0 * p1 + p2;
    let c1 = -2.0 * p0 + 2.0 * p1;
    let c0 = p0 - v;

    filter_t(roots::solve_quadratic(c0, c1, c2))
}

/// Find all `t` values matching `v` for a linear curve.
fn solve_line_t_for_v(
    p0: f64,
    p1: f64,
    v: f64,
) -> ArrayVec<[f64; MAX_SOLVE]> {
    let c1 = -p0 + p1;
    let c0 = p0 - v;

    filter_t(roots::solve_linear(c0, c1))
}

/// Filter out all t values that are not between 0 and 1.
fn filter_t(vec: ArrayVec<impl Array<Item=f64>>) -> ArrayVec<[f64; MAX_SOLVE]> {
    const EPSILON: f64 = 1e-6;
    vec.into_iter()
        .filter(|&t| -EPSILON <= t && t <= 1.0 + EPSILON)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{value_no_nans, BezPath, Point};
    use super::*;

    fn seg(d: &str) -> PathSeg {
        BezPath::from_svg(d).unwrap().segments().next().unwrap()
    }

    #[test]
    fn test_bez_point_for_t() {
        let bez = CubicBez {
            p0: Point::new(0.0, 0.0),
            p1: Point::new(35.0, 0.0),
            p2: Point::new(80.0, 35.0),
            p3: Point::new(80.0, 70.0),
        };

        assert_approx_eq!(bez.eval(0.0), bez.p0);
        assert_approx_eq!(bez.eval(1.0), bez.p3);

        let point = Point::new(32.7, 8.5);
        assert_approx_eq!(bez.eval(0.3), point, tolerance=0.1);
    }

    fn test_curves() -> Vec<PathSeg> {
        vec![
            PathSeg::Line(Line {
                p0: Point::new(0.0, 0.0),
                p1: Point::new(35.0, 10.0),
            }),
            PathSeg::Quad(QuadBez {
                p0: Point::new(0.0, 0.0),
                p1: Point::new(35.0, 0.0),
                p2: Point::new(80.0, 35.0),
            }),
            PathSeg::Cubic(CubicBez {
                p0: Point::new(0.0, 0.0),
                p1: Point::new(35.0, 0.0),
                p2: Point::new(80.0, 35.0),
                p3: Point::new(80.0, 70.0),
            }),
        ]
    }

    #[test]
    fn test_bez_solve_for_coordinate_for_different_sampled_points() {
        let eps = 1e-3;
        for seg in test_curves() {
            for &t in &[0.01, 0.2, 0.5, 0.7, 0.99] {
                let Point { x, y } = seg.eval(t);

                assert_approx_eq!(seg.solve_t_for_x(x).to_vec(), vec![t], tolerance=eps);
                assert_approx_eq!(seg.solve_t_for_y(y).to_vec(), vec![t], tolerance=eps);
                assert_approx_eq!(seg.solve_y_for_x(x).to_vec(), vec![y], tolerance=eps);
                assert_approx_eq!(seg.solve_x_for_y(y).to_vec(), vec![x], tolerance=eps);
            }
        }
    }

    #[test]
    fn test_bez_solve_for_coordinate_out_of_bounds() {
        for seg in test_curves() {
            assert!(seg.solve_x_for_y(-10.0).is_empty());
            assert!(seg.solve_x_for_y(100.0).is_empty());
            assert!(seg.solve_y_for_x(-20.0).is_empty());
            assert!(seg.solve_y_for_x(100.0).is_empty());
        }
    }

    #[test]
    fn test_intersect_monotone_two_intersections() {
        let a = Monotone(seg("M9 31C37.5 31 59 61 59 81"));
        let b = Monotone(seg("M21 20C21 40 42.5 70 71 70"));

        assert_approx_eq!(
            intersect::<_, [_; 3]>(&a, &b, 0.01).to_vec(),
            vec![Point::new(24.0, 34.0), Point::new(56.0, 67.0)],
            tolerance = 0.5,
        );
    }

    #[test]
    fn test_intersect_monotone_three_intersections() {
        let a = Monotone(seg("M59 81C14 74.5 37.5 31 9 31"));
        let b = Monotone(seg("M17 31C17 81 50 53 50 81"));

        let mut vec = intersect::<_, [_; 3]>(&a, &b, 0.01).to_vec();
        vec.sort_by(|a, b| value_no_nans(&a.y, &b.y));

        assert_approx_eq!(
            vec,
            vec![
                Point::new(17.0, 32.5),
                Point::new(31.5, 63.5),
                Point::new(50.0, 79.0),
            ],
            tolerance = 0.25,
        );
    }

    #[test]
    fn test_intersect_not_monotone_five_intersections() {
        let a = seg("M53 69C82 12 -2 -11 23 69");
        let b = seg("M31 63C-71 14 187 75 11 17");

        let mut vec = intersect::<_, [_; 5]>(&a, &b, 0.01).to_vec();
        vec.sort_by(|a, b| value_no_nans(&a.y, &b.y));

        assert_approx_eq!(
            vec,
            vec![
                Point::new(25.0, 21.5),
                Point::new(56.5, 33.0),
                Point::new(18.0, 42.0),
                Point::new(59.0, 44.0),
                Point::new(20.0, 57.5),
            ],
            tolerance = 0.5,
        );
    }
}
