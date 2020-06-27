use std::fmt;
use std::iter::Sum;
use std::ops::*;
use std::str::FromStr;

/// The base type for all distances and sizes in space.
#[derive(Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct Length {
    /// The length in typographic points (1/72 inches).
    pt: f32,
}

impl Length {
    /// The zero length.
    pub const ZERO: Length = Length { pt: 0.0 };

    /// The infinite length.
    ///
    /// This may not make much sense conceptually, but it's nonetheless useful
    /// for initializing values which depend on comparisons.
    pub const INF: Length = Length { pt: f32::INFINITY };

    /// The negative infinite length.
    pub const NEG_INF: Length = Length { pt: f32::NEG_INFINITY };

    /// Create a length from an amount of points.
    pub const fn pt(pt: f32) -> Length {
        Length { pt }
    }

    /// Create a length from an amount of millimeters.
    pub fn mm(mm: f32) -> Length {
        Length { pt: 2.83465 * mm }
    }

    /// Create a length from an amount of centimeters.
    pub fn cm(cm: f32) -> Length {
        Length { pt: 28.3465 * cm }
    }

    /// Create a length from an amount of inches.
    pub fn inches(inches: f32) -> Length {
        Length { pt: 72.0 * inches }
    }

    /// Convert this length into points.
    pub fn to_pt(self) -> f32 {
        self.pt
    }

    /// Convert this length into millimeters.
    pub fn to_mm(self) -> f32 {
        self.pt * 0.352778
    }

    /// Convert this length into centimeters.
    pub fn to_cm(self) -> f32 {
        self.pt * 0.0352778
    }

    /// Convert this length into inches.
    pub fn to_inches(self) -> f32 {
        self.pt * 0.0138889
    }

    /// The maximum of this and the other length.
    pub fn max(self, other: Length) -> Length {
        if self > other { self } else { other }
    }

    /// The minimum of this and the other length.
    pub fn min(self, other: Length) -> Length {
        if self <= other { self } else { other }
    }

    /// Set this length to the maximum of itself and the other length.
    pub fn make_max(&mut self, other: Length) {
        *self = self.max(other);
    }

    /// Set this length to the minimum of itself and the other length.
    pub fn make_min(&mut self, other: Length) {
        *self = self.min(other);
    }
}

/// Shorthand for [`Length::pt`].
pub const fn pt(pt: f32) -> Length {
    Length { pt }
}

/// Alternative form for [`Length::min`].
pub fn min(a: Length, b: Length) -> Length {
    a.min(b)
}

/// Alternative form for [`Length::max`].
pub fn max(a: Length, b: Length) -> Length {
    a.max(b)
}

impl_approx_eq!(Length [pt]);

impl Add for Length {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { pt: self.pt + other.pt }
    }
}

impl AddAssign for Length {
    fn add_assign(&mut self, other: Self) {
        self.pt += other.pt;
    }
}

impl Sub for Length {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self { pt: self.pt - other.pt }
    }
}

impl SubAssign for Length {
    fn sub_assign(&mut self, other: Self) {
        self.pt -= other.pt;
    }
}

impl Mul<f32> for Length {
    type Output = Self;

    fn mul(self, other: f32) -> Self {
        Self { pt: self.pt * other }
    }
}

impl MulAssign<f32> for Length {
    fn mul_assign(&mut self, other: f32) {
        self.pt *= other;
    }
}

impl Mul<Length> for f32 {
    type Output = Length;

    fn mul(self, other: Length) -> Length {
        Length {
            pt: self * other.pt,
        }
    }
}

impl Div for Length {
    type Output = f32;

    fn div(self, other: Self) -> f32 {
        self.pt / other.pt
    }
}

impl Div<f32> for Length {
    type Output = Self;

    fn div(self, other: f32) -> Self {
        Self { pt: self.pt / other }
    }
}

impl DivAssign<f32> for Length {
    fn div_assign(&mut self, other: f32) {
        self.pt /= other;
    }
}

impl Neg for Length {
    type Output = Self;

    fn neg(self) -> Self {
        Self { pt: -self.pt }
    }
}

impl Sum for Length {
    fn sum<I: Iterator<Item = Length>>(iter: I) -> Length {
        iter.fold(Length::ZERO, Add::add)
    }
}

impl fmt::Debug for Length {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}pt", self.pt)
    }
}

impl FromStr for Length {
    type Err = ParseLengthError;

    fn from_str(src: &str) -> Result<Length, ParseLengthError> {
        let scale = match () {
            _ if src.ends_with("pt") => Length::pt,
            _ if src.ends_with("mm") => Length::mm,
            _ if src.ends_with("cm") => Length::cm,
            _ if src.ends_with("in") => Length::inches,
            _ => return Err(ParseLengthError),
        };

        match src[..src.len() - 2].parse::<f32>() {
            Ok(value) => Ok(scale(value)),
            Err(_) => Err(ParseLengthError),
        }
    }
}

/// An error that can occur when parsing a length from a string.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ParseLengthError;

impl std::error::Error for ParseLengthError {}

impl fmt::Display for ParseLengthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("invalid string for length")
    }
}

/// A flexible (_base_ / _shrink_ / _stretch_) length.
///
/// It has a base value, but can be shrunk down to `base - shrink` and stretched
/// up to `base + stretch`.
#[derive(Default, Copy, Clone, PartialEq, PartialOrd)]
pub struct FlexLength {
    pub base: Length,
    pub shrink: Length,
    pub stretch: Length,
}

impl FlexLength {
    /// The flex length that has all components set to zero.
    pub const ZERO: FlexLength = FlexLength {
        base: Length::ZERO,
        shrink: Length::ZERO,
        stretch: Length::ZERO,
    };

    /// Create a new flex length from `shrink`, `base` and `stretch` values.
    pub fn new(base: Length, shrink: Length, stretch: Length) -> FlexLength {
        FlexLength { base, shrink, stretch }
    }

    /// Create a new flex length fixed to an `base` value.
    ///
    /// This sets both `shrink` and `stretch` to zero.
    pub fn fixed(base: Length) -> FlexLength {
        FlexLength {
            base,
            shrink: Length::ZERO,
            stretch: Length::ZERO,
        }
    }

    /// The result of applied the given adjustment to this flex length.
    ///
    /// An adjustment of:
    /// - 0 will just keep the `base` value
    /// - -1 will shrink as much as possible leaving `base - shrink`
    /// - 2 will stretch by a factor of 2 yielding `base + 2 * stretch`.
    pub fn adjusted(self, adjustment: f32) -> Length {
        if adjustment < 0.0 {
            self.base + adjustment * self.shrink
        } else {
            self.base + adjustment * self.stretch
        }
    }
}

impl_approx_eq!(FlexLength [base, shrink, stretch]);

impl Add for FlexLength {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            base: self.base + other.base,
            shrink: self.shrink + other.shrink,
            stretch: self.stretch + other.stretch,
        }
    }
}

impl AddAssign for FlexLength {
    fn add_assign(&mut self, other: Self) {
        self.base += other.base;
        self.shrink += other.shrink;
        self.stretch += other.stretch;
    }
}

impl Sub for FlexLength {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            base: self.base - other.base,
            shrink: self.shrink - other.shrink,
            stretch: self.stretch - other.stretch,
        }
    }
}

impl SubAssign for FlexLength {
    fn sub_assign(&mut self, other: Self) {
        self.base -= other.base;
        self.shrink -= other.shrink;
        self.stretch -= other.stretch;
    }
}

impl fmt::Debug for FlexLength {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},{},{})", self.base, self.shrink, self.stretch)
    }
}
