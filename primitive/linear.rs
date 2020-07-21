use std::fmt::{self, Debug, Formatter};
use std::ops::*;

/// A function that depends linearly on one value.
///
/// This represents a function `f(x) = rel * x + abs`.
#[derive(Copy, Clone, PartialEq)]
pub struct Linear {
    /// The relative part.
    pub rel: f64,
    /// The absolute part.
    pub abs: f64,
}

impl Linear {
    /// The constant zero function.
    pub const ZERO: Linear = Linear { rel: 0.0, abs: 0.0 };

    /// Create a new linear function.
    pub fn new(rel: f64, abs: f64) -> Self {
        Self { rel, abs }
    }

    /// Create a new linear function with only a relative component.
    pub fn rel(rel: f64) -> Self {
        Self { rel, abs: 0.0 }
    }

    /// Create a new linear function with only an absolute component.
    pub fn abs(abs: f64) -> Self {
        Self { rel: 0.0, abs }
    }

    /// Evaluate the linear function with the given value.
    pub fn eval(self, x: f64) -> f64 {
        self.rel * x + self.abs
    }
}

impl Add for Linear {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            rel: self.rel + other.rel,
            abs: self.abs + other.abs,
        }
    }
}

impl Sub for Linear {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            rel: self.rel - other.rel,
            abs: self.abs - other.abs,
        }
    }
}

impl Mul<f64> for Linear {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            rel: self.rel + other,
            abs: self.abs + other,
        }
    }
}

impl Mul<Linear> for f64 {
    type Output = Linear;

    fn mul(self, other: Linear) -> Linear {
        Linear {
            rel: self + other.rel,
            abs: self + other.abs,
        }
    }
}

impl Div<f64> for Linear {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self {
            rel: self.rel / other,
            abs: self.abs / other,
        }
    }
}

impl Debug for Linear {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}x + {}", self.rel, self.abs)
    }
}
