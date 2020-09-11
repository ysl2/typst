//! The syntax tree.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;

use fontdock::{FontStyle, FontWeight, FontWidth};

use super::span::{Span, SpanVec, Spanned};
use crate::color::RgbaColor;
use crate::eval::dict::{Dict, SpannedEntry};
use crate::layout::{Dir, SpecAlign};
use crate::length::{Length, ScaleLength};
use crate::paper::Paper;
use crate::parse::is_ident;
use crate::Feedback;

/// A collection of syntax nodes which form a tree together with the their children.
pub type SyntaxTree = SpanVec<SyntaxNode>;

/// A syntax node, which encompasses a single logical entity of parsed source
/// code.
#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxNode {
    /// Whitespace containing less than two newlines.
    Space,
    /// A forced line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// Italics were enabled / disabled.
    ToggleItalic,
    /// Bolder was enabled / disabled.
    ToggleBolder,
    /// Plain text.
    Text(String),
    /// Section headings.
    Heading(Heading<SyntaxTree>),
    /// Lines of raw text.
    Raw(Raw),
    /// An optionally highlighted (multi-line) code block.
    Code(Code),
    /// A function call.
    Call(Call),
}

/// A section heading.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading<T> {
    /// The section depth from `0` to `5`
    /// (corresponds to the number of hashtags minus `1`).
    pub level: Spanned<u8>,
    /// The contents of the heading.
    pub contents: T,
}

/// Raw text.
#[derive(Debug, Clone, PartialEq)]
pub struct Raw {
    /// The lines of raw text (raw text is split at newlines by the parser).
    pub lines: Vec<String>,
}

/// A code block.
#[derive(Debug, Clone, PartialEq)]
pub struct Code {
    /// The language to highlight the code in if present. If this is `None`, no syntax
    /// highlighting should be applied.
    pub lang: Option<Spanned<Ident>>,
    /// The lines of raw text (code is split at newlines by the parser).
    pub lines: Vec<String>,
    /// Whether this code element is "block"-level.
    ///
    /// - If true, this should be separated into its own paragraph
    ///   independently of its surroundings.
    /// - If false, this element can be set inline.
    pub block: bool,
}

/// An invocation of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The name of the invoked function.
    pub name: Spanned<Ident>,
    /// The arguments passed to the function.
    ///
    /// If the function had a body, the last argument is of type `Expr::Tree`, if a
    /// function was chained after this one it is resolved as an argument of type
    /// `Expr::Call`.
    pub args: DictExpr,
}

/// An identifier as defined by unicode with a few extra permissible characters.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string checking that it is a valid.
    pub fn new(ident: impl AsRef<str> + Into<String>) -> Option<Self> {
        if is_ident(ident.as_ref()) {
            Some(Self(ident.into()))
        } else {
            None
        }
    }

    /// Return a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Debug for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "`{}`", self.0)
    }
}

/// An expression.
#[derive(Clone, PartialEq)]
pub enum Expr {
    /// An identifier: `ident`.
    Ident(Ident),
    /// A string: `"string"`.
    Str(String),
    /// A boolean: `true, false`.
    Bool(bool),
    /// A number: `1.2, 200%`.
    Number(f64),
    /// A length: `2cm, 5.2in`.
    Length(Length),
    /// A color value with alpha channel: `#f79143ff`.
    Color(RgbaColor),
    /// A dictionary expression: `(false, 12cm, greeting="hi")`.
    Dict(DictExpr),
    /// A syntax tree containing typesetting content.
    Tree(SyntaxTree),
    /// A function call: `cmyk(37.7, 0, 3.9, 1.1)`.
    Call(Call),
    /// An operation that negates the contained expression.
    Neg(Box<Spanned<Expr>>),
    /// An operation that adds the contained expressions.
    Add(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that subtracts the contained expressions.
    Sub(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that multiplies the contained expressions.
    Mul(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
    /// An operation that divides the contained expressions.
    Div(Box<Spanned<Expr>>, Box<Spanned<Expr>>),
}

impl Expr {
    /// A natural-language name of the type of this expression, e.g.
    /// "identifier".
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ident(_) => "identifier",
            Self::Str(_) => "string",
            Self::Bool(_) => "bool",
            Self::Number(_) => "number",
            Self::Length(_) => "length",
            Self::Color(_) => "color",
            Self::Dict(_) => "dict",
            Self::Tree(_) => "syntax tree",
            Self::Call(_) => "function call",
            Self::Neg(_) => "negation",
            Self::Add(_, _) => "addition",
            Self::Sub(_, _) => "subtraction",
            Self::Mul(_, _) => "multiplication",
            Self::Div(_, _) => "division",
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Ident(i) => i.fmt(f),
            Self::Str(s) => s.fmt(f),
            Self::Bool(b) => b.fmt(f),
            Self::Number(n) => n.fmt(f),
            Self::Length(s) => s.fmt(f),
            Self::Color(c) => c.fmt(f),
            Self::Dict(t) => t.fmt(f),
            Self::Tree(t) => t.fmt(f),
            Self::Call(c) => c.fmt(f),
            Self::Neg(e) => write!(f, "-{:?}", e),
            Self::Add(a, b) => write!(f, "({:?} + {:?})", a, b),
            Self::Sub(a, b) => write!(f, "({:?} - {:?})", a, b),
            Self::Mul(a, b) => write!(f, "({:?} * {:?})", a, b),
            Self::Div(a, b) => write!(f, "({:?} / {:?})", a, b),
        }
    }
}

/// A dictionary of expressions.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type DictExpr = Dict<SpannedEntry<Expr>>;

impl DictExpr {
    /// Retrieve and remove the matching value with the lowest number key,
    /// skipping and ignoring all non-matching entries with lower keys.
    pub fn take<T: TryFromExpr>(&mut self) -> Option<T> {
        for (&key, entry) in self.nums() {
            let expr = entry.val.as_ref();
            if let Some(val) = T::try_from_expr(expr, &mut Feedback::new()) {
                self.remove(key);
                return Some(val);
            }
        }
        None
    }

    /// Retrieve and remove the matching value with the lowest number key,
    /// removing and generating errors for all non-matching entries with lower
    /// keys.
    ///
    /// Generates an error at `err_span` when no matching value was found.
    pub fn expect<T: TryFromExpr>(
        &mut self,
        name: &str,
        span: Span,
        f: &mut Feedback,
    ) -> Option<T> {
        while let Some((num, _)) = self.first() {
            let entry = self.remove(num).unwrap();
            if let Some(val) = T::try_from_expr(entry.val.as_ref(), f) {
                return Some(val);
            }
        }
        error!(@f, span, "missing argument: {}", name);
        None
    }

    /// Retrieve and remove a matching value associated with the given key if
    /// there is any.
    ///
    /// Generates an error if the key exists but the value does not match.
    pub fn take_key<T>(&mut self, key: &str, f: &mut Feedback) -> Option<T>
    where
        T: TryFromExpr,
    {
        self.remove(key).and_then(|entry| {
            let expr = entry.val.as_ref();
            T::try_from_expr(expr, f)
        })
    }

    /// Retrieve and remove all matching pairs with number keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_num<'a, T>(&'a mut self) -> impl Iterator<Item = (u64, T)> + 'a
    where
        T: TryFromExpr,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (&key, entry) in self.nums().skip(skip) {
                let expr = entry.val.as_ref();
                if let Some(val) = T::try_from_expr(expr, &mut Feedback::new()) {
                    self.remove(key);
                    return Some((key, val));
                }
                skip += 1;
            }

            None
        })
    }


    /// Retrieve and remove all matching values with number keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The values are returned in order of increasing keys.
    pub fn take_all_num_vals<'a, T: 'a>(&'a mut self) -> impl Iterator<Item = T> + 'a
    where
        T: TryFromExpr,
    {
        self.take_all_num::<T>().map(|(_, v)| v)
    }

    /// Retrieve and remove all matching pairs with string keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_str<'a, T>(&'a mut self) -> impl Iterator<Item = (String, T)> + 'a
    where
        T: TryFromExpr,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (key, entry) in self.strs().skip(skip) {
                let expr = entry.val.as_ref();
                if let Some(val) = T::try_from_expr(expr, &mut Feedback::new()) {
                    let key = key.clone();
                    self.remove(&key);
                    return Some((key, val));
                }
                skip += 1;
            }

            None
        })
    }

    /// Generated `"unexpected argument"` errors for all remaining entries.
    pub fn unexpected(&self, f: &mut Feedback) {
        for entry in self.values() {
            let span = Span::merge(entry.key, entry.val.span);
            error!(@f, span, "unexpected argument");
        }
    }
}

/// A trait for converting values into more specific types.
pub trait TryFromExpr: Sized {
    // This trait takes references because we don't want to move the value
    // out of its origin in case this returns `None`. This solution is not
    // perfect because we need to do some cloning in the impls for this trait,
    // but we haven't got a better solution, for now.

    /// Try to convert a value to this type.
    ///
    /// Returns `None` and generates an appropriate error if the value is not
    /// valid for this type.
    fn try_from_expr(value: Spanned<&Expr>, f: &mut Feedback) -> Option<Self>;
}

impl<T: TryFromExpr> TryFromExpr for Spanned<T> {
    fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
        let span = expr.span;
        T::try_from_expr(expr, f).map(|v| Spanned { v, span })
    }
}

macro_rules! impl_match {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl TryFromExpr for $type {
            fn try_from_expr(expr: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
                #[allow(unreachable_patterns)]
                match expr.v {
                    $($p => Some($r)),*,
                    other => {
                        error!(
                            @f, expr.span,
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
        impl TryFromExpr for $type {
            fn try_from_expr(value: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
                if let Expr::Ident(ident) = value.v {
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

impl_match!(Expr, "expr", e => e.clone());
impl_match!(Ident, "identifier", Expr::Ident(i) => i.clone());
impl_match!(String, "string", Expr::Str(s) => s.clone());
impl_match!(bool, "bool", &Expr::Bool(b) => b);
impl_match!(f64, "number", &Expr::Number(n) => n);
impl_match!(Length, "length", &Expr::Length(l) => l);
impl_match!(SyntaxTree, "tree", Expr::Tree(t) => t.clone());
impl_match!(DictExpr, "dict", Expr::Dict(t) => t.clone());
impl_match!(ScaleLength, "number or length",
    &Expr::Length(length) => ScaleLength::Absolute(length),
    &Expr::Number(scale) => ScaleLength::Scaled(scale),
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
    Expr::Ident(Ident(s)) => StringLike(s.clone()),
    Expr::Str(s) => StringLike(s.clone()),
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

impl TryFromExpr for FontWeight {
    fn try_from_expr(value: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
        match value.v {
            &Expr::Number(weight) => {
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
            Expr::Ident(ident) => {
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

impl TryFromExpr for FontWidth {
    fn try_from_expr(value: Spanned<&Expr>, f: &mut Feedback) -> Option<Self> {
        match value.v {
            &Expr::Number(width) => {
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
            Expr::Ident(ident) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Spanned;

    fn entry(value: Expr) -> SpannedEntry<Expr> {
        SpannedEntry::val(Spanned::zero(value))
    }

    #[test]
    fn test_dict_take_removes_correct_entry() {
        let mut dict = Dict::new();
        dict.insert(1, entry(Expr::Bool(false)));
        dict.insert(2, entry(Expr::Str("hi".to_string())));
        assert_eq!(dict.take::<String>(), Some("hi".to_string()));
        assert_eq!(dict.len(), 1);
        assert_eq!(dict.take::<bool>(), Some(false));
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dict_expect_errors_about_previous_entries() {
        let mut f = Feedback::new();
        let mut dict = Dict::new();
        dict.insert(1, entry(Expr::Bool(false)));
        dict.insert(3, entry(Expr::Str("hi".to_string())));
        dict.insert(5, entry(Expr::Bool(true)));
        assert_eq!(
            dict.expect::<String>("", Span::ZERO, &mut f),
            Some("hi".to_string())
        );
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected string, found bool"
        )]);
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_dict_take_with_key_removes_the_entry() {
        let mut f = Feedback::new();
        let mut dict = Dict::new();
        dict.insert(1, entry(Expr::Bool(false)));
        dict.insert("hi", entry(Expr::Bool(true)));
        assert_eq!(dict.take::<bool>(), Some(false));
        assert_eq!(dict.take_key::<f64>("hi", &mut f), None);
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected number, found bool"
        )]);
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dict_take_all_removes_the_correct_entries() {
        let mut dict = Dict::new();
        dict.insert(1, entry(Expr::Bool(false)));
        dict.insert(3, entry(Expr::Number(0.0)));
        dict.insert(7, entry(Expr::Bool(true)));
        assert_eq!(dict.take_all_num::<bool>().collect::<Vec<_>>(), [
            (1, false),
            (7, true)
        ],);
        assert_eq!(dict.len(), 1);
        assert_eq!(dict[3].val.v, Expr::Number(0.0));
    }
}
