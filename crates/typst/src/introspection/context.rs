use std::hash::Hash;

use crate::foundations::{StyleChain, Styles};
use crate::introspection::Location;

/// The context something is processed in.
///
/// This intentionally does not implement `Copy` because reusing the same
/// context multiple times is typically a mistake. In places, where multiple
/// things need to be layouted, use a `Locator`.
#[derive(Debug, Clone, Hash)]
pub struct Context<'a> {
    /// The current styles.
    pub styles: StyleChain<'a>,
    /// The hierarchical disambiguation.
    pub location: Location,
}

impl<'a> Context<'a> {
    /// Create the initial context at the root of the hierarchy.
    pub fn root(styles: &'a Styles) -> Self {
        Self {
            styles: StyleChain::new(styles),
            location: Location::root(),
        }
    }

    /// Produce a variant of this context.
    pub fn variant(&self, n: usize) -> Self {
        Self {
            styles: self.styles,
            location: self.location.variant(n),
        }
    }
}
