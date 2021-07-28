use super::*;
use crate::layout::{FixedNode, GridNode, PadNode, StackChild, StackNode, TrackSizing};
use crate::paper::Paper;

/// `page`: Configure pages.
pub fn page(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let page = ctx.state.page_mut();

    if let Some(name) = args.eat::<Spanned<EcoString>>() {
        if let Some(paper) = Paper::from_name(&name.v) {
            page.class = Some(paper.class());
            page.size = paper.size().to_spec().map(Some);
        } else {
            ctx.diag(error!(name.span, "invalid paper name"));
        }
    }

    if let Some(width) = args.named(ctx, "width") {
        page.class = None;
        page.size.horizontal = Some(width);
    }

    if let Some(height) = args.named(ctx, "height") {
        page.class = None;
        page.size.vertical = Some(height);
    }

    if let Some(margins) = args.named(ctx, "margins") {
        page.margins = Sides::splat(Some(margins));
    }

    page.margins.left.set_if(args.named(ctx, "left"));
    page.margins.top.set_if(args.named(ctx, "top"));
    page.margins.right.set_if(args.named(ctx, "right"));
    page.margins.bottom.set_if(args.named(ctx, "bottom"));

    page.flipped ^= args.named(ctx, "flip").unwrap_or(false);

    ctx.template.push_pagebreak(&ctx.state, false);

    Value::None
}

/// `pagebreak`: Start a new page.
pub fn pagebreak(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    ctx.template.push_pagebreak(&ctx.state, true);
    Value::None
}

/// `h`: Horizontal spacing.
pub fn h(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    if let Some(spacing) = args.expect(ctx, "spacing") {
        ctx.template.push_inline_spacing(spacing);
    }
    Value::None
}

/// `v`: Vertical spacing.
pub fn v(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    if let Some(spacing) = args.expect(ctx, "spacing") {
        ctx.template.push_block_spacing(spacing);
    }
    Value::None
}

/// `align`: Configure the alignment along the layouting axes.
pub fn align(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let first = args.eat::<Align>();
    let second = args.eat::<Align>();

    let mut horizontal = args.named(ctx, "horizontal");
    let mut vertical = args.named(ctx, "vertical");

    for value in first.into_iter().chain(second) {
        match value.axis() {
            Some(SpecAxis::Horizontal) | None if horizontal.is_none() => {
                horizontal = Some(value);
            }
            Some(SpecAxis::Vertical) | None if vertical.is_none() => {
                vertical = Some(value);
            }
            _ => {}
        }
    }

    ctx.state.aligns.cross.set_if(horizontal);

    if let Some(vertical) = vertical {
        ctx.state.aligns.main = Some(vertical);
        ctx.template.push_parbreak(&ctx.state);
    }

    Value::None
}

/// `box`: Place content in a rectangular box.
pub fn boxed(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");
    let body: Template = args.eat().unwrap_or_default();
    if let Some(stack) = body.into_stack() {
        Value::Template(Template::from_inline_node(
            FixedNode { width, height, child: stack.into() },
            &ctx.state,
        ))
    } else {
        Value::Error
    }
}

/// `block`: Place content in a block.
pub fn block(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let body: Template = args.expect(ctx, "body").unwrap_or_default();
    if let Some(stack) = body.into_stack() {
        Value::Template(Template::from_block_node(stack, &ctx.state))
    } else {
        Value::Error
    }
}

/// `pad`: Pad content at the sides.
pub fn pad(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let base = args.eat();
    let padding = Sides::new(
        args.named(ctx, "left").or(base).unwrap_or_default(),
        args.named(ctx, "top").or(base).unwrap_or_default(),
        args.named(ctx, "right").or(base).unwrap_or_default(),
        args.named(ctx, "bottom").or(base).unwrap_or_default(),
    );

    let body: Template = args.expect(ctx, "body").unwrap_or_default();
    if let Some(stack) = body.into_stack() {
        Value::Template(Template::from_block_node(
            PadNode { padding, child: stack.into() },
            &ctx.state,
        ))
    } else {
        Value::Error
    }
}

/// `stack`: Stack children along an axis.
pub fn stack(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let dir = args.named(ctx, "dir");

    let children = args
        .all()
        .flat_map(|child: Template| child.into_stack())
        .map(|stack| StackChild::Node(stack.into(), ctx.state.aligns))
        .collect();

    Value::Template(Template::from_block_node(
        StackNode {
            dirs: Gen::new(None, dir),
            aspect: None,
            children,
        },
        &ctx.state,
    ))
}

/// `grid`: Arrange children into a grid.
pub fn grid(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let columns = args.named(ctx, "columns").unwrap_or_default();
    let rows = args.named(ctx, "rows").unwrap_or_default();

    let column_dir = args.named(ctx, "column-dir");
    let row_dir = args.named(ctx, "row-dir");

    let gutter = args
        .named(ctx, "gutter")
        .map(|v| vec![TrackSizing::Linear(v)])
        .unwrap_or_default();

    let gutter_columns = args
        .named(ctx, "gutter-columns")
        .unwrap_or_else(|| gutter.clone());

    let gutter_rows = args.named(ctx, "gutter-rows").unwrap_or(gutter);

    let children = args
        .all()
        .flat_map(|child: Template| child.into_stack())
        .map(Into::into)
        .collect();

    Value::Template(Template::from_block_node(
        GridNode {
            dirs: Gen::new(column_dir, row_dir),
            tracks: Gen::new(columns, rows),
            gutter: Gen::new(gutter_columns, gutter_rows),
            children,
        },
        &ctx.state,
    ))
}

/// Defines size of rows and columns in a grid.
type Tracks = Vec<TrackSizing>;

castable! {
    Tracks: "array of `auto`s, linears, and fractionals",
    Value::Int(count) => vec![TrackSizing::Auto; count.max(0) as usize],
    Value::Array(values) => values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect(),
}

castable! {
    TrackSizing: "`auto`, linear, or fractional",
    Value::Auto => TrackSizing::Auto,
    Value::Length(v) => TrackSizing::Linear(v.into()),
    Value::Relative(v) => TrackSizing::Linear(v.into()),
    Value::Linear(v) => TrackSizing::Linear(v),
    Value::Fractional(v) => TrackSizing::Fractional(v),
}
