//! Execution of syntax trees, producing DOMs.

pub mod table;

mod convert;
mod eval;
mod scope;
mod value;

pub use convert::TryFromValue;
pub use eval::Eval;
pub use scope::Scope;
pub use value::*;

use std::rc::Rc;

use crate::dom::{DomNode, DomTree, Heading, Style, StyledNode};
use crate::syntax::{Span, Spanned, SyntaxNode, SyntaxTree};
use crate::Feedback;
use crate::Pass;

/// Execute a syntax tree to produce a stateless DOM tree.
///
/// The given `style` and `funcs` are the base style and function scope that may be
/// overriden by tree elements.
pub fn exec(tree: SyntaxTree, style: Rc<Style>, funcs: Scope) -> Pass<DomTree> {
    let mut ctx = ExecCtx::new(style, funcs);
    let dom = ctx.process_tree(tree);
    Pass::new(dom, ctx.f)
}

/// The context for execution.
///
/// This stores accumulated feedback and keeps the state that may change over the course
/// of execution. When execution reaches a node, the current style is cloned and applied
/// to it. This is cheap because `style` is ref-counted. When the style changes in any
/// way, a new style is created, leaving the old style untouched and reusing as much of
/// the child styles through ref-counting.
pub struct ExecCtx {
    /// The feedback accumulated during execution.
    pub f: Feedback,
    /// The active style.
    pub style: Rc<Style>,
    /// The active function scope.
    pub funcs: Scope,
}

impl ExecCtx {
    pub fn new(style: Rc<Style>, funcs: Scope) -> Self {
        Self { f: Feedback::new(), style, funcs }
    }

    pub fn process_tree(&mut self, tree: SyntaxTree) -> DomTree {
        let mut dom = DomTree::new();

        for syntax_node in tree {
            let node = match syntax_node.v {
                SyntaxNode::Space => DomNode::Space,
                SyntaxNode::Linebreak => DomNode::Linebreak,
                SyntaxNode::Parbreak => DomNode::Parbreak,
                SyntaxNode::ToggleItalic => {
                    let style = Rc::make_mut(&mut self.style);
                    let text = Rc::make_mut(&mut style.text);
                    text.italic ^= true;
                    continue;
                }
                SyntaxNode::ToggleBolder => {
                    let style = Rc::make_mut(&mut self.style);
                    let text = Rc::make_mut(&mut style.text);
                    text.bolder ^= true;
                    continue;
                }
                SyntaxNode::Text(text) => DomNode::Text(text),
                SyntaxNode::Heading(heading) => DomNode::Heading(Heading {
                    level: heading.level,
                    body: self.process_tree(heading.tree),
                }),
                SyntaxNode::Raw(raw) => DomNode::Raw(raw),
                SyntaxNode::Code(code) => DomNode::Code(code),
                SyntaxNode::Call(call) => {
                    let spanned = Spanned::new(call.eval(self), syntax_node.span);
                    dom.extend(self.process_value(spanned));
                    continue;
                }
            };

            dom.push(Spanned::new(self.make_node(node), syntax_node.span));
        }

        dom
    }

    pub fn process_value(&mut self, value: Spanned<Value>) -> DomTree {
        match value.v {
            Value::Tree(tree) => tree,

            // Forward to each entry, separated with spaces.
            Value::Table(table) => {
                let mut tree = DomTree::new();

                let mut end = None;
                for entry in table.into_values() {
                    if let Some(last_end) = end {
                        let node = self.make_node(DomNode::Space);
                        let span = Span::new(last_end, entry.key.start);
                        tree.push(Spanned::new(node, span));
                    }

                    end = Some(entry.val.span.end);
                    tree.extend(self.process_value(entry.val));
                }

                tree
            }

            // Fallback: Format with Debug.
            val => {
                let fmt = format!("{:?}", val);
                let node = self.make_node(DomNode::Text(fmt));
                vec![Spanned::new(node, value.span)]
            }
        }
    }

    pub fn make_node(&self, node: DomNode) -> StyledNode {
        StyledNode { node, style: Rc::clone(&self.style) }
    }
}
