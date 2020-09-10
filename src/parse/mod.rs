//! Tokenization and parsing of source code into syntax trees.

mod escaping;
mod parser;
mod tokens;

pub use parser::parse;
pub use tokens::*;

#[cfg(test)]
mod check {
    use crate::syntax::{Pos, Span, Spanned};
    use std::fmt::Debug;

    /// Assert that expected and found are equal, printing both and panicking
    /// and the source of their test case if they aren't.
    ///
    /// When `cmp_spans` is false, spans are ignored.
    pub fn check<T>(src: &str, exp: T, found: T, cmp_spans: bool)
    where
        T: Debug + PartialEq,
    {
        Span::set_cmp(cmp_spans);
        let equal = exp == found;
        Span::set_cmp(true);

        if !equal {
            println!("source:   {:?}", src);
            if cmp_spans {
                println!("expected: {:#?}", exp);
                println!("found:    {:#?}", found);
            } else {
                println!("expected: {:?}", exp);
                println!("found:    {:?}", found);
            }
            panic!("test failed");
        }
    }

    pub fn s<T>(sl: usize, sc: usize, el: usize, ec: usize, v: T) -> Spanned<T> {
        Spanned::new(v, Span::new(Pos::new(sl, sc), Pos::new(el, ec)))
    }

    // Enables tests to optionally specify spans.
    impl<T> From<T> for Spanned<T> {
        fn from(t: T) -> Self {
            Spanned::zero(t)
        }
    }
}

#[cfg(test)]
mod tests;
