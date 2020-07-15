use std::fmt;
use std::ops::*;

/// A flexible (_base_ / _shrink_ / _stretch_) value.
///
/// It has a base value, but can be shrunk down to `base - shrink` and stretched
/// up to `base + stretch`.
#[derive(Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct Flex {
    pub base: f64,
    pub shrink: f64,
    pub stretch: f64,
}

impl Flex {
    /// The flex length that has all components set to zero.
    pub const ZERO: Flex = Flex { base: 0.0, shrink: 0.0, stretch: 0.0 };

    /// Create a new flex length from `shrink`, `base` and `stretch` values.
    pub fn new(base: f64, shrink: f64, stretch: f64) -> Flex {
        Flex { base, shrink, stretch }
    }

    /// Create a new flex length fixed to an `base` value.
    ///
    /// This sets both `shrink` and `stretch` to zero.
    pub fn fixed(base: f64) -> Flex {
        Flex {
            base,
            shrink: 0.0,
            stretch: 0.0,
        }
    }

    /// The result of applied the given adjustment to this flex length.
    ///
    /// An adjustment of:
    /// - 0 will just keep the `base` value
    /// - -1 will shrink as much as possible leaving `base - shrink`
    /// - 2 will stretch by a factor of 2 yielding `base + 2 * stretch`.
    pub fn adjusted(self, adjustment: f64) -> f64 {
        if adjustment < 0.0 {
            self.base + adjustment * self.shrink
        } else {
            self.base + adjustment * self.stretch
        }
    }
}

impl_approx_eq!(Flex [base, shrink, stretch]);

impl Add for Flex {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            base: self.base + other.base,
            shrink: self.shrink + other.shrink,
            stretch: self.stretch + other.stretch,
        }
    }
}

impl AddAssign for Flex {
    fn add_assign(&mut self, other: Self) {
        self.base += other.base;
        self.shrink += other.shrink;
        self.stretch += other.stretch;
    }
}

impl Sub for Flex {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            base: self.base - other.base,
            shrink: self.shrink - other.shrink,
            stretch: self.stretch - other.stretch,
        }
    }
}

impl SubAssign for Flex {
    fn sub_assign(&mut self, other: Self) {
        self.base -= other.base;
        self.shrink -= other.shrink;
        self.stretch -= other.stretch;
    }
}

impl fmt::Debug for Flex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},{},{})", self.base, self.shrink, self.stretch)
    }
}
