use std::ops::Deref;

use super::value::{FuncValue, TableValue, Value};
use crate::layout::{Dir, SpecAlign};
use crate::length::{Length, ScaleLength};
use crate::paper::Paper;
use crate::syntax::span::Spanned;
use crate::syntax::tree::{Ident, SyntaxTree};
use crate::Feedback;
use fontdock::{FontStyle, FontWeight, FontWidth};

/// A trait for converting values into more specific types.
pub trait TryFromValue: Sized {
    // This trait takes references because we don't want to move the value
    // out of its origin in case this returns `None`. This solution is not
    // perfect because we need to do some cloning in the impls for this trait,
    // but we haven't got a better solution, for now.

    /// Try to convert a value to this type.
    ///
    /// Returns `None` and generates an appropriate error if the value is not
    /// valid for this type.
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self>;
}

macro_rules! impl_match {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl TryFromValue for $type {
            fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match value.v {
                    $($p => Some($r)),*,
                    other => {
                        error!(
                            @f, value.span,
                            "expected {}, found {}", $name, other.name()
                        );
                        None
                    }
                }
            }
        }
    };
}

macro_rules! impl_ident {
    ($type:ty, $name:expr, $parse:expr) => {
        impl TryFromValue for $type {
            fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
                if let Value::Ident(ident) = value.v {
                    let val = $parse(ident.as_str());
                    if val.is_none() {
                        error!(@f, value.span, "invalid {}", $name);
                    }
                    val
                } else {
                    error!(
                        @f, value.span,
                        "expected {}, found {}", $name, value.v.name()
                    );
                    None
                }
            }
        }
    };
}

impl<T: TryFromValue> TryFromValue for Spanned<T> {
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
        let span = value.span;
        T::try_from_value(value, f).map(|v| Spanned { v, span })
    }
}

impl_match!(Value, "value", v => v.clone());
impl_match!(Ident, "identifier", Value::Ident(i) => i.clone());
impl_match!(String, "string", Value::Str(s) => s.clone());
impl_match!(bool, "bool", &Value::Bool(b) => b);
impl_match!(f64, "number", &Value::Number(n) => n);
impl_match!(Length, "length", &Value::Length(l) => l);
impl_match!(SyntaxTree, "tree", Value::Tree(t) => t.clone());
impl_match!(TableValue, "table", Value::Table(t) => t.clone());
impl_match!(FuncValue, "function", Value::Func(f) => f.clone());
impl_match!(ScaleLength, "number or length",
    &Value::Length(length) => ScaleLength::Absolute(length),
    &Value::Number(scale) => ScaleLength::Scaled(scale),
);

/// A value type that matches identifiers and strings and implements
/// `Into<String>`.
pub struct StringLike(pub String);

impl Deref for StringLike {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl From<StringLike> for String {
    fn from(like: StringLike) -> String {
        like.0
    }
}

impl_match!(StringLike, "identifier or string",
    Value::Ident(Ident(s)) => StringLike(s.clone()),
    Value::Str(s) => StringLike(s.clone()),
);

impl_ident!(Dir, "direction", |s| match s {
    "ltr" => Some(Self::LTR),
    "rtl" => Some(Self::RTL),
    "ttb" => Some(Self::TTB),
    "btt" => Some(Self::BTT),
    _ => None,
});

impl_ident!(SpecAlign, "alignment", |s| match s {
    "left" => Some(Self::Left),
    "right" => Some(Self::Right),
    "top" => Some(Self::Top),
    "bottom" => Some(Self::Bottom),
    "center" => Some(Self::Center),
    _ => None,
});

impl_ident!(FontStyle, "font style", FontStyle::from_name);
impl_ident!(Paper, "paper", Paper::from_name);

impl TryFromValue for FontWeight {
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
        match value.v {
            &Value::Number(weight) => {
                const MIN: u16 = 100;
                const MAX: u16 = 900;

                Some(Self(if weight < MIN as f64 {
                    error!(@f, value.span, "the minimum font weight is {}", MIN);
                    MIN
                } else if weight > MAX as f64 {
                    error!(@f, value.span, "the maximum font weight is {}", MAX);
                    MAX
                } else {
                    weight.round() as u16
                }))
            }
            Value::Ident(ident) => {
                let weight = Self::from_name(ident.as_str());
                if weight.is_none() {
                    error!(@f, value.span, "invalid font weight");
                }
                weight
            }
            other => {
                error!(
                    @f, value.span,
                    "expected font weight (name or number), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}

impl TryFromValue for FontWidth {
    fn try_from_value(value: Spanned<&Value>, f: &mut Feedback) -> Option<Self> {
        match value.v {
            &Value::Number(width) => {
                const MIN: u16 = 1;
                const MAX: u16 = 9;

                Self::new(if width < MIN as f64 {
                    error!(@f, value.span, "the minimum font width is {}", MIN);
                    MIN
                } else if width > MAX as f64 {
                    error!(@f, value.span, "the maximum font width is {}", MAX);
                    MAX
                } else {
                    width.round() as u16
                })
            }
            Value::Ident(ident) => {
                let width = Self::from_name(ident.as_str());
                if width.is_none() {
                    error!(@f, value.span, "invalid font width");
                }
                width
            }
            other => {
                error!(
                    @f, value.span,
                    "expected font width (name or number), found {}",
                    other.name(),
                );
                None
            }
        }
    }
}
