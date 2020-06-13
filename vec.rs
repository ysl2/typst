use std::fmt::{self, Debug, Formatter};
use std::iter::Sum;
use std::ops::*;
use super::{Length, Point};

/// A vector (_x_ / _y_) in 2D space.
#[derive(Default, Copy, Clone, PartialEq)]
pub struct Vec2 {
    /// The horizontal coordinate.
    pub x: Length,
    /// The vertical coordinate.
    pub y: Length,
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Vec2 = Vec2 {
        x: Length::ZERO,
        y: Length::ZERO,
    };

    /// Create a new vector from `x` and `y` coordinates.
    pub fn new(x: Length, y: Length) -> Vec2 {
        Vec2 { x, y }
    }

    /// Create a new vector with `x` set to a value and `y` set to zero.
    pub fn with_x(x: Length) -> Vec2 {
        Vec2 { x, y: Length::ZERO }
    }

    /// Create a new vector with `y` set to a value and `x` set to zero.
    pub fn with_y(y: Length) -> Vec2 {
        Vec2 { x: Length::ZERO, y }
    }

    /// Create a new vector with `x` and `y` set to the same value.
    pub fn uniform(v: Length) -> Vec2 {
        Vec2 { x: v, y: v }
    }

    /// Returns the point defined by this vector.
    pub fn to_point(self) -> Point {
        Point { x: self.x, y: self.y }
    }
}

impl_approx_eq!(Vec2 [x, y]);

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, other: f32) -> Vec2 {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, other: f32) {
        self.x *= other;
        self.y *= other;
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;

    fn mul(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self * other.x,
            y: self * other.y,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;

    fn div(self, other: f32) -> Vec2 {
        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, other: f32) {
        self.x /= other;
        self.y /= other;
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Sum for Vec2 {
    fn sum<I: Iterator<Item = Vec2>>(iter: I) -> Vec2 {
        iter.fold(Vec2::ZERO, Add::add)
    }
}

impl Debug for Vec2 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}
