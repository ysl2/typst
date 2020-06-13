use std::fmt::{self, Debug, Formatter};
use std::ops::*;
use super::{Length, Vec2};

/// A position (_x_ / _y_) in 2D space.
#[derive(Default, Copy, Clone, PartialEq)]
pub struct Point {
    /// The horizontal coordinate.
    pub x: Length,
    /// The vertical coordinate.
    pub y: Length,
}

impl Point {
    /// The zero (origin) point.
    pub const ZERO: Point = Point {
        x: Length::ZERO,
        y: Length::ZERO,
    };

    /// Create a new point from `x` and `y` coordinates.
    pub fn new(x: Length, y: Length) -> Point {
        Point { x, y }
    }

    /// Create a new point with `x` set to a value and `y` set to zero.
    pub fn with_x(x: Length) -> Point {
        Point { x, y: Length::ZERO }
    }

    /// Create a new point with `y` set to a value and `x` set to zero.
    pub fn with_y(y: Length) -> Point {
        Point { x: Length::ZERO, y }
    }

    /// Create a new point with `x` and `y` set to the same value.
    pub fn uniform(v: Length) -> Point {
        Point { x: v, y: v }
    }

    /// A point with the minimum coordinates of this and another point.
    pub fn min(self, other: Point) -> Point {
        Point {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// A point with the maximum coordinates of this and another point.
    pub fn max(self, other: Point) -> Point {
        Point {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }
}

impl_approx_eq!(Point [x, y]);

impl Add<Vec2> for Point {
    type Output = Self;

    fn add(self, other: Vec2) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl AddAssign<Vec2> for Point {
    fn add_assign(&mut self, other: Vec2) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl Sub for Point {
    type Output = Vec2;

    fn sub(self, other: Self) -> Vec2 {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Sub<Vec2> for Point {
    type Output = Self;

    fn sub(self, other: Vec2) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl SubAssign<Vec2> for Point {
    fn sub_assign(&mut self, other: Vec2) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl Debug for Point {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}
