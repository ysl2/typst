use std::ops::Mul;
use super::ApproxEq;

/// A value that is either absolute or relative.
///
/// This can capture, for example, both `5cm` and `60%`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Scale<T> {
    /// An absolute value.
    Abs(T),
    /// A relative value.
    Rel(f64),
}

impl<T> Scale<T>
where
    T: Mul<f64, Output=T>
{
    /// Returns either the absolute value or computes the relative value as a
    /// fraction of `one`.
    ///
    /// # Example
    /// ```
    /// # use layr::{assert_approx_eq, geom::Scale};
    /// assert_approx_eq!(Scale::Rel(0.5).resolve(5.0), 2.5);
    /// ```
    pub fn resolve(self, one: T) -> T {
        match self {
            Scale::Abs(t) => t,
            Scale::Rel(p) => one * p,
        }
    }
}

impl<T: ApproxEq> ApproxEq for Scale<T>{
    fn approx_eq(&self, other: &Self, tolerance: f64) -> bool {
        match (self, other) {
            (Scale::Abs(x), Scale::Abs(y)) => x.approx_eq(y, tolerance),
            (Scale::Rel(x), Scale::Rel(y)) => x.approx_eq(y, tolerance),
            _ => false,
        }
    }
}
