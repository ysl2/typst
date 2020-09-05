//! Basic building blocks of layouts.

use std::fmt::{self, Debug, Formatter};

use fontdock::FaceId;
use ttf_parser::GlyphId;

/// A layout element, the basic building block layouts are composed of.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutElement {
    Text(Shaped),
}

/// A shaped run of text.
#[derive(Clone, PartialEq)]
pub struct Shaped {
    /// The shaped text.
    pub text: String,
    /// The font face the text was shaped with.
    pub face: FaceId,
    /// The shaped glyphs.
    pub glyphs: Vec<GlyphId>,
    /// The horizontal offsets of the glyphs. This is indexed parallel to `glyphs`.
    /// Vertical offets are not yet supported.
    pub offsets: Vec<f64>,
    /// The font size.
    pub size: f64,
}

impl Shaped {
    /// Create a new shape run with empty `text`, `glyphs` and `offsets`.
    pub fn new(face: FaceId, size: f64) -> Self {
        Self {
            text: String::new(),
            face,
            glyphs: vec![],
            offsets: vec![],
            size,
        }
    }

    /// Encode the glyph ids into a big-endian byte buffer.
    pub fn encode_glyphs_be(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 * self.glyphs.len());
        for &GlyphId(g) in &self.glyphs {
            bytes.push((g >> 8) as u8);
            bytes.push((g & 0xff) as u8);
        }
        bytes
    }
}

impl Debug for Shaped {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Shaped({})", self.text)
    }
}
