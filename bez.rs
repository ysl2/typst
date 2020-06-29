use arrayvec::ArrayVec;
use super::{roots, CubicBez, ParamCurve};

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

impl ParamCurveSolve for CubicBez {
    fn solve_t_for_x(&self, x: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_t_for_v(self.p0.x, self.p1.x, self.p2.x, self.p3.x, x)
    }

    fn solve_t_for_y(&self, y: f64) -> ArrayVec<[f64; MAX_SOLVE]> {
        solve_t_for_v(self.p0.y, self.p1.y, self.p2.y, self.p3.y, y)
    }
}

/// Find all `t` values where the curve has the given `v` value in the dimension
/// for which the control values are given.
fn solve_t_for_v(
    p0: f64,
    p1: f64,
    p2: f64,
    p3: f64,
    v: f64,
) -> ArrayVec<[f64; 3]> {
    const EPS: f64 = 1e-4;

    let c3 = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let c2 = 3.0 * p0 - 6.0 * p1 + 3.0 * p2;
    let c1 = -3.0 * p0 + 3.0 * p1;
    let c0 = p0 - v;

    roots::solve_cubic(c0, c1, c2, c3)
        .into_iter()
        .filter(|&t| -EPS <= t && t <= 1.0 + EPS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::Point;
    use super::*;

    fn simple_curve() -> CubicBez {
        CubicBez {
            p0: Point::new(0.0, 0.0),
            p1: Point::new(35.0, 0.0),
            p2: Point::new(80.0, 35.0),
            p3: Point::new(80.0, 70.0),
        }
    }

    #[test]
    fn test_bez_point_for_t() {
        let bez = simple_curve();

        assert_approx_eq!(bez.eval(0.0), bez.p0);
        assert_approx_eq!(bez.eval(1.0), bez.p3);

        let point = Point::new(32.7, 8.5);
        assert_approx_eq!(bez.eval(0.3), point, tolerance=0.1);
    }

    #[test]
    fn test_bez_solve_for_coordinate_for_different_sampled_points() {
        let eps = 1e-3;
        let bez = simple_curve();

        let ts = [0.01, 0.2, 0.5, 0.7, 0.99];
        for &t in &ts {
            let Point { x, y } = bez.eval(t);

            assert_approx_eq!(bez.solve_t_for_x(x).to_vec(), vec![t], tolerance=eps);
            assert_approx_eq!(bez.solve_t_for_y(y).to_vec(), vec![t], tolerance=eps);
            assert_approx_eq!(bez.solve_y_for_x(x).to_vec(), vec![y], tolerance=eps);
            assert_approx_eq!(bez.solve_x_for_y(y).to_vec(), vec![x], tolerance=eps);
        }
    }

    #[test]
    fn test_bez_solve_for_coordinate_out_of_bounds() {
        let bez = simple_curve();

        assert!(bez.solve_x_for_y(-10.0).is_empty());
        assert!(bez.solve_x_for_y(100.0).is_empty());
        assert!(bez.solve_y_for_x(-20.0).is_empty());
        assert!(bez.solve_y_for_x(100.0).is_empty());
    }
}
