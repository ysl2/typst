use std::convert::TryFrom;
use std::fmt::{self, Debug, Formatter};
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use std::rc::Rc;

use crate::diag::StrResult;
use crate::geom::{GenAxis, Linear};
use crate::layout::{BlockNode, Decoration, InlineNode, PageNode};
use crate::style::{Style, TextStyle};
use crate::util::EcoString;

/// A node: `[*Hi* there]`.
#[derive(Clone)]
pub enum Node {
    /// A word space.
    Space,
    /// A line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// A page break.
    Pagebreak,
    /// Spacing.
    Spacing(GenAxis, Linear),
    /// Plain text.
    Text(EcoString, Rc<TextStyle>),
    /// A decorated node.
    Decorated(Decoration, Box<Self>),
    /// An arbitrary inline-level node.
    Inline(InlineNode),
    /// An arbitrary block-level node.
    Block(BlockNode),
    /// A page-level node.
    Page(PageNode),
    /// A list of nodes.
    List(Vec<Node>),
}

/// The three different levels a node can be at.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Level {
    /// Inline-level nodes are layouted as part of paragraph layout.
    Inline,
    /// Block-level nodes can be layouted into a sequence of regions.
    Block,
    /// Page-level node layout directly produces frames representing pages.
    Page,
}

impl Node {
    /// Create an empty node.
    pub fn empty() -> Self {
        Self::List(vec![])
    }

    /// Create an inline-level node.
    pub fn inline(node: impl Into<InlineNode>) -> Self {
        Self::Inline(node.into())
    }

    /// Create a block-level node.
    pub fn block(node: impl Into<BlockNode>) -> Self {
        Self::Block(node.into())
    }

    /// Lift this node to the given level.
    pub fn lift(&mut self, _level: Level, _style: &Style) -> Self {
        println!("Lift To {:?}: {:#?}", _level, self);
        todo!()
    }

    /// Lift and convert to a block-level node.
    pub fn to_block(self, _style: &Style) -> BlockNode {
        println!("To Block: {:#?}", self);
        todo!()
    }

    /// Lift and convert to page-level nodes.
    pub fn to_pages(self, _style: &Style) -> Vec<PageNode> {
        println!("To Pages: {:#?}", self);
        todo!()
    }

    /// Decorate the node.
    pub fn decorate(self, deco: Decoration) -> Self {
        Self::Decorated(deco, Box::new(self))
    }

    /// Repeat this node `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this template {} times", n))?;

        Ok(Self::List(vec![self.clone(); count]))
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::empty()
    }
}

impl Add for Node {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign for Node {
    fn add_assign(&mut self, rhs: Self) {
        if let Self::List(left) = self {
            if let Self::List(right) = rhs {
                left.extend(right);
            } else {
                left.push(rhs);
            }
        } else {
            *self = Self::List(vec![std::mem::take(self), rhs]);
        }
    }
}

impl Sum for Node {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::empty(), Add::add)
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Space => f.pad("Space"),
            Self::Linebreak => f.pad("Linebreak"),
            Self::Parbreak => f.pad("Parbreak"),
            Self::Pagebreak => f.pad("Pagebreak"),
            Self::Text(text, _) => write!(f, "Text({})", text),
            Self::Spacing(axis, amount) => write!(f, "Spacing({:?}, {:?})", axis, amount),
            Self::Inline(node) => node.fmt(f),
            Self::Decorated(deco, node) => {
                f.debug_tuple("Decorated").field(deco).field(node).finish()
            }
            Self::Block(node) => node.fmt(f),
            Self::Page(node) => node.fmt(f),
            Self::List(list) => f.debug_list().entries(list).finish(),
        }
    }
}
