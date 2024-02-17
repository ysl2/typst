use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;
use siphasher::sip128::Hasher128;
use usvg::Node;

use crate::diag::{format_xml_like_error, StrResult};
use crate::foundations::Bytes;
use crate::layout::Axes;
use crate::text::{FontVariant, FontWeight};
use crate::World;

/// A decoded SVG.
#[derive(Clone, Hash)]
pub struct SvgImage(Arc<Repr>);

/// The internal representation.
struct Repr {
    data: Bytes,
    size: Axes<f64>,
    font_hash: u128,
    tree: usvg::Tree,
}

impl SvgImage {
    /// Decode an SVG image without fonts.
    #[comemo::memoize]
    pub fn new(data: Bytes) -> StrResult<SvgImage> {
        let opts = usvg::Options::default();
        let fontdb = fontdb::Database::new();
        let tree =
            usvg::Tree::from_data(&data, &opts, &fontdb).map_err(format_usvg_error)?;
        Ok(Self(Arc::new(Repr { data, size: tree_size(&tree), font_hash: 0, tree })))
    }

    /// Decode an SVG image with access to fonts.
    #[comemo::memoize]
    pub fn with_fonts(
        data: Bytes,
        world: Tracked<dyn World + '_>,
        families: &[String],
    ) -> StrResult<SvgImage> {
        // Disable usvg's default to "Times New Roman". Instead, we default to
        // the empty family and later, when we traverse the SVG, we check for
        // empty and non-existing family names and replace them with the true
        // fallback family. This way, we can memoize SVG decoding with and without
        // fonts if the SVG does not contain text.
        let opts = usvg::Options { font_family: String::new(), ..Default::default() };

        let empty = fontdb::Database::new();
        let mut tree =
            usvg::Tree::from_data(&data, &opts, &empty).map_err(format_usvg_error)?;

        let mut font_hash = 0;
        if tree.has_text_nodes() {
            println!("Tree: {tree:#?}");

            let (fontdb, hash) = load_svg_fonts(world, &tree, families);
            tree = usvg::Tree::from_data(&data, &opts, &fontdb)
                .map_err(format_usvg_error)?;
            font_hash = hash;
        }

        Ok(Self(Arc::new(Repr { data, size: tree_size(&tree), font_hash, tree })))
    }

    /// The raw image data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The SVG's width in pixels.
    pub fn width(&self) -> f64 {
        self.0.size.x
    }

    /// The SVG's height in pixels.
    pub fn height(&self) -> f64 {
        self.0.size.y
    }

    /// The parsed SVG tree.
    pub fn tree(&self) -> &usvg::Tree {
        &self.0.tree
    }
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // An SVG might contain fonts, which must be incorporated into the hash.
        // We can't hash a usvg tree directly, but the raw SVG data + a hash of
        // all used fonts gives us something similar.
        self.data.hash(state);
        self.font_hash.hash(state);
    }
}

/// Discover and load the fonts referenced by an SVG.
fn load_svg_fonts(
    world: Tracked<dyn World + '_>,
    tree: &usvg::Tree,
    families: &[String],
) -> (fontdb::Database, u128) {
    let book = world.book();
    let mut fontdb = fontdb::Database::new();
    let mut hasher = siphasher::sip128::SipHasher13::new();
    let mut loaded = HashMap::<usize, Option<String>>::new();

    // Loads a font into the database and return it's usvg-compatible name.
    let mut load_into_db = |id: usize| -> Option<String> {
        loaded
            .entry(id)
            .or_insert_with(|| {
                let font = world.font(id)?;
                println!(
                    "Providing {}",
                    font.find_name(ttf_parser::name_id::FAMILY).unwrap()
                );
                fontdb.load_font_source(fontdb::Source::Binary(Arc::new(
                    font.data().clone(),
                )));
                font.data().hash(&mut hasher);
                font.find_name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
                    .or_else(|| font.find_name(ttf_parser::name_id::FAMILY))
            })
            .clone()
    };

    // Determine the best font for each text node.
    for child in tree.root().children() {
        traverse_svg(child, &mut |node| {
            let usvg::Node::Text(text) = node else { return };
            println!("Text: {text:#?}");
            for chunk in text.chunks() {
                'spans: for span in chunk.spans() {
                    let Some(text) = chunk.text().get(span.start()..span.end()) else {
                        continue;
                    };
                    let variant = FontVariant {
                        style: span.font().style().into(),
                        weight: FontWeight::from_number(span.font().weight()),
                        stretch: span.font().stretch().into(),
                    };

                    // Find a font that covers the whole text among the span's fonts
                    // and the current document font families.
                    let mut like = None;
                    for family in span
                        .font()
                        .families()
                        .iter()
                        .filter_map(|family| match family {
                            usvg::FontFamily::Named(named) => Some(named),
                            _ => None,
                        })
                        .chain(families)
                    {
                        let Some(id) = book.select(&family.to_lowercase(), variant)
                        else {
                            continue;
                        };
                        let Some(info) = book.info(id) else { continue };
                        like.get_or_insert(info);

                        if text.chars().all(|c| info.coverage.contains(c as u32)) {
                            if let Some(_) = load_into_db(id) {
                                continue 'spans;
                            }
                        }
                    }

                    // If we didn't find a match, select a fallback font.
                    if let Some(id) = book.select_fallback(like, variant, text) {
                        load_into_db(id);
                    }
                }
            }
        });
    }

    (fontdb, hasher.finish128().as_u128())
}

/// Search for all font families referenced by an SVG.
fn traverse_svg<F>(node: &usvg::Node, f: &mut F)
where
    F: FnMut(&usvg::Node),
{
    f(node);

    node.subroots(|subroot| {
        for child in subroot.children() {
            traverse_svg(child, f);
        }
    });

    if let Node::Group(group) = node {
        for child in group.children() {
            traverse_svg(child, f);
        }
    }
}

/// The ceiled pixel size of an SVG.
fn tree_size(tree: &usvg::Tree) -> Axes<f64> {
    Axes::new(tree.size().width() as f64, tree.size().height() as f64)
}

/// Format the user-facing SVG decoding error message.
fn format_usvg_error(error: usvg::Error) -> EcoString {
    match error {
        usvg::Error::NotAnUtf8Str => "file is not valid utf-8".into(),
        usvg::Error::MalformedGZip => "file is not compressed correctly".into(),
        usvg::Error::ElementsLimitReached => "file is too large".into(),
        usvg::Error::InvalidSize => {
            "failed to parse SVG (width, height, or viewbox is invalid)".into()
        }
        usvg::Error::ParsingFailed(error) => format_xml_like_error("SVG", error),
    }
}
