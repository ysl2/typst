use super::{Point, Size};

/// An axis-aligned rectangle defined by a minimum and maximum point.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rect {
    /// The minimum (bottom-left) point.
    pub min: Point,
    /// The maximum (top-right) point.
    pub max: Point,
}

impl Rect {
    /// Create a rectangle from minimum and maximum point.
    pub fn new(min: Point, max: Point) -> Rect {
        Rect { min, max }
    }

    /// The size (width / height) of this rectangle.
    pub fn size(self) -> Size {
        Size::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }

    /// The tightest rectangle that contains this and another rectangle.
    pub fn union(self, other: Rect) -> Rect {
        Rect {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

impl_approx_eq!(Rect [min, max]);
