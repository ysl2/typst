use std::ops::Deref;

use super::Value;
use crate::diag::{bail, SourceResult};
use crate::syntax::Span;

/// A value that can possibly be mutated.
pub enum MaybeMut<'a> {
    /// A mutable value that can be assigned to.
    Mut(&'a mut Value),
    /// An immutable value that cannot be assigned to.
    Im(Value, Span, Immutability),
}

impl<'a> MaybeMut<'a> {
    /// Create an immutable `MaybeMut` from a temporary value.
    pub fn temp(value: Value, span: Span) -> Self {
        Self::Im(value, span, Immutability::Temp)
    }

    /// Try to mutate if possible.
    pub fn mutate(self) -> SourceResult<&'a mut Value> {
        match self {
            Self::Mut(v) => Ok(v),
            Self::Im(_, span, reason) => bail!(span, "{}", reason.error()),
        }
    }

    /// Extract an owned value.
    pub fn take(self) -> Value {
        match self {
            Self::Mut(v) => v.clone(),
            Self::Im(v, ..) => v,
        }
    }

    /// Attach a span to the value, if possible.
    pub fn spanned(self, span: Span) -> Self {
        match self {
            Self::Im(v, im, s) => Self::Im(v.spanned(span), im, s),
            v => v,
        }
    }
}

impl Deref for MaybeMut<'_> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Mut(v) => v,
            Self::Im(v, ..) => v,
        }
    }
}

/// The reason a value is immutable.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Immutability {
    /// The value is temporary because it resulting from an expression like
    /// `1 + 2`.
    Temp,
    /// The value is constant because it is a standard library definition.
    Const,
    /// The value stems from a captured variable in a closure.
    Captured,
}

impl Immutability {
    /// The error message when trying to assign to the immutable value.
    fn error(&self) -> &'static str {
        match self {
            Self::Temp => "cannot mutate a temporary value",
            Self::Const => "cannot mutate a constant",
            Self::Captured => {
                "variables from outside the function are \
                 read-only and cannot be modified"
            }
        }
    }
}
