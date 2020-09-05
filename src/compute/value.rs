//! Computational values: Syntactical expressions can be evaluated into these.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::convert::TryFromValue;
use super::table::{SpannedEntry, Table};
use crate::color::RgbaColor;
use crate::layout::Env;
use crate::length::Length;
use crate::syntax::span::{Span, Spanned};
use crate::syntax::tree::{Ident, SyntaxNode, SyntaxTree};
use crate::{DynFuture, Feedback};

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
    /// A syntax tree containing typesetting content.
    Tree(SyntaxTree),
    /// An executable function.
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
    /// Transform this value into a syntax tree.
    ///
    /// If this is already a tree, it is simply unwrapped, otherwise it's transformed into
    /// a tree that represents it reasonably.
    pub fn repr_tree(self) -> SyntaxTree {
        match self.v {
            Value::Tree(tree) => tree,

            // Forward to each entry, separated with spaces.
            Value::Table(table) => {
                let mut tree = SyntaxTree::new();

                let mut end = None;
                for entry in table.into_values() {
                    if let Some(last_end) = end {
                        let span = Span::new(last_end, entry.key.start);
                        tree.push(Spanned::new(SyntaxNode::Spacing, span));
                    }

                    end = Some(entry.val.span.end);
                    tree.extend(entry.val.repr_tree());
                }

                tree
            }

            // Fallback: Format with Debug.
            val => vec![Spanned::new(
                SyntaxNode::Text(format!("{:?}", val)),
                self.span,
            )],
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

/// An executable function value.
///
/// The first argument is a table containing the arguments passed to the
/// function. The function may be asynchronous (as such it returns a dynamic
/// future) and it may emit diagnostics, which are contained in the returned
/// `Pass`. In the end, the function must evaluate to `Value`. Your typical
/// typesetting function will return a `Commands` value which will instruct the
/// layouting engine to do what the function pleases.
///
/// The dynamic function object is wrapped in an `Rc` to keep `Value` clonable.
#[derive(Clone)]
pub struct FuncValue(pub Rc<FuncType>);

type FuncType = dyn Fn(Span, TableValue, &mut Env) -> DynFuture<Value>;

impl Deref for FuncValue {
    type Target = FuncType;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Eq for FuncValue {}

impl PartialEq for FuncValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
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

impl TableValue {
    /// Retrieve and remove the matching value with the lowest number key,
    /// skipping and ignoring all non-matching entries with lower keys.
    pub fn take<T: TryFromValue>(&mut self) -> Option<T> {
        for (&key, entry) in self.nums() {
            let expr = entry.val.as_ref();
            if let Some(val) = T::try_from_value(expr, &mut Feedback::new()) {
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
    pub fn expect<T: TryFromValue>(
        &mut self,
        name: &str,
        span: Span,
        f: &mut Feedback,
    ) -> Option<T> {
        while let Some((num, _)) = self.first() {
            let entry = self.remove(num).unwrap();
            if let Some(val) = T::try_from_value(entry.val.as_ref(), f) {
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
        T: TryFromValue,
    {
        self.remove(key).and_then(|entry| {
            let expr = entry.val.as_ref();
            T::try_from_value(expr, f)
        })
    }

    /// Retrieve and remove all matching pairs with number keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_num<'a, T>(&'a mut self) -> impl Iterator<Item = (u64, T)> + 'a
    where
        T: TryFromValue,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (&key, entry) in self.nums().skip(skip) {
                let expr = entry.val.as_ref();
                if let Some(val) = T::try_from_value(expr, &mut Feedback::new()) {
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
        T: TryFromValue,
    {
        self.take_all_num::<T>().map(|(_, v)| v)
    }

    /// Retrieve and remove all matching pairs with string keys, skipping and
    /// ignoring non-matching entries.
    ///
    /// The pairs are returned in order of increasing keys.
    pub fn take_all_str<'a, T>(&'a mut self) -> impl Iterator<Item = (String, T)> + 'a
    where
        T: TryFromValue,
    {
        let mut skip = 0;
        std::iter::from_fn(move || {
            for (key, entry) in self.strs().skip(skip) {
                let expr = entry.val.as_ref();
                if let Some(val) = T::try_from_value(expr, &mut Feedback::new()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(value: Value) -> SpannedEntry<Value> {
        SpannedEntry::val(Spanned::zero(value))
    }

    #[test]
    fn test_table_take_removes_correct_entry() {
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert(2, entry(Value::Str("hi".to_string())));
        assert_eq!(table.take::<String>(), Some("hi".to_string()));
        assert_eq!(table.len(), 1);
        assert_eq!(table.take::<bool>(), Some(false));
        assert!(table.is_empty());
    }

    #[test]
    fn test_table_expect_errors_about_previous_entries() {
        let mut f = Feedback::new();
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert(3, entry(Value::Str("hi".to_string())));
        table.insert(5, entry(Value::Bool(true)));
        assert_eq!(
            table.expect::<String>("", Span::ZERO, &mut f),
            Some("hi".to_string())
        );
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected string, found bool"
        )]);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_table_take_with_key_removes_the_entry() {
        let mut f = Feedback::new();
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert("hi", entry(Value::Bool(true)));
        assert_eq!(table.take::<bool>(), Some(false));
        assert_eq!(table.take_key::<f64>("hi", &mut f), None);
        assert_eq!(f.diagnostics, [error!(
            Span::ZERO,
            "expected number, found bool"
        )]);
        assert!(table.is_empty());
    }

    #[test]
    fn test_table_take_all_removes_the_correct_entries() {
        let mut table = Table::new();
        table.insert(1, entry(Value::Bool(false)));
        table.insert(3, entry(Value::Number(0.0)));
        table.insert(7, entry(Value::Bool(true)));
        assert_eq!(table.take_all_num::<bool>().collect::<Vec<_>>(), [
            (1, false),
            (7, true)
        ],);
        assert_eq!(table.len(), 1);
        assert_eq!(table[3].val.v, Value::Number(0.0));
    }
}
