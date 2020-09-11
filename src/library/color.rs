use super::*;
use crate::color::RgbaColor;

/// `rgb`: Create an RGB(A) color.
pub fn rgb(span: Span, mut args: DictExpr, ctx: &mut EvalCtx) -> Value {
    let r = args.expect::<Spanned<f64>>("red value", span, &mut ctx.f);
    let g = args.expect::<Spanned<f64>>("green value", span, &mut ctx.f);
    let b = args.expect::<Spanned<f64>>("blue value", span, &mut ctx.f);
    let a = args.take::<Spanned<f64>>();

    let mut clamp = |component: Option<Spanned<f64>>, default| {
        component
            .map(|c| {
                if c.v < 0.0 || c.v > 255.0 {
                    error!(@ctx.f, c.span, "should be between 0 and 255")
                }
                c.v.min(255.0).max(0.0).round() as u8
            })
            .unwrap_or(default)
    };

    let color = RgbaColor::new(clamp(r, 0), clamp(g, 0), clamp(b, 0), clamp(a, 255));

    args.unexpected(&mut ctx.f);
    Value::Color(color)
}
