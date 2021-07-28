use crate::eval::{LineState, TextState};
use crate::layout::Paint;

use super::*;

/// `font`: Configure the font.
pub fn font(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let text = ctx.state.text_mut();

    let families: Vec<_> = args.all().collect();
    let list = if families.is_empty() {
        args.named(ctx, "family")
    } else {
        Some(FontDef(families))
    };

    if let Some(FontDef(list)) = list {
        text.families_mut().list = Some(list);
    }

    if let Some(FamilyDef(serif)) = args.named(ctx, "serif") {
        text.families_mut().serif = Some(Rc::new(serif));
    }

    if let Some(FamilyDef(sans_serif)) = args.named(ctx, "sans-serif") {
        text.families_mut().sans_serif = Some(Rc::new(sans_serif));
    }

    if let Some(FamilyDef(monospace)) = args.named(ctx, "monospace") {
        text.families_mut().monospace = Some(Rc::new(monospace));
    }

    text.size.set_if(args.eat().or_else(|| args.named(ctx, "size")));
    text.style.set_if(args.named(ctx, "style"));
    text.weight.set_if(args.named(ctx, "weight"));
    text.stretch.set_if(args.named(ctx, "stretch"));
    text.fill.set_if(args.named(ctx, "fill").map(Paint::Color));
    text.top_edge.set_if(args.named(ctx, "top-edge"));
    text.bottom_edge.set_if(args.named(ctx, "bottom-edge"));

    Value::None
}

struct FontDef(Vec<FontFamily>);

castable! {
    FontDef: "font family or array of font families",
    Value::Str(string) => Self(vec![FontFamily::Named(string.to_lowercase())]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()
    ),
    @family: FontFamily => Self(vec![family.clone()]),
}

struct FamilyDef(Vec<String>);

castable! {
    FamilyDef: "string or array of strings",
    Value::Str(string) => Self(vec![string.to_lowercase()]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .map(|string: EcoString| string.to_lowercase())
        .collect()
    ),
}

/// `par`: Configure paragraphs.
pub fn par(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let text = ctx.state.text_mut();

    text.par_spacing.set_if(args.named(ctx, "spacing"));
    text.line_spacing.set_if(args.named(ctx, "leading"));
    text.word_spacing.set_if(args.named(ctx, "word-spacing"));
    ctx.template.push_parbreak(&ctx.state);

    Value::None
}

/// `lang`: Configure the language.
pub fn lang(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let iso = args.eat::<EcoString>();

    if let Some(dir) = args.named::<Spanned<Dir>>(ctx, "dir") {
        if dir.v.axis() == SpecAxis::Horizontal {
            ctx.state.dir = Some(dir.v)
        } else {
            ctx.diag(error!(dir.span, "must be horizontal"))
        }
    } else if let Some(iso) = iso {
        ctx.state.dir = Some(lang_dir(&iso));
    }

    Value::None
}

/// The default direction for the language identified by `iso`.
fn lang_dir(iso: &str) -> Dir {
    match iso.to_ascii_lowercase().as_str() {
        "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Dir::RTL,
        "en" | "fr" | "de" => Dir::LTR,
        _ => Dir::LTR,
    }
}

/// `strike`: Enable striken-through text.
pub fn strike(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl(ctx, args, |font| &mut font.strikethrough)
}

/// `underline`: Enable underlined text.
pub fn underline(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl(ctx, args, |font| &mut font.underline)
}

/// `overline`: Add an overline above text.
pub fn overline(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl(ctx, args, |font| &mut font.overline)
}

fn line_impl(
    ctx: &mut EvalContext,
    args: &mut FuncArgs,
    substate: fn(&mut TextState) -> &mut Option<Rc<LineState>>,
) -> Value {
    let stroke = args.eat().or_else(|| args.named(ctx, "stroke"));
    let thickness = args.eat().or_else(|| args.named(ctx, "thickness"));
    let offset = args.named(ctx, "offset");
    let extent = args.named(ctx, "extent").unwrap_or_default();

    let mut state = State::default();
    *substate(ctx.state.text_mut()) = Some(Rc::new(LineState {
        stroke: stroke.map(Paint::Color),
        thickness,
        offset,
        extent,
    }));

    let mut body: Template = args.expect(ctx, "body").unwrap_or_default();
    body.apply(&state);

    Value::Template(body)
}
