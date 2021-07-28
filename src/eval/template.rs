use std::mem;
use std::ops::{Add, AddAssign};

use super::State;
use crate::eco::EcoString;
use crate::geom::Length;
use crate::layout::{LayoutNode, LayoutTree, PageNode, ParChild, ParNode, StackNode};

/// A structured representation of partially styled content.
///
/// Can be layouted or instantiated to become part of a larger template,
/// inheriting the style of the instantiation site.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Template {
    /// A tree of finished page runs.
    tree: LayoutTree,
    /// A page of finished paragraphs.
    page: PageNode,
    /// The last paragraph.
    par: ParNode,
}

impl Template {
    /// Create a new, empty template.
    pub fn new() -> Self {
        Self {
            tree: LayoutTree::new(),
            page: PageNode::new(),
            par: ParNode::new(),
        }
    }

    // Apply an outer, surrounding state to the template.
    pub fn apply(&mut self, outer: &State) {
        todo!()
    }

    /// Create a template from a single inline node.
    pub fn from_inline_node(node: impl Into<LayoutNode>, state: &State) -> Self {
        let mut template = Self::new();
        template.push_inline_node(node, state);
        template
    }

    /// Create a template from a single block node.
    pub fn from_block_node(node: impl Into<LayoutNode>, state: &State) -> Self {
        let mut template = Self::new();
        template.push_block_node(node, state);
        template
    }

    /// Insert text into the template.
    pub fn push_text(&mut self, text: &str, state: &State) {
        self.par.push_text(text, state.aligns.cross, state.text);
    }

    /// Insert a word space into the paragraph.
    pub fn push_space(&mut self, state: &State) {
        self.par.push_space(state.aligns.cross, state.text);
    }

    /// Insert a linebreak into the paragraph.
    pub fn push_linebreak(&mut self, state: &State) {
        self.par.push_linebreak(state.aligns.cross, state.text);
    }

    /// Insert a paragraph break.
    pub fn push_parbreak(&mut self, state: &State) {
        self.finish_par();
        self.page
            .stack
            .push_soft_spacing(state.text.and_then(|text| text.par_spacing));
        self.par.line_spacing = state.text.and_then(|text| text.line_spacing);
        self.par.aligns = state.aligns;
    }

    /// Insert a pagebreak.
    pub fn push_pagebreak(&mut self, state: &State, hard: bool) {
        self.finish_page();
        self.page.size = state.page.map(|page| page.size).unwrap_or_default();
        self.page.hard = hard;
        self.par.line_spacing = state.text.and_then(|text| text.line_spacing);
        self.par.aligns = state.aligns;
    }

    /// Insert an arbitrary layoutable node into the active paragraph.
    pub fn push_inline_node(&mut self, node: impl Into<LayoutNode>, state: &State) {
        self.par.push_node(node, state.aligns.cross);
    }

    /// Insert an arbitrary layoutable node into the active stack.
    ///
    /// This will finish the active paragraph.
    pub fn push_block_node(&mut self, node: impl Into<LayoutNode>, state: &State) {
        self.page.stack.push_node(node, state.aligns);
    }

    /// Insert spacing into the active paragraph.
    pub fn push_inline_spacing(&mut self, spacing: Length) {
        self.par.push_spacing(spacing);
    }

    /// Insert spacing into the active stack.
    pub fn push_block_spacing(&mut self, spacing: Length) {
        self.page.stack.push_hard_spacing(spacing);
    }

    /// Convert into a paragraph.
    ///
    /// Returns `None` if the template contains a paragraph break.
    pub fn into_par(self) -> Option<ParNode> {
        (self.tree.is_empty() && self.page.is_empty()).then(|| self.par)
    }

    /// Convert into a stack.
    ///
    /// Returns `None` if the template contains a page break.
    pub fn into_stack(mut self) -> Option<StackNode> {
        self.tree.is_empty().then(|| {
            self.finish_stack();
            self.page.stack
        })
    }

    /// Convert into a layoutable top-level tree.
    pub fn into_tree(mut self) -> LayoutTree {
        self.finish_page();
        self.tree
    }

    /// Push the active paragraph into the active stack if it's not empty.
    fn finish_par(&mut self) {
        let par = mem::take(&mut self.par);
        if !par.is_empty() {
            self.page.stack.push_node(par, par.aligns);
        }
    }

    /// Remove excess soft spacing from the active stack, making it ready to be
    /// used as a layout node.
    ///
    /// Also finishes the paragraph.
    fn finish_stack(&mut self) {
        self.finish_par();
        self.page.stack.trim();
    }

    /// Push the active page into the tree if it's not empty or should be kept
    /// according to its [hardness](PageNode::hard).
    ///
    /// Also finishes the paragraph and stack.
    fn finish_page(&mut self) {
        self.finish_stack();
        let page = mem::take(&mut self.page);
        if page.hard || !page.is_empty() {
            self.tree.push_page(page);
        }
    }
}

impl Default for Template {
    fn default() -> Self {
        Self::new()
    }
}

impl From<EcoString> for Template {
    fn from(string: EcoString) -> Self {
        // FIXME
        let mut template = Self::new();
        template.push_text(&string, &State::default());
        template
    }
}

impl Add for Template {
    type Output = Self;

    fn add(mut self, other: Self) -> Self::Output {
        self += other;
        self
    }
}

impl AddAssign for Template {
    fn add_assign(&mut self, other: Self) {
        if other.tree.is_empty() && other.page.stack.is_empty() {
            // Try merging on the paragraph level.
            if self.par.dir == other.par.dir
                && self.par.aligns == other.par.aligns
                && self.par.line_spacing == other.par.line_spacing
            {
                let mut children = other.par.children.into_iter();
                if let Some(child) = children.next() {
                    match child {
                        ParChild::Text(text, align, state) => {
                            self.par.push_text(&text, align, state)
                        }
                        other => self.par.children.push(other),
                    }
                }
                self.par.children.extend(children);
                return;
            }
        } else if other.tree.is_empty() {
            // Try merging on the stack level. Start with finishing paragraph
            // because what follows has its own stack.
            self.finish_par();
            self.page.stack.push_soft_spacing(None);
            self.page.stack.children.extend(other.page.stack.children);
            self.par = other.par;
        } else {
            // Merge on the tree level. Start with finishing page because what
            // follows has its own pages.
            self.finish_page();
            self.tree.pages.extend(other.tree.pages);
            self.page = other.page;
            self.par = other.par;
        }
    }
}
