//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens][tokens]. Then, a [parser] constructs a syntax tree from
//!   the token stream. The structures describing the tree can be found in the
//!   [syntax] module.
//! - **Execution:** The next step is to [execute] the parsed "script" to build a
//!   reusable [DOM-like representation] of the document. The DOM nodes are
//!   self-contained with their style and thus order-independent. This lack of
//!   global state makes the DOM much better suited for layouting than the syntax tree.
//! - **Layouting:** The next step is to transform the DOM tree into a
//!   portable representation of the typesetted document. Types for this can be
//!   found in the [layout] module. The final output consists of a vector of
//!   [`Layouts`] (or pages), ready for exporting.
//! - **Exporting:** The finished layouts can finally be exported into a supported
//!   output format. Submodules for these formats are located in the [export] module.
//!   Currently, the only supported output format is [_PDF_].
//!
//! [tokens]: parsing/struct.Tokens.html
//! [parser]: parsing/fn.parse.html
//! [syntax]: syntax/index.html
//! [execute]: exec/fn.exec.html
//! [DOM-like representation]: dom/index.html
//! [layout]: layout/index.html
//! [`Layouts`]: layout/struct.Layout.html
//! [export]: export/index.html
//! [_PDF_]: export/pdf/index.html

#[macro_use]
mod macros;
#[macro_use]
pub mod diagnostic;

pub mod color;
pub mod dom;
pub mod exec;
pub mod export;
pub mod font;
pub mod geom;
pub mod layout;
pub mod length;
pub mod library;
pub mod paper;
pub mod parse;
pub mod prelude;
pub mod syntax;

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::diagnostic::Diagnostics;
use crate::dom::Style;
use crate::exec::Scope;
use crate::font::SharedFontLoader;
use crate::layout::Layout;
use crate::syntax::{Decos, Offset, Pos};

/// A dynamic future type which allows recursive invocation of async functions
/// when used as the return type.
pub type DynFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Layout source code directly (combines parsing, execution and layouting).
pub async fn typeset(
    src: &str,
    loader: SharedFontLoader,
    style: Rc<Style>,
    funcs: Scope,
) -> Pass<Vec<Layout>> {
    let Pass { output: tree, mut feedback } = parse::parse(src);
    let Pass { output: dom, feedback: f2 } = exec::exec(tree, style, funcs);
    let Pass { output: layouts, feedback: f3 } = layout::layout(&dom, loader).await;

    feedback.extend(f2);
    feedback.extend(f3);

    Pass::new(layouts, feedback)
}

/// The result of some pass: Some output `T` and feedback data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pass<T> {
    /// The output of this compilation pass.
    pub output: T,
    /// User feedback data accumulated in this pass.
    pub feedback: Feedback,
}

impl<T> Pass<T> {
    /// Create a new pass from output and feedback data.
    pub fn new(output: T, feedback: Feedback) -> Self {
        Self { output, feedback }
    }

    /// Create a new pass with empty feedback.
    pub fn ok(output: T) -> Self {
        Self { output, feedback: Feedback::new() }
    }

    /// Map the output type and keep the feedback data.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Pass<U> {
        Pass {
            output: f(self.output),
            feedback: self.feedback,
        }
    }
}

/// Diagnostic and semantic syntax highlighting data.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Feedback {
    /// Diagnostics about the source code.
    pub diagnostics: Diagnostics,
    /// Decorations of the source code for semantic syntax highlighting.
    pub decos: Decos,
}

impl Feedback {
    /// Create a new feedback instance without errors and decos.
    pub fn new() -> Self {
        Self { diagnostics: vec![], decos: vec![] }
    }

    /// Merged two feedbacks into one.
    pub fn merge(mut a: Self, b: Self) -> Self {
        a.extend(b);
        a
    }

    /// Add other feedback data to this feedback.
    pub fn extend(&mut self, more: Self) {
        self.diagnostics.extend(more.diagnostics);
        self.decos.extend(more.decos);
    }

    /// Add more feedback whose spans are local and need to be offset by an
    /// `offset` to be correct in this feedback's context.
    pub fn extend_offset(&mut self, more: Self, offset: Pos) {
        self.diagnostics.extend(more.diagnostics.offset(offset));
        self.decos.extend(more.decos.offset(offset));
    }
}
