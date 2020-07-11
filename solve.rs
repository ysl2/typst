use arrayvec::{Array, ArrayVec};
use super::*;

/// The maximum number of solved `t` values for a coordinate value that can be
/// reported in the `ParamCurveSolve` trait.
pub const MAX_SOLVE: usize = 3;

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
    const EPSILON: f64 = 1e-6;

    let c3 = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let c2 = 3.0 * p0 - 6.0 * p1 + 3.0 * p2;
    let c1 = -3.0 * p0 + 3.0 * p1;
    let c0 = p0 - v;

    // Solve quadratic to prevent loss of precision when c3 is very small.
    if c3.abs() < EPSILON {
        filter_t(roots::solve_quadratic(c0, c1, c2))
    } else {
        filter_t(roots::solve_cubic(c0, c1, c2, c3))
    }
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
    use super::*;

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
}
