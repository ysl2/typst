use std::fmt::Debug;
use std::mem;
use std::rc::Rc;

use super::{Env, Exec, ExecWithMap, Property};
use crate::eval::{ExprMap, Template};
use crate::geom::{Align, Dir, Gen, GenAxis, Length, Linear, Sides, Size};
use crate::layout::{
    LayoutNode, LayoutTree, PadNode, PageRun, ParChild, ParNode, StackChild, StackNode,
};
use crate::syntax::SyntaxTree;
use crate::util::EcoString;
use crate::Context;

/// The context for execution.
pub struct ExecContext {
    /// The active execution environment.
    env: Env,
    /// The tree of finished page runs.
    tree: LayoutTree,
    /// When we are building the top-level stack, this contains metrics of the
    /// page. While building a group stack through `exec_group`, this is `None`.
    page: Option<PageBuilder>,
    /// The currently built stack of paragraphs.
    stack: StackBuilder,
}

impl ExecContext {
    /// Create a new execution context.
    pub fn new(ctx: &mut Context) -> Self {
        Self {
            env: ctx.env.clone(),
            tree: LayoutTree { runs: vec![] },
            page: Some(PageBuilder::new(&ctx.env, true)),
            stack: StackBuilder::new(&ctx.env),
        }
    }

    /// Return a snapshot of the current environment.
    pub fn save(&self) -> Env {
        self.env.clone()
    }

    /// Restore a snapshot of the environment.
    pub fn restore(&self, snapshot: Env) {
        self.env = snapshot;
    }

    /// Insert a property into the context's environment.
    pub fn set<P>(&mut self, property: P)
    where
        P: Property + Debug + Clone,
    {
        self.env.set(property);
    }

    /// Push a word space into the active paragraph.
    pub fn space(&mut self) {
        self.stack.par.push_soft(self.make_text_node(' '));
    }

    /// Apply a forced line break.
    pub fn linebreak(&mut self) {
        self.stack.par.push_hard(self.make_text_node('\n'));
    }

    /// Apply a forced paragraph break.
    pub fn parbreak(&mut self) {
        let amount = self.env.par_spacing();
        self.stack.finish_par(&self.env);
        self.stack.push_soft(StackChild::Spacing(amount.into()));
    }

    /// Apply a forced page break.
    pub fn pagebreak(&mut self, keep: bool, hard: bool) {
        if let Some(builder) = &mut self.page {
            let page = mem::replace(builder, PageBuilder::new(&self.env, hard));
            let stack = mem::replace(&mut self.stack, StackBuilder::new(&self.env));
            self.tree.runs.extend(page.build(stack.build(), keep));
        }
    }

    /// Push text into the active paragraph.
    ///
    /// The text is split into lines at newlines.
    pub fn text(&mut self, text: impl Into<EcoString>) {
        self.stack.par.push(self.make_text_node(text));
    }

    /// Push an inline node into the active paragraph.
    pub fn inline(&mut self, node: impl Into<LayoutNode>) {
        let align = self.env.aligns.cross;
        self.stack.par.push(ParChild::Any(node.into(), align));
    }

    /// Push a block node into the active stack, finishing the active paragraph.
    pub fn block(&mut self, node: impl Into<LayoutNode>) {
        self.parbreak();
        let aligns = self.env.aligns;
        self.stack.push(StackChild::Any(node.into(), aligns));
        self.parbreak();
    }

    /// Push spacing into the active paragraph or stack depending on the `axis`.
    pub fn spacing(&mut self, axis: GenAxis, amount: Linear) {
        match axis {
            GenAxis::Main => {
                self.stack.finish_par(&self.env);
                self.stack.push_hard(StackChild::Spacing(amount));
            }
            GenAxis::Cross => {
                self.stack.par.push_hard(ParChild::Spacing(amount));
            }
        }
    }

    /// Execute a template and return the result as a stack node.
    pub fn exec_template(&mut self, template: &Template) -> StackNode {
        self.exec_to_stack(|ctx| template.exec(ctx))
    }

    /// Execute a syntax tree with a map and return the result as a stack node.
    pub fn exec_tree(&mut self, tree: &SyntaxTree, map: &ExprMap) -> StackNode {
        self.exec_to_stack(|ctx| tree.exec_with_map(ctx, map))
    }

    /// Execute something and return the result as a stack node.
    pub fn exec_to_stack(&mut self, f: impl FnOnce(&mut Self)) -> StackNode {
        let snapshot = self.save();
        let page = self.page.take();
        let stack = mem::replace(&mut self.stack, StackBuilder::new(&self.env));

        f(self);

        self.restore(snapshot);
        self.page = page;
        mem::replace(&mut self.stack, stack).build()
    }

    /// Finish execution and return the created layout tree.
    pub fn finish(mut self) -> LayoutTree {
        assert!(self.page.is_some());
        self.pagebreak(true, false);
        self.tree
    }

    /// Construct a text node with the given text and settings from the active
    /// environment.
    fn make_text_node(&self, text: impl Into<EcoString>) -> ParChild {
        ParChild::Text(
            text.into(),
            self.env.aligns.cross,
            Rc::clone(&self.env.font),
        )
    }
}

struct PageBuilder {
    size: Size,
    padding: Sides<Linear>,
    hard: bool,
}

impl PageBuilder {
    fn new(env: &Env, hard: bool) -> Self {
        Self {
            size: env.page.size,
            padding: env.page.margins(),
            hard,
        }
    }

    fn build(self, child: StackNode, keep: bool) -> Option<PageRun> {
        let Self { size, padding, hard } = self;
        (!child.children.is_empty() || (keep && hard)).then(|| PageRun {
            size,
            child: PadNode { padding, child: child.into() }.into(),
        })
    }
}

struct StackBuilder {
    dirs: Gen<Dir>,
    children: Vec<StackChild>,
    last: Last<StackChild>,
    par: ParBuilder,
}

impl StackBuilder {
    fn new(env: &Env) -> Self {
        Self {
            dirs: env.dirs,
            children: vec![],
            last: Last::None,
            par: ParBuilder::new(env),
        }
    }

    fn push(&mut self, child: StackChild) {
        self.children.extend(self.last.any());
        self.children.push(child);
    }

    fn push_soft(&mut self, child: StackChild) {
        self.last.soft(child);
    }

    fn push_hard(&mut self, child: StackChild) {
        self.last.hard();
        self.children.push(child);
    }

    fn finish_par(&mut self, env: &Env) {
        let par = mem::replace(&mut self.par, ParBuilder::new(env));
        if let Some(par) = par.build() {
            self.push(par);
        }
    }

    fn build(self) -> StackNode {
        let Self { dirs, mut children, par, mut last } = self;
        if let Some(par) = par.build() {
            children.extend(last.any());
            children.push(par);
        }
        StackNode { dirs, aspect: None, children }
    }
}

struct ParBuilder {
    aligns: Gen<Align>,
    dir: Dir,
    line_spacing: Length,
    children: Vec<ParChild>,
    last: Last<ParChild>,
}

impl ParBuilder {
    fn new(env: &Env) -> Self {
        Self {
            aligns: env.aligns,
            dir: env.dirs.cross,
            line_spacing: env.line_spacing(),
            children: vec![],
            last: Last::None,
        }
    }

    fn push(&mut self, child: ParChild) {
        if let Some(soft) = self.last.any() {
            self.push_inner(soft);
        }
        self.push_inner(child);
    }

    fn push_soft(&mut self, child: ParChild) {
        self.last.soft(child);
    }

    fn push_hard(&mut self, child: ParChild) {
        self.last.hard();
        self.push_inner(child);
    }

    fn push_inner(&mut self, child: ParChild) {
        if let ParChild::Text(curr_text, curr_props, curr_align) = &child {
            if let Some(ParChild::Text(prev_text, prev_props, prev_align)) =
                self.children.last_mut()
            {
                if prev_align == curr_align && prev_props == curr_props {
                    prev_text.push_str(&curr_text);
                    return;
                }
            }
        }

        self.children.push(child);
    }

    fn build(self) -> Option<StackChild> {
        let Self { aligns, dir, line_spacing, children, .. } = self;
        (!children.is_empty()).then(|| {
            let node = ParNode { dir, line_spacing, children };
            StackChild::Any(node.into(), aligns)
        })
    }
}

/// Finite state machine for spacing coalescing.
enum Last<N> {
    None,
    Any,
    Soft(N),
}

impl<N> Last<N> {
    fn any(&mut self) -> Option<N> {
        match mem::replace(self, Self::Any) {
            Self::Soft(soft) => Some(soft),
            _ => None,
        }
    }

    fn soft(&mut self, soft: N) {
        if let Self::Any = self {
            *self = Self::Soft(soft);
        }
    }

    fn hard(&mut self) {
        *self = Self::None;
    }
}
