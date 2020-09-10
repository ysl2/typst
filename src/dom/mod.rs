//! A styled, order-independent representation of a document.

mod style;

pub use style::*;

use std::rc::Rc;

use crate::syntax::{Code, Raw, SpanVec, Spanned};

pub type DomTree = SpanVec<StyledNode>;

#[derive(Debug, Clone, PartialEq)]
pub struct StyledNode {
    pub node: DomNode,
    pub style: Rc<Style>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomNode {
    Space,
    Linebreak,
    Parbreak,
    Text(String),
    Heading(Heading),
    Raw(Raw),
    Code(Code),
}

/// A section heading.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    /// The section depth (number of  hashtags minus 1).
    pub level: Spanned<u8>,
    pub body: DomTree,
}
