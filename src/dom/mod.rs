//! A styled, order-independent representation of a document.

mod style;

pub use style::*;

use std::any::Any;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

use crate::layout::Layoutable;
use crate::syntax::{Heading, Raw, SpanVec};

/// A collection of DOM nodes which form a tree together with the their children.
pub type DomTree = SpanVec<DomNode>;

/// A self-contained node in the DOM.
///
/// There are a number of pre-defined nodes, but custom behaviour is enabled through the
/// `DynNode` variant.
#[derive(Debug, Clone, PartialEq)]
pub enum DomNode {
    /// A whitespace node with a given width.
    Space { width: f64 },
    /// A line break node with height and padding for the line that is started.
    Linebreak { line_height: f64, line_padding: f64 },
    /// A paragraph node with an amount of spacing to be applied between the finished and
    /// the next paragraph (if one follows).
    Parbreak { par_spacing: f64 },

    /// A text node to be set with the associated style.
    Text { text: String, style: Rc<TextStyle> },
    /// An optionally syntax-highlighted block of raw text or code.
    Raw { raw: Raw, style: Rc<TextStyle> },

    /// A section heading.
    Heading(Heading<DomTree>),

    /// A dynamic node which can implement custom layouting behaviour.
    Dyn(BoxedNode),
}

/// A wrapper around a boxed dynamic node.
//
// Note: This is needed because the compiler can't `derive(PartialEq)`
//       for `DomNode` when directly putting the boxed node in there,
//       see https://github.com/rust-lang/rust/issues/31740
#[derive(Debug, Clone)]
pub struct BoxedNode(pub Box<dyn DynNode>);

impl BoxedNode {
    /// Wrap a type implementing `DynNode`.
    pub fn new<T: DynNode>(inner: T) -> Self {
        Self(Box::new(inner))
    }
}

impl PartialEq for BoxedNode {
    fn eq(&self, other: &Self) -> bool {
        &self.0 == &other.0
    }
}

impl Deref for BoxedNode {
    type Target = dyn DynNode;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// A dynamic DOM node, which can implement custom layouting behaviour.
///
/// This trait just combines the requirements for types to qualify as dynamic nodes. The
/// interesting part happens in the inherited traits like `Layoutable`.
///
/// The trait itself also contains three helper methods to make `Box<dyn DynNode>` able to
/// implement `Clone` and `PartialEq`. However, these are automatically provided by a
/// blanket impl as long as the type in question implements `Debug`, `Layoutable`,
/// `PartialEq`, `Clone` and is `'static`.
pub trait DynNode: Debug + Layoutable + 'static {
    /// Convert into a `dyn Any` to enable downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Check for equality with another trait object.
    fn dyn_eq(&self, other: &dyn DynNode) -> bool;

    /// Clone into a trait object.
    fn dyn_clone(&self) -> Box<dyn DynNode>;
}

impl<T> DynNode for T
where
    T: Debug + Layoutable + PartialEq + Clone + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn DynNode) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_clone(&self) -> Box<dyn DynNode> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn DynNode> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

impl PartialEq for Box<dyn DynNode> {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_ref())
    }
}
