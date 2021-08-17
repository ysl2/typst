use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

/// The execution environment.
#[derive(Default, Clone)]
pub struct Env(HashMap<TypeId, Rc<dyn Bounds>>);

impl Env {
    /// Create a new, empty environment
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a property into the environment.
    pub fn set<P>(&mut self, property: P)
    where
        P: Property + Debug + Clone,
    {
        self.0.insert(TypeId::of::<P>(), Rc::new(property));
    }

    /// Get the value of a property.
    ///
    /// If this environment doesn't contain the property, the chained
    /// environments are checked. If none of these has a value either, the
    /// property's default value is returned.
    pub fn get<P>(&self) -> &P
    where
        P: Property,
    {
        self.0
            .get(&TypeId::of::<P>())
            .and_then(|entry| entry.as_any().downcast_ref())
            .unwrap_or(P::DEFAULT)
    }
}

impl Debug for Env {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.0.values()).finish()
    }
}

trait Bounds: Debug + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> Bounds for T
where
    T: Property + Debug + Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An inheritable property.
pub trait Property: 'static {
    /// The property's default value.
    const DEFAULT: &'static Self;
}

/// Built-in property values.
pub mod property {
    use super::Property;
    use crate::font::{FontStretch, FontStyle, FontWeight, VerticalFontMetric};
    use crate::geom::*;
    use crate::paper::{PaperClass, PAPER_A4};

    /// Implement the property trait for a struct.
    macro_rules! property {
        ($type:ty, $default:expr) => {
            impl Property for $type {
                const DEFAULT: &'static Self = &$default;
            }
        };
    }

    /// Defines the direction along which block-level elements flow.
    #[derive(Debug, Copy, Clone)]
    pub struct MainDir(pub Dir);

    /// Defines the direction along which inline-level elements are set.
    #[derive(Debug, Copy, Clone)]
    pub struct CrossDir(pub Dir);

    /// Defines the alignment for block-level elements.
    #[derive(Debug, Copy, Clone)]
    pub struct MainAlign(pub Align);

    /// Defines the alignment of inline-level elements.
    #[derive(Debug, Copy, Clone)]
    pub struct CrossAlign(pub Align);

    /// Defines the width of pages.
    #[derive(Debug, Copy, Clone)]
    pub struct PageWidth(pub Length);

    /// Defines the height of pages.
    #[derive(Debug, Copy, Clone)]
    pub struct PageHeight(pub Length);

    /// Defines whether pages should be flipped.
    #[derive(Debug, Copy, Clone)]
    pub struct Flipped;

    /// Defines whether text should be strong.
    #[derive(Debug, Copy, Clone)]
    pub struct Strong;

    /// Defines whether text should be emphasized.
    #[derive(Debug, Copy, Clone)]
    pub struct Emph;

    /// Defines whether text should be set in monospace.
    #[derive(Debug, Copy, Clone)]
    pub struct Monospace;

    /// Defines the size of text.
    #[derive(Debug, Copy, Clone)]
    pub struct FontSize(pub Linear);

    /// Defines the top edge of the text bounding box.
    #[derive(Debug, Copy, Clone)]
    pub struct TopEdge(pub VerticalFontMetric);

    /// Defines the bottom edge of the text bounding box.
    #[derive(Debug, Copy, Clone)]
    pub struct BottomEdge(pub VerticalFontMetric);

    // Implement the property trait.
    property!(MainDir, Self(Dir::LTR));
    property!(CrossDir, Self(Dir::TTB));
    property!(MainAlign, Self(Align::Start));
    property!(CrossAlign, Self(Align::Start));
    property!(PaperClass, PAPER_A4.class());
    property!(PageWidth, Self(PAPER_A4.size().width));
    property!(PageHeight, Self(PAPER_A4.size().height));
    property!(Flipped, Self);
    property!(Strong, Self);
    property!(Emph, Self);
    property!(Monospace, Self);
    property!(FontSize, Self(Length::pt(11.0).into()));
    property!(FontStyle, Self::Normal);
    property!(FontWeight, Self::REGULAR);
    property!(FontStretch, Self::NORMAL);
    property!(TopEdge, Self(VerticalFontMetric::CapHeight));
    property!(BottomEdge, Self(VerticalFontMetric::Baseline));
}
