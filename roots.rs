//! Root-finding for polynomials up to degree 3.

use arrayvec::{ArrayVec, Array};
use roots::Roots;

/// Find all roots of the linear polynomial `ax + b`.
pub fn solve_linear(a: f32, b: f32) -> ArrayVec<[f32; 1]> {
    root_vec(roots::find_roots_linear(a, b))
}

/// Find all roots of the quadratic polynomial `ax^2 + bx + c`.
pub fn solve_quadratic(a: f32, b: f32, c: f32) -> ArrayVec<[f32; 2]> {
    root_vec(roots::find_roots_quadratic(a, b, c))
}

/// Find all roots of the cubic polynomial `ax^3 + bx^2 + cx + d`.
pub fn solve_cubic(a: f32, b: f32, c: f32, d: f32) -> ArrayVec<[f32; 3]> {
    root_vec(roots::find_roots_cubic(a, b, c, d))
}

/// Convert a `Roots` enum to an `ArrayVec`.
fn root_vec<A: Array<Item=f32>>(roots: Roots<f32>) -> ArrayVec<A> {
    let mut vec = ArrayVec::new();

    match roots {
        Roots::No(_) => {}
        Roots::One([t1]) => {
            vec.push(t1);
        }
        Roots::Two([t1, t2]) => {
            vec.push(t1);
            vec.push(t2);
        }
        Roots::Three([t1, t2, t3]) => {
            vec.push(t1);
            vec.push(t2);
            vec.push(t3);
        }
        Roots::Four([t1, t2, t3, t4]) => {
            vec.push(t1);
            vec.push(t2);
            vec.push(t3);
            vec.push(t4);
        }
    }

    vec
}
