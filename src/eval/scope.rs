//! Mapping from identifiers to functions.

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use super::value::FuncValue;

/// A map from identifiers to functions.
#[derive(Default, Clone)]
pub struct Scope {
    functions: HashMap<String, FuncValue>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self { functions: HashMap::new() }
    }

    /// Return the function with the given name if there is one.
    pub fn get(&self, name: &str) -> Option<&FuncValue> {
        self.functions.get(name)
    }

    /// Associate the given name with the function.
    pub fn insert(&mut self, name: impl Into<String>, function: FuncValue) {
        self.functions.insert(name.into(), function);
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.functions.keys()).finish()
    }
}
