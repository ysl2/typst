use std::f64::consts::SQRT_2;

use decorum::N64;

use super::*;
use crate::layout::{
    BackgroundNode, BackgroundShape, FixedNode, ImageNode, PadNode, Paint,
};

/// `image`: An image.
pub fn image(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let path = args.expect::<Spanned<EcoString>>(ctx, "path to image file");
    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");

    if let Some(path) = &path {
        if let Some(file) = ctx.resolve(&path.v, path.span) {
            if let Some(id) = ctx.images.load(file) {
                return Value::Template(Template::from_inline_node(
                    ImageNode { id, width, height },
                    &ctx.state,
                ));
            } else {
                ctx.diag(error!(path.span, "failed to load image"));
            }
        }
    }

    Value::Error
}

/// `rect`: A rectangle with optional content.
pub fn rect(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");
    let fill = args.named(ctx, "fill");
    let body = args.eat().unwrap_or_default();
    rect_impl(ctx, width, height, None, fill, body)
}

/// `square`: A square with optional content.
pub fn square(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let length = args.named::<Length>(ctx, "length").map(Linear::from);
    let width = length.or_else(|| args.named(ctx, "width"));
    let height = width.is_none().then(|| args.named(ctx, "height")).flatten();
    let fill = args.named(ctx, "fill");
    let body = args.eat().unwrap_or_default();
    rect_impl(ctx, width, height, Some(N64::from(1.0)), fill, body)
}

fn rect_impl(
    ctx: &mut EvalContext,
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: Template,
) -> Value {
    if let Some(mut stack) = body.into_stack() {
        stack.aspect = aspect;

        let fixed = FixedNode { width, height, child: stack.into() };
        if let Some(fill) = fill {
            ctx.template.push_inline_node(
                BackgroundNode {
                    shape: BackgroundShape::Rect,
                    fill: Paint::Color(fill),
                    child: fixed.into(),
                },
                &ctx.state,
            );
        } else {
            ctx.template.push_inline_node(fixed, &ctx.state);
        }

        Value::None
    } else {
        Value::Error
    }
}

/// `ellipse`: An ellipse with optional content.
pub fn ellipse(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");
    let fill = args.named(ctx, "fill");
    let body = args.eat().unwrap_or_default();
    ellipse_impl(ctx, width, height, None, fill, body)
}

/// `circle`: A circle with optional content.
pub fn circle(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let diameter = args.named::<Length>(ctx, "radius").map(|r| 2.0 * Linear::from(r));
    let width = diameter.or_else(|| args.named(ctx, "width"));
    let height = width.is_none().then(|| args.named(ctx, "height")).flatten();
    let fill = args.named(ctx, "fill");
    let body = args.eat().unwrap_or_default();
    ellipse_impl(ctx, width, height, Some(N64::from(1.0)), fill, body)
}

fn ellipse_impl(
    ctx: &mut EvalContext,
    width: Option<Linear>,
    height: Option<Linear>,
    aspect: Option<N64>,
    fill: Option<Color>,
    body: Template,
) -> Value {
    if let Some(mut stack) = body.into_stack() {
        // This padding ratio ensures that the rectangular padded region fits
        // perfectly into the ellipse.
        const PAD: f64 = 0.5 - SQRT_2 / 4.0;

        stack.aspect = aspect;

        let fixed = FixedNode {
            width,
            height,
            child: PadNode {
                padding: Sides::splat(Relative::new(PAD).into()),
                child: stack.into(),
            }
            .into(),
        };

        if let Some(fill) = fill {
            ctx.template.push_inline_node(
                BackgroundNode {
                    shape: BackgroundShape::Ellipse,
                    fill: Paint::Color(fill),
                    child: fixed.into(),
                },
                &ctx.state,
            );
        } else {
            ctx.template.push_inline_node(fixed, &ctx.state);
        }

        Value::None
    } else {
        Value::Error
    }
}
