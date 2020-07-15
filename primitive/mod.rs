//! Geometric primitives.

mod dim;
mod flex;
mod scale;

pub use kurbo::{Affine, Insets, Point, Size, TranslateScale, Vec2};
pub use dim::{Dim, VDim};
pub use flex::Flex;
pub use scale::Scale;

/// A float range.
pub type Range = std::ops::Range<f64>;

impl_approx_eq!(Range [start, end]);
impl_approx_eq!(Point [x, y]);
impl_approx_eq!(Vec2 [x, y]);
impl_approx_eq!(Size [width, height]);
impl_approx_eq!(Insets [x0, x1, y0, y1]);
