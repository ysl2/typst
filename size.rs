use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::ops::*;
use super::{Length, Range};

/// Alias because associated constants cannot be used.
const ZERO: Length = Length::ZERO;

/// The size (_width_ / _height_) of an object.
#[derive(Default, Copy, Clone, PartialEq)]
pub struct Size {
    /// The width of the object.
    pub width: Length,
    /// The height of the object.
    pub height: Length,
}

impl Size {
    /// The size wich has both values set to zero.
    pub const ZERO: Size = Size {
        width: ZERO,
        height: ZERO,
    };

    /// Create a new size from `width` and `height`.
    pub fn new(width: Length, height: Length) -> Size {
        Size { width, height }
    }

    /// Create a new size with the same value for `width` and `height`.
    pub fn uniform(value: Length) -> Size {
        Size {
            width: value,
            height: value,
        }
    }

    /// A size with the minimum width and height values of this and another
    /// size.
    pub fn min(self, other: Size) -> Size {
        Size {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }

    /// A size with the maximum width and height values of this and another
    /// size.
    pub fn max(self, other: Size) -> Size {
        Size {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }
}

impl_approx_eq!(Size [width, height]);

impl Mul<f32> for Size {
    type Output = Self;

    fn mul(self, other: f32) -> Self {
        Self {
            width: self.width * other,
            height: self.height * other,
        }
    }
}

impl MulAssign<f32> for Size {
    fn mul_assign(&mut self, other: f32) {
        self.width *= other;
        self.height *= other;
    }
}

impl Mul<Size> for f32 {
    type Output = Size;

    fn mul(self, other: Size) -> Size {
        Size {
            width: self * other.width,
            height: self * other.height,
        }
    }
}

impl Div<f32> for Size {
    type Output = Self;

    fn div(self, other: f32) -> Self {
        Self {
            width: self.width / other,
            height: self.height / other,
        }
    }
}

impl DivAssign<f32> for Size {
    fn div_assign(&mut self, other: f32) {
        self.width /= other;
        self.height /= other;
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            width: -self.width,
            height: -self.height,
        }
    }
}

impl Debug for Size {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}*{})", self.width, self.height)
    }
}

/// Vertical dimensions (_height_ / _depth_) of an object with baseline.
///
/// Similarly to an iceberg or a plant, objects rise above (height) and below
/// (depth) a baseline. When a line is layouted, all objects are arranged such
/// that their ground level (baseline) is at equal height.
///
/// Note that VDims can be compared:
/// ```
/// # use layr::geom::{pt, VDim};
/// let line = VDim::new(pt(20.0), pt(4.0));
/// let word = VDim::new(pt(16.0), pt(4.0));
///
/// assert!(word <= line);
/// assert!(!(word >= line));
/// ```
/// For `a < b` to be true, both the height and depth of `a` must be smaller
/// than that of `b`. Note that `!(a <= b)` does not imply `b >= a` since for
/// `a` the height could be larger while for `b` the depth. This means that
/// v-dims are not totally ordered.
#[derive(Default, Copy, Clone, PartialEq)]
pub struct VDim {
    /// The rise above the baseline.
    pub height: Length,
    /// The descent below the baseline.
    pub depth: Length,
}

impl VDim {
    /// The v-dim wich has both values set to zero.
    pub const ZERO: VDim = VDim {
        height: ZERO,
        depth: ZERO,
    };

    /// Create a new v-dim from `height` and `depth`.
    pub fn new(height: Length, depth: Length) -> VDim {
        VDim { height, depth }
    }

    /// Create a new v-dim with the same value for `height` and `depth`.
    pub fn uniform(value: Length) -> VDim {
        VDim {
            height: value,
            depth: value,
        }
    }

    /// A v-dim with the minimum height and depth values of this and another
    /// v-dim.
    pub fn min(self, other: VDim) -> VDim {
        VDim {
            height: self.height.min(other.height),
            depth: self.depth.min(other.depth),
        }
    }

    /// A v-dim with the maximum height and depth values of this and another
    /// v-dim.
    pub fn max(self, other: VDim) -> VDim {
        VDim {
            height: self.height.max(other.height),
            depth: self.depth.max(other.depth),
        }
    }

    /// The vertical range spanned by an element with this v-dim placed on the
    /// given baseline.
    pub fn v_range(self, baseline: Length) -> Range {
        Range::new(baseline - self.height, baseline + self.depth)
    }
}

impl PartialOrd for VDim {
    fn partial_cmp(&self, other: &VDim) -> Option<Ordering> {
        if self.lt(other) {
            Some(Ordering::Less)
        } else if self.gt(other) {
            Some(Ordering::Greater)
        } else if self.eq(other) {
            Some(Ordering::Equal)
        } else {
            None
        }
    }

    fn lt(&self, other: &VDim) -> bool {
        self.height < other.height && self.depth < other.depth
    }

    fn le(&self, other: &VDim) -> bool {
        self.height <= other.height && self.depth <= other.depth
    }

    fn ge(&self, other: &VDim) -> bool {
        self.height >= other.height && self.depth >= other.depth
    }

    fn gt(&self, other: &VDim) -> bool {
        self.height > other.height && self.depth > other.depth
    }
}

impl_approx_eq!(VDim [height, depth]);

impl Debug for VDim {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "(+{}-{})", self.height, self.depth)
    }
}

/// Dimensions (_width_ / _height_ / _depth_) of an object with baseline.
#[derive(Default, Copy, Clone, PartialEq)]
pub struct Dim {
    /// The width of the object.
    pub width: Length,
    /// The height of the object (extent above the basleline).
    pub height: Length,
    /// The depth of the object (extent below the baseline).
    pub depth: Length,
}

impl Dim {
    /// The dimensions wich have all three values set to zero.
    pub const ZERO: Dim = Dim {
        width: ZERO,
        height: ZERO,
        depth: ZERO,
    };

    /// Create a new instance from `width`, `height` and `depth`.
    pub fn new(width: Length, height: Length, depth: Length) -> Dim {
        Dim { width, height, depth }
    }

    /// Create a new instance from `width` and `v`ertical dimensions.
    pub fn with_vdim(width: Length, v: VDim) -> Dim {
        Dim { width, height: v.height, depth: v.height }
    }

    /// Get the height and depth as a v-dim.
    pub fn vdim(self) -> VDim {
        VDim { height: self.height, depth: self.depth }
    }
}

impl_approx_eq!(Dim [width, height, depth]);

impl Debug for Dim {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}*{:?})", self.width, self.vdim())
    }
}

/// Margins (_left_ / _top_ / _right_ / _bottom_) to be applied inside or
/// outside of an object.
#[derive(Copy, Clone, PartialEq)]
pub struct Margins {
    pub left: Length,
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
}

impl Margins {
    /// Margins with zero value for all sides.
    pub const ZERO: Margins = Margins {
        left: ZERO,
        top: ZERO,
        right: ZERO,
        bottom: ZERO,
    };

    /// Create a new instance from the four values.
    pub fn new(left: Length, top: Length, right: Length, bottom: Length) -> Margins {
        Margins { left, top, right, bottom }
    }

    /// Create a new instance with the same value for all sides.
    pub fn uniform(v: Length) -> Margins {
        Margins {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }

    /// Create a new instance with the same value for the opposing sides.
    pub fn uniform_axes(horizontal: Length, vertical: Length) -> Margins {
        Margins {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }
}

impl_approx_eq!(Margins [left, top, right, bottom]);

impl Debug for Margins {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "({},{},{},{})",
            self.left, self.top, self.right, self.bottom
        )
    }
}
