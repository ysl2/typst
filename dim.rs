use std::cmp::Ordering;
use std::fmt;
use super::Range;

/// Dimensions (_width_ / _height_ / _depth_) of an object with baseline.
#[derive(Default, Copy, Clone, PartialEq)]
pub struct Dim {
    /// The width of the object.
    pub width: f64,
    /// The height of the object (extent above the basleline).
    pub height: f64,
    /// The depth of the object (extent below the baseline).
    pub depth: f64,
}

impl Dim {
    /// The dimensions wich have all three values set to zero.
    pub const ZERO: Dim = Dim { width: 0.0, height: 0.0, depth: 0.0 };

    /// Create a new instance from `width`, `height` and `depth`.
    pub fn new(width: f64, height: f64, depth: f64) -> Dim {
        Dim { width, height, depth }
    }

    /// Create a new instance from `width` and `v`ertical dimensions.
    pub fn with_vdim(width: f64, v: VDim) -> Dim {
        Dim { width, height: v.height, depth: v.height }
    }

    /// Get the height and depth as a v-dim.
    pub fn vdim(self) -> VDim {
        VDim { height: self.height, depth: self.depth }
    }
}

impl_approx_eq!(Dim [width, height, depth]);

impl fmt::Debug for Dim {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}*{:?})", self.width, self.vdim())
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
/// # use layr::geom::VDim;
/// let line = VDim::new(20.0, 4.0);
/// let word = VDim::new(16.0, 4.0);
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
    pub height: f64,
    /// The descent below the baseline.
    pub depth: f64,
}

impl VDim {
    /// The v-dim wich has both values set to zero.
    pub const ZERO: VDim = VDim { height: 0.0, depth: 0.0 };

    /// Create a new v-dim from `height` and `depth`.
    pub fn new(height: f64, depth: f64) -> VDim {
        VDim { height, depth }
    }

    /// Create a new v-dim with the same value for `height` and `depth`.
    pub fn uniform(value: f64) -> VDim {
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
    pub fn v_range(self, baseline: f64) -> Range {
        Range::new(baseline - self.height, baseline + self.depth)
    }
}

impl_approx_eq!(VDim [height, depth]);

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

impl fmt::Debug for VDim {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(+{}-{})", self.height, self.depth)
    }
}
