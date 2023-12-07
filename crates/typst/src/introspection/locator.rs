use std::collections::HashMap;
use std::hash::Hash;

use crate::foundations::StyleChain;
use crate::introspection::{Context, Location};

/// Provides locations for elements in the document.
///
/// A [`Location`] is just a 128-bit hash that uniquely identifies an element in
/// the document.
#[derive(Clone)]
pub struct Locator {
    location: Location,
    disambiguation: HashMap<u128, usize>,
}

impl Locator {
    /// Create a new locator with the given parent location.
    pub fn new(location: Location) -> Self {
        Self { location, disambiguation: HashMap::new() }
    }

    /// Same as `generate_location`, but constructs a context from the given
    /// styles and the resulting location.
    pub fn generate<'a, T: Hash>(
        &mut self,
        styles: StyleChain<'a>,
        key: T,
    ) -> Context<'a> {
        Context {
            styles,
            location: self.generate_location(crate::util::hash128(&key)),
        }
    }

    /// Retrieve a unique location, where disambiguation is assisted by the
    /// given `hash`. The hash doesn't have to be unique among its peers, but if
    /// it is, the generated locations are more stable across multiple
    /// compilations.
    pub fn generate_location<T: Hash>(&mut self, key: T) -> Location {
        let hash = crate::util::hash128(&key);
        let entry = self.disambiguation.entry(hash).or_default();
        let disambiguator = *entry;
        *entry += 1;
        Location(crate::util::hash128(&(&self.location, hash, disambiguator)))
    }
}
