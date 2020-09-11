use fontdock::{FontStyle, FontWeight, FontWidth};

use super::*;
use crate::length::ScaleLength;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - The font size (optional, length or relative to previous font size).
/// - A font family fallback list (optional, identifiers or strings).
///
/// # Keyword arguments
/// - `style`: `normal`, `italic` or `oblique`.
/// - `weight`: `100` - `900` or a name like `thin`.
/// - `width`: `1` - `9` or a name like `condensed`.
/// - Any other keyword argument whose value is a dictionary of strings defines
///   a fallback class, for example:
///   ```typst
///   [font: serif = ("Source Serif Pro", "Noto Serif")]
///   ```
///   This class can be used in the fallback list or other fallback classes as long
///   as the resulting fallback tree is acylic.
///   ```typst
///   [font: "My Serif", serif]
///   ```
pub fn font(_: Span, mut args: DictExpr, ctx: &mut EvalCtx) -> Value {
    let body = args
        .take::<SyntaxTree>()
        .map(|tree| (tree, Rc::clone(&ctx.state.text)));

    let text_style = Rc::make_mut(&mut ctx.state.text);

    if let Some(size) = args.take::<ScaleLength>() {
        match size {
            ScaleLength::Absolute(length) => {
                text_style.font_size = length.as_raw();
                text_style.font_scale = 1.0;
            }
            ScaleLength::Scaled(scale) => text_style.font_scale = scale,
        }
    }

    let mut needs_flattening = false;
    let list: Vec<_> = args
        .take_all_num_vals::<StringLike>()
        .map(|s| s.to_lowercase())
        .collect();

    if !list.is_empty() {
        *Rc::make_mut(&mut text_style.fallback).list_mut() = list;
        needs_flattening = true;
    }

    for (class, mut dict) in args.take_all_str::<DictExpr>() {
        let fallback = dict
            .take_all_num_vals::<StringLike>()
            .map(|s| s.to_lowercase())
            .collect();

        Rc::make_mut(&mut text_style.fallback).set_class_list(class, fallback);
        needs_flattening = true;
    }

    if needs_flattening {
        Rc::make_mut(&mut text_style.fallback).flatten();
    }

    if let Some(style) = args.take_key::<FontStyle>("style", &mut ctx.f) {
        text_style.variant.style = style;
    }

    if let Some(weight) = args.take_key::<FontWeight>("weight", &mut ctx.f) {
        text_style.variant.weight = weight;
    }

    if let Some(width) = args.take_key::<FontWidth>("width", &mut ctx.f) {
        text_style.variant.width = width;
    }

    args.unexpected(&mut ctx.f);

    if let Some((tree, prev)) = body {
        let dom = tree.eval(ctx);
        ctx.state.text = prev;
        Value::Tree(dom)
    } else {
        Value::None
    }
}
