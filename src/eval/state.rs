use std::rc::Rc;

use crate::color::{Color, RgbaColor};
use crate::font::{
    FontFamily, FontStretch, FontStyle, FontVariant, FontWeight, VerticalFontMetric,
};
use crate::geom::*;
use crate::layout::Paint;
use crate::paper::{PaperClass, PAPER_A4};

/// Active style properties during evaluation of a template.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct State {
    /// The direction for text and other inline objects.
    pub dir: Option<Dir>,
    /// The alignments of layouts in their parents.
    pub aligns: Gen<Option<Align>>,
    /// The page settings.
    pub page: Option<Rc<PageState>>,
    /// The text settings.
    pub text: Option<Rc<TextState>>,
}

impl State {
    /// Access the `page` state mutably.
    pub fn page_mut(&mut self) -> &mut PageState {
        Rc::make_mut(self.page.get_or_insert_with(Default::default))
    }

    /// Access the `text` state mutably.
    pub fn text_mut(&mut self) -> &mut TextState {
        Rc::make_mut(self.text.get_or_insert_with(Default::default))
    }
}

/// Defines active page properties.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PageState {
    /// Whether the page is flipped (in landscape mode).
    pub flipped: bool,
    /// The class of this page.
    pub class: Option<PaperClass>,
    /// The size of the page.
    pub size: Spec<Option<Length>>,
    /// The amount of white space on each side of the page. If a side is set to
    /// `None`, but a paper class is defined the default for the paper class is
    /// used.
    pub margins: Sides<Option<Linear>>,
}

/// Defines active text properties.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TextState {
    /// Whether the strong toggle is active or inactive. (This determines
    /// whether the next `*` adds or removes font weight.)
    pub strong: bool,
    /// Whether the emphasis toggle is active or inactive. (This determines
    /// whether the next `_` makes italic or non-italic.)
    pub emph: bool,
    /// Whether the monospace toggle is active or inactive.
    pub monospace: bool,
    /// The top-level list of font families plus extra lists for stylistic
    /// groups like serif or sans-serif. (The final sequence of tried families
    /// also depends on `monospace`.)
    pub families: Option<Rc<FamilyState>>,
    /// The style of the font. (The final style also depends on `emph`.)
    pub style: Option<FontStyle>,
    /// The weight of the font. (The final weight also depends on `strong`.)
    pub weight: Option<FontWeight>,
    /// The width of the font.
    pub stretch: Option<FontStretch>,
    /// The font size (dependent on outer font size).
    pub size: Option<Linear>,
    /// The color glyphs.
    pub fill: Option<Paint>,
    /// The top end of the text bounding box.
    pub top_edge: Option<VerticalFontMetric>,
    /// The bottom end of the text bounding box.
    pub bottom_edge: Option<VerticalFontMetric>,
    /// The spacing between words (dependent on scaled font size).
    pub word_spacing: Option<Linear>,
    /// The spacing between lines (dependent on scaled font size).
    pub line_spacing: Option<Linear>,
    /// The spacing between paragraphs (dependent on scaled font size).
    pub par_spacing: Option<Linear>,
    /// The specifications for a strikethrough line, if any.
    pub strikethrough: Option<Rc<LineState>>,
    /// The specifications for an underline, if any.
    pub underline: Option<Rc<LineState>>,
    /// The specifications for an overline, if any.
    pub overline: Option<Rc<LineState>>,
}

impl TextState {
    /// Access the `font_families` mutably.
    pub fn families_mut(&mut self) -> &mut FamilyState {
        Rc::make_mut(self.families.get_or_insert_with(Default::default))
    }

    /// The resolved family iterator.
    pub fn families<'a>(
        &'a self,
        defaults: &'a Defaults,
    ) -> impl Iterator<Item = &str> + Clone {
        macro_rules! family {
            ($state_field:ident, $defaults_field:ident) => {
                self.families
                    .as_ref()
                    .and_then(|families| families.$state_field.as_ref())
                    .map(|v| v.as_slice())
                    .unwrap_or(&defaults.$defaults_field)
            };
        }

        let list = family!(list, font_families);
        let serif = family!(serif, serif_families);
        let sans_serif = family!(sans_serif, sans_serif_families);
        let monospace = family!(monospace, monospace_families);

        let head = self.monospace.then(|| monospace).unwrap_or_default();
        let core = list.iter().flat_map(move |family| {
            match family {
                FontFamily::Named(name) => std::slice::from_ref(name),
                FontFamily::Serif => serif,
                FontFamily::SansSerif => sans_serif,
                FontFamily::Monospace => monospace,
            }
        });

        head.iter()
            .chain(core)
            .chain(&defaults.base_families)
            .map(String::as_str)
    }

    /// The resolved variant with `strong` and `emph` factored in.
    pub fn variant(&self, defaults: &Defaults) -> FontVariant {
        let mut weight = self.weight.unwrap_or(defaults.font_variant.weight);
        if self.strong {
            weight = weight.thicken(300);
        }

        let mut style = self.style.unwrap_or(defaults.font_variant.style);
        if self.emph {
            style = match style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        let stretch = self.stretch.unwrap_or(defaults.font_variant.stretch);
        FontVariant::new(style, weight, stretch)
    }

    /// The resolved font size.
    pub fn size(&self, defaults: &Defaults) -> Length {
        self.size
            .map_or(defaults.font_size, |s| s.resolve(defaults.font_size))
    }

    /// The resolved word spacing.
    pub fn word_spacing(&self, defaults: &Defaults) -> Length {
        self.word_spacing
            .unwrap_or(defaults.word_spacing)
            .resolve(self.size(defaults))
    }

    /// The resolved line spacing.
    pub fn line_spacing(&self, defaults: &Defaults) -> Length {
        self.line_spacing
            .unwrap_or(defaults.line_spacing)
            .resolve(self.size(defaults))
    }

    /// The resolved paragraph spacing.
    pub fn par_spacing(&self, defaults: &Defaults) -> Length {
        self.par_spacing
            .unwrap_or(defaults.par_spacing)
            .resolve(self.size(defaults))
    }
}

/// Defines active font family lists.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct FamilyState {
    /// Basic list.
    pub list: Option<Vec<FontFamily>>,
    /// Definition of serif font families.
    pub serif: Option<Rc<Vec<String>>>,
    /// Definition of sans-serif font families.
    pub sans_serif: Option<Rc<Vec<String>>>,
    /// Definition of monospace font families used for raw text.
    pub monospace: Option<Rc<Vec<String>>>,
}

/// Defines a line that is positioned over, under or on top of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LineState {
    /// Stroke color of the line, defaults to the text color if `None`.
    pub stroke: Option<Paint>,
    /// Thickness of the line's strokes (dependent on scaled font size), read
    /// from the font tables if `None`.
    pub thickness: Option<Linear>,
    /// Position of the line relative to the baseline (dependent on scaled font
    /// size), read from the font tables if `None`.
    pub offset: Option<Linear>,
    /// Amount that the line will be longer or shorter than its associated text
    /// (dependent on scaled font size).
    pub extent: Linear,
}

/// Defaults for style properties that aren't defined in a [`State`].
pub struct Defaults {
    pub text: TextState,
    /// The default directions for
    /// - text and inline objects (cross)
    /// - paragraphs and pages (main)
    ///
    /// Note that the cross direction _must_ be horizontal and the main
    /// direction _must_ be vertical (at least for now).
    pub dirs: Gen<Dir>,
    /// The default alignments of layouts in their parents.
    pub aligns: Gen<Align>,
    /// The default page size.
    pub page_size: Size,
    /// The default margins for pages based on the paper class or `None` for
    /// custom page sizes.
    pub page_margins: Box<dyn Fn(Option<PaperClass>) -> Sides<Linear>>,
    /// The default list of font families to try.
    pub font_families: Vec<FontFamily>,
    /// The default list of serif font families.
    pub serif_families: Vec<String>,
    /// The default list of sans-serif font families.
    pub sans_serif_families: Vec<String>,
    /// The default list of monospace font families.
    pub monospace_families: Vec<String>,
    /// A base list of font families that are tried as last resort.
    pub base_families: Vec<String>,
    /// The default font variant.
    pub font_variant: FontVariant,
    /// The default font size.
    pub font_size: Length,
    /// The default glyph color.
    pub font_fill: Paint,
    /// The default top end of the text bounding box.
    pub top_edge: VerticalFontMetric,
    /// The default bottom end of the text bounding box.
    pub bottom_edge: VerticalFontMetric,
    /// The default spacing between words.
    pub word_spacing: Linear,
    /// The default spacing between lines.
    pub line_spacing: Linear,
    /// The default spacing between paragraphs.
    pub par_spacing: Linear,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            text: TextState::default(),
            dirs: Gen::new(Dir::LTR, Dir::TTB),
            aligns: Gen::splat(Align::Start),
            page_size: PAPER_A4.size(),
            page_margins: Box::new(|class| {
                class.unwrap_or(PaperClass::Base).default_margins()
            }),
            font_families: vec![FontFamily::Serif],
            serif_families: vec!["eb garamond".into()],
            sans_serif_families: vec!["pt sans".into()],
            monospace_families: vec!["inconsolata".into()],
            base_families: vec!["twitter color emoji".into(), "latin modern math".into()],
            font_variant: FontVariant {
                style: FontStyle::Normal,
                weight: FontWeight::REGULAR,
                stretch: FontStretch::NORMAL,
            },
            font_size: Length::pt(11.0),
            font_fill: Paint::Color(Color::Rgba(RgbaColor::BLACK)),
            top_edge: VerticalFontMetric::CapHeight,
            bottom_edge: VerticalFontMetric::Baseline,
            word_spacing: Relative::new(0.25).into(),
            line_spacing: Relative::new(0.5).into(),
            par_spacing: Relative::new(1.0).into(),
        }
    }
}
