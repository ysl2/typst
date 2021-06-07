use super::*;

/// `h`: Horizontal spacing.
///
/// # Positional parameters
/// - Amount of spacing: of type `linear` relative to current font size.
///
/// # Return value
/// A template that inserts horizontal spacing.
pub fn h(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    spacing_impl(ctx, args, GenAxis::Cross)
}

/// `v`: Vertical spacing.
///
/// # Positional parameters
/// - Amount of spacing: of type `linear` relative to current font size.
///
/// # Return value
/// A template that inserts vertical spacing.
pub fn v(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    spacing_impl(ctx, args, GenAxis::Main)
}

fn spacing_impl(
    ctx: &mut EvalContext,
    args: &mut FuncArgs,
    axis: GenAxis,
) -> Value {
    let spacing: Option<Linear> = args.eat_expect(ctx, "spacing");
    if let Some(linear) = spacing {
        let amount = linear.resolve(ctx.state.font.resolve_size());
        ctx.push_spacing(axis, amount);
    }
    Value::None
}
