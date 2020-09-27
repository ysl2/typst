//! Evaluation of syntax trees into DOMs.
//!
//! # The evaluation context
//! The backing structure for all evaluation is the [evaluation context]. This context
//! contains the accumulated feedback and the active state.
//!
//! # State changes
//! When the state changes over the course of evaluation, it is not updated in place.
//! Instead, a fresh copy is created for each modification, which is not too costly
//! because it reuses as much as possible of the old state through nested reference
//! counting. The old state if left untouched as finished nodes may refer to it.
//!
//! [evaluation context]: ./struct.EvalCtx.html

pub mod dict;

mod scope;
pub mod value;

pub use scope::*;
pub use value::*;

use std::rc::Rc;

use crate::dom::{DomNode, DomTree, PageStyle, TextStyle};
use crate::layout::primitive::LayoutSystem;
use crate::syntax::{
    Call, Deco, DictExpr, Expr, Heading, Spanned, SyntaxNode, SyntaxTree,
};
use crate::{Feedback, Pass};

use dict::SpannedEntry;

/// Evaluate a syntax tree into a stateless DOM tree.
///
/// The given `state` and `funcs` are the base state and function scope that may be
/// updated by tree elements.
pub fn eval(tree: SyntaxTree, state: State, funcs: Scope) -> Pass<DomTree> {
    let mut ctx = EvalCtx::new(state, funcs);
    let dom = tree.eval(&mut ctx);
    Pass::new(dom, ctx.f)
}

/// The evaluation context.
///
/// This stores accumulated feedback and keeps the current state.
pub struct EvalCtx {
    /// The feedback accumulated during evaluation.
    pub f: Feedback,
    /// The active evaluation state.
    pub state: State,
    /// The active function scope.
    pub funcs: Scope,
}

impl EvalCtx {
    /// Create a new evaluation context with empty feedback.
    pub fn new(state: State, funcs: Scope) -> Self {
        Self {
            f: Feedback::new(),
            state,
            funcs,
        }
    }
}

/// The active evaluation state.
///
/// The state is ref-counted nestedly to make cheap copies possible. See the module
/// documentation for more details.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct State {
    /// The active layouting system / directions.
    pub sys: LayoutSystem,
    /// The style for text.
    pub text: Rc<TextStyle>,
    /// The style for pages.
    pub page: Rc<PageStyle>,
}

/// Evaluate an expression into an output value.
///
/// Note: Evaluation is not necessarily pure, it may change the active state.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(self, ctx: &mut EvalCtx) -> Self::Output;
}

impl Eval for SyntaxTree {
    type Output = DomTree;

    fn eval(self, ctx: &mut EvalCtx) -> Self::Output {
        let mut dom = DomTree::new();

        for Spanned {
            v: syntax_node,
            span,
        } in self
        {
            let dom_node = match syntax_node {
                SyntaxNode::Space => DomNode::Space {
                    width: ctx.state.text.font_size(),
                },
                SyntaxNode::Linebreak => DomNode::Linebreak {
                    line_height: ctx.state.text.line_height(),
                    line_padding: ctx.state.text.line_padding(),
                },
                SyntaxNode::Parbreak => DomNode::Parbreak {
                    par_spacing: ctx.state.text.par_spacing(),
                },

                SyntaxNode::ToggleItalic => {
                    ctx.f.decos.push(Spanned::new(Deco::Italic, span));
                    Rc::make_mut(&mut ctx.state.text).italic ^= true;
                    continue;
                }
                SyntaxNode::ToggleBolder => {
                    ctx.f.decos.push(Spanned::new(Deco::Bold, span));
                    Rc::make_mut(&mut ctx.state.text).bolder ^= true;
                    continue;
                }

                SyntaxNode::Text(text) => {
                    if ctx.state.text.italic {
                        ctx.f.decos.push(Spanned::new(Deco::Italic, span));
                    }
                    if ctx.state.text.bolder {
                        ctx.f.decos.push(Spanned::new(Deco::Bold, span));
                    }
                    DomNode::Text {
                        text,
                        style: Rc::clone(&ctx.state.text),
                    }
                }
                SyntaxNode::Raw(raw) => DomNode::Raw {
                    raw,
                    style: Rc::clone(&ctx.state.text),
                },

                SyntaxNode::Heading(heading) => DomNode::Heading(heading.eval(ctx)),

                SyntaxNode::Call(call) => {
                    let value = Spanned::new(call.eval(ctx), span);
                    dom.extend(value.flatten_tree());
                    continue;
                }
            };

            dom.push(Spanned::new(dom_node, span));
        }

        dom
    }
}

impl Eval for Heading<SyntaxTree> {
    type Output = Heading<DomTree>;

    fn eval(self, ctx: &mut EvalCtx) -> Self::Output {
        Heading {
            level: self.level,
            contents: self.contents.map(|contents| contents.eval(ctx)),
        }
    }
}

impl Eval for Call {
    type Output = Value;

    fn eval(self, ctx: &mut EvalCtx) -> Self::Output {
        let span = self.name.span;
        let name = self.name.v.as_str();

        if let Some(func) = ctx.funcs.get(name) {
            (*func.clone())(span, self.args, ctx)
        } else {
            if !name.is_empty() {
                error!(@ctx.f, span, "unknown function");
                ctx.f.decos.push(Spanned::new(Deco::Unresolved, span));
            }
            Value::Dict(self.args.eval(ctx))
        }
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(self, ctx: &mut EvalCtx) -> Value {
        match self {
            Self::Ident(i) => Value::Ident(i),
            Self::Str(s) => Value::Str(s),
            Self::Bool(b) => Value::Bool(b),
            Self::Number(n) => Value::Number(n),
            Self::Length(s) => Value::Length(s),
            Self::Color(c) => Value::Color(c),
            Self::Dict(t) => Value::Dict(t.eval(ctx)),
            Self::Tree(t) => Value::Tree(t.eval(ctx)),
            Self::Call(call) => call.eval(ctx),
            Self::Neg(_) => todo!("eval neg"),
            Self::Add(_, _) => todo!("eval add"),
            Self::Sub(_, _) => todo!("eval sub"),
            Self::Mul(_, _) => todo!("eval mul"),
            Self::Div(_, _) => todo!("eval div"),
        }
    }
}

impl Eval for DictExpr {
    type Output = DictValue;

    fn eval(self, ctx: &mut EvalCtx) -> Self::Output {
        let mut dict = DictValue::new();

        for (key, entry) in self.into_iter() {
            let val = entry.val.v.eval(ctx);
            let spanned = Spanned::new(val, entry.val.span);
            let entry = SpannedEntry::new(entry.key, spanned);
            dict.insert(key, entry);
        }

        dict
    }
}
