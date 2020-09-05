//! Layouting of syntax trees into box layouts.

pub mod elements;
pub mod primitive;
pub mod shaping;

/// Basic types used across the layouting engine.
pub mod prelude {
    pub use super::primitive::*;
    pub use super::Layout;
    pub use Dir::*;
    pub use GenAlign::*;
    pub use GenAxis::*;
    pub use SpecAlign::*;
    pub use SpecAxis::*;
}

pub use primitive::*;

use crate::compute::Scope;
use crate::font::SharedFontLoader;
use crate::geom::{Dim, Point, Size};
use crate::style::LayoutStyle;
use crate::syntax::tree::SyntaxTree;
use crate::{Feedback, Pass};

use elements::LayoutElement;

/// A layout consisting of atomic elements.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The dimensions of the layout.
    ///
    /// A layout has a width, height and depth. The total height of the layout is
    /// `height + depth` and the distribution of total height to height and depth
    /// determines the baseline of the layout and thus how this layout is aligned with
    /// other layouts in a line of text.
    pub dim: Dim,
    pub elements: Vec<(Point, LayoutElement)>,
}

impl Layout {
    pub fn new(dim: Dim) -> Self {
        Self { dim, elements: vec![] }
    }

    pub fn size(&self) -> Size {
        self.dim.to_size()
    }

    pub fn push(&mut self, pos: Point, element: LayoutElement) {
        self.elements.push((pos, element));
    }

    pub fn push_group(&mut self, pos: Point, group: Layout) {
        for (delta, element) in group.elements {
            self.push(pos + delta.to_vec2(), element);
        }
    }
}

/// Process a syntax tree into a collection of layouts.
#[allow(unused_variables)]
pub async fn layout(
    tree: &SyntaxTree,
    loader: SharedFontLoader,
    state: State,
) -> Pass<Vec<Layout>> {
    todo!("layout")
}

/// The layouting environment.
pub struct Env {
    /// The accumulated feedback.
    pub f: Feedback,
    /// The font loader to retrieve fonts from.
    pub loader: SharedFontLoader,
    /// The current execution state. As long as the available fonts are the same,
    /// layouting is pure with respect to the layouted thing and this state.
    pub state: State,
}

/// The execution state.
#[derive(Debug, Default, Clone)]
pub struct State {
    /// The scope which contains function definitions.
    pub scope: Scope,
    /// The current style configuration.
    pub style: LayoutStyle,
}
