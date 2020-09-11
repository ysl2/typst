//! Computational values, which syntactical expressions can be evaluated into.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::table::{SpannedEntry, Table};
use super::EvalCtx;
use crate::color::RgbaColor;
use crate::dom::DomTree;
use crate::length::Length;
use crate::syntax::Spanned;
use crate::syntax::{Ident, Span, TableExpr};

/// A computational value.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// The none value.
    None,
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
    /// A table value: `(false, 12cm, greeting="hi")`.
    Table(TableValue),
    /// A dom-tree containing layoutable content.
    Tree(DomTree),
    /// A value, which represents an executable function.
    Func(FuncValue),
}

impl Value {
    /// A natural-language name of the type of this expression, e.g.
    /// "identifier".
    pub fn name(&self) -> &'static str {
        use Value::*;
        match self {
            None => "none",
            Ident(_) => "identifier",
            Str(_) => "string",
            Bool(_) => "bool",
            Number(_) => "number",
            Length(_) => "length",
            Color(_) => "color",
            Table(_) => "table",
            Tree(_) => "syntax tree",
            Func(_) => "function",
        }
    }
}

impl Spanned<Value> {
    /// Flatten all DOM trees contained in this value into one.
    pub fn flatten_tree(self) -> DomTree {
        match self.v {
            // Tree is just passed through.
            Value::Tree(tree) => tree,

            // Forward to each table entry to find nested trees.
            Value::Table(table) => table
                .into_values()
                .flat_map(|entry| entry.val.flatten_tree())
                .collect(),

            _ => vec![],
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Value::*;
        match self {
            None => f.pad("none"),
            Ident(i) => i.fmt(f),
            Str(s) => s.fmt(f),
            Bool(b) => b.fmt(f),
            Number(n) => n.fmt(f),
            Length(s) => s.fmt(f),
            Color(c) => c.fmt(f),
            Table(t) => t.fmt(f),
            Tree(t) => t.fmt(f),
            Func(c) => c.fmt(f),
        }
    }
}

/// A value, which represents an executable function.
///
/// The dynamic function object is wrapped in an `Rc` to keep `Value` clonable.
#[derive(Clone)]
pub struct FuncValue(pub Rc<FuncType>);
type FuncType = dyn Fn(Span, TableExpr, &mut EvalCtx) -> Value;

impl FuncValue {
    /// Create a new function value from a rust function or closure.
    pub fn new<F: 'static>(f: F) -> Self
    where
        F: Fn(Span, TableExpr, &mut EvalCtx) -> Value,
    {
        Self(Rc::new(f))
    }
}

impl Eq for FuncValue {}

impl PartialEq for FuncValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for FuncValue {
    type Target = FuncType;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Debug for FuncValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("<function>")
    }
}

/// A table of values.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type TableValue = Table<SpannedEntry<Value>>;
