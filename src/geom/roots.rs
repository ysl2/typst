//! Root-finding for polynomials up to degree 3.

use arrayvec::ArrayVec;

pub use kurbo::common::{solve_cubic, solve_quadratic};

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
