//! The syntax tree.

use std::fmt::{self, Debug, Formatter};

use super::span::{SpanVec, Spanned};
use crate::color::RgbaColor;
use crate::eval::table::{SpannedEntry, Table};
use crate::length::Length;
use crate::parse::is_ident;

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
    pub args: TableExpr,
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
    /// A table expression: `(false, 12cm, greeting="hi")`.
    Table(TableExpr),
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
        use Expr::*;
        match self {
            Ident(_) => "identifier",
            Str(_) => "string",
            Bool(_) => "bool",
            Number(_) => "number",
            Length(_) => "length",
            Color(_) => "color",
            Table(_) => "table",
            Tree(_) => "syntax tree",
            Call(_) => "function call",
            Neg(_) => "negation",
            Add(_, _) => "addition",
            Sub(_, _) => "subtraction",
            Mul(_, _) => "multiplication",
            Div(_, _) => "division",
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expr::*;
        match self {
            Ident(i) => i.fmt(f),
            Str(s) => s.fmt(f),
            Bool(b) => b.fmt(f),
            Number(n) => n.fmt(f),
            Length(s) => s.fmt(f),
            Color(c) => c.fmt(f),
            Table(t) => t.fmt(f),
            Tree(t) => t.fmt(f),
            Call(c) => c.fmt(f),
            Neg(e) => write!(f, "-{:?}", e),
            Add(a, b) => write!(f, "({:?} + {:?})", a, b),
            Sub(a, b) => write!(f, "({:?} - {:?})", a, b),
            Mul(a, b) => write!(f, "({:?} * {:?})", a, b),
            Div(a, b) => write!(f, "({:?} / {:?})", a, b),
        }
    }
}

/// A table of expressions.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type TableExpr = Table<SpannedEntry<Expr>>;
