//! Layouting of DOMs into collections of layouts.

pub mod elements;
pub mod primitive;
pub mod shaping;
pub mod stack;

pub use primitive::*;

use std::ops::Deref;

use crate::dom::{DomNode, DomTree};
use crate::font::SharedFontLoader;
use crate::geom::shape::{BezPath, Rect, ShapeGroup};
use crate::geom::{Dim, Point, Size};
use crate::Pass;

use elements::LayoutElement;
use shaping::{shape, ShapeOptions};
use stack::{StackLayouter, StackOptions};

/// Process a syntax tree into a collection of layouts.
pub async fn layout(tree: &DomTree, loader: SharedFontLoader) -> Pass<Vec<Layout>> {
    let mut loader = loader.borrow_mut();

    // FIXME: Don't assume page style.
    let page = crate::dom::PageStyle::default();
    let margins = page.margins();
    let area = Area {
        size: page.size,
        usable: page.size.to_rect().inset(margins),
        shape: None,
    };

    let areas = Areas::new(vec![area], Overflow::Spill);
    let mut stack = StackLayouter::new(areas, StackOptions { dir: Dir::TTB });

    for node in tree {
        match &node.v {
            DomNode::Text { text, style } => {
                let layout = shape(text, ShapeOptions {
                    loader: &mut loader,
                    style: &style,
                    dir: Dir::LTR,
                })
                .await;

                stack.layout_movable(GenAlign::Start, layout)
            }

            _ => {}
        }
    }

    Pass::ok(stack.finish())
}

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

    pub fn push_layout(&mut self, pos: Point, layout: Layout) {
        for (delta, element) in layout.elements {
            self.push(pos + delta.to_vec2(), element);
        }
    }
}

pub trait Layoutable {}

pub trait Layouter {
    fn remaining(&self) -> (Option<&Area>, &Areas);
    fn spacing(&mut self, axis: SpecAxis, amount: f64);
    fn layout_movable(&mut self, align: GenAlign, layout: Layout);
    fn layout_immovable(&mut self, pos: Point, collider: Collider, layout: Layout);
}

#[derive(Debug, Clone)]
pub enum LayoutItem {
    Space,
    Parbreak,
    Layout(GenAlign, Layout),
    Spacing(SpecAxis, f64),
}

#[derive(Debug, Clone)]
pub struct Areas {
    vec: Vec<Area>,
    overflow: Overflow,
}

impl Areas {
    pub fn new(vec: Vec<Area>, overflow: Overflow) -> Self {
        Self { vec, overflow }
    }

    pub fn next(&mut self) -> Option<Area> {
        if self.vec.is_empty() {
            None
        } else if self.vec.len() > 1 || self.overflow == Overflow::Stop {
            Some(self.vec.remove(0))
        } else {
            Some(self.vec[0].clone())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Area {
    pub size: Size,
    pub usable: Rect,
    pub shape: Option<ShapeGroup>,
}

#[allow(unused)]
impl Area {
    pub fn place(&self, dim: Dim, side: Side) -> Option<Point> {
        const EPS: f64 = 1e-4;

        // TODO: Support shapes and more than just top.
        assert_eq!(side, Side::Top);
        assert!(self.shape.is_none());

        if self.usable.width() + EPS > dim.width
            && self.usable.height() + EPS > dim.height + dim.depth
        {
            Some(Point::new(self.usable.x0, self.usable.y0 + dim.height))
        } else {
            None
        }
    }

    pub fn shrink_by(&mut self, by: f64, side: Side) {
        match side {
            Side::Left => self.usable.x0 = (self.usable.x0 + by).min(self.usable.x1),
            Side::Right => self.usable.x1 = (self.usable.x1 - by).max(self.usable.x0),
            Side::Top => self.usable.y0 = (self.usable.y0 + by).min(self.usable.y1),
            Side::Bottom => self.usable.y1 = (self.usable.y1 - by).max(self.usable.y0),
        }
    }

    pub fn shrink_to(&mut self, to: f64, side: Side) {
        match side {
            Side::Left => self.usable.x0 = to.min(self.usable.x1),
            Side::Right => self.usable.x1 = to.max(self.usable.x0),
            Side::Top => self.usable.y0 = to.min(self.usable.y1),
            Side::Bottom => self.usable.y1 = to.max(self.usable.y0),
        }
    }

    pub fn add(&mut self, path: &BezPath) {
        todo!("add")
    }

    pub fn subtract(&mut self, path: &BezPath) {
        todo!("subtract")
    }
}

impl Deref for Areas {
    type Target = [Area];

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Collider {
    None,
    Tight,
    Bounds,
    Row,
    Column,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Overflow {
    Stop,
    Spill,
}
