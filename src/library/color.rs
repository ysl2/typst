use super::*;
use crate::color::RgbaColor;

/// `rgb`: Create a color from red, green, blue and optional alpha component.
pub fn rgb(span: Span, mut args: DictExpr, ctx: &mut EvalCtx) -> Value {
    let r = args.expect::<Spanned<f64>>("red value", span, &mut ctx.f);
    let g = args.expect::<Spanned<f64>>("green value", span, &mut ctx.f);
    let b = args.expect::<Spanned<f64>>("blue value", span, &mut ctx.f);
    let a = args.take::<Spanned<f64>>();
    args.unexpected(&mut ctx.f);

    let mut clamp = |component: Option<Spanned<f64>>, default| {
        if let Some(Spanned { v: c, span }) = component {
            if c < 0.0 || c > 255.0 {
                error!(@ctx.f, span, "should be between 0 and 255")
            }
            c.max(0.0).min(255.0) as u8
        } else {
            default
        }
    };

    Value::Color(RgbaColor::new(
        clamp(r, 0),
        clamp(g, 0),
        clamp(b, 0),
        clamp(a, 255),
    ))
}
