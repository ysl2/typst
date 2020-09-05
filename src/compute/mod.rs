//! Building blocks for the computational part.

pub mod table;

mod convert;
mod scope;
mod value;

pub use convert::TryFromValue;
pub use scope::Scope;
pub use value::*;
